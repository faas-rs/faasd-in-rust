use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::{self, Endpoint};
use crate::provider::ContainerdProvider;

fn upstream(addr: IpAddr) -> http::Uri {
    format!("http://{addr}:8080").parse().unwrap()
}

impl ContainerdProvider {
    /// Resolve a function's IP via three-tier fallback.
    ///
    /// Tier 1: netns exists → read IP from netns, repair sled if stale.
    /// Tier 2: netns absent but sled has record → check containerd for
    ///          running container; if found, return cached IP (stale but
    ///          still routable via CNI bridge), mark dirty.
    /// Tier 3: netns absent, no sled record → 404.
    pub async fn resolve(&self, query: Query) -> Result<http::Uri, ResolveError> {
        let endpoint = Endpoint::from(query);
        log::trace!("Resolving function: {:?}", endpoint);

        // Tier 1: netns is truth — always check it first when possible
        if let Some(ip) = cni::cni_impl::netns_get_ip(&endpoint) {
            // netns has IP → verify/repair sled cache
            let meta = match self.cache.get(&endpoint) {
                Ok(Some(m)) => m,
                Ok(None) => {
                    // netns exists but sled missing — crash recovery gap repair
                    log::warn!("netns exists for {} but sled record missing; repairing", endpoint);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_millis() as u64);
                    let repair = crate::state::DeployMeta {
                        ip,
                        image: String::new(),
                        created_at: now,
                        labels: std::collections::HashMap::new(),
                        dirty: Some("crash recovery: sled record missing".into()),
                    };
                    self.cache.insert(&endpoint, &repair).ok();
                    return Ok(upstream(ip));
                }
                Err(_) => return Ok(upstream(ip)), // sled error, but netns is truth
            };
            // IP mismatch: netns IP changed (e.g. CNI rebuilt). Trust netns.
            if meta.ip != ip {
                log::warn!(
                    "IP mismatch for {}: sled={}, netns={}; updating sled",
                    endpoint, meta.ip, ip
                );
                self.cache.insert(
                    &endpoint,
                    &crate::state::DeployMeta { ip, ..meta },
                ).ok();
            } else {
                // IP matches → cache is fresh, optionally clear stale dirty flag
                if meta.dirty.is_some() {
                    self.cache.clear_dirty(&endpoint).ok();
                }
            }
            return Ok(upstream(ip));
        }

        // Tier 2: netns absent, check sled cache
        match self.cache.get(&endpoint) {
            Ok(Some(meta)) => {
                // netns absent, sled has record — check if container still exists
                if crate::impls::backend().container_exists(&endpoint).await {
                    // Container alive but netns gone — last resort, return cached IP
                    log::warn!(
                        "netns absent for {} but container exists; returning cached IP {}",
                        endpoint, meta.ip
                    );
                    self.cache.mark_dirty(
                        &endpoint,
                        "netns absent, containerd container still exists"
                    ).ok();
                    return Ok(upstream(meta.ip));
                }
                // Container gone too — clean up sled
                log::info!(
                    "netns and container absent for {}; removing sled record", endpoint
                );
                self.cache.remove(&endpoint).ok();
                Err(ResolveError::NotFound("container not found".to_string()))
            }
            Ok(None) => {
                Err(ResolveError::NotFound("container not found".to_string()))
            }
            Err(e) => {
                log::error!("Cache read failed for {}: {:?}", endpoint, e);
                Err(ResolveError::Internal(e.to_string()))
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
