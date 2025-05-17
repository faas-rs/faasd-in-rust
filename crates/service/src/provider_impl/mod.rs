pub mod function;

use std::collections::HashMap;

use provider::types::function::{Deployment, Query};
use std::sync::Mutex;

use crate::impls::function::FunctionInstance;

pub struct CtrdProvider {
    pub ctr_instance_map: Mutex<HashMap<Query, FunctionInstance>>,
    // pub backend: ContainerdService,
    // pub self_ref: Weak<Self>,
    pub task_tracker: tokio_util::task::TaskTracker,
}

impl Drop for CtrdProvider {
    fn drop(&mut self) {
        self.ctr_instance_map
            .lock()
            .unwrap()
            .drain()
            .for_each(|(_q, container)| {
                self.task_tracker.spawn(async move {
                    let _ = container.delete().await;
                });
            });
    }
}

impl provider::provider::Provider for CtrdProvider {
    fn resolve(
        &self,
        function: Query,
    ) -> impl std::future::Future<
        Output = Result<url::Url, provider::handlers::function::ResolveError>,
    > + Send {
        self._resolve(function)
    }

    fn deploy(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), provider::handlers::function::DeployError>> + Send
    {
        self._deploy(param)
    }

    fn delete(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<(), provider::handlers::function::DeleteError>> + Send
    {
        self._delete(function)
    }

    fn task_tracker(&self) -> tokio_util::task::TaskTracker {
        self.task_tracker.clone()
    }
}
