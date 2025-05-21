pub mod function;

use std::{path::Path, sync::Arc};

use gateway::{
    handlers::function::{DeleteError, DeployError, ListError, ResolveError, UpdateError},
    provider::Provider,
    types::function::{Deployment, Query, Status},
};

pub struct ContainerdProvider {
    // pub ctr_instance_map: tokio::sync::Mutex<HashMap<Query, FunctionInstance>>,
    database: sled::Db,
}

impl ContainerdProvider {
    pub fn new<P: AsRef<Path>>(path: P) -> Arc<Self> {
        Arc::new(ContainerdProvider {
            // ctr_instance_map: tokio::sync::Mutex::new(HashMap::new()),
            database: sled::open(path).unwrap(),
        })
    }
}

impl Provider for ContainerdProvider {
    async fn resolve(&self, function: Query) -> Result<actix_http::uri::Builder, ResolveError> {
        self._resolve(function).await
    }

    async fn deploy(&self, param: Deployment) -> Result<(), DeployError> {
        self._deploy(param).await
    }

    async fn delete(&self, function: Query) -> Result<(), DeleteError> {
        self._delete(function).await
    }

    async fn list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
        self._list(namespace).await
    }

    async fn update(&self, param: Deployment) -> Result<(), UpdateError> {
        self._update(param).await
    }

    async fn status(&self, function: Query) -> Result<Status, ResolveError> {
        self._status(function).await
    }
}
