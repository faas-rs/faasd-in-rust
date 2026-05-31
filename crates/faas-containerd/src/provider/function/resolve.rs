use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::{self, Endpoint};
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> http::Uri {
    format!("http://{addr}:8080").parse().unwrap()
}

impl ContainerdProvider {
    pub async fn resolve(
        &self,
        query: Query,
    ) -> Result<http::Uri, ResolveError> {
        let endpoint = Endpoint::from(query);
        log::trace!("Resolving function: {:?}", endpoint);

        let record = match self.cache.get(&endpoint) {
            Ok(Some(r)) => r,
            Ok(None) => {
                log::trace!("No cache entry for {}", endpoint);
                return Err(ResolveError::NotFound("container not found".to_string()));
            }
            Err(e) => {
                log::error!("Failed to read cache for {}: {:?}", endpoint, e);
                return Err(ResolveError::Internal(e.to_string()));
            }
        };

        // Verify the underlying CNI network still exists
        if cni::cni_impl::check_network_exists(record.ip) {
            log::trace!("CNI network confirmed for {} = {}", endpoint, record.ip);
            Ok(upstream(record.ip))
        } else {
            // Network gone — stale cache.  Clean up and return 503.
            log::error!(
                "CNI network missing for {} = {} (drift detected, clearing cache)",
                endpoint,
                record.ip,
            );
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
