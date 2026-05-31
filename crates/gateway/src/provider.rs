use crate::types::*;
use async_trait::async_trait;

#[async_trait]
pub trait Provider: Send + Sync + 'static {
    async fn deploy(&self, param: Deployment) -> Result<(), DeployError>;
    async fn delete(&self, function: Query) -> Result<(), DeleteError>;
    async fn resolve(&self, function: Query) -> Result<http::Uri, ResolveError>;
    async fn list(&self, namespace: String) -> Result<Vec<Status>, ListError>;
    async fn update(&self, param: Deployment) -> Result<(), UpdateError>;
    async fn status(&self, function: Query) -> Result<Status, ResolveError>;
}
