use gateway::{
    handlers::function::ResolveError,
    types::function::{Query, Status},
};

use crate::{
    impls::{backend, container::ContainerError},
    provider::ContainerdProvider,
};

impl ContainerdProvider {
    pub(crate) async fn _status(&self, function: Query) -> Result<Status, ResolveError> {
        let container = backend().load_container(&function).await.map_err(|e| {
            log::error!(
                "failed to load container for function {:?} because {:?}",
                function,
                e
            );
            match e {
                ContainerError::NotFound => ResolveError::NotFound(e.to_string()),
                ContainerError::Internal => ResolveError::Internal(e.to_string()),
                _ => ResolveError::Invalid(e.to_string()),
            }
        })?;

        let created_at = container.created_at.unwrap().to_string();
        let mut replicas = 0;

        let namespace = function.namespace.unwrap();
        let service = function.service;

        match backend().get_task(&service, &namespace).await {
            Ok(task) => {
                let status = task.status;
                if status == 2 || status == 3 {
                    replicas = 1;
                }
            }
            Err(e) => {
                log::warn!(
                    "failed to get task for function {:?} because {:?}",
                    &service,
                    e
                );
            }
        }

        // 大部分字段并未实现，使用None填充
        let status = Status {
            name: container.id,
            namespace: Some(namespace),
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

        Ok(status)
    }
}
