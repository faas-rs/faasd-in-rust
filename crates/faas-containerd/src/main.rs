//! faas-containerd entry point — asupersync runtime.
//!
//! Boots the asupersync Runtime, initializes the containerd gRPC backend,
//! and serves the gateway HTTP server.  netns is the authoritative
//! state machine; sled is a write-through performance cache.
//!
//! On startup, scans `/var/run/netns/faasdrs-*` for orphaned netns
//! (crash between netns creation and sled write) and reconciles.
//!
//! Graceful shutdown via SIGINT/SIGTERM: ctrl_c() cancels all Cx, the
//! runtime drains in-flight deploy/delete tasks (cleanup runs), then exits.

use std::fs;

use faas_containerd::consts::DEFAULT_FAASDRS_DATA_DIR;

/// Parse a netns name like "faasdrs-default-hello" into an Endpoint.
fn parse_ns_endpoint(ns_name: &str) -> Option<faas_containerd::impls::cni::Endpoint> {
    let remainder = ns_name.strip_prefix("faasdrs-")?;
    let dash_pos = remainder.find('-')?;
    let ns = &remainder[..dash_pos];
    let fn_name = &remainder[dash_pos + 1..];
    Some(faas_containerd::impls::cni::Endpoint::new(fn_name, ns))
}

/// Scan `/var/run/netns/faasdrs-*` and reconcile each found netns
/// against sled and containerd.  Returns (repaired, cleaned).
async fn startup_netns_scan(
    provider: &faas_containerd::provider::ContainerdProvider,
) -> (usize, usize) {
    let mut repaired = 0usize;
    let mut cleaned = 0usize;

    let dir_iter = match fs::read_dir("/var/run/netns") {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot read /var/run/netns: {e}");
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

        // Only process faasd-managed netns
        let endpoint = match parse_ns_endpoint(&name_str) {
            Some(ep) => ep,
            None => continue,
        };

        // Normal path: sled has record for this endpoint → not an orphan
        if provider.cache.get(&endpoint).ok().flatten().is_some() {
            continue;
        }

        // Orphan netns: crash happened between netns creation and sled write
        log::warn!("Orphan netns found: {}", name_str);

        use faas_containerd::impls::backend;
        if backend().container_exists(&endpoint).await {
            // Container intact → repair sled from netns
            if let Some(ip) = faas_containerd::impls::cni::cni_impl::netns_get_ip(&endpoint) {
                // Insert into in-memory IP cache
                provider.resolved_ips.lock().unwrap().insert(endpoint.clone(), ip);

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis() as u64);
                let meta = faas_containerd::state::DeployMeta {
                    image: String::new(),
                    created_at: now,
                    labels: std::collections::HashMap::new(),
                    dirty: Some("crash recovery: orphan netns repaired".into()),
                };
                provider.cache.insert(&endpoint, &meta).ok();
                repaired += 1;
                log::info!("Orphan netns {} repaired with IP {}", name_str, ip);
                continue;
            }
        }

        // Container absent → full cleanup of orphan netns
        faas_containerd::provider::function::delete::cleanup_containerd_resources(
            &provider.cache,
            &endpoint,
        )
        .await;
        if let Ok(ns) = netns_rs::NetNs::get(endpoint.to_string()) {
            ns.remove().ok();
        }
        cleaned += 1;
        log::info!("Orphan netns {} cleaned up", name_str);
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
        let provider =
            faas_containerd::provider::ContainerdProvider::new(DEFAULT_FAASDRS_DATA_DIR);

        // ── Startup recovery: scan netns for orphans ───────────────
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

        // ── Start HTTP gateway (background) ─────────────────────────
        let gw_handle = handle.clone();
        let gw_provider = provider.clone();
        let _gateway = handle.spawn(async move {
            if let Err(e) = gateway::serve(gw_provider, port, &gw_handle).await {
                log::error!("Gateway server error: {e}");
                std::process::exit(1);
            }
        });

        // ── Wait for shutdown signal ────────────────────────────────
        // ctrl_c cancels all Cx → every cx.checkpoint() returns
        // Cancelled → deploy/delete cleanup runs during drain →
        // netns.remove() + cleanup_containerd_resources → clean exit.
        asupersync::signal::ctrl_c()
            .await
            .unwrap_or_else(|e| log::error!("ctrl_c signal error: {e}"));
        log::info!("Received shutdown signal, draining...");
    });
}
