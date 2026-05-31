use gateway::types::{ListError, Status};

use crate::{
    impls::{backend, cni::Endpoint, task::TaskError},
    provider::ContainerdProvider,
};

/// Parse a containerd container ID like "faasdrs-default-hello" back into
/// an Endpoint. Returns None if the ID doesn't have the faasdrs- prefix.
fn parse_faasd_id(id: &str, namespace: &str) -> Option<Endpoint> {
    let remainder = id.strip_prefix("faasdrs-")?;
    // remainder is "{namespace}-{function_name}"
    // Split at first '-' to get namespace prefix, rest is function_name
    let dash_pos = remainder.find('-')?;
    let ns = &remainder[..dash_pos];
    let fn_name = &remainder[dash_pos + 1..];
    if ns == namespace {
        Some(Endpoint::new(fn_name, namespace))
    } else {
        None
    }
}

impl ContainerdProvider {
    pub async fn list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
        let containers = backend().list_container(&namespace).await.map_err(|e| {
            log::error!(
                "failed to get container list for namespace {} because {:?}",
                namespace,
                e
            );
            ListError::Internal(e.to_string())
        })?;
        let mut statuses: Vec<Status> = Vec::new();
        for container in containers {
            // Only process faasd-managed containers
            let endpoint = match parse_faasd_id(&container.id, &namespace) {
                Some(ep) => ep,
                None => continue,
            };

            let created_at = container.created_at.unwrap().to_string();
            let mut replicas = 0;

            match backend().get_task(&endpoint).await {
                Ok(task) => {
                    let status = task.status;
                    if status == 2 || status == 3 {
                        replicas = 1;
                    }
                }
                Err(TaskError::NotFound) => {
                    log::info!(
                        "task not found for endpoint {:?}, treating replicas=0",
                        endpoint
                    );
                    replicas = 0;
                }
                Err(e) => {
                    log::warn!(
                        "failed to get task for function {:?} because {:?}",
                        endpoint,
                        e
                    );
                }
            }

            let status = Status {
                function_name: endpoint.function_name,
                namespace: Some(endpoint.namespace),
                image: container.image,
                env_process: None,
                env_vars: None,
                constraints: None,
                secrets: None,
                labels: None,
                annotations: None,
                limits: None,
                requests: None,
                read_only_root_filesystem: false,
                invocation_count: None,
                replicas: Some(replicas),
                available_replicas: Some(replicas),
                created_at: Some(created_at),
                usage: None,
            };
            statuses.push(status);
        }

        Ok(statuses)
    }
}
