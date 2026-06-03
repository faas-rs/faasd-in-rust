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
  │                       │                         ├─ Bracket release phase:
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
      │                          │                         │  (direct containerd calls,
      │                          │                         │   no sled CAS for delete path)
      │                          │                         │  kill / remove / delete — all
      │                          │                         │  idempotent.  NotFound → skip.
      │                          │                         ├─ cache.remove() → Absent
      │                          │                         ├─ return Ok(())
      │                          │                         │
      ├─ ... continue deploy ... │                         │
      ├─ commit_deploy()         │                         │
      │  CAS InFlight→Cached ───►│                         │
      │  Err (key is Absent!)    │                         │
      │  → Err branch runs       │                         │
      │    release_deploy (nop)  │                         │
      │    cleanup (nop)         │                         │
```

**Race result**: Delete wins.  Deploy's commit fails (key gone) → Err branch
runs idempotent cleanup → consistent Absent state in sled.

---

## 3. Ctrl-C (SIGINT) During Deploy — Graceful Shutdown

main.rs registers `asupersync::signal::ctrl_c().await`.  The gateway runs as a
background `handle.spawn()`, and block_on waits for the shutdown signal.

```
      ├─ deploy() in progress: InFlight acquired
      ├─ do_deploy_impl() somewhere in steps 1-5
      │
      ├─ SIGINT received
      ├─ ctrl_c() future resolves
      ├─ block_on returns → Runtime drops → close() called
      │
      │  close():
      │    ├─ begin_drain() on all regions
      │    ├─ cancel all Cx
      │    ├─ worker threads poll tasks to completion
      │    │   ┌─ Current gRPC step completes (or 30s timeout)
      │    │   ├─ next cx.checkpoint() → Cancelled
      │    │   ├─ do_deploy_impl() returns Err(Cancelled)
      │    │   ├─ Bracket release phase (committed=false):
      │    │   │    release_deploy() → Absent
      │    │   │    cleanup_containerd_resources()
      │    │   │      each step: 10s timeout, Dirty on timeout
      │    │   └─ return
      │    ├─ begin_finalize()
      │    └─ join worker threads
      │
      ▼
      sled: Absent (or Dirty if cleanup timed out)
      containerd: clean (or partial if Dirty)
```

**Result**: Graceful drain ensures deploy cleanup runs.  InFlight → Absent.
No stale lock.  Worst case: cleanup step times out → Dirty → startup recovery
handles it.

---

## 4. Ctrl-C (SIGINT) During Delete — Graceful Shutdown

Same drain mechanism.  Delete has no CAS lock, so no stale lock risk
regardless.  Partial cleanup during drain → startup recovery handles
Dirty records if any step timed out.

```
      ├─ delete() / cleanup_containerd_resources() in progress
      ├─ SIGINT → ctrl_c → close() → drain
      │
      │  ┌─ Current step completes or times out (10s each)
      │  ├─ Remaining steps run during drain
      │  └─ cache.remove() runs if not Dirty
      │
      ▼
      sled: Absent (or Dirty if step timed out)
      containerd: clean (or partial if Dirty)
```

---

## 5. Panic During Deploy

```
      ├─ do_deploy_impl() panics or Cx panicked
      ├─ Bracket poll catches panic via catch_unwind
      ├─ transitions to Releasing phase
      ├─ committed == false → release runs:
      │    release_deploy() → Absent
      │    cleanup_containerd_resources()
      │
      ▼
      sled: Absent (or Dirty if cleanup timed out)
      containerd: clean (or partial if Dirty)

Panic is no longer a stale-lock hazard.  The Bracket combinator catches
panics in both use and release phases, and ALWAYS transitions to the
release function before propagating the panic payload.
```

## 6. Panic During Bracket Release Phase

```
      ├─ Bracket Releasing phase runs cleanup_containerd_resources()
      ├─ kill_task() panics
      ├─ Bracket catch_unwind in release poll catches it
      ├─ phase → Done
      ├─ panic payload propagated
      ├─ task boundary catches it (PanicIsolationConfig)
      │
      ▼
      Partial cleanup.  Containerd resources may remain.
      Startup recovery handles Dirty records from per-step timeouts.
```
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

**If deploy fails after delete succeeds**: Update returns 500.  Sled is Absent,
containerd clean.  Function is gone — user must retry with fresh deploy.

**If delete leaves Dirty**: Update's deploy sees key exists (TAG_DIRTY) →
try_acquire_deploy → Conflict.  Must manually clean up the Dirty record first.

---

## Bracket Combinator Design

asupersync's `bracket(acquire, use_fn, release)` combinator replaces the
hand-rolled `match result { Err(_) => cleanup() }` pattern:

```
Bracket::new(acquire, use_fn, release)
  ├─ acquire: Future<Res> → try_acquire_deploy (CAS None→InFlight)
  ├─ use:     Res → Future<T> → do_deploy_impl + commit_deploy on success
  └─ release: Res → Future<()> → check committed flag:
       ├─ committed → no-op (deploy succeeded)
       └─ !committed → release_deploy + cleanup_containerd_resources
```

**Guarantees** (enforced by the `Bracket` future's `poll` and `Drop`):

| Scenario | Release runs? | Mechanism |
|---|---|---|
| Normal success | Yes | Normal poll: Acquiring→Using→Releasing→Done |
| Normal failure | Yes | Use returns Err → Releasing phase runs normally |
| Cancel (Cx cancelled) | Yes | Next checkpoint in use → Cancelled → Releasing |
| Panic in use | Yes | `catch_unwind` in poll → Releasing phase |
| Panic in release | Partial | `catch_unwind` catches, propagates, does not block |
| Future dropped (force) | Best-effort | `Drop` impl: 10,000 poll budget with noop waker |

**Why not hand-rolled Err branch cleanup?**

1. Cancel skips the Err branch (Cx cancelled → error propagates to caller,
   but the `match` that calls cleanup() is skipped because the task is done)
2. Panic skips the Err branch entirely
3. Bracket makes cleanup structural: the runtime guarantees release runs,
   not a conditional branch the programmer might forget

**Drain vs. Drop**: During graceful shutdown (drain), the Bracket's release
phase runs through the normal poll path with proper wakers — network calls
complete, timeouts fire. The Drop path (noop-waker bounded loop) is only
reached if the future is force-dropped, which our architecture avoids via
drain-then-exit.
---

## Summary Table

| Event | sled after | containerd after | Future ops blocked? |
|---|---|---|---|
| Deploy success | Cached(ip) | running | No |
| Delete success | Absent | clean | No |
| Client disconnect | Absent | cleaned up | No |
| Delete during deploy | Absent (delete wins) | cleaned up | No |
| gRPC timeout → cleanup timeout | Dirty(reason) | partial | Yes (Dirty blocks deploy) |
| SIGINT during deploy | Absent (drain) | cleaned up | No |
| SIGINT during delete | Absent or Dirty | clean or partial | No (or Dirty blocks deploy) |
| Panic during deploy | Absent (Bracket catches) | cleaned up | No |
| Panic during deploy release | Absent or Dirty | partial | No (or Dirty blocks) |
| Panic during delete | Cached or Dirty | partial | No |
| Update: delete OK, deploy fails | Absent | clean | No (function gone) |
| Update: delete → Dirty | Dirty | partial | **Yes** (Dirty blocks deploy) |

Bracket combinator catches panics in use/release phases; release always runs.
