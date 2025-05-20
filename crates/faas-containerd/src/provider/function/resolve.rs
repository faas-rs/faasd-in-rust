use std::net::IpAddr;

use actix_http::uri::Builder;
use gateway::handlers::function::ResolveError;
use gateway::types::function::Query;

use crate::consts::DEFAULT_FUNCTION_NAMESPACE;
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> Builder {
    actix_http::Uri::builder()
        .scheme("http")
        .authority(format!("{}:{}", addr, 8080))
}

impl ContainerdProvider {
    pub(crate) async fn _resolve(
        &self,
        mut query: Query,
    ) -> Result<actix_http::uri::Builder, ResolveError> {
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

        Ok(upstream(addr))
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_uri() {
        let addr = IpAddr::V4(Ipv4Addr::new(10, 42, 2, 48));
        let uri = super::upstream(addr).path_and_query("").build().unwrap();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), addr.to_string());
        assert_eq!(uri.authority().unwrap().port_u16(), Some(8080));
        assert_eq!(uri.to_string(), format!("http://{}:8080/", addr));
    }
}
