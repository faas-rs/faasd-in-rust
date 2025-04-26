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

// openfaas API文档和faasd源码的响应不能完全对齐，这里参考源码的响应码设置
// 考虑到部分操作可能返回500错误，但是faasd并没有做internal server error的处理（可能上层有中间件捕获），这里应该需要做500的处理
pub async fn function_list_handler(req: HttpRequest) -> impl Responder {
    let namespace = req.match_info().get("namespace").unwrap_or("");
    if namespace.is_empty() {
        return HttpResponse::BadRequest().body("provide namespace in path");
    }
    let namespaces = match ContainerdManager::list_namespaces().await {
        Ok(namespace) => namespace,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list namespaces:{}", e));
        }
    };
    if !namespaces.contains(&namespace.to_string()) {
        return HttpResponse::BadRequest()
            .body(format!("Namespace '{}' does not exist", namespace));
    }

    let container_list = match ContainerdManager::list_container_into_string(namespace).await {
        Ok(container_list) => container_list,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list container:{}", e));
        }
    };
    log::info!("container_list: {:?}", container_list);

    match get_function_list(container_list, namespace).await {
        Ok(functions) => HttpResponse::Ok().body(serde_json::to_string(&functions).unwrap()),
        Err(e) => HttpResponse::BadRequest().body(format!("Failed to get function list: {}", e)),
    }
}

async fn get_function_list(
    container_list: Vec<String>,
    namespace: &str,
) -> Result<Vec<Function>, CustomError> {
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
