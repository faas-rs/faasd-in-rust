use actix_web::{
    App, HttpServer,
    dev::Server,
    web::{self, ServiceConfig},
};

use std::{collections::HashMap, sync::Arc};

use crate::{
    handlers,
    // metrics::HttpMetrics,
    provider::Provider,
    types::config::FaaSConfig,
};

pub fn config_app<P: Provider>(provider: Arc<P>) -> impl FnOnce(&mut ServiceConfig) {
    // let _registry = Registry::new();

    let provider = web::Data::from(provider);
    let app_state = web::Data::new(AppState {
        // metrics: HttpMetrics::new(),
        credentials: None,
    });
    move |cfg: &mut ServiceConfig| {
        cfg.app_data(app_state)
            .app_data(provider)
            .service(
                web::scope("/system")
                    .service(
                        web::resource("/functions")
                            .route(web::get().to(handlers::function::list::<P>))
                            .route(web::put().to(handlers::function::update::<P>))
                            .route(web::post().to(handlers::function::deploy::<P>))
                            .route(web::delete().to(handlers::function::delete::<P>)), // .route(web::put().to(handlers::update_function)),
                    )
                    .service(
                        web::resource("/function/{functionName}")
                            .route(web::get().to(handlers::function::status::<P>)),
                    ), //         .service(
                       //             web::resource("/scale-function/{name}")
                       //                 .route(web::post().to(handlers::scale_function)),
                       //         )
                       //         .service(web::resource("/info").route(web::get().to(handlers::info)))
                       //         .service(
                       //             web::resource("/secrets")
                       //                 .route(web::get().to(handlers::secrets))
                       //                 .route(web::post().to(handlers::secrets))
                       //                 .route(web::put().to(handlers::secrets))
                       //                 .route(web::delete().to(handlers::secrets)),
                       //         )
                       //         .service(web::resource("/logs").route(web::get().to(handlers::logs)))
                       //         .service(
                       //             web::resource("/namespaces")
                       //                 .route(web::get().to(handlers::list_namespaces))
                       //                 .route(web::post().to(handlers::mutate_namespace)),
                       //         ),
                       // )
            )
            .service(
                web::scope("/function").service(
                    web::resource(
                        "/{service_and_optional_namespace:[^{}/]+(?:\\.[^{}/]+)?}{rest_path:/?.*}",
                    )
                    .route(web::to(handlers::proxy::proxy::<P>)),
                ),
            );
        // .route("/metrics", web::get().to(handlers::telemetry))
        // .route("/healthz", web::get().to(handlers::health));
    }
}

//应用程序状态，存储共享的数据，如配置、指标、认证信息等，为业务函数提供支持
#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    // config: FaaSConfig,   //应用程序的配置，用于识别是否开启Basic Auth等
    // metrics: HttpMetrics, //用于监视http请求的持续时间和总数
    // metrics: HttpMetrics, //用于监视http请求的持续时间和总数
    credentials: Option<HashMap<String, String>>, //当有认证信息的时候，获取认证信息
}

// this is a blocking serve function
pub fn serve<P: Provider>(provider: Arc<P>) -> std::io::Result<Server> {
    log::info!("Checking config file");
    let config = FaaSConfig::new();
    let port = config.tcp_port.unwrap_or(8080);

    // 如果启用了Basic Auth，从指定路径读取认证凭证并存储在应用程序状态中
    // TODO: Authentication Logic

    let server = HttpServer::new(move || App::new().configure(config_app(provider.clone())))
        .bind(("0.0.0.0", port))?
        .run();

    Ok(server)
}
#[cfg(test)]
mod tests {
    const PROXY_PATH: &str =
        "/function/{service_and_optional_namespace:[^{}/]+(?:\\.[^{}/]+)?}{rest_path:/?.*}";
    use actix_web::{App, HttpResponse, Responder, test, web};
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    struct FunctionPathParams {
        service_and_optional_namespace: String,
        rest_path: String,
    }
    async fn fake_handler(dispatch: web::Path<FunctionPathParams>) -> impl Responder {
        let combine_name = &dispatch.service_and_optional_namespace;
        let path = &dispatch.rest_path;
        HttpResponse::Ok().body(format!("{}{}", combine_name, path)) // path 是 "/path"
    }

    #[actix_web::test]
    async fn test_proxy_dispatch() {
        let app = test::init_service(App::new().route(PROXY_PATH, web::to(fake_handler))).await;

        let req = test::TestRequest::get()
            .uri("/function/name.namespace/path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "name.namespace/path");

        let req = test::TestRequest::get()
            .uri("/function/name.namespace")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "name.namespace");

        let req = test::TestRequest::get()
            .uri("/function/name/path")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "name/path");

        let req = test::TestRequest::get()
            .uri("/function/name.namespace/path1/path2")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "name.namespace/path1/path2");
    }
}
