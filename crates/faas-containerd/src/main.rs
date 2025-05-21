use faas_containerd::consts::DEFAULT_FAASDRS_DATA_DIR;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    faas_containerd::init_backend().await;
    let provider = faas_containerd::provider::ContainerdProvider::new(DEFAULT_FAASDRS_DATA_DIR);

    // leave for shutdown containers (stop tasks)
    let _handle = provider.clone();

    tokio::spawn(async move {
        log::info!("Setting up signal handlers for graceful shutdown");
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();
        tokio::select! {
            _ = sigint.recv() => log::info!("SIGINT received, starting graceful shutdown..."),
            _ = sigterm.recv() => log::info!("SIGTERM received, starting graceful shutdown..."),
            _ = sigquit.recv() => log::info!("SIGQUIT received, starting graceful shutdown..."),
        }
        // for (_q, ctr) in handle.ctr_instance_map.lock().await.drain() {
        //     let _ = ctr.delete().await;
        // }
        log::info!("Successfully shutdown all containers");
    });

    gateway::bootstrap::serve(provider)
        .unwrap_or_else(|e| {
            log::error!("Failed to start server: {}", e);
            std::process::exit(1);
        })
        .await
}
