use actix_web::{App, HttpServer, web};

use provider::{
    handlers::{delete::delete_handler, deploy::deploy_handler},
    proxy::proxy_handler::proxy_handler,
    types::config::FaaSConfig,
};
use service::containerd_manager::{ContainerdManager,CtrInstance};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::time::sleep;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let socket_path = std::env::var("SOCKET_PATH")
        .unwrap_or_else(|_| "/run/containerd/containerd.sock".to_string());
    CtrInstance::init(&socket_path).await;
    let containerdmanager = ContainerdManager::new();
    let containerdmanager_clone = containerdmanager.clone();
    let faas_config = FaaSConfig::new();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(faas_config.clone()))
            .app_data(web::Data::new(containerdmanager.clone()))
            .route("/system/functions", web::post().to(deploy_handler))
            .route("/system/functions", web::delete().to(delete_handler))
            .route("/function/{name}{path:/?.*}", web::to(proxy_handler))
            
        // 更多路由配置...
    })
    .bind("0.0.0.0:8090")?
        // disable default signal handling
        .disable_signals()
        .run();

    let server_handle = server.handle();
    let task_shutdown_marker = Arc::new(AtomicBool::new(false));

    let server_task = tokio::spawn(server);

    let shutdown = tokio::spawn(async move {
        // listen for ctrl-c
        tokio::signal::ctrl_c().await.unwrap();
        containerdmanager_clone.get_self()
        .write()
        .unwrap()
        .clear();
        sleep(Duration::from_secs(3)).await;
        // start shutdown of tasks
        let server_stop = server_handle.stop(true);
        task_shutdown_marker.store(true, Ordering::SeqCst);

        // await shutdown of tasks
        server_stop.await;
    });

    let _ = tokio::try_join!(server_task, shutdown).expect("unable to join tasks");

    Ok(())
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
