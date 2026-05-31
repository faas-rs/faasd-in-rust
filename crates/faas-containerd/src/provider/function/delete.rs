use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni};
use crate::provider::ContainerdProvider;
use gateway::types::{DeleteError, Query};

/// Containerd-driven resource cleanup.  Idempotent — every step tolerates NotFound.
/// Called by both `delete()` and `deploy()`'s cancel/error path.
pub async fn delete_containerd_resources(endpoint: &Endpoint) {
    // Task
    if backend().task_exists(endpoint).await {
        match backend().kill_task_with_timeout(endpoint).await {
            Ok(()) | Err(crate::impls::task::TaskError::NotFound) => {}
            Err(e) => log::error!("kill task {}: {:?}", endpoint, e),
        }
    }

    // Snapshot
    if backend().snapshot_exists(endpoint).await
        && let Err(e) = backend().remove_snapshot(endpoint).await
    {
        log::error!("remove snapshot {}: {:?}", endpoint, e);
    }

    // Container
    if backend().container_exists(endpoint).await
        && let Err(e) = backend().delete_container(endpoint).await
    {
        log::error!("delete container {}: {:?}", endpoint, e);
    }

    // CNI
    if let Some(cx) = asupersync::Cx::current()
        && let Err(e) = cni::cni_impl::delete_cni_network(&cx, endpoint)
    {
        log::error!("delete cni {}: {:?}", endpoint, e);
    }
}

impl ContainerdProvider {
    /// Delete a function.  containerd is the truth — every resource is
    /// checked and deleted individually.  sled record is cleared at the end.
    pub async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();
        log::info!("Deleting function: {:?}", endpoint);

        let mut errors: Vec<String> = Vec::new();

        delete_containerd_resources(&endpoint).await;

        // Clear IP cache
        if let Err(e) = self.cache.remove(&endpoint) {
            log::error!("cache remove {}: {:?}", endpoint, e);
            errors.push(format!("cache remove: {}", e));
        }

        if errors.is_empty() {
            log::info!("Function {} deleted", endpoint);
            Ok(())
        } else {
            Err(DeleteError::Internal(errors.join("; ")))
        }
    }
}
