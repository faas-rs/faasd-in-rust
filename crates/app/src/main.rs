use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use service::Service;

pub mod handlers;
pub mod types;

use handlers::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let service = Arc::new(
        Service::new("/run/containerd/containerd.sock")
            .await
            .unwrap(),
    );

    println!("I'm running!");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(service.clone()))
            .route("/create-container", web::post().to(create_container))
            .route("/remove-container", web::post().to(remove_container))
            .route("/containers", web::get().to(get_container_list))
        // 更多路由配置...
    })
    .bind("0.0.0.0:18080")?
    .run()
    .await
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
        println!("CNI_BIN_DIR: {bin}");
        println!("CNI_CONF_DIR: {conf}");
        println!("CNI_TOOL: {tool}");
        // for (key, value) in &result {
        //     println!("{}={}", key, value);
        // }
        assert!(!result.is_empty());
    }
}
