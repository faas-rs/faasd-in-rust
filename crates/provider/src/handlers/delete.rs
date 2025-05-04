use super::function_list::Function;
use crate::{
    consts,
    handlers::{function_get::get_function, utils::CustomError},
};
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use service::containerd_manager::ContainerdManager;

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

    let _function =
        match get_function(&function_name, &namespace, containerd_manager.as_ref()).await {
            Ok(function) => function,
            Err(e) => {
                log::error!("Failed to get function: {}", e);
                return HttpResponse::NotFound()
                    .body(format!("Function '{}' not found ", function_name));
            }
        };

    match delete(&_function, &namespace, &containerd_manager).await {
        Ok(()) => {
            HttpResponse::Ok().body(format!("Function {} deleted successfully.", function_name))
        }
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Failed to delete function: {}", e))
        }
    }
}

async fn delete(
    function: &Function,
    namespace: &str,
    containerd_manager: &ContainerdManager,
) -> Result<(), CustomError> {
    let function_name = function.name.clone();
    if function.replicas != 0 {
        log::info!("function.replicas: {:?}", function.replicas);
        cni::delete_cni_network(namespace, &function_name);
        log::info!("delete_cni_network ok");
    } else {
        log::info!("function.replicas: {:?}", function.replicas);
    }

    containerd_manager
        .delete_ctrinstance((String::from(namespace), function_name))
        .await;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerInfo {
    pub function_name: String,
    pub namespace: Option<String>,
}
