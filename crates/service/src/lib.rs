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

type NetnsMap = Arc<RwLock<HashMap<String, NetworkConfig>>>;
lazy_static::lazy_static! {
    static ref GLOBAL_NETNS_MAP: NetnsMap = Arc::new(RwLock::new(HashMap::new()));
}

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

    pub fn extract_ns_cid(&self) -> Option<(&str, &str)> {
        let last_part = self.netns.as_str().split('/').next_back()?;
        // 按 '-' 分割最后一部分
        let parts: Vec<&str> = last_part.split('-').collect();
        // 确保有且仅有两个部分
        if parts.len() == 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    }
}
impl Drop for NetworkConfig {
    fn drop(&mut self) {
        let (ns, cid) = self.extract_ns_cid().unwrap();
        println!("111111111111111111111111111111111111111111111111111111111111111");
        delete_cni_network(ns, cid);
    }
}

lazy_static::lazy_static! {
    pub static ref CONTAINER_MAP: Arc<RwLock<HashMap<String,CtrInstance>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct CtrInstance {
    cid: String,
    image: String,
    ns: String,
    service: Arc<Service>,
    //net: Option<NetworkConfig>
}
impl CtrInstance {
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        service: Arc<Service>,
        cid: String,
        image: String,
        ns: String,
    ) -> Result<(), Err> {
        service
            .create_container(image.as_str(), cid.as_str(), ns.as_str())
            .await?;
        CONTAINER_MAP.write().unwrap().insert(
            cid.clone(),
            CtrInstance {
                cid,
                service,
                image,
                ns,
            },
        );
        Ok(())
    }
    pub async fn create_and_start_task(&self) -> Result<(), Err> {
        let (_, networkconfig) = self
            .service
            .create_and_start_task(&self.cid, &self.ns, &self.image)
            .await?;
        save_network_config(&self.cid, networkconfig);
        Ok(())
    }
}

impl Drop for CtrInstance {
    fn drop(&mut self) {
        let service = self.service.clone();
        let cid = self.cid.clone();
        let ns =self.ns.clone();
        let join = tokio::spawn(async move {
            let result = service.remove_container(cid.as_str(), ns.as_str()).await;
        });
        
    }
}


