pub mod containerd_manager;
pub mod image_manager;
pub mod spec;
pub mod systemd;


use cni::delete_cni_network;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

// config.json,dockerhub密钥
// const DOCKER_CONFIG_DIR: &str = "/var/lib/faasd/.docker/";



#[derive(Debug, Clone)]
pub struct NetworkConfig {
    ip: String,
    ports: Vec<String>,
}

impl NetworkConfig {
    pub fn new(ip: String, ports: Vec<String>) -> Self {
        NetworkConfig { ip, ports }
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
impl Drop for NetworkConfig {
    fn drop(&mut self) {
        
    }
}






