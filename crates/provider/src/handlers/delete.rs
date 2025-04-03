use crate::{
    consts,
    handlers::{
        function_get::get_function,
        utils::{CustomError, map_service_error},
    },
    types::config::IAmHandler,
};
use actix_web::web::BytesMut;
use actix_web::{Error, HttpRequest, HttpResponse, Responder, error, web};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use service::Service;
use std::sync::Arc;
pub struct DeleteHandler {
    service: Arc<Service>,
}

impl IAmHandler for DeleteHandler {
    type Input = DeleteContainerInfo;

    async fn execute(&mut self, input: Self::Input) -> impl Responder {
        let mut payload = input.payload;
        let body = match get_delete_data_from_req(&mut payload).await {
            Ok(body) => body,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .body(format!("Failed to read request body: {}", e));
            }
        };
        let request: DeleteContainerRequest = match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(e) => {
                eprintln!("Failed to parse body: {}", e);
                return HttpResponse::BadRequest().body(format!("Invalid payload: {}", e));
            }
        };
        let namespace = request
            .namespace
            .clone()
            .filter(|ns| !ns.is_empty())
            .unwrap_or_else(|| consts::DEFAULT_FUNCTION_NAMESPACE.to_string());
        let function_name = request.function_name;

        match delete(&function_name, &namespace, &self.service).await {
            Ok(_) => HttpResponse::Ok().body(format!("function {} deleted successfully", function_name)),
            Err(e) => HttpResponse::InternalServerError().body(format!(
                    "failed to delete function {} in namespace {} because {}",
                    function_name,namespace, e
            )),
        }
    }
}

async fn delete(
    function_name: &str,
    namespace: &str,
    service: &Arc<Service>,
) -> Result<(), CustomError> {
    let namespaces = service
        .list_namespaces()
        .await
        .map_err(|e| map_service_error(e))?;
    if !namespaces.contains(&namespace.to_string()) {
        return Err(CustomError::ActixError(error::ErrorBadRequest(format!(
            "Namespace '{}' not valid or does not exist",
            namespace
        ))));
    }
    let function = match get_function(&service, &function_name, &namespace).await {
        Ok(function) => function,
        Err(e) => {
            return Err(CustomError::FunctionError(e));
        }
    };
    if function.replicas != 0 {
        cni::cni_network::delete_cni_network(namespace, &function_name);
    }
    match service.remove_container(&function_name, &namespace).await {
        Ok(_) => Ok(()),
        Err(e) => {
            return Err(CustomError::OtherError(e.to_string()));
        }
    }
}

async fn get_delete_data_from_req(payload: &mut web::Payload) -> Result<BytesMut, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
pub struct DeleteContainerInfo {
    req: HttpRequest,
    payload: web::Payload,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteContainerRequest {
    pub function_name: String,
    pub namespace: Option<String>,
}
