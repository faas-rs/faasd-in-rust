//! faas-containerd entry point — asupersync runtime.
//!
//! Boots the asupersync Runtime, initializes the containerd gRPC backend,
//! and serves the gateway HTTP server.  containerd is the authoritative
//! state machine; sled caches only IP addresses.
//!
//! On startup, scans sled for Dirty records (from prior-iteration timeouts
//! where cleanup couldn't complete) and attempts recovery.

use faas_containerd::consts::DEFAULT_FAASDRS_DATA_DIR;

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
        let provider = faas_containerd::provider::ContainerdProvider::new(DEFAULT_FAASDRS_DATA_DIR);

        // ── Startup recovery: scan Dirty records ────────────────────
        // Dirty records represent functions whose cleanup timed out in a
        // prior process lifetime.  Attempt recovery before accepting traffic.
        let mut recovered = 0usize;
        let mut unrecovered = 0usize;

        for res in provider.cache.iter_dirty() {
            match res {
                Ok((endpoint, record)) => {
                    log::warn!(
                        "dirty record: {}, reason={}, attempts={}, dirty_at={}",
                        endpoint,
                        record.reason,
                        record.attempts,
                        record.dirty_at
                    );

                    // Attempt recovery: delete any leftover containerd resources
                    provider.recover_dirty(&endpoint, &record.reason).await;

                    // Check if still dirty after recovery attempt
                    if provider.cache.is_dirty(&endpoint).unwrap_or(false) {
                        unrecovered += 1;
                        log::error!(
                            "recovery failed for {}, reason={}, attempts={}",
                            endpoint,
                            record.reason,
                            record.attempts.saturating_add(1)
                        );
                        provider.cache.increment_dirty_attempts(&endpoint).ok();
                    } else {
                        recovered += 1;
                        log::info!("recovered dirty record: {}", endpoint);
                    }
                }
                Err(e) => {
                    log::error!("error iterating dirty records: {}", e);
                }
            }
        }

        if recovered > 0 || unrecovered > 0 {
            log::warn!(
                "startup recovery: {} recovered, {} still dirty",
                recovered,
                unrecovered
            );
        }

        // ── Start HTTP gateway ──────────────────────────────────────
        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        if let Err(e) = gateway::serve(provider, port, &handle).await {
            log::error!("Gateway server error: {e}");
            std::process::exit(1);
        }
    });
}
