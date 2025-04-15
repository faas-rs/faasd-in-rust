use crate::{
    consts,
    handlers::utils::CustomError,
    types::function_deployment::{DeployFunctionInfo, FunctionDeployment},
};
use actix_web::{HttpResponse, Responder, web};

use service::{Service, image_manager::ImageManager};
use std::sync::Arc;

pub async fn deploy_handler(
    service: web::Data<Arc<Service>>,
    info: web::Json<DeployFunctionInfo>,
) -> impl Responder {
    let image = info.image.clone();
    let function_name = info.function_name.clone();
    let namespace = info
        .namespace
        .clone()
        .unwrap_or(consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

    let config = FunctionDeployment {
        service: function_name,
        image,
        namespace: Some(namespace),
    };

    match deploy(&service, &config).await {
        Ok(()) => HttpResponse::Accepted().body(format!(
            "Function {} deployment initiated successfully .",
            config.service
        )),
        Err(e) => HttpResponse::InternalServerError().body(format!(
            "failed to deploy function {}, because {}",
            config.service, e
        )),
    }
}

async fn deploy(service: &Arc<Service>, config: &FunctionDeployment) -> Result<(), CustomError> {
    // let namespaces = service
    //     .list_namespaces()
    //     .await
    //     .map_err(|e| map_service_error(e))?;
    let namespace = config.namespace.clone().unwrap();

    // if !namespaces.contains(&namespace) {
    //     return Err(CustomError::ActixError(error::ErrorBadRequest(format!(
    //         "Namespace '{}' not valid or does not exist",
    //         namespace
    //     ))));
    // }
    log::info!(
        "Namespace '{}' validated.",
        config.namespace.clone().unwrap()
    );

    let container_list = service
        .get_container_list(&namespace)
        .await
        .map_err(CustomError::from)?;

    if container_list.contains(&config.service) {
        return Err(CustomError::OtherError(
            "container has been existed".to_string(),
        ));
    }

    //todo 这里暂时将client设为pub
    let client = service.client.as_ref();
    ImageManager::prepare_image(client, &config.image, &namespace, true)
        .await
        .map_err(CustomError::from)?;
    log::info!("Image '{}' validated ,", &config.image);

    service
        .create_container(&config.image, &config.service, &namespace)
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;

    log::info!(
        "Container {} created using image {} in namespace {}",
        &config.service,
        &config.image,
        namespace
    );

    service
        .create_and_start_task(&config.service, &namespace, &config.image)
        .await
        .map_err(|e| {
            CustomError::OtherError(format!(
                "failed to start task for container {},{}",
                &config.service, e
            ))
        })?;
    log::info!(
        "Task for container {} was created successfully",
        &config.service
    );

    Ok(())
}
