pub mod consts;
pub mod impls;
pub mod provider;
pub mod systemd;

use tokio::signal::unix::{SignalKind, signal};
use tokio_util::task::TaskTracker;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let provider = provider::ContainerdProvider::new();
    let handle = provider.clone();

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
        let tracker = TaskTracker::new();
        handle
            .ctr_instance_map
            .lock()
            .await
            .drain()
            .for_each(|(_q, container)| {
                tracker.spawn(async move {
                    let _ = container.delete().await;
                });
            });
        tracker.close();
        tracker.wait().await;
        log::info!("Successfully shutdown all containers");
    });

    gateway::bootstrap::serve(provider)
        .unwrap_or_else(|e| {
            log::error!("Failed to start server: {}", e);
            std::process::exit(1);
        })
        .await
}
