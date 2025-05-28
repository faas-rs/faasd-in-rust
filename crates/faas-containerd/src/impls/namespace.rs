use std::collections::HashMap;

use containerd_client::services::v1::{
    CreateNamespaceRequest, DeleteNamespaceRequest, ListNamespacesRequest, Namespace,
    UpdateNamespaceRequest,
};
use derive_more::Display;

use super::ContainerdService;

#[derive(Debug, Display)]
pub enum NamespaceServiceError {
    AlreadyExists,
    NotFound,
    Internal(String),
}

impl From<tonic::Status> for NamespaceServiceError {
    fn from(status: tonic::Status) -> Self {
        use tonic::Code::*;
        match status.code() {
            NotFound => NamespaceServiceError::NotFound,
            AlreadyExists => NamespaceServiceError::AlreadyExists,
            _ => NamespaceServiceError::Internal(status.message().to_string()),
        }
    }
}

impl ContainerdService {
    // 创建命名空间
    pub async fn create_namespace(
        &self,
        namespace: &str,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceServiceError> {
        let exist = self.namespace_exist(namespace).await?;
        if exist.is_some() {
            log::info!("Namespace {} already exists", namespace);
            return Err(NamespaceServiceError::AlreadyExists);
        }

        let mut c = self.client.namespaces();
        let ns_name = namespace;

        let namespace = Namespace {
            name: namespace.to_string(),
            labels,
        };
        let req = CreateNamespaceRequest {
            namespace: Some(namespace),
        };
        c.create(req).await.map_err(|e| {
            log::error!("Failed to create namespace: {}", e);
            NamespaceServiceError::Internal(e.to_string())
        })?;
        log::info!("Namespace {} created", ns_name);
        Ok(())
    }

    // 删除命名空间
    pub async fn delete_namespace(&self, namespace: &str) -> Result<(), NamespaceServiceError> {
        let exist = self.namespace_exist(namespace).await?;
        if exist.is_none() {
            log::info!("Namespace {} not found", namespace);
            return Err(NamespaceServiceError::NotFound);
        }
        let mut c = self.client.namespaces();
        let ns_name = namespace;
        let req = DeleteNamespaceRequest {
            name: namespace.to_string(),
        };
        c.delete(req).await.map_err(|e| {
            log::error!("Failed to delete namespace: {}", e);
            NamespaceServiceError::Internal(e.to_string())
        })?;
        log::info!("Namespace {} deleted", ns_name);
        Ok(())
    }

    // 判断命名空间是否存在
    pub async fn namespace_exist(
        &self,
        namespace: &str,
    ) -> Result<Option<Namespace>, NamespaceServiceError> {
        let mut c = self.client.namespaces();
        let req = ListNamespacesRequest {
            ..Default::default()
        };
        let ns_list_resp = c.list(req).await.map_err(|e| {
            log::error!("Failed to list namespaces: {}", e);
            NamespaceServiceError::Internal(e.to_string())
        })?;
        let ns_list = ns_list_resp.into_inner().namespaces;
        for ns in ns_list {
            if ns.name == namespace {
                return Ok(Some(ns));
            }
        }
        Ok(None)
    }

    // 获取命名空间列表
    pub async fn list_namespace(&self) -> Result<Vec<Namespace>, NamespaceServiceError> {
        let mut c = self.client.namespaces();
        let req = ListNamespacesRequest {
            ..Default::default()
        };
        let ns_list_resp = c.list(req).await.map_err(|e| {
            log::error!("Failed to list namespaces: {}", e);
            NamespaceServiceError::Internal(e.to_string())
        })?;
        let ns_list = ns_list_resp.into_inner().namespaces;
        Ok(ns_list)
    }

    // 更新命名空间信息
    pub async fn update_namespace(
        &self,
        namespace: &str,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceServiceError> {
        let exist = self.namespace_exist(namespace).await?;
        if exist.is_none() {
            log::info!("Namespace {} not found", namespace);
            return Err(NamespaceServiceError::NotFound);
        }
        let ns_name = namespace;
        let namespace = Namespace {
            name: namespace.to_string(),
            labels,
        };
        let mut c = self.client.namespaces();
        let req = UpdateNamespaceRequest {
            namespace: Some(namespace),
            ..Default::default()
        };
        c.update(req).await.map_err(|e| {
            log::error!("Failed to update namespace: {}", e);
            NamespaceServiceError::Internal(e.to_string())
        })?;
        log::info!("Namespace {} updated", ns_name);
        Ok(())
    }
}
