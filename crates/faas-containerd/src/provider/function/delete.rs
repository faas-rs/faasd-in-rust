use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni, task::TaskError};
use crate::provider::ContainerdProvider;
use crate::state::{InstanceState, ResourceSet};
use gateway::types::{DeleteError, Query};

impl ContainerdProvider {
    pub async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();
        log::trace!("Deleting function: {:?}", endpoint);

        let store = self.state_store.clone();
        let ep = endpoint.clone();

        // ── Step 1: Read current record ──────────────────────────
        let current = store
            .get(&ep)
            .map_err(|e| DeleteError::Internal(format!("state store read: {}", e)))?;

        // Absent or no record → idempotent success
        let current = match current {
            None => return Ok(()),
            Some(rec) if rec.state == InstanceState::Absent => return Ok(()),
            Some(rec) => rec,
        };

        // ── Step 2: Transition to CleaningUp (if not already) ────
        let cleaning_up = if current.state != InstanceState::CleaningUp {
            let resources = current.resources;
            let record = current
                .with_state(InstanceState::CleaningUp)
                .with_resources(resources);
            store
                .put(&record)
                .map_err(|e| DeleteError::Internal(format!("state store write: {}", e)))?;
            record
        } else {
            current
        };

        let resources = cleaning_up.resources;
        let mut errors: Vec<String> = Vec::new();

        // ── Step 3: Clean up in reverse creation order ───────────
        // TASK → SNAPSHOT → CONTAINER → NETNS

        // TASK
        if resources.contains(ResourceSet::TASK) {
            match backend().kill_task_with_timeout(&ep).await {
                Ok(()) | Err(TaskError::NotFound) => {
                    // success or already gone
                }
                Err(e) => {
                    log::error!("Failed to kill task {}: {:?}", ep, e);
                    errors.push(format!("kill task: {}", e));
                }
            }
        }

        // SNAPSHOT
        if resources.contains(ResourceSet::SNAPSHOT) {
            if let Err(e) = backend().remove_snapshot(&ep).await {
                log::error!("Failed to remove snapshot {}: {:?}", ep, e);
                errors.push(format!("remove snapshot: {}", e));
            }
        }

        // CONTAINER
        if resources.contains(ResourceSet::CONTAINER) {
            if let Err(e) = backend().delete_container(&ep).await {
                log::error!("Failed to delete container {}: {:?}", ep, e);
                errors.push(format!("delete container: {}", e));
            }
        }

        // NETNS
        if resources.contains(ResourceSet::NETNS) {
            if let Some(cx) = asupersync::Cx::current() {
                if let Err(e) = cni::cni_impl::delete_cni_network(&cx, &ep) {
                    log::error!("Failed to delete CNI network {}: {:?}", ep, e);
                    errors.push(format!("delete CNI network: {}", e));
                }
            }
        }

        // ── Step 4: Remove sled record ───────────────────────────
        if let Err(e) = store.remove(&ep) {
            log::error!("Failed to remove state record for {}: {:?}", ep, e);
            errors.push(format!("remove state record: {}", e));
        }

        // ── Step 5: Report outcome ───────────────────────────────
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DeleteError::Internal(errors.join("; ")))
        }
    }
}
