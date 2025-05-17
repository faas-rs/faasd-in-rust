use provider::handlers::function::ResolveError;
use provider::types::function::Query;

use crate::provider_impl::CtrdProvider;

impl CtrdProvider {
    pub(crate) async fn _resolve(&self, query: Query) -> Result<url::Url, ResolveError> {
        let addr = self
            .ctr_instance_map
            .lock()
            .unwrap()
            .get(&query)
            .ok_or(ResolveError::NotFound)?
            .address();
        // TODO: didn't check instance is still alive

        url::Url::parse(&format!("http://{}", addr)).map_err(|_| ResolveError::Internal)
    }
}
