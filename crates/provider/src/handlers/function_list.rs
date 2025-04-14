use std::{collections::HashMap, time::SystemTime};

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub namespace: String,
    pub image: String,
    pub pid: u32,
    pub replicas: i32,
    pub address: String,
    pub labels: HashMap<String, String>,
    // pub annotations: HashMap<String, String>,
    // pub secrets: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub env_process: String,
    // pub memory_limit: i64,
    pub created_at: SystemTime,
}
