use std::{collections::HashMap, time::SystemTime};

use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use service::containerd_manager::ContainerdManager;

use super::{function_get::get_function, utils::CustomError};

#[derive(Debug, Serialize, Deserialize)]
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

pub async fn function_list_handler(req: HttpRequest) -> impl Responder {
    let namespace = req.match_info().get("namespace").unwrap_or("");
    if namespace.is_empty() {
        return HttpResponse::BadRequest().body("provide namespace in path");
    }
    match get_function_list(namespace).await {
        Ok(functions) => HttpResponse::Ok().body(serde_json::to_string(&functions).unwrap()),
        Err(e) => HttpResponse::from_error(e),
    }
}

async fn get_function_list(namespace: &str) -> Result<Vec<Function>, CustomError> {
    let namespaces = match ContainerdManager::list_namespaces().await {
        Ok(namespace) => namespace,
        Err(e) => {
            return Err(CustomError::OtherError(format!(
                "Failed to list namespaces:{}",
                e
            )));
        }
    };
    if !namespaces.contains(&namespace.to_string()) {
        return Err(CustomError::OtherError(format!(
            "Namespace '{}' not valid or does not exist",
            namespace
        )));
    }
    let container_list = match ContainerdManager::list_container_into_string(namespace).await {
        Ok(container_list) => container_list,
        Err(e) => {
            return Err(CustomError::OtherError(format!(
                "Failed to list container:{}",
                e
            )));
        }
    };
    log::info!("container_list: {:?}", container_list);
    let mut functions: Vec<Function> = Vec::new();
    for cid in container_list {
        log::info!("cid: {}", cid);
        let function = match get_function(&cid, namespace).await {
            Ok(function) => function,
            Err(e) => return Err(CustomError::FunctionError(e)),
        };
        functions.push(function);
    }
    Ok(functions)
}
