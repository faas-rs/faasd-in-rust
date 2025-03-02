use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDeployment {
    pub service: String,
    pub image: String,
    pub namespace: Option<String>,
    pub env_process: Option<String>,
    pub env_vars: Option<HashMap<String, String>>,
    pub constraints: Option<Vec<String>>,
    pub secrets: Option<Vec<String>>,
    pub labels: Option<HashMap<String, String>>,
    pub annotations: Option<HashMap<String, String>>,
    pub limits: Option<FunctionResources>,
    pub requests: Option<FunctionResources>,
    pub read_only_root_filesystem: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionResources {
    pub memory: Option<String>,
    pub cpu: Option<String>,
    pub nvidia_gpu: Option<String>,
    pub amd_gpu: Option<String>,
    pub intel_gpu: Option<String>,
}
