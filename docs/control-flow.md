# Control Flow: containerd-as-truth

## Architecture

```
HTTP request → asupersync Http1Listener → dispatch() → Provider trait → ContainerdProvider
                                                                          ├─ sled CacheStore (CAS)
                                                                          └─ containerd gRPC (tonic_bridge)
```

## Concurrency Model

Single design: `ContainerdProvider` is `Arc`-shared, one per process.  Containerd
is the authoritative state machine.  sled provides **mutual exclusion** via
CAS compare-and-swap, not via Rust locks.  All gRPC calls are wrapped in
`asupersync::time::timeout(wall_now(), 30s, …)` — no containerd call can block
a worker thread indefinitely.

The `CacheStore` wire format:

| Key state | Meaning |
|---|---|
| Absent | No function, no operation in flight |
| `[0]` — InFlight | Deploy in progress (CAS lock) |
| `[1, json…]` — Cached(ip) | Deploy completed |
| `[2, json…]` — Dirty(reason, at, attempts) | Cleanup timed out |

---

## 1. Client Disconnect During Deploy

```
Client                asupersync              faas-containerd
  │                       │                         │
  ├─ POST /deploy ───────►│                         │
  │                       ├─ Cx::spawn ────────────►│ deploy() acquires InFlight
  │                       │                         ├─ cx.checkpoint()
  │                       │                         ├─ pull_image() [timeout 30s]
  │                       │                         ├─ cx.checkpoint()
  │   ◄── TCP RST ────────┤                         ├─ create_container() [30s]
  │                       │                         │  ...
  │                       ├─ cancel Cx ────────────►│
  │                       │                         ├─ next cx.checkpoint() → Cancelled
  │                       │                         ├─ deploy() Err branch:
  │                       │                         │    release_deploy() → Absent
  │                       │                         │    cleanup_containerd_resources()
  │                       │                         │      kill_task     [timeout 10s]
  │                       │                         │      remove_snap   [timeout 10s]
  │                       │                         │      delete_ctr    [timeout 10s]
  │                       │                         │      delete_cni    [sync, ~100ms]
  │                       │                         ├─ return Cancelled
  │                       ├─ HTTP 500 ─────────────►│
```

**Key property**: Cancellation is detected only at `cx.checkpoint()` boundaries, not
mid-step.  If the client disconnects during `pull_image()`, the pull runs to
completion (or 30s timeout) before the next checkpoint detects cancellation.
Steps are individually time-bounded (30s), so the worst-case latency from
client disconnect to resource release is ≤30s.

If any cleanup step times out (10s), the endpoint is marked **Dirty** and the
next step proceeds — a hung container won't block deletion of the CNI network.

---

## 2. Delete Arriving During Deploy

```
Thread A (deploy)               sled                 Thread B (delete)
      │                          │                         │
      ├─ try_acquire_deploy()    │                         │
      │  CAS None→InFlight ─────►│                         │
      │  returns Ok(true)        │                         │
      │                          │                         ├─ delete()
      │                          │                         ├─ cleanup_containerd_resources()
      │                          │                         │  (runs directly on containerd,
      │                          │                         │   no sled CAS for delete path)
      │                          │                         │  kill / remove / delete — all
      │                          │                         │  idempotent.  If task doesn't
      │                          │                         │  exist (deploy hasn't created
      │                          │                         │  it yet), NotFound → skip.
      │                          │                         ├─ cache.remove() → deletes
      │                          │                         │  InFlight key
      │                          │                         ├─ return Ok(())
      │                          │                         │
      ├─ ... continue deploy ... │                         │
      ├─ commit_deploy()         │                         │
      │  CAS InFlight→Cached ───►│                         │
      │  Err (key is Absent!)    │                         │
      │  → internal error,       │                         │
      │    cleanup triggers      │                         │
      │                          │                         │
```

**Race result**: Deploy's commit fails (sled key gone).  Deploy falls into
Err branch, calls release_deploy (idempotent) + cleanup_containerd_resources
(also idempotent — all resources already deleted).  The function ends up
Absent in sled and deleted in containerd.  **Consistent outcome**: the delete
wins, and deploy tears itself down cleanly.

**Follow-up deploy**: If a third deploy arrives, it sees sled Absent →
try_acquire_deploy succeeds → fresh deploy.

---

## 3. Ctrl-C (SIGINT) During Deploy

```
faasd process
      │
      ├─ deploy() in progress
      │  sled: InFlight
      │  containerd: image pulled, container created, task running
      │
      ├─ SIGINT received
      ├─ asupersync runtime drains workers, drops all in-flight tasks
      │
      ├─ PROCESS EXITS
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled state:   InFlight (stale)              │
   │ containerd:   image/container/task exist    │
   │ CNI:          netns + IP allocated          │
   └─────────────────────────────────────────────┘

   On next startup:
     1. Startup scan checks only Dirty records — InFlight is NOT Dirty.
     2. Next deploy for same function: try_acquire_deploy fails (InFlight exists) → Conflict 409.
     3. Manual intervention required: delete the function first.
```

**Known limitation**: The startup recovery only handles Dirty records.  Stale
InFlight keys from a crash-before-commit are not automatically reclaimed.
This is a design trade-off: InFlight keys prevent concurrent operations correctly,
but a process crash leaks them.

**Mitigation**: An operator tool could scan for InFlight keys older than N
minutes and release them.  Or the startup scan could check InFlight age vs.
process uptime and auto-release stale ones.  Not yet implemented.

**Note on containerd**: The leftover container/task/image in containerd are
NOT leaked — they're identified by the `faasdrs-{ns}-{fn}` prefix.  A
subsequent delete call can find and remove them regardless of sled state.

---

## 4. Ctrl-C (SIGINT) During Delete

```
      ├─ delete() in progress
      │  sled: Cached(ip) or Dirty(reason)
      │  containerd: partial deletion (some resources removed, some still exist)
      │
      ├─ SIGINT → process exits
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled state:   Cached(ip) or Dirty (stale)   │
   │ containerd:   partial state (some exist)    │
   └─────────────────────────────────────────────┘

   On next startup:
     1. If sled marks Dirty → startup recovery runs cleanup_containerd_resources
        (idempotent, each step tolerates NotFound).  Partial deletion continues.
     2. If sled marks Cached but resources still exist → no recovery.  A new
        delete call will clean up.
```

**No lock blocking**: Delete doesn't use sled CAS.  No stale lock prevents
future operations.

---

## 5. Panic During Deploy

```
      ├─ do_deploy_impl() in progress
      │  sled: InFlight
      │  containerd: partial deployment state
      │
      ├─ panic!("...")
      ├─ asupersync runtime catches panic at task boundary
      ├─ future is DROPPED (no Err branch executed)
      │
      ├─ deploy() outer function also dropped — cleanup code never runs
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled state:   InFlight (stale)              │
   │ containerd:   partial state                 │
   │ CNI:          possibly leaked netns/IP      │
   └─────────────────────────────────────────────┘

   Same as SIGINT case above.  Stale InFlight blocks future deploys.
```

**Mitigation**: std::panic::catch_unwind around do_deploy_impl would catch
the panic and run the cleanup path.  Not yet implemented — async + catch_unwind
require `AssertUnwindSafe` and careful setup.

---

## 6. Normal Deploy Success

```
      ├─ try_acquire_deploy → InFlight
      ├─ do_deploy_impl():
      │    cx.checkpoint() ✓
      │    pull_image          [30s timeout]
      │    cx.checkpoint() ✓
      │    create_container    [30s timeout]
      │    cx.checkpoint() ✓
      │    create_cni_network  [sync, ~100ms]
      │    cx.checkpoint() ✓
      │    prepare_snapshot    [30s timeout]
      │    cx.checkpoint() ✓
      │    new_task            [30s timeout]
      │    cx.checkpoint() ✓
      ├─ commit_deploy: InFlight → Cached(ip)
      ├─ return Ok(())
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled:        Cached(ip)                     │
   │ containerd:  container + task running       │
   │ CNI:         netns + IP active              │
   │ gateway:     HTTP 202                       │
   └─────────────────────────────────────────────┘
```

## 7. Normal Delete Success

```
      ├─ cleanup_containerd_resources():
      │    kill_task           [10s timeout]
      │    remove_snapshot     [10s timeout]
      │    delete_container    [10s timeout]
      │    delete_cni_network  [sync, ~100ms]
      ├─ cache.remove() → Absent
      ├─ return Ok(())
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled:        Absent                         │
   │ containerd:  no faasdrs-* resources         │
   │ CNI:         netns removed, IP released     │
   │ gateway:     HTTP 202                       │
   └─────────────────────────────────────────────┘
```

## 8. Update (Delete + Deploy)

```
      ├─ delete(existing function):
      │    cleanup_containerd_resources()
      │    cache.remove()
      ├─ deploy(new config):
      │    try_acquire_deploy → InFlight
      │    do_deploy_impl() → Ok
      │    commit_deploy → Cached(new_ip)
      │
      ▼
   ┌─────────────────────────────────────────────┐
   │ sled:        Cached(new_ip)                 │
   │ containerd:  new container + task running   │
   │ gateway:     HTTP 202                       │
   └─────────────────────────────────────────────┘
```

**If deploy fails after delete succeeds**: Update returns 500.  Sled is Absent
(delete cleared it).  Containerd resources are absent (delete cleared them).
The function is gone — user must retry with a fresh deploy.

**If delete times out**: Steps get marked Dirty.  Deploy's try_acquire_deploy
may see InFlight (stale from prior delete?  No — delete doesn't use CAS lock).
Actually, delete doesn't use CAS at all.  So if delete produces Dirty records,
deploy proceeds independently.  The Dirty record for the old deployment
coexists with a fresh Cached record for the new deployment — no conflict,
different sled keys (Dirty uses TAG_DIRTY, Cached uses TAG_CACHED).

Wait — they share the same key name.  If delete's cleanup timed out, the
sled key was marked Dirty (TAG_DIRTY).  Then update calls delete (skips
cache.remove because is_dirty), then calls deploy: try_acquire_deploy sees
the key exists (TAG_DIRTY) → Conflict.

**Gap**: Update fails if delete left a Dirty record.  The operator must
manually clean up.

---

## Summary Table

| Event | sled state after | containerd after | Future ops blocked? |
|---|---|---|---|
| Deploy success | Cached(ip) | running | No |
| Delete success | Absent | clean | No |
| Client disconnect during deploy | Absent (released) | cleaned up | No |
| Delete during deploy (race) | Absent (delete wins) | cleaned up | No |
| Deploy → gRPC timeout → cleanup timeout | Dirty(reason) | partial, marked Dirty | Yes (InFlight → cleanup tried; Dirty present) |
| SIGINT during deploy | InFlight (stale) ⚠️ | partial state ⚠️ | **Yes — InFlight blocks future deploys** |
| SIGINT during delete | Cached or Dirty | partial state | No (delete has no CAS lock) |
| Panic during deploy | InFlight (stale) ⚠️ | partial state ⚠️ | **Yes — InFlight blocks future deploys** |
| Panic during delete | Cached or Dirty | partial state | No |
| Update: delete OK, deploy fails | Absent | clean | No (function is gone) |
| Update: delete → Dirty, deploy fails | Dirty | partial | **Yes (Dirty key blocks deploy)** |

⚠️ = known gap: stale InFlight keys from crashes are not auto-recovered.
