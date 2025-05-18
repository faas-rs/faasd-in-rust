pub mod cni;
pub mod container;
pub mod error;
pub mod function;
pub mod oci_image;
pub mod snapshot;
pub mod spec;
pub mod task;

lazy_static::lazy_static! {
    pub static ref BACKEND: ContainerdService = ContainerdService::new();
}

const DEFAULT_CTRD_SOCK: &str = "/run/containerd/containerd.sock";

#[derive(Clone)]
pub struct ContainerdService {
    pub client: std::sync::Arc<containerd_client::Client>,
}

impl Default for ContainerdService {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerdService {
    /// This is poor design, but we need to create the client in a tokio runtime
    /// so that we can use it like a singleton.
    pub fn new() -> Self {
        let handle = tokio::spawn(async {
            containerd_client::Client::from_path(
                std::env::var("SOCKET_PATH").unwrap_or(String::from(DEFAULT_CTRD_SOCK)),
            )
            .await
            .expect("Failed to create containerd client")
        });
        let client = tokio::runtime::Handle::current()
            .block_on(handle)
            .expect("Failed to create containerd client");
        ContainerdService {
            client: std::sync::Arc::new(client),
        }
    }
}
