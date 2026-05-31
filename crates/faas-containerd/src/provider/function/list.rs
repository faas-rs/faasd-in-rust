use crate::provider::ContainerdProvider;
use bollard::container::ListContainersOptions;
use gateway::types::{ListError, Status};

impl ContainerdProvider {
    pub async fn function_list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .map_err(|e| ListError::Internal(e.to_string()))?;

        let mut out = Vec::new();
        for c in containers {
            let name = c
                .names
                .as_ref()
                .and_then(|ns| ns.first())
                .cloned()
                .unwrap_or_default();
            let name = name.strip_prefix('/').unwrap_or(&name);
            if !name.starts_with("faasdrs-") {
                continue;
            }

            let remainder = &name["faasdrs-".len()..];
            if let Some(dash) = remainder.find('-') {
                let ns = &remainder[..dash];
                if ns != namespace {
                    continue;
                }
                let fn_name = &remainder[dash + 1..];
                let running = c.state.as_deref() == Some("running");
                out.push(Status {
                    name: fn_name.to_string(),
                    namespace: ns.to_string(),
                    image: c.image.unwrap_or_default(),
                    available_replicas: if running { 1 } else { 0 },
                    created_at: c.created.unwrap_or_default().to_string(),
                    status: c.status.unwrap_or_default(),
                    ..Default::default()
                });
            }
        }
        Ok(out)
    }
}
