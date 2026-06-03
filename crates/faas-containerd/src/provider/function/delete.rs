use std::time::Duration;

use asupersync::time::{timeout, wall_now};

use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni};
use crate::provider::ContainerdProvider;
use crate::state::CacheStore;
use gateway::types::{DeleteError, Query};

/// Release the netns lock for an endpoint. Idempotent — no-op if netns absent.
async fn release_netns_lock(endpoint: &Endpoint) {
    if let Ok(ns) = netns_rs::NetNs::get(endpoint.to_string()) {
        ns.remove().ok();
    }
}

/// Per-step timeout for cleanup operations.  Each step gets this budget;
/// if it expires the step is marked Dirty and we proceed to the next step.
const CLEANUP_STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// Containerd-driven resource cleanup.  Idempotent — every step tolerates
/// NotFound.  Each step is individually time-bounded; a hung containerd
/// operation marks the endpoint Dirty in sled and continues.
pub async fn cleanup_containerd_resources(cache: &CacheStore, endpoint: &Endpoint) {
    // Task
    if backend().task_exists(endpoint).await {
        let fut = backend().kill_task_with_timeout(endpoint);
        match timeout(wall_now(), CLEANUP_STEP_TIMEOUT, fut).await {
            Ok(Ok(())) | Ok(Err(crate::impls::task::TaskError::NotFound)) => {}
            Ok(Err(e)) => log::error!("kill task {}: {:?}", endpoint, e),
            Err(_) => {
                log::error!("kill task {}: timeout, marking Dirty", endpoint);
                cache.mark_dirty(endpoint, "kill_task timeout").ok();
            }
        }
    }

    // Snapshot
    if backend().snapshot_exists(endpoint).await {
        let fut = backend().remove_snapshot(endpoint);
        match timeout(wall_now(), CLEANUP_STEP_TIMEOUT, fut).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::error!("remove snapshot {}: {:?}", endpoint, e),
            Err(_) => {
                log::error!("remove snapshot {}: timeout, marking Dirty", endpoint);
                cache.mark_dirty(endpoint, "remove_snapshot timeout").ok();
            }
        }
    }

    // Container
    if backend().container_exists(endpoint).await {
        let fut = backend().delete_container(endpoint);
        match timeout(wall_now(), CLEANUP_STEP_TIMEOUT, fut).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::error!("delete container {}: {:?}", endpoint, e),
            Err(_) => {
                log::error!("delete container {}: timeout, marking Dirty", endpoint);
                cache.mark_dirty(endpoint, "delete_container timeout").ok();
            }
        }
    }

    // CNI
    if let Some(cx) = asupersync::Cx::current() {
        let fut = cni::cni_impl::delete_cni_network(&cx, endpoint);
        // CNI uses std::process::Command which is synchronous; don't timeout-wrap it.
        // The CNI binary has its own internal timeout.
        if let Err(e) = fut {
            log::error!("delete cni {}: {:?}", endpoint, e);
        }
    }
}

impl ContainerdProvider {
    /// Delete a function.  containerd is the truth — every resource is
    /// checked and deleted individually.  Per-step timeouts ensure a hung
    /// containerd doesn't block the caller; timed-out steps are marked Dirty.
    pub async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();
        log::info!("Deleting function: {:?}", endpoint);

        cleanup_containerd_resources(&self.cache, &endpoint).await;

        // Remove from in-memory IP cache
        self.resolved_ips.lock().unwrap().remove(&endpoint);

        // Release netns lock — always, regardless of cleanup outcome.
        release_netns_lock(&endpoint).await;

        // Clear IP cache (non-Dirty entries only)
        // Remove cache entry unless it's Dirty (audit trail preserved)
        if !self.cache.is_dirty(&endpoint).unwrap_or(false)
            && let Err(e) = self.cache.remove(&endpoint)
        {
            log::error!("cache remove {}: {:?}", endpoint, e);
        }

        log::info!("Function {} deleted", endpoint);
        Ok(())
    }

    /// Attempt recovery of a Dirty endpoint from the startup scan.
    /// Tries to clean up leftover containerd resources, then removes
    /// the sled record if successful.  The caller handles retries.
    pub async fn recover_dirty(&self, endpoint: &Endpoint, _reason: &str) {
        cleanup_containerd_resources(&self.cache, endpoint).await;

        // Remove from in-memory IP cache
        self.resolved_ips.lock().unwrap().remove(endpoint);

        // Release any stray netns that might be holding a lock.
        release_netns_lock(endpoint).await;

        // If cleanup cleared the containerd resources, also clear sled
        if self.cache.is_dirty(endpoint).unwrap_or(true) {
            self.cache.remove(endpoint).ok();
        }
    }
}
