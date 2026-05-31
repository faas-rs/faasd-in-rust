use crate::provider::ContainerdProvider;
use bollard::container::InspectContainerOptions;
use gateway::types::{Query, ResolveError, Status};

impl ContainerdProvider {
    pub async fn function_status(&self, q: Query) -> Result<Status, ResolveError> {
        let ns = q.namespace.as_deref().unwrap_or("openfaas-fn");
        let name = format!("faasdrs-{}-{}", ns, q.function_name);

        let info = self
            .docker
            .inspect_container(&name, None::<InspectContainerOptions>)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("No such container") || msg.contains("not found") {
                    ResolveError::NotFound(msg)
                } else {
                    ResolveError::Internal(msg)
                }
            })?;

        let state = info.state.as_ref();
        let running = state.and_then(|s| s.running).unwrap_or(false);

        Ok(Status {
            name: q.function_name,
            namespace: ns.to_string(),
            image: info
                .config
                .as_ref()
                .and_then(|c| c.image.clone())
                .unwrap_or_default(),
            available_replicas: if running { 1 } else { 0 },
            created_at: info.created.map(|t| t.to_string()).unwrap_or_default(),
            status: format!(
                "{:?}",
                state
                    .and_then(|s| s.status.as_ref())
                    .unwrap_or(&bollard::models::ContainerStateStatusEnum::EMPTY)
            ),
            ..Default::default()
        })
    }
}
