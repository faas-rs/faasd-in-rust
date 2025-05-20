pub mod cni;
pub mod container;
pub mod error;
pub mod function;
pub mod oci_image;
pub mod snapshot;
pub mod spec;
pub mod task;

use std::sync::OnceLock;

pub static __BACKEND: OnceLock<ContainerdService> = OnceLock::new();

pub(crate) fn backend() -> &'static ContainerdService {
    __BACKEND.get().unwrap()
}

/// TODO: Panic on failure, should be handled in a better way
pub async fn init_backend() {
    let socket =
        std::env::var("SOCKET_PATH").unwrap_or(crate::consts::DEFAULT_CTRD_SOCK.to_string());
    let client = containerd_client::Client::from_path(socket).await.unwrap();

    __BACKEND.set(ContainerdService { client }).ok().unwrap();
    cni::init_cni_network().unwrap();
}

pub struct ContainerdService {
    pub client: containerd_client::Client,
}
