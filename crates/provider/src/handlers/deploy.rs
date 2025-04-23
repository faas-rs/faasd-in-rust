use crate::{
    consts,
    handlers::utils::CustomError,
    types::function_deployment::{DeployFunctionInfo, FunctionDeployment},
};
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder, web};
use service::{
    containerd_manager::{ContainerdManager, CtrInstance},
    image_manager::ImageManager,
};
// 参考响应状态 https://github.com/openfaas/faas/blob/7803ea1861f2a22adcbcfa8c79ed539bc6506d5b/api-docs/spec.openapi.yml#L121C1-L140C45
// 请求体反序列化失败，自动返回400错误
pub async fn deploy_handler(
    info: web::Json<DeployFunctionInfo>,
    containerd_manager: web::Data<ContainerdManager>,
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

    match deploy(&config, containerd_manager).await {
        Ok(()) => HttpResponse::Accepted().body(format!(
            "Function {} deployment initiated successfully .",
            function_name
        )),
        Err(e) => HttpResponse::BadRequest().body(format!(
            "failed to deploy function {}, because {}",
            function_name, e
        )),
    }
}

async fn deploy(
    config: &FunctionDeployment,
    containerd_manager: Data<ContainerdManager>,
) -> Result<(), CustomError> {
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
    log::info!("Image '{}' validated ,", image);

    let mut ctr = CtrInstance::new(
        String::from(&config.service),
        String::from(&config.image),
        String::from(&namespace),
    )
    .await
    .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;

    CtrInstance::create_and_start_task(&mut ctr)
        .await
        .map_err(|e| {
            CustomError::OtherError(format!(
                "failed to start task for container {},{}",
                function_name, e
            ))
        })?;

    containerd_manager.get_ref().insert_to_manager(
        (String::from(&namespace), String::from(&config.service)),
        ctr,
    );
    log::info!(
        "Container {} created using image {} in namespace {}",
        &config.service,
        &config.image,
        namespace
    );
    log::info!(
        "Task for container {} was created successfully",
        function_name
    );

    Ok(())
}
