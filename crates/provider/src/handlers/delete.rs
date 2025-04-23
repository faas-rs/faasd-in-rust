use crate::{
    consts,
    handlers::{function_get::get_function, utils::CustomError},
};
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use service::containerd_manager::{ContainerdManager, CtrInstance};

use super::function_list::Function;

// 参考响应状态：https://github.com/openfaas/faas/blob/7803ea1861f2a22adcbcfa8c79ed539bc6506d5b/api-docs/spec.openapi.yml#L141C2-L162C45
// 请求体反序列化失败，自动返回400错误
pub async fn delete_handler(
    info: web::Json<DeleteContainerInfo>,
    containerd_manager: web::Data<ContainerdManager>,
) -> impl Responder {
    let function_name = info.function_name.clone();
    let namespace = info
        .namespace
        .clone()
        .unwrap_or_else(|| consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

    match delete(&function_name, &namespace, containerd_manager.get_ref()).await {
        Ok(()) => {
            HttpResponse::Ok().body(format!("function {} deleted successfully", function_name))
        }
        Err(e) => e.error_response(),
    }
}

async fn delete(
    function_name: &str,
    namespace: &str,
    containerd_manager: &ContainerdManager,
) -> Result<(), CustomError> {
    let namespaces = CtrInstance::list_namespaces().await.unwrap();
    if !namespaces.contains(&namespace.to_string()) {
        return HttpResponse::NotFound().body(format!("Namespace '{}' does not exist", namespace));
    }
    let _function = match get_function(&function_name, &namespace, containerd_manager).await {
        Ok(function) => function,
        Err(e) => {
            log::error!("Failed to get function: {}", e);
            return HttpResponse::NotFound()
                .body(format!("Function '{}' not found ", function_name));
        }
    };    /*match delete(&function, &namespace).await {
        Ok(()) => {
            HttpResponse::Ok().body(format!("Function {} deleted successfully.", function_name))
        }
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Failed to delete function: {}", e))
        }
    }
}

async fn delete(function: &Function, namespace: &str) -> Result<(), CustomError> {
    let function_name = function.name.clone();
    if function.replicas != 0 {
        log::info!("function.replicas: {:?}", function.replicas);
        cni::delete_cni_network(namespace, &function_name);
        log::info!("delete_cni_network ok");
    } else {
        log::info!("function.replicas: {:?}", function.replicas);
    }
    ContainerdManager::delete_container(&function_name, namespace)
        .await
        .map_err(|e| {
            log::error!("Failed to delete container: {}", e);
            CustomError::OtherError(format!("Failed to delete container: {}", e))
        })?;
    Ok(())*/
    containerd_manager.remove_from_manager((String::from(namespace), String::from(function_name)));
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerInfo {
    pub function_name: String,
    pub namespace: Option<String>,
}
