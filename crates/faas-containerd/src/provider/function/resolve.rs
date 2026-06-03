use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::cni_impl;
use crate::impls::cni::Endpoint;
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> http::Uri {
    format!("http://{addr}:8080").parse().unwrap()
}

impl ContainerdProvider {
    /// Resolve a function's upstream URI.
    ///
    /// Two-level routing controlled by `Query::cache_miss`:
    /// - `false` (fast path): sled cache only → hit or 404.
    /// - `true`  (ground truth): netns → update sled → return IP;
    ///   netns absent checks containerd; uses sled as last-resort fallback.
    pub async fn resolve(&self, query: Query) -> Result<http::Uri, ResolveError> {
        let endpoint = Endpoint::from(query.clone());
        log::trace!("Resolving: {:?}", endpoint);

        if query.cache_miss {
            // Ground truth
            self.resolve_ground_truth(&endpoint).await
        } else {
            // Fast path: sled cache only
            self.resolve_fast(&endpoint)
        }
    }

    /// Fast path: sled cache lookup only.
    fn resolve_fast(&self, endpoint: &Endpoint) -> Result<http::Uri, ResolveError> {
        match self.resolved_ips.lock().unwrap().get(endpoint) {
            Some(ip) => Ok(upstream(*ip)),
            None => Err(ResolveError::NotFound(format!(
                "function {} not found in cache",
                endpoint
            ))),
        }
    }

    /// Ground truth: netns → containerd container check → sled fallback.
    async fn resolve_ground_truth(
        &self,
        endpoint: &Endpoint,
    ) -> Result<http::Uri, ResolveError> {
        if let Some(ip) = cni_impl::netns_get_ip(endpoint) {
            // netns has IP → cache it, repair sled dirty flags, return IP
            self.resolved_ips.lock().unwrap().insert(endpoint.clone(), ip);
            self.repair_sled_from_netns(endpoint);
            return Ok(upstream(ip));
        }

        // netns absent — check containerd for running container
        // Container alive but netns gone — try sled cache as last resort
        if crate::impls::backend().container_exists(endpoint).await
            && let Some(ip) = self.resolved_ips.lock().unwrap().get(endpoint).copied()
        {
            log::warn!(
                "netns absent for {} but container exists; returning cached IP {}",
                endpoint, ip
            );
            self.cache.mark_dirty(
                endpoint,
                "netns absent, container still exists",
            ).ok();
            return Ok(upstream(ip));
        }

        // Container gone too — clean sled, return 404
        log::info!("netns and container absent for {}; removing sled record", endpoint);
        self.cache.remove(endpoint).ok();
        Err(ResolveError::NotFound(format!(
            "function {} not found (netns + container absent)",
            endpoint
        )))
    }

    /// Repair sled cache with IP from netns — crash recovery / drift correction.
    fn repair_sled_from_netns(&self, endpoint: &Endpoint) {
        if let Ok(Some(meta)) = self.cache.get(endpoint)
            && meta.dirty.is_some()
        {
            self.cache.clear_dirty(endpoint).ok();
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
