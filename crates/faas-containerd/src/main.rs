//! faas-containerd entry point — asupersync runtime.
//!
//! Boots the asupersync Runtime, initializes the containerd gRPC backend,
//! and serves the gateway HTTP server.  netns is the authoritative
//! state machine; sled is a write-through performance cache.
//!
//! On startup, dual-track scans sled records first (authority), then
//! netns orphans (leftover).  In-progress states from crash → Broken.
//!
//! Graceful shutdown via SIGINT/SIGTERM: ctrl_c() cancels all Cx, the
//! runtime drains in-flight deploy/delete tasks (cleanup runs), then exits.

use std::fs;

use std::sync::Arc;

use faas_containerd::consts::DEFAULT_FAASDRS_DATA_DIR;
use faas_containerd::state::DirtyState;
use gateway::GatewayCache;

/// Parse a netns name like "faasdrs-default-hello" into an Endpoint.
fn parse_ns_endpoint(ns_name: &str) -> Option<faas_containerd::impls::cni::Endpoint> {
    let remainder = ns_name.strip_prefix("faasdrs-")?;
    let dash_pos = remainder.find('-')?;
    let ns = &remainder[..dash_pos];
    let fn_name = &remainder[dash_pos + 1..];
    Some(faas_containerd::impls::cni::Endpoint::new(fn_name, ns))
}

/// Dual-track startup recovery: sled records first (authority), then netns
/// orphans. Returns (repaired, cleaned).
async fn startup_netns_scan(
    provider: &faas_containerd::provider::ContainerdProvider,
) -> (usize, usize) {
    use faas_containerd::impls::backend;

    let mut repaired = 0usize;
    let mut cleaned = 0usize;

    // ── Phase 1: scan sled records ──────────────────────────────
    for res in provider.cache.iter() {
        let (endpoint, meta) = match res {
            Ok(v) => v,
            Err(_) => continue,
        };
        match &meta.dirty {
            // In-progress states at boot → crash residue → promote to Broken
            DirtyState::Deploying | DirtyState::Deleting | DirtyState::Repairing => {
                log::warn!(
                    "startup: {} had dirty={:?} → marking Broken (crash residue)",
                    endpoint,
                    meta.dirty
                );
                let reason = format!("crash during {:?}", meta.dirty);
                provider
                    .cache
                    .mark_dirty(&endpoint, DirtyState::Broken(reason))
                    .ok();
                // Attempt cleanup of any containerd leftovers
                faas_containerd::provider::function::delete::cleanup_containerd_resources(
                    &provider.cache,
                    &endpoint,
                )
                .await;
                if let Ok(ns) = netns_rs::NetNs::get(endpoint.to_string()) {
                    ns.remove().ok();
                }
                cleaned += 1;
            }
            // Clean or Broken: verify netns and container match sled
            DirtyState::Clean | DirtyState::Broken(_) => {
                if faas_containerd::impls::cni::cni_impl::netns_get_ip(&endpoint).is_some() {
                    log::debug!("startup: {} netns present, sled {:?}", endpoint, meta.dirty);
                    repaired += 1;
                    continue;
                }
                // Netns absent — check if container still exists
                if backend().container_exists(&endpoint).await {
                    log::warn!(
                        "startup: {} has container but no netns, keeping sled {:?}",
                        endpoint,
                        meta.dirty
                    );
                    continue;
                }
                // Both netns and container absent → stale sled record
                log::warn!(
                    "startup: {} stale sled record (no netns, no container), removing",
                    endpoint
                );
                provider.cache.remove(&endpoint).ok();
                cleaned += 1;
            }
        }
    }

    // ── Phase 2: sweep netns for orphans (no sled record) ───────
    let dir_iter = match fs::read_dir("/var/run/netns") {
        Ok(d) => d,
        Err(e) => {
            log::warn!("startup: cannot read /var/run/netns: {e}");
            return (repaired, cleaned);
        }
    };

    for entry in dir_iter {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        let endpoint = match parse_ns_endpoint(&name_str) {
            Some(ep) => ep,
            None => continue,
        };

        // Already handled in Phase 1 (sled record exists)
        if provider.cache.get(&endpoint).ok().flatten().is_some() {
            continue;
        }

        // Orphan netns: no sled record → cleanup
        log::warn!("startup: orphan netns {} — cleaning up", name_str);
        faas_containerd::provider::function::delete::cleanup_containerd_resources(
            &provider.cache,
            &endpoint,
        )
        .await;
        if let Ok(ns) = netns_rs::NetNs::get(endpoint.to_string()) {
            ns.remove().ok();
        }
        cleaned += 1;
    }

    (repaired, cleaned)
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let rt = asupersync::runtime::RuntimeBuilder::new()
        .worker_threads(4)
        .build()
        .expect("Failed to build asupersync runtime");

    let handle = rt.handle();

    rt.block_on(async move {
        // ── Init containerd backend ─────────────────────────────────
        faas_containerd::init_backend(handle.clone());

        // ── Init provider ───────────────────────────────────────────
        let provider = faas_containerd::provider::ContainerdProvider::new(
            DEFAULT_FAASDRS_DATA_DIR,
            handle.clone(),
        );

        // ── Startup recovery: dual-track sled + netns scan ─────────
        let (repaired, cleaned) = startup_netns_scan(&provider).await;
        if repaired > 0 || cleaned > 0 {
            log::warn!(
                "startup netns scan: {} repaired, {} cleaned",
                repaired,
                cleaned
            );
        }

        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let cache = Arc::new(GatewayCache::new());

        // ── Start HTTP gateway (background) ─────────────────────────
        let gw_handle = handle.clone();
        let gw_provider = provider.clone();
        let gw_cache = cache.clone();
        let _gateway = handle.spawn(async move {
            if let Err(e) = gateway::serve(gw_provider, gw_cache, port, &gw_handle).await {
                log::error!("Gateway server error: {e}");
                std::process::exit(1);
            }
        });

        // ── Wait for shutdown signal ────────────────────────────────
        asupersync::signal::ctrl_c()
            .await
            .unwrap_or_else(|e| log::error!("ctrl_c signal error: {e}"));
        log::info!("Received shutdown signal, draining...");
    });
}
