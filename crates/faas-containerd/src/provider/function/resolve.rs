use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::{self, Endpoint};
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> http::Uri {
    format!("http://{addr}:8080").parse().unwrap()
}

impl ContainerdProvider {
    pub async fn resolve(&self, query: Query) -> Result<http::Uri, ResolveError> {
        let endpoint = Endpoint::from(query);
        log::trace!("Resolving function: {:?}", endpoint);

        let record = match self.cache.get_ip(&endpoint) {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err(ResolveError::NotFound("container not found".to_string()));
            }
            Err(e) => {
                log::error!("Cache read failed for {}: {:?}", endpoint, e);
                return Err(ResolveError::Internal(e.to_string()));
            }
        };

        if cni::cni_impl::check_network_exists(record.ip) {
            Ok(upstream(record.ip))
        } else {
            log::error!("CNI network missing for {} = {}", endpoint, record.ip);
            self.cache.remove(&endpoint).ok();
            Err(ResolveError::Internal("CNI network not exists".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_uri() {
        let addr = IpAddr::V4(Ipv4Addr::new(10, 42, 2, 48));
        let uri = super::upstream(addr);
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), addr.to_string());
        assert_eq!(uri.authority().unwrap().port_u16(), Some(8080));
        assert!(uri.to_string().starts_with(&format!("http://{addr}:8080")));
    }
}
