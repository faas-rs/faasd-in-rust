use crate::impls::{backend, cni, task::TaskError};
use crate::state::{InstanceRecord, InstanceState, StateStore};

/// Drives startup reconciliation and graceful shutdown.
///
/// On startup, verifies all non-Absent records and cleans up any
/// unrecoverable instances.  On shutdown, drains and removes all
/// active instances.
pub struct Reconciler {
    state_store: StateStore,
}

impl Reconciler {
    pub fn new(state_store: StateStore) -> Self {
        Self {
            state_store,
        }
    }

    /// Run startup reconciliation over every non-Absent record.
    ///
    /// - **Active**: checks whether the containerd task still exists.
    ///   If the task is running the instance is kept; otherwise it is
    ///   cleaned up.
    /// - **All other non-Absent states**: conservatively cleaned up.
    pub async fn reconcile_all(&self) {
        let mut recovered: u64 = 0;
        let mut cleaned_up: u64 = 0;

        let records: Vec<InstanceRecord> = self
            .state_store
            .iter_non_absent()
            .filter_map(|r| r.ok())
            .collect();

        for record in records {
            match &record.state {
                InstanceState::Active => {
                    match backend().get_task(&record.endpoint).await {
                        Ok(_) => {
                            log::info!(
                                "Reconciliation: {} is running — keeping Active",
                                record.endpoint
                            );
                            recovered += 1;
                        }
                        Err(TaskError::NotFound) => {
                            log::warn!(
                                "Reconciliation: {} task missing — cleaning up",
                                record.endpoint
                            );
                            self.cleanup_instance(&record).await;
                            cleaned_up += 1;
                        }
                        Err(e) => {
                            log::error!(
                                "Reconciliation: {} task check failed ({:?}) — cleaning up",
                                record.endpoint,
                                e
                            );
                            self.cleanup_instance(&record).await;
                            cleaned_up += 1;
                        }
                    }
                }
                other => {
                    log::info!(
                        "Reconciliation: {} in state {:?} — cleaning up",
                        record.endpoint,
                        other
                    );
                    self.cleanup_instance(&record).await;
                    cleaned_up += 1;
                }
            }
        }

        log::info!(
            "Reconciliation: {} recovered, {} cleaned up",
            recovered,
            cleaned_up
        );
    }

    /// Graceful shutdown: transition every active instance through
    /// Draining → Stopping → CleaningUp, then remove resources and
    /// the record.
    pub async fn shutdown_all(&self) {
        let records: Vec<InstanceRecord> = self
            .state_store
            .iter_non_absent()
            .filter_map(|r| r.ok())
            .filter(|r| r.state.is_active())
            .collect();

        let count = records.len();
        log::info!("Shutting down {} active instances", count);

        for record in records {
            let draining = record.clone().with_state(InstanceState::Draining);
            let _ = self.state_store.put(&draining);

            let stopping = draining.with_state(InstanceState::Stopping);
            let _ = self.state_store.put(&stopping);

            self.cleanup_instance(&stopping).await;
        }

        log::info!("Shutdown complete: {} instances cleaned up", count);
    }

    /// Remove all resources for an instance and delete its record.
    ///
    /// Every step tolerates "not found" errors so the procedure is
    /// idempotent and safe to call on partially-deployed instances.
    async fn cleanup_instance(&self, record: &InstanceRecord) {
        let endpoint = &record.endpoint;

        // Persist CleaningUp before mutating the world.
        let cleaning = record.clone().with_state(InstanceState::CleaningUp);
        if let Err(e) = self.state_store.put(&cleaning) {
            log::error!("Failed to persist CleaningUp for {}: {:?}", endpoint, e);
        }

        // Kill task (NotFound is expected for partial deploys).
        match backend().kill_task_with_timeout(endpoint).await {
            Ok(_) | Err(TaskError::NotFound) => {}
            Err(e) => log::error!("Error killing task for {}: {:?}", endpoint, e),
        }

        // Delete container.
        if let Err(e) = backend().delete_container(endpoint).await {
            log::error!("Error deleting container for {}: {:?}", endpoint, e);
        }

        // Remove snapshot.
        if let Err(e) = backend().remove_snapshot(endpoint).await {
            log::error!("Error removing snapshot for {}: {:?}", endpoint, e);
        }

        // Delete CNI network (netns + bridge).
        if let Some(cx) = asupersync::Cx::current() {
            if let Err(e) = cni::cni_impl::delete_cni_network(&cx, endpoint) {
                log::error!("Error deleting CNI network for {}: {:?}", endpoint, e);
            }
        }

        // Remove the sled record (transition to Absent).
        if let Err(e) = self.state_store.remove(endpoint) {
            log::error!("Failed to remove record for {}: {:?}", endpoint, e);
        }
    }
}
