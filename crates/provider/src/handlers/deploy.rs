use crate::{
    consts,
    handlers::utils::CustomError,
    types::function_deployment::{DeployFunctionInfo, FunctionDeployment},
};
use actix_web::{HttpResponse, Responder, web};
use actix_web::web::Data;
use service::{containerd_manager::{self, ContainerdManager, CtrInstance}, image_manager::ImageManager};

pub async fn deploy_handler(info: web::Json<DeployFunctionInfo> , containerd_manager: web::Data<ContainerdManager>) -> impl Responder {
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

    match deploy(&config,containerd_manager).await {
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

async fn deploy(config: &FunctionDeployment,containerd_manager:Data<ContainerdManager>) -> Result<(), CustomError> {
    let namespace = config.namespace.clone().unwrap();

    log::info!(
        "Namespace '{}' validated.",
        config.namespace.clone().unwrap()
    );

    let container_list = CtrInstance::list_container_into_string(&namespace)
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

    let ctr = CtrInstance::new(String::from(&config.image), String::from(&config.service), String::from(&namespace))
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;
    containerd_manager.get_ref()
    .insert_to_manager((String::from(&namespace),String::from(&config.service)), ctr);
    log::info!(
        "Container {} created using image {} in namespace {}",
        &config.service,
        &config.image,
        namespace
    );

    CtrInstance::new_task(&config.service, &namespace, &config.image)
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
