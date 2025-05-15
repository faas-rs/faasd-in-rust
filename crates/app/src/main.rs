use actix_web::{App, HttpServer, web};
use provider::{
    handlers::{
        delete::delete_handler, deploy::deploy_handler, function_list::function_list_handler,
    },
    proxy::proxy_handler::proxy_handler,
    types::config::FaaSConfig,
};
use service::containerd_manager::{ContainerdManager, TRACKER};
use tokio::time::{Duration, sleep};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let socket_path = std::env::var("SOCKET_PATH")
        .unwrap_or_else(|_| "/run/containerd/containerd.sock".to_string());
    ContainerdManager::init(&socket_path).await;
    let ctr_instance_map = ContainerdManager::new();
    let ctr_instance_map_clone = ctr_instance_map.clone();
    let faas_config = FaaSConfig::new();

    log::info!("I'm running!");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(faas_config.clone()))
            .app_data(web::Data::new(ctr_instance_map.clone()))
            .route("/system/functions", web::post().to(deploy_handler))
            .route("/system/functions", web::delete().to(delete_handler))
            .route("/function/{name}{path:/?.*}", web::to(proxy_handler))
            .route(
                "/system/functions/{namespace}",
                web::get().to(function_list_handler),
            )
        // 更多路由配置...
    })
    .bind("0.0.0.0:8090")?
    // disable default signal handling
    .disable_signals()
    .run();

    let server_handle = server.handle();

    let server_task = tokio::spawn(server);

    let shutdown = tokio::spawn(async move {
        // listen for ctrl-c
        tokio::signal::ctrl_c().await.unwrap();
        ctr_instance_map_clone.get_self().lock().await.clear();
        //先停止服务器防止close后还有delete 请求
        server_handle.stop(true).await;
        sleep(Duration::from_millis(100)).await;
        TRACKER.close();
        TRACKER.wait().await;
        // start shutdown of tasks

        // await shutdown of tasks
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
