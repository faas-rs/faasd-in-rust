use crate::provider::ContainerdProvider;
use gateway::types::{Query, ResolveError};

fn upstream(ip: std::net::IpAddr) -> http::Uri {
    format!("http://{ip}:8080").parse().unwrap()
}

impl ContainerdProvider {
    pub async fn function_resolve(&self, q: Query) -> Result<http::Uri, ResolveError> {
        let ns = q.namespace.as_deref().unwrap_or("openfaas-fn");
        let key = format!("faasdrs-{}-{}", ns, q.function_name);
        match self.cache.get_ip(&key) {
            Ok(Some(r)) => Ok(upstream(r.ip)),
            Ok(None) => Err(ResolveError::NotFound("not found".into())),
            Err(e) => Err(ResolveError::Internal(e.to_string())),
        }
    }
}
