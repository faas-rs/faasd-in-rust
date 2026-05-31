//! faas-containerd entry point — asupersync runtime.
//!
//! Boots the asupersync Runtime, initializes the containerd gRPC backend,
//! and serves the gateway HTTP server.  containerd is the authoritative
//! state machine; sled caches only IP addresses.

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
