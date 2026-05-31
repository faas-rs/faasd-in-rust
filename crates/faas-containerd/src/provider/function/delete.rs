use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni};
use crate::provider::ContainerdProvider;
use gateway::types::{DeleteError, Query};

impl ContainerdProvider {
    /// Containerd-driven delete: queries containerd for each resource
    /// type and deletes what exists.  Every step tolerates NotFound.
    /// Idempotent — safe to call on partially-deployed instances.
    pub async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();
        log::info!("Deleting function: {:?}", endpoint);

        let mut errors: Vec<String> = Vec::new();

        // ── Task ────────────────────────────────────────────────────
        if backend().task_exists(&endpoint).await {
            match backend().kill_task_with_timeout(&endpoint).await {
                Ok(()) | Err(crate::impls::task::TaskError::NotFound) => {
                    log::trace!("Task removed for {}", endpoint);
                }
                Err(e) => {
                    log::error!("Failed to kill task {}: {:?}", endpoint, e);
                    errors.push(format!("kill task: {}", e));
                }
            }
        }

        // ── Snapshot ─────────────────────────────────────────────────
        if backend().snapshot_exists(&endpoint).await {
            if let Err(e) = backend().remove_snapshot(&endpoint).await {
                log::error!("Failed to remove snapshot {}: {:?}", endpoint, e);
                errors.push(format!("remove snapshot: {}", e));
            } else {
                log::trace!("Snapshot removed for {}", endpoint);
            }
        }

        // ── Container ────────────────────────────────────────────────
        if backend().container_exists(&endpoint).await {
            if let Err(e) = backend().delete_container(&endpoint).await {
                log::error!("Failed to delete container {}: {:?}", endpoint, e);
                errors.push(format!("delete container: {}", e));
            } else {
                log::trace!("Container deleted for {}", endpoint);
            }
        }

        // ── CNI network ──────────────────────────────────────────────
        if let Some(cx) = asupersync::Cx::current() {
            if let Err(e) = cni::cni_impl::delete_cni_network(&cx, &endpoint) {
                log::error!("Failed to delete CNI network {}: {:?}", endpoint, e);
                errors.push(format!("delete cni: {}", e));
            } else {
                log::trace!("CNI network removed for {}", endpoint);
            }
        }

        // ── Clear IP cache ───────────────────────────────────────────
        if let Err(e) = self.cache.remove(&endpoint) {
            log::error!("Failed to clear cache for {}: {:?}", endpoint, e);
            errors.push(format!("cache remove: {}", e));
        }

        if errors.is_empty() {
            log::info!("Function {} deleted successfully", endpoint);
            Ok(())
        } else {
            Err(DeleteError::Internal(errors.join("; ")))
        }
    }
}
