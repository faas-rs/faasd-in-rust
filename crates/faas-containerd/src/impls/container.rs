use containerd_client::{
    services::v1::{Container, DeleteContainerRequest, GetContainerRequest, ListContainersRequest},
    with_namespace,
};

use derive_more::Display;

use containerd_client::services::v1::container::Runtime;

use super::{ContainerdService, backend, cni::Endpoint, function::ContainerStaticMetadata};
use tonic::Request;

#[derive(Debug, Display)]
pub enum ContainerError {
    NotFound,
    AlreadyExists,
    Internal,
}

impl ContainerdService {
    /// 创建容器
    pub async fn create_container(
        &self,
        metadata: &ContainerStaticMetadata,
    ) -> Result<Container, ContainerError> {
        let container = Container {
            id: metadata.endpoint.service.clone(),
            image: metadata.image.clone(),
            runtime: Some(Runtime {
                name: "io.containerd.runc.v2".to_string(),
                options: None,
            }),
            spec: Some(backend().get_spec(metadata).await.map_err(|_| {
                log::error!("Failed to get spec");
                ContainerError::Internal
            })?),
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            snapshot_key: metadata.endpoint.service.clone(),
            ..Default::default()
        };

        let mut cc = backend().client.containers();
        let req = containerd_client::services::v1::CreateContainerRequest {
            container: Some(container),
        };

        let resp = cc
            .create(with_namespace!(req, metadata.endpoint.namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to create container: {}", e);
                ContainerError::Internal
            })?;

        resp.into_inner().container.ok_or(ContainerError::Internal)
    }

    /// 删除容器
    pub async fn delete_container(&self, endpoint: &Endpoint) -> Result<(), ContainerError> {
        let Endpoint {
            service: cid,
            namespace: ns,
        } = endpoint;
        let mut cc = self.client.containers();

        let delete_request = DeleteContainerRequest { id: cid.clone() };

        cc.delete(with_namespace!(delete_request, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to delete container: {}", e);
                ContainerError::Internal
            })
            .map(|_| ())
    }

    /// 根据查询条件加载容器参数
    pub async fn load_container(&self, endpoint: &Endpoint) -> Result<Container, ContainerError> {
        let mut cc = self.client.containers();

        let request = GetContainerRequest {
            id: endpoint.service.clone(),
        };

        let resp = cc
            .get(with_namespace!(request, endpoint.namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to list containers: {}", e);
                ContainerError::Internal
            })?;

        resp.into_inner().container.ok_or(ContainerError::NotFound)
    }

    /// 获取容器列表
    pub async fn list_container(&self, namespace: &str) -> Result<Vec<Container>, ContainerError> {
        let mut cc = self.client.containers();

        let request = ListContainersRequest {
            ..Default::default()
        };

        let resp = cc
            .list(with_namespace!(request, namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to list containers: {}", e);
                ContainerError::Internal
            })?;

        Ok(resp.into_inner().containers)
    }

    /// 不儿，这也要单独一个函数？
    #[deprecated]
    pub async fn list_container_into_string(
        &self,
        ns: &str,
    ) -> Result<Vec<String>, ContainerError> {
        self.list_container(ns)
            .await
            .map(|ctrs| ctrs.into_iter().map(|ctr| ctr.id).collect())
    }
}
