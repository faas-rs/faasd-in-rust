use std::collections::HashMap;
use std::future::Future;

use http::Uri;

use crate::types::*;

pub trait Provider: Send + Sync + 'static {
    fn deploy(&self, param: Deployment) -> impl Future<Output = Result<(), DeployError>> + Send;
    fn delete(&self, function: Query) -> impl Future<Output = Result<(), DeleteError>> + Send;
    fn resolve(&self, function: Query) -> impl Future<Output = Result<Uri, ResolveError>> + Send;
    fn list(
        &self,
        namespace: String,
    ) -> impl Future<Output = Result<Vec<Status>, ListError>> + Send;
    fn update(&self, param: Deployment) -> impl Future<Output = Result<(), UpdateError>> + Send;
    fn status(&self, function: Query) -> impl Future<Output = Result<Status, ResolveError>> + Send;
    fn create_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> impl Future<Output = Result<(), NamespaceError>> + Send;
    fn update_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> impl Future<Output = Result<(), NamespaceError>> + Send;
    fn delete_namespace(
        &self,
        namespace: String,
    ) -> impl Future<Output = Result<(), NamespaceError>> + Send;
    fn get_namespace(
        &self,
        namespace: String,
    ) -> impl Future<Output = Result<Namespace, NamespaceError>> + Send;
    fn namespace_list(&self)
    -> impl Future<Output = Result<Vec<Namespace>, NamespaceError>> + Send;
}
