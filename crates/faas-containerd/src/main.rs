use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub mod consts;
pub mod impls;
pub mod provider_impl;
pub mod systemd;

fn main() {
    let provider = Arc::new(provider_impl::CtrdProvider {
        ctr_instance_map: Mutex::new(HashMap::new()),
        task_tracker: tokio_util::task::TaskTracker::new(),
    });

    provider::bootstrap::serve(provider).unwrap_or_else(|e| {
        log::error!("Failed to start server: {}", e);
        std::process::exit(1);
    });
}
