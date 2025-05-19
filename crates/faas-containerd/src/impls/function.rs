use std::net::IpAddr;

use gateway::types::function;

use crate::{consts, impls::BACKEND};

use super::{cni::CNIEndpoint, error::ContainerdError};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ContainerStaticMetadata {
    pub image: String,
    pub container_id: String,
    pub namespace: String,
}

impl From<function::Deployment> for ContainerStaticMetadata {
    fn from(info: function::Deployment) -> Self {
        ContainerStaticMetadata {
            container_id: info.service,
            image: info.image,
            namespace: info
                .namespace
                .unwrap_or(consts::DEFAULT_FUNCTION_NAMESPACE.to_string()),
        }
    }
}

impl From<ContainerStaticMetadata> for function::Query {
    fn from(metadata: ContainerStaticMetadata) -> Self {
        function::Query {
            service: metadata.container_id,
            namespace: if metadata.namespace == consts::DEFAULT_FUNCTION_NAMESPACE {
                None
            } else {
                Some(metadata.namespace)
            },
        }
    }
}

/// A function is a container instance with correct cni connected
#[derive(Debug)]
pub struct FunctionInstance {
    container: containerd_client::services::v1::Container,
    namespace: String,
    /// ip addr inside cni
    network: CNIEndpoint,
    // port: Vec<u16>, default use 8080
    // manager: Weak<crate::provider::ContainerdProvider>,
    _created_at: chrono::DateTime<chrono::Utc>,
}

impl FunctionInstance {
    pub async fn new(metadata: ContainerStaticMetadata) -> Result<Self, ContainerdError> {
        BACKEND.prepare_snapshot(&metadata).await?;

        let container = BACKEND.create_container(&metadata).await.map_err(|e| {
            log::error!("Failed to create container: {:?}", e);
            ContainerdError::CreateContainerError(String::new())
        })?;

        let network = CNIEndpoint::new(&metadata.container_id, &metadata.namespace)?;

        // TODO: Use ostree-ext
        // let img_conf = BACKEND.get_runtime_config(&metadata.image).unwrap();

        BACKEND
            .new_task(&metadata.container_id, &metadata.namespace)
            .await?;

        Ok(Self {
            container,
            namespace: metadata.namespace,
            network,
            _created_at: chrono::Utc::now(),
        })
    }

    pub async fn delete(&self) -> Result<(), ContainerdError> {
        let container_id = self.container.id.clone();
        let namespace = self.namespace.clone();

        BACKEND
            .kill_task_with_timeout(&container_id, &namespace)
            .await?;

        BACKEND
            .delete_container(&container_id, &namespace)
            .await
            .map_err(|e| {
                log::error!("Failed to delete container: {:?}", e);
                ContainerdError::DeleteContainerError(String::new())
            })
    }

    pub fn address(&self) -> IpAddr {
        self.network.address()
    }
}
