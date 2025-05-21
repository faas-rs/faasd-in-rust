use std::net::{IpAddr, Ipv4Addr};

use actix_http::uri::Builder;
use gateway::handlers::function::ResolveError;
use gateway::types::function::Query;

use crate::impls::cni::{self, Endpoint};
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> Builder {
    actix_http::Uri::builder()
        .scheme("http")
        .authority(format!("{}:{}", addr, 8080))
}

impl ContainerdProvider {
    pub(crate) async fn _resolve(
        &self,
        query: Query,
    ) -> Result<actix_http::uri::Builder, ResolveError> {
        let endpoint = Endpoint::from(query);
        log::trace!("Resolving function: {:?}", endpoint);
        let addr_oct = self
            .database
            .get(endpoint.to_string())
            .map_err(|e| {
                log::error!("Failed to get container address: {:?}", e);
                ResolveError::Internal(e.to_string())
            })?
            .ok_or(ResolveError::NotFound("container not found".to_string()))?;

        log::trace!("Container address: {:?}", addr_oct.as_array::<4>());

        // We force the address to be IPv4 here
        let addr = IpAddr::V4(Ipv4Addr::from_octets(*addr_oct.as_array::<4>().unwrap()));

        // Check if the coresponding netns is still alive
        // We can achieve this by checking the /run/cni/faasrs-cni-bridge,
        // if the ip filename is still there

        if cni::cni_impl::check_network_exists(addr) {
            log::trace!("CNI network exists for {}", addr);
            Ok(upstream(addr))
        } else {
            log::error!("CNI network not exists for {}", addr);
            let _ = self.database.remove(endpoint.to_string());
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
        let uri = super::upstream(addr).path_and_query("").build().unwrap();
        assert_eq!(uri.scheme_str(), Some("http"));
        assert_eq!(uri.authority().unwrap().host(), addr.to_string());
        assert_eq!(uri.authority().unwrap().port_u16(), Some(8080));
        assert_eq!(uri.to_string(), format!("http://{}:8080/", addr));
    }
}
