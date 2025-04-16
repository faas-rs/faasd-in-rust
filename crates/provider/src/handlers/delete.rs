use crate::{
    consts,
    handlers::{function_get::get_function, utils::CustomError},
};
use actix_web::{HttpResponse, Responder, ResponseError, error, web};
use serde::{Deserialize, Serialize};
use service::containerd_manager::ContainerdManager;

pub async fn delete_handler(info: web::Json<DeleteContainerInfo>) -> impl Responder {
    let function_name = info.function_name.clone();
    let namespace = info
        .namespace
        .clone()
        .unwrap_or_else(|| consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

    match delete(&function_name, &namespace).await {
        Ok(()) => {
            HttpResponse::Ok().body(format!("function {} deleted successfully", function_name))
        }
        Err(e) => e.error_response(),
    }
}

async fn delete(function_name: &str, namespace: &str) -> Result<(), CustomError> {
    let namespaces = ContainerdManager::list_namespaces().await.unwrap();
    if !namespaces.contains(&namespace.to_string()) {
        return Err(CustomError::ActixError(error::ErrorBadRequest(format!(
            "Namespace '{}' not valid or does not exist",
            namespace
        ))));
    }
    let function = get_function(function_name, namespace).await.map_err(|e| {
        log::error!("Failed to get function: {}", e);
        CustomError::ActixError(error::ErrorNotFound(format!(
            "Function '{}' not found in namespace '{}'",
            function_name, namespace
        )))
    })?;
    if function.replicas != 0 {
        log::info!("function.replicas: {:?}", function.replicas);
        cni::delete_cni_network(namespace, function_name);
        log::info!("delete_cni_network ok");
    } else {
        log::info!("function.replicas: {:?}", function.replicas);
    }
    ContainerdManager::delete_container(function_name, namespace)
        .await
        .map_err(|e| {
            log::error!("Failed to delete container: {}", e);
            CustomError::ActixError(error::ErrorInternalServerError(format!(
                "Failed to delete container: {}",
                e
            )))
        })?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerInfo {
    pub function_name: String,
    pub namespace: Option<String>,
}
