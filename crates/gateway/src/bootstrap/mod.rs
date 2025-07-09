use crate::oauth::auth_handler::protected_endpoint;
use crate::{
    handlers::{self, proxy::PROXY_DISPATCH_PATH},
    models::db,
    oauth::auth_handler,
    provider::Provider,
    types::config::FaaSConfig,
};
use actix_web::{
    App, HttpServer,
    dev::Server,
    web::{self, ServiceConfig},
};
use actix_web_httpauth::middleware::HttpAuthentication;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use std::env;
use std::{collections::HashMap, sync::Arc};

pub fn config_app<P: Provider>(
    provider: Arc<P>,
    db_pool: Pool<AsyncPgConnection>,
    faas_config: FaaSConfig,
) -> impl FnOnce(&mut ServiceConfig) {
    // let _registry = Registry::new();
    let provider = web::Data::from(provider);
    let app_state = web::Data::new(AppState {
        // metrics: HttpMetrics::new(),
        credentials: None,
    });
    move |cfg: &mut ServiceConfig| {
        cfg.app_data(app_state)
            .app_data(provider)
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(faas_config.clone()))
            .service(
                web::scope("/auth")
                    .route(
                        "/register",
                        web::post().to(auth_handler::register_user_handler),
                    )
                    .route("/login", web::post().to(auth_handler::login_handler)),
            )
            .service(
                web::scope("/system")
                    .wrap(HttpAuthentication::bearer(protected_endpoint))
                    .service(
                        web::resource("/functions")
                            .route(web::get().to(handlers::function::list::<P>))
                            .route(web::put().to(handlers::function::update::<P>))
                            .route(web::post().to(handlers::function::deploy::<P>))
                            .route(web::delete().to(handlers::function::delete::<P>)),
                    )
                    .service(
                        web::resource("/function/{functionName}")
                            .route(web::get().to(handlers::function::status::<P>)),
                    )
                    .service(
                        web::resource("/namespace/{namespace}")
                            .route(web::to(handlers::namespace::mut_namespace::<P>)),
                    )
                    .service(
                        web::resource("/namespaces")
                            .route(web::get().to(handlers::namespace::namespace_list::<P>)),
                    ),
                //         .service(
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
                web::scope("/function")
                    .wrap(HttpAuthentication::bearer(protected_endpoint))
                    .service(
                        web::resource(PROXY_DISPATCH_PATH)
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
pub async fn serve<P: Provider>(provider: Arc<P>) -> std::io::Result<Server> {
    log::info!("Checking config file");
    let config = FaaSConfig::new();
    let port = config.tcp_port.unwrap_or(8080);
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = db::create_pool(&database_url).await?;
    // let pool = setup_test_db().await.expect("failed to set up test");
    let server = HttpServer::new(move || {
        App::new().configure(config_app(
            provider.clone(),
            db_pool.clone(),
            config.clone(),
        ))
    })
    .bind(("0.0.0.0", port))?
    .run();

    Ok(server)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::handlers::proxy::{PROXY_DISPATCH_PATH, ProxyQuery};

    use actix_web::{App, HttpResponse, Responder, test, web};

    async fn dispatcher(any: web::Path<String>) -> impl Responder {
        let meta = ProxyQuery::from_str(&any).unwrap();
        HttpResponse::Ok().body(format!(
            "{}|{}|{}",
            meta.query.service,
            meta.query.namespace.unwrap_or_default(),
            meta.path
        ))
    }

    #[actix_web::test]
    async fn test_proxy() {
        let app = test::init_service(
            App::new().service(web::resource(PROXY_DISPATCH_PATH).route(web::get().to(dispatcher))),
        )
        .await;

        let (unslash, slash, resp0, a0) = (
            "/service.namespace/path",
            "/service.namespace/path/",
            "service|namespace|/path",
            "service|namespace|/path/",
        );
        let (unslash1, slash1, resp1, a1) = (
            "/service/path",
            "/service/path/",
            "service||/path",
            "service||/path/",
        );
        let (unslash2, slash2, resp2, a2) = (
            "/service.namespace",
            "/service.namespace/",
            "service|namespace|",
            "service|namespace|/",
        );
        let (unslash3, slash3, resp3, a3) = ("/service", "/service/", "service||", "service||/");

        let req = test::TestRequest::get().uri(unslash).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, resp0);

        let req = test::TestRequest::get().uri(slash).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, a0);

        let req = test::TestRequest::get().uri(unslash1).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, resp1);

        let req = test::TestRequest::get().uri(slash1).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, a1);

        let req = test::TestRequest::get().uri(unslash2).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, resp2);

        let req = test::TestRequest::get().uri(slash2).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, a2);

        let req = test::TestRequest::get().uri(unslash3).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, resp3);

        let req = test::TestRequest::get().uri(slash3).to_request();
        let resp = test::call_and_read_body(&app, req).await;
        assert_eq!(resp, a3);

        // test with empty path
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}
