use serde::{Deserialize, Serialize};

pub mod config;
pub mod function_deployment;

#[derive(Serialize, Deserialize)]
pub struct CreateContainerInfo {
    pub container_id: String,
    pub image: String,
    pub ns: String,
}
