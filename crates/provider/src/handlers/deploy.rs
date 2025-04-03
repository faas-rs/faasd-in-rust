use crate::{
    consts,
    handlers::utils::{CustomError, map_service_error},
    types::{
        config::IAmHandler,
        function_deployment::{DeployFunctionInfo, FunctionDeployment},
    },
};
use actix_web::web::BytesMut;
use actix_web::{Error, HttpResponse, Responder, error, web};
use futures::StreamExt;
use service::Service;
use std::sync::Arc;

pub struct DeployHandler {
    //pub config:FunctionDeployment,
    service: Arc<Service>,
}

impl IAmHandler for DeployHandler {
    type Input = DeployFunctionInfo;

    async fn execute(&mut self, input: Self::Input) -> impl Responder {
        let req = input.req.clone();
        let mut payload = input.payload;

        let body = match get_deploy_data_from_req(&mut payload).await {
            Ok(body) => body,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .body(format!("Failed to read request body: {}", e));
            }
        };
        //let body_str = String::from_utf8(body.to_vec()).unwrap();
        //self.config = serde_json::from_str(&body_str).unwrap();
        let config: FunctionDeployment = match serde_json::from_slice(&body) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to parse body: {}", e);
                return HttpResponse::BadRequest().body(format!("Invalid payload: {}", e));
            }
        };

        let namespace = config
            .namespace
            .clone()
            .filter(|ns| !ns.is_empty())
            .unwrap_or_else(|| consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

        match deploy(&self.service, &config, &namespace).await {
            Ok(_) => HttpResponse::Accepted().body(format!(
                "Function {} deployment initiated successfully in namespace '{}'.",
                config.service, namespace
            )),
            Err(e) => HttpResponse::InternalServerError().body(format!(
                "failed to deploy function {} in namespace {} because {}",
                config.service, namespace, e
            )),
        }
    }
}

async fn deploy(
    service: &Arc<Service>,
    config: &FunctionDeployment,
    namespace: &str,
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
    println!("Namespace '{}' validated.", namespace);

    let container_list = service
        .get_container_list(namespace)
        .await
        .map_err(|e| CustomError::from(e))?;

    if container_list.contains(&config.service) {
        return Err(CustomError::OtherError(
            "container has been existed".to_string(),
        ));
    }

    service
        .prepare_image(&config.image, namespace, false)
        .await
        .map_err(map_service_error)?;
    println!("Image '{}' validated", &config.image);

    service
        .create_container(&config.image, &config.service, namespace)
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;

    println!(
        "Container {} created using image {} in namespace {}",
        &config.service, &config.image, namespace
    );

    service
        .create_and_start_task(&config.service, namespace)
        .await
        .map_err(|e| {
            CustomError::OtherError(format!("failed to start task for container {}", e))
        })?;
    println!(
        "Task for container {} was created successfully",
        &config.service
    );

    Ok(())
}

async fn get_deploy_data_from_req(payload: &mut web::Payload) -> Result<BytesMut, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
