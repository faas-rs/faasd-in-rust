use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::{self, Endpoint};
use crate::provider::ContainerdProvider;
use crate::state::InstanceState;

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

        let record = match self.state_store.get(&endpoint) {
            Ok(Some(r)) => r,
            Ok(None) => {
                log::trace!("No record found for {}", endpoint);
                return Err(ResolveError::NotFound("container not found".to_string()));
            }
            Err(e) => {
                log::error!("Failed to read state for {}: {:?}", endpoint, e);
                return Err(ResolveError::Internal(e.to_string()));
            }
        };

        match &record.state {
            InstanceState::Active => {
                let addr = record.ip_address.ok_or_else(|| {
                    log::error!("Active record for {} has no IP address", endpoint);
                    ResolveError::Internal("missing IP address".to_string())
                })?;

                // Consistency check: verify CNI network still exists
                if cni::cni_impl::check_network_exists(addr) {
                    log::trace!("CNI network exists for {} = {}", endpoint, addr);
                    Ok(upstream(addr))
                } else {
                    log::error!(
                        "CNI network missing for {} = {} (drift detected)",
                        endpoint,
                        addr
                    );
                    // Don't remove the record here — let reconciliation handle it
                    Err(ResolveError::Internal("CNI network not exists".to_string()))
                }
            }
            InstanceState::Error(inner) => {
                log::warn!(
                    "Function {} is in Error({}) state: {:?}",
                    endpoint,
                    inner,
                    record.error_reason
                );
                Err(ResolveError::Internal(format!(
                    "function in error state: {:?}",
                    record.error_reason
                )))
            }
            other => {
                log::trace!("Function {} is in {} state, not ready", endpoint, other);
                Err(ResolveError::NotFound(format!(
                    "function not active (state: {})",
                    other
                )))
            }
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
