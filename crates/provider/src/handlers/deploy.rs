use crate::{
    consts,
    handlers::utils::CustomError,
    types::function_deployment::{DeployFunctionInfo, FunctionDeployment},
};
use actix_web::{HttpResponse, Responder, web};

use service::{containerd_manager::ContainerdManager, image_manager::ImageManager};

pub async fn deploy_handler(info: web::Json<DeployFunctionInfo>) -> impl Responder {
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

    match deploy(&config).await {
        Ok(()) => HttpResponse::Accepted().body(format!(
            "Function {} deployment initiated successfully.",
            config.service
        )),
        Err(e) => HttpResponse::InternalServerError().body(format!(
            "failed to deploy function {}, because {}",
            config.service, e
        )),
    }
}

async fn deploy(config: &FunctionDeployment) -> Result<(), CustomError> {
    let namespace = config.namespace.clone().unwrap();

    log::info!(
        "Namespace '{}' validated.",
        config.namespace.clone().unwrap()
    );

    let container_list = ContainerdManager::list_container_into_string(&namespace)
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to list container:{}", e)))?;

    if container_list.contains(&config.service) {
        return Err(CustomError::OtherError(
            "container has been existed".to_string(),
        ));
    }

    ImageManager::prepare_image(&config.image, &namespace, true)
        .await
        .map_err(CustomError::from)?;
    log::info!("Image '{}' validated ,", &config.image);

    ContainerdManager::create_container(&config.image, &config.service, &namespace)
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;

    log::info!(
        "Container {} created using image {} in namespace {}",
        &config.service,
        &config.image,
        namespace
    );

    ContainerdManager::new_task(&config.service, &namespace)
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
