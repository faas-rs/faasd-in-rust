use crate::containerd_manager::CLIENT;
use containerd_client::{
    Client,
    services::v1::{CreateNamespaceRequest, DeleteNamespaceRequest, Namespace},
};
use std::sync::Arc;

pub struct NSManager {
    namespaces: Vec<Namespace>,
}

impl NSManager {
    async fn get_client() -> Arc<Client> {
        CLIENT
            .get()
            .unwrap_or_else(|| panic!("Client not initialized, Please run init first"))
            .clone()
    }

    pub async fn create_namespace(&mut self, name: &str) -> Result<(), NameSpaceError> {
        let client = Self::get_client().await;
        let mut ns_client = client.namespaces();

        let request = CreateNamespaceRequest {
            namespace: Some(Namespace {
                name: name.to_string(),
                ..Default::default()
            }),
        };

        let response = ns_client.create(request).await.map_err(|e| {
            NameSpaceError::CreateError(format!("Failed to create namespace {}: {}", name, e))
        })?;

        self.namespaces
            .push(response.into_inner().namespace.unwrap());
        Ok(())
    }

    pub async fn delete_namespace(&mut self, name: &str) -> Result<(), NameSpaceError> {
        let client = Self::get_client().await;
        let mut ns_client = client.namespaces();

        let req = DeleteNamespaceRequest {
            name: name.to_string(),
        };

        ns_client.delete(req).await.map_err(|e| {
            NameSpaceError::DeleteError(format!("Failed to delete namespace {}: {}", name, e))
        })?;

        self.namespaces.retain(|ns| ns.name != name);
        Ok(())
    }

    pub async fn list_namespace(&self) -> Result<Vec<Namespace>, NameSpaceError> {
        // 这里没有选择直接列举所有的命名空间，而是返回当前对象中存储的命名空间
        // 觉得应该只能看见自己创建的命名空间而不能看见其他人的命名空间
        // 是不是应该把namespaces做持久化存储，作为一个用户自己的namespace
        Ok(self.namespaces.clone())
    }
}

#[derive(Debug)]
pub enum NameSpaceError {
    CreateError(String),
    DeleteError(String),
    ListError(String),
}

impl std::fmt::Display for NameSpaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameSpaceError::CreateError(msg) => write!(f, "Create Namespace Error: {}", msg),
            NameSpaceError::DeleteError(msg) => write!(f, "Delete Namespace Error: {}", msg),
            NameSpaceError::ListError(msg) => write!(f, "List Namespace Error: {}", msg),
        }
    }
}

impl std::error::Error for NameSpaceError {}
