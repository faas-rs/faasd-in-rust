use tokio_util::task::TaskTracker;

use crate::{
    handlers::function::{DeleteError, DeployError, ResolveError},
    types::function::{Deployment, Query},
};

pub trait Provider: Send + Sync + 'static {
    /// Should return a valid upstream url
    fn resolve(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<url::Url, ResolveError>> + Send;

    /// Deploy a function
    fn deploy(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), DeployError>> + Send;

    /// Delete a function
    fn delete(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<(), DeleteError>> + Send;

    fn task_tracker(&self) -> TaskTracker;
}
