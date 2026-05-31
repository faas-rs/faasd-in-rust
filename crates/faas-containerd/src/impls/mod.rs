pub mod cni;
pub mod container;
pub mod error;
pub mod function;
pub mod namespace;
pub mod oci_image;
pub mod snapshot;
pub mod spec;
pub mod task;

use std::sync::OnceLock;

use asupersync::runtime::RuntimeHandle;
use containerd_client::Client;

use crate::tonic_bridge;

pub static __BACKEND: OnceLock<ContainerdService> = OnceLock::new();

pub(crate) fn backend() -> &'static ContainerdService {
    __BACKEND.get().unwrap()
}

/// Initialize the containerd backend.
///
/// Builds a tonic `Channel` using asupersync's runtime for all I/O and
/// task spawning — no tokio runtime needed.
pub fn init_backend(handle: RuntimeHandle) {
    let socket =
        std::env::var("SOCKET_PATH").unwrap_or(crate::consts::DEFAULT_CTRD_SOCK.to_string());

    let channel = tonic_bridge::connect_channel(handle, &socket);
    let client = Client::from(channel);

    __BACKEND
        .set(ContainerdService { client })
        .ok()
        .expect("ContainerdService already initialized");

    cni::init_cni_network().unwrap();
}

pub struct ContainerdService {
    pub client: containerd_client::Client,
}
