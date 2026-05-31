use faas_containerd::provider::ContainerdProvider;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let provider = ContainerdProvider::new("/var/lib/faasdrs/data");

    // Ensure Docker network exists
    if let Err(e) = provider
        .docker
        .create_network(bollard::network::CreateNetworkOptions {
            name: "faasrs0",
            driver: "bridge",
            ..Default::default()
        })
        .await
    {
        // Network likely exists — ignore
        tracing::warn!("Network creation: {:?}", e);
    }

    let app = gateway::app(provider);
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("Starting gateway on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
