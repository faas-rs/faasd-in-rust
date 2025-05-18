pub mod function;

use std::{collections::HashMap, sync::Arc};

use gateway::{
    handlers::function::{DeleteError, DeployError, ResolveError},
    provider::Provider,
    types::function::{Deployment, Query},
};

use crate::impls::function::FunctionInstance;

pub struct ContainerdProvider {
    pub ctr_instance_map: tokio::sync::Mutex<HashMap<Query, FunctionInstance>>,
}

impl ContainerdProvider {
    /// Must be called under tokio runtime
    /// This function will setup signal handlers for graceful shutdown
    pub fn new() -> Arc<Self> {
        Arc::new(ContainerdProvider {
            ctr_instance_map: tokio::sync::Mutex::new(HashMap::new()),
        })
    }
}

impl Provider for ContainerdProvider {
    fn resolve(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<url::Url, ResolveError>> + Send {
        self._resolve(function)
    }

    fn deploy(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), DeployError>> + Send {
        self._deploy(param)
    }

    fn delete(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<(), DeleteError>> + Send {
        self._delete(function)
    }
}
