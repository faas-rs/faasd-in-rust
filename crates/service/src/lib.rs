pub mod containerd_manager;
pub mod image_manager;
pub mod spec;
pub mod systemd;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

// config.json,dockerhub密钥
// const DOCKER_CONFIG_DIR: &str = "/var/lib/faasd/.docker/";

type NetnsMap = Arc<RwLock<HashMap<String, NetworkConfig>>>;
lazy_static::lazy_static! {
    static ref GLOBAL_NETNS_MAP: NetnsMap = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    netns: String,
    ip: String,
    ports: Vec<String>,
}

impl NetworkConfig {
    pub fn new(netns: String, ip: String, ports: Vec<String>) -> Self {
        NetworkConfig { netns, ip, ports }
    }

    pub fn get_netns(&self) -> String {
        self.netns.clone()
    }

    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }

    pub fn get_address(&self) -> String {
        format!(
            "{}:{}",
            self.ip.split('/').next().unwrap_or(""),
            self.ports[0].split('/').next().unwrap_or("")
        )
    }
}
