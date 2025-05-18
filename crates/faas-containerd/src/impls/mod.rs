pub mod cni;
pub mod container;
pub mod error;
pub mod function;
pub mod oci_image;
pub mod snapshot;
pub mod spec;
pub mod task;

use std::sync::LazyLock;

pub static BACKEND: LazyLock<ContainerdService> = LazyLock::new(ContainerdService::new);

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
    pub fn new() -> Self {
        let fetch_client = async {
            containerd_client::Client::from_path(
                std::env::var("SOCKET_PATH").unwrap_or(String::from(DEFAULT_CTRD_SOCK)),
            )
            .await
            .expect("Failed to create containerd client")
        };
        let client = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We're already in a runtime, use the current one
            tokio::task::block_in_place(|| handle.block_on(fetch_client))
        } else {
            // We're not in a runtime, create a new one
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(fetch_client)
        };
        ContainerdService {
            client: std::sync::Arc::new(client),
        }
    }
}
