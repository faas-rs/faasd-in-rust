use gateway::handlers::function::ResolveError;
use gateway::types::function::Query;

use crate::consts::DEFAULT_FUNCTION_NAMESPACE;
use crate::provider::ContainerdProvider;

impl ContainerdProvider {
    pub(crate) async fn _resolve(&self, mut query: Query) -> Result<actix_http::Uri, ResolveError> {
        query
            .namespace
            .get_or_insert(DEFAULT_FUNCTION_NAMESPACE.to_string());
        let addr = self
            .ctr_instance_map
            .lock()
            .await
            .get(&query)
            .ok_or(ResolveError::NotFound("container not found".to_string()))?
            .address();
        // TODO: didn't check instance is still alive

        actix_http::Uri::builder()
            .scheme("http")
            .authority(addr.to_string())
            .build()
            .map_err(|e| ResolveError::Internal(e.to_string()))
    }
}
