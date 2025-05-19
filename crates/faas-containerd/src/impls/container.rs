use containerd_client::{
    services::v1::{Container, DeleteContainerRequest, GetContainerRequest, ListContainersRequest},
    with_namespace,
};

use gateway::types::function::Query;

use crate::consts::DEFAULT_FUNCTION_NAMESPACE;

use containerd_client::services::v1::container::Runtime;

use super::{BACKEND, ContainerdService, function::ContainerStaticMetadata};
use tonic::Request;

#[derive(Debug)]
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
            id: metadata.container_id.clone(),
            image: metadata.image.clone(),
            runtime: Some(Runtime {
                name: "io.containerd.runc.v2".to_string(),
                options: None,
            }),
            spec: Some(BACKEND.get_spec(metadata).await.map_err(|_| {
                log::error!("Failed to get spec");
                ContainerError::Internal
            })?),
            snapshotter: "overlayfs".to_string(),
            snapshot_key: metadata.container_id.clone(),
            ..Default::default()
        };

        let mut cc = BACKEND.client.containers();
        let req = containerd_client::services::v1::CreateContainerRequest {
            container: Some(container),
        };

        let resp = cc
            .create(with_namespace!(req, metadata.namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to create container: {}", e);
                ContainerError::Internal
            })?;

        resp.into_inner().container.ok_or(ContainerError::Internal)
    }

    /// 删除容器
    pub async fn delete_container(
        &self,
        container_id: &str,
        namespace: &str,
    ) -> Result<(), ContainerError> {
        let mut cc = self.client.containers();

        let delete_request = DeleteContainerRequest {
            id: container_id.to_string(),
        };

        cc.delete(with_namespace!(delete_request, namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to delete container: {}", e);
                ContainerError::Internal
            })
            .map(|_| ())
    }

    /// 根据查询条件加载容器参数
    pub async fn load_container(&self, query: &Query) -> Result<Container, ContainerError> {
        let mut cc = self.client.containers();

        let request = GetContainerRequest {
            id: query.service.clone(),
        };

        let namespace = query
            .namespace
            .clone()
            .unwrap_or(DEFAULT_FUNCTION_NAMESPACE.to_string());

        let resp = cc
            .get(with_namespace!(request, namespace))
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
