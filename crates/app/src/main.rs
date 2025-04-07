use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use service::Service;

pub mod handlers;
pub mod types;

use handlers::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
