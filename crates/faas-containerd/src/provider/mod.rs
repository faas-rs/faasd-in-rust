pub mod function;

use std::{path::Path, sync::Arc};

use gateway::provider::Provider;
use gateway::types::*;

use crate::state::CacheStore;

use bollard::Docker;

pub struct ContainerdProvider {
    pub docker: Docker,
    pub cache: CacheStore,
    pub network: String,
}

impl ContainerdProvider {
    pub fn new<P: AsRef<Path>>(path: P) -> Arc<Self> {
        let docker = Docker::connect_with_local_defaults().expect("Docker socket");
        Arc::new(ContainerdProvider {
            docker,
            cache: CacheStore::new(sled::open(path).unwrap()),
            network: "faasrs0".into(),
        })
    }
}

#[async_trait::async_trait]
impl Provider for ContainerdProvider {
    async fn deploy(&self, p: Deployment) -> Result<(), DeployError> {
        self.function_deploy(p).await
    }

    async fn delete(&self, q: Query) -> Result<(), DeleteError> {
        self.function_delete(q).await
    }

    async fn resolve(&self, q: Query) -> Result<http::Uri, ResolveError> {
        self.function_resolve(q).await
    }

    async fn list(&self, ns: String) -> Result<Vec<Status>, ListError> {
        self.function_list(ns).await
    }

    async fn update(&self, p: Deployment) -> Result<(), UpdateError> {
        self.function_update(p).await
    }

    async fn status(&self, q: Query) -> Result<Status, ResolveError> {
        self.function_status(q).await
    }
}
