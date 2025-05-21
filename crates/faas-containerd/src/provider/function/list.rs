use gateway::{handlers::function::ListError, types::function::Status};

use crate::{
    impls::{backend, cni::Endpoint, task::TaskError},
    provider::ContainerdProvider,
};

impl ContainerdProvider {
    pub(crate) async fn _list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
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
            let endpoint = Endpoint {
                service: container.id.clone(),
                namespace: namespace.clone(),
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
                Err(TaskError::NotFound) => continue,
                Err(e) => {
                    log::warn!(
                        "failed to get task for function {:?} because {:?}",
                        &endpoint,
                        e
                    );
                }
            }

            // 大部分字段并未实现，使用None填充
            let status = Status {
                name: endpoint.service,
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
