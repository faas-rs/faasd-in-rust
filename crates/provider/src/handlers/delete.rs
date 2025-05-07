use crate::{consts, handlers::utils::CustomError};
use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use service::containerd_manager::ContainerdManager;

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
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Failed to delete function: {}", e))
        }
    }
}

async fn delete(
    function_name: &str,
    namespace: &str,
    containerd_manager: &ContainerdManager,
) -> Result<(), CustomError> {
    let _namespaces = ContainerdManager::list_namespaces().await.unwrap();

    containerd_manager
        .delete_ctrinstance((String::from(namespace), String::from(function_name)))
        .await;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerInfo {
    pub function_name: String,
    pub namespace: Option<String>,
}
