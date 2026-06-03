use std::net::IpAddr;

use gateway::types::{Query, ResolveError};

use crate::impls::cni::cni_impl;
use crate::impls::cni::Endpoint;
use crate::provider::ContainerdProvider;
use crate::state::{CacheStore, DirtyState};

fn upstream(addr: IpAddr) -> http::Uri {
    format!("http://{addr}:8080").parse().unwrap()
}

impl ContainerdProvider {
    /// Resolve function IP.
    ///
    /// Fast path: netns → IP (bypasses sled entirely).
    ///
    /// Degraded (sled Clean, netns absent): mark `Repairing`, spawn background
    /// CNI repair (max 3 retries), return `Busy` immediately.  Caller retries.
    ///
    /// Active lock (`Deploying`|`Deleting`|`Repairing`): `Busy` (503).
    /// Terminal (`Broken`): `Unavailable` (502).
    pub async fn resolve(&self, query: Query) -> Result<http::Uri, ResolveError> {
        let endpoint = Endpoint::from(query);
        log::trace!("Resolving: {:?}", endpoint);

        // Fast path: netns has IP → return immediately (ignores sled)
        if let Some(ip) = cni_impl::netns_get_ip(&endpoint) {
            return Ok(upstream(ip));
        }

        // Netns absent — check sled
        let meta = match self.cache.get(&endpoint) {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(ResolveError::NotFound(format!(
                    "function {} not found in cache",
                    endpoint
                )));
            }
            Err(e) => {
                return Err(ResolveError::Internal(format!(
                    "cache read error for {}: {}",
                    endpoint, e
                )));
            }
        };

        match &meta.dirty {
            DirtyState::Deploying | DirtyState::Deleting => {
                Err(ResolveError::Busy(format!(
                    "function {} is {:?}",
                    endpoint, meta.dirty
                )))
            }

            DirtyState::Repairing => {
                // Another resolve already spawned repair — don't re-trigger
                Err(ResolveError::Busy(format!(
                    "function {} repair in progress",
                    endpoint
                )))
            }

            DirtyState::Broken(reason) => Err(ResolveError::Unavailable(format!(
                "function {} is unavailable: {}",
                endpoint, reason
            ))),

            DirtyState::Clean => {
                // Clean record but netns absent → degraded, spawn background repair
                log::warn!(
                    "netns absent for {} but sled clean; spawning background repair",
                    endpoint
                );

                self.cache
                    .mark_dirty(&endpoint, DirtyState::Repairing)
                    .ok();

                let cache = self.cache.clone();
                let ep = endpoint.clone();
                let handle = self.handle.clone();

                handle.spawn(async move {
                    run_background_repair(cache, ep).await;
                });

                Err(ResolveError::Busy(format!(
                    "function {} repair spawned, retry shortly",
                    endpoint
                )))
            }
        }
    }
}

/// Attempt CNI repair with up to 3 retries.
/// On success → clear dirty (back to Clean).
/// On all failures → mark Broken.
async fn run_background_repair(cache: CacheStore, endpoint: Endpoint) {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        log::info!(
            "Background repair attempt {}/{} for {}",
            attempt,
            MAX_RETRIES,
            endpoint
        );

        match attempt_repair(&endpoint).await {
            Ok(_) => {
                log::info!("Background repair succeeded for {} on attempt {}", endpoint, attempt);
                cache.clear_dirty(&endpoint).ok();
                return;
            }
            Err(e) => {
                log::error!(
                    "Background repair attempt {}/{} failed for {}: {}",
                    attempt,
                    MAX_RETRIES,
                    endpoint,
                    e
                );
                if attempt == MAX_RETRIES {
                    cache
                        .mark_dirty(
                            &endpoint,
                            DirtyState::Broken(format!("repair failed after {} retries: {}", MAX_RETRIES, e)),
                        )
                        .ok();
                } else {
                    // Brief backoff between retries — give containerd/netns time
                    asupersync::time::sleep(
                        asupersync::time::wall_now(),
                        std::time::Duration::from_millis(200),
                    )
                    .await;
                }
            }
        }
    }
}

/// Single repair attempt: check container exists, recreate netns, run CNI.
async fn attempt_repair(endpoint: &Endpoint) -> Result<IpAddr, String> {
    // Check container still exists before building netns
    use crate::impls::backend;
    if !backend().container_exists(endpoint).await {
        return Err("container gone — cannot repair".into());
    }

    let ns = netns_rs::NetNs::new(endpoint.to_string()).map_err(|e| {
        format!("NetNs::new failed: {e}")
    })?;

    // NetNs must be held open while CNI configures the interfaces inside it.
    // Drop is deferred until after setup_cni_network returns.
    let ip = cni_impl::setup_cni_network(endpoint).map_err(|e| {
        format!("CNI setup failed: {e}")
    })?;

    // Keep ns alive for the scope of setup_cni_network only
    drop(ns);

    Ok(ip)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use super::upstream;

    #[test]
    fn test_upstream_ipv4() {
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let uri = upstream(ip);
        assert_eq!(uri.to_string(), "http://127.0.0.1:8080/");
    }
}
