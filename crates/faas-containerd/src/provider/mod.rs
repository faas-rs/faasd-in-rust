pub mod function;
use std::{collections::HashMap, path::Path, sync::Arc};

use gateway::provider::Provider;
use gateway::types::*;

use crate::state::StateStore;

pub struct ContainerdProvider {
    pub state_store: StateStore,
}

impl ContainerdProvider {
    pub fn new<P: AsRef<Path>>(path: P) -> Arc<Self> {
        Arc::new(ContainerdProvider {
            state_store: StateStore::new(sled::open(path).unwrap()),
        })
    }
}

impl Provider for ContainerdProvider {
    async fn deploy(&self, param: Deployment) -> Result<(), DeployError> {
        self.deploy(param).await
    }

    async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        self.delete(function).await
    }

    async fn resolve(&self, function: Query) -> Result<http::Uri, ResolveError> {
        self.resolve(function).await
    }

    async fn list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
        self.list(namespace).await
    }

    async fn update(&self, param: Deployment) -> Result<(), UpdateError> {
        self.update(param).await
    }

    async fn status(&self, function: Query) -> Result<Status, ResolveError> {
        self.status(function).await
    }

    async fn create_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceError> {
        self.create_namespace(namespace, labels).await
    }

    async fn update_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceError> {
        self.update_namespace(namespace, labels).await
    }

    async fn delete_namespace(&self, namespace: String) -> Result<(), NamespaceError> {
        self.delete_namespace(namespace).await
    }

    async fn get_namespace(&self, namespace: String) -> Result<Namespace, NamespaceError> {
        self.get_namespace(namespace).await
    }

    async fn namespace_list(&self) -> Result<Vec<Namespace>, NamespaceError> {
        self.namespace_list().await
    }
}
