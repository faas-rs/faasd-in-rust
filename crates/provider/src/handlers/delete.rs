use crate::{
    consts,
    handlers::{
        function_get::get_function,
        utils::{CustomError, map_service_error},
    },
};
use actix_web::{HttpResponse, Responder, error, web};
use serde::{Deserialize, Serialize};
use service::Service;
use std::sync::Arc;

pub async fn delete_handler(
    service: web::Data<Arc<Service>>,
    info: web::Json<DeleteContainerInfo>,
) -> impl Responder {
    let function_name = info.function_name.clone();
    let namespace = info
        .namespace
        .clone()
        .unwrap_or_else(|| consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

    match delete(&function_name, &namespace, &service).await {
        Ok(()) => {
            HttpResponse::Ok().body(format!("function {} deleted successfully", function_name))
        }
        Err(e) => HttpResponse::InternalServerError().body(format!(
            "failed to delete function {} in namespace {} because {}",
            function_name, namespace, e
        )),
    }
}

async fn delete(
    function_name: &str,
    namespace: &str,
    service: &Arc<Service>,
) -> Result<(), CustomError> {
    let namespaces = service.list_namespaces().await.map_err(map_service_error)?;
    if !namespaces.contains(&namespace.to_string()) {
        return Err(CustomError::ActixError(error::ErrorBadRequest(format!(
            "Namespace '{}' not valid or does not exist",
            namespace
        ))));
    }
    let function = get_function(service, function_name, namespace).await?;
    if function.replicas != 0 {
        println!("  delete_cni_network ing {:?}", function.replicas);
        cni::delete_cni_network(namespace, function_name);
    } else {
        println!("  function.replicas {:?}", function.replicas);
    }
    service
        .remove_container(function_name, namespace)
        .await
        .map_err(map_service_error)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerInfo {
    pub function_name: String,
    pub namespace: Option<String>,
}
