use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use service::Service;

pub mod handlers;
pub mod types;

use handlers::*;
use provider::{
    handlers::{delete::delete_handler, deploy::deploy_handler},
    proxy::proxy_handler::proxy_handler,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let service = Arc::new(
        Service::new("/run/containerd/containerd.sock".to_string())
            .await
            .unwrap(),
    );

    println!("I'm running!");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(service.clone()))
            .route("/create-container", web::post().to(create_container))
            .route("/remove-container", web::post().to(remove_container))
            .route("/containers", web::get().to(get_container_list))
            .route("/system/functions", web::post().to(deploy_handler))
            .route("/system/functions", web::delete().to(delete_handler))
            .route("/function/{name}{path:/?.*}", web::to(proxy_handler))
        // 更多路由配置...
    })
    .bind("0.0.0.0:8090")?;

    println!("0.0.0.0:8090");

    server.run().await
}
