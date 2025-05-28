use std::collections::HashMap;

use crate::{
    handlers::{
        function::{DeleteError, DeployError, ListError, ResolveError, UpdateError},
        namespace::NamespaceError,
    },
    types::{
        function::{Deployment, Query, Status},
        namespace::Namespace,
    },
};

pub trait Provider: Send + Sync + 'static {
    /// Should return a valid upstream url
    fn resolve(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<actix_http::uri::Builder, ResolveError>> + Send;

    // `/system/functions` endpoint

    /// Get a list of deployed functions
    fn list(
        &self,
        namespace: String,
    ) -> impl std::future::Future<Output = Result<Vec<Status>, ListError>> + Send;

    /// Deploy a new function
    fn deploy(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), DeployError>> + Send;

    /// Update a function spec
    fn update(
        &self,
        param: Deployment,
    ) -> impl std::future::Future<Output = Result<(), UpdateError>> + Send;

    /// Delete a function
    fn delete(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<(), DeleteError>> + Send;

    // `/system/function/{functionName}` endpoint
    /// Get the status of a function by name
    fn status(
        &self,
        function: Query,
    ) -> impl std::future::Future<Output = Result<Status, ResolveError>> + Send;

    fn create_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<(), NamespaceError>> + Send;

    fn update_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> impl std::future::Future<Output = Result<(), NamespaceError>> + Send;

    fn delete_namespace(
        &self,
        namespace: String,
    ) -> impl std::future::Future<Output = Result<(), NamespaceError>> + Send;

    fn get_namespace(
        &self,
        namespace: String,
    ) -> impl std::future::Future<Output = Result<Namespace, NamespaceError>> + Send;

    fn namespace_list(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<Namespace>, NamespaceError>> + Send;
}
