use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use provider::{
    handlers::{delete::delete_handler, deploy::deploy_handler, invoke_resolver::InvokeResolver},
    proxy::proxy_handler::proxy_handler,
    types::config::FaaSConfig,
};
use service::Service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let service = Arc::new(
        Service::new("/run/containerd/containerd.sock")
            .await
            .unwrap(),
    );

    let resolver = Some(InvokeResolver::new(service.clone()).await);
    let faas_config = FaaSConfig::new();

    log::info!("I'm running!");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(service.clone()))
            .app_data(web::Data::new(resolver.clone()))
            .app_data(web::Data::new(faas_config.clone()))
            .route("/system/functions", web::post().to(deploy_handler))
            .route("/system/functions", web::delete().to(delete_handler))
            .route("/function/{name}{path:/?.*}", web::to(proxy_handler))
        // 更多路由配置...
    })
    .bind("0.0.0.0:8090")?;

    log::info!("Running on 0.0.0.0:8090...");

    server.run().await
}

// 测试env能够正常获取
#[cfg(test)]
mod tests {
    #[test]
    fn test_env() {
        dotenv::dotenv().ok();
        let result: Vec<(String, String)> = dotenv::vars().collect();
        let bin = std::env::var("CNI_BIN_DIR").unwrap_or_else(|_| "Not set".to_string());
        let conf = std::env::var("CNI_CONF_DIR").unwrap_or_else(|_| "Not set".to_string());
        let tool = std::env::var("CNI_TOOL").unwrap_or_else(|_| "Not set".to_string());
        log::debug!("CNI_BIN_DIR: {bin}");
        log::debug!("CNI_CONF_DIR: {conf}");
        log::debug!("CNI_TOOL: {tool}");
        // for (key, value) in &result {
        //     println!("{}={}", key, value);
        // }
        assert!(!result.is_empty());
    }
}
