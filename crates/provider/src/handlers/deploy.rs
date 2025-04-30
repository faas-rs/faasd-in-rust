use crate::{consts, handlers::utils::CustomError, types::function_deployment::DeployFunctionInfo};
use actix_web::{HttpResponse, Responder, web};

use service::{containerd_manager::ContainerdManager, image_manager::ImageManager};

// 参考响应状态 https://github.com/openfaas/faas/blob/7803ea1861f2a22adcbcfa8c79ed539bc6506d5b/api-docs/spec.openapi.yml#L121C1-L140C45
// 请求体反序列化失败，自动返回400错误
pub async fn deploy_handler(info: web::Json<DeployFunctionInfo>) -> impl Responder {
    let image = info.image.clone();
    let function_name = info.function_name.clone();
    let namespace = info
        .namespace
        .clone()
        .unwrap_or(consts::DEFAULT_FUNCTION_NAMESPACE.to_string());

    log::info!("Namespace '{}' validated.", &namespace);

    let container_list = match ContainerdManager::list_container_into_string(&namespace).await {
        Ok(container_list) => container_list,
        Err(e) => {
            log::error!("Failed to list container: {}", e);
            return HttpResponse::InternalServerError()
                .body(format!("Failed to list container: {}", e));
        }
    };

    if container_list.contains(&function_name) {
        return HttpResponse::BadRequest().body(format!(
            "Function '{}' already exists in namespace '{}'",
            function_name, namespace
        ));
    }

    match deploy(&function_name, &image, &namespace).await {
        Ok(()) => HttpResponse::Accepted().body(format!(
            "Function {} deployment initiated successfully.",
            function_name
        )),
        Err(e) => HttpResponse::BadRequest().body(format!(
            "failed to deploy function {}, because {}",
            function_name, e
        )),
    }
}

async fn deploy(function_name: &str, image: &str, namespace: &str) -> Result<(), CustomError> {
    ImageManager::prepare_image(image, namespace, true)
        .await
        .map_err(CustomError::from)?;
    log::info!("Image '{}' validated ,", image);

    ContainerdManager::create_container(image, function_name, namespace)
        .await
        .map_err(|e| CustomError::OtherError(format!("failed to create container:{}", e)))?;

    log::info!(
        "Container {} created using image {} in namespace {}",
        function_name,
        image,
        namespace
    );

    ContainerdManager::new_task(function_name, namespace)
        .await
        .map_err(|e| {
            CustomError::OtherError(format!(
                "failed to start task for container {},{}",
                function_name, e
            ))
        })?;
    log::info!(
        "Task for container {} was created successfully",
        function_name
    );

    Ok(())
}
