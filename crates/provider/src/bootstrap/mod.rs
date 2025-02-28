use actix_web::{web, App, HttpServer, HttpResponse, middleware, guard, Responder};
use prometheus::Registry;
use std::collections::HashMap;

use crate::{
    auth,
    metrics::{self, HttpMetrics},
    //httputil,
    //proxy,
    types::config::FaaSConfig,
    handlers,
};


//用于函数/服务名称的表达式
const NAME_EXPRESSION: &str = r"-a-zA-Z_0-9\.";

//应用程序状态，存储共享的数据，如配置、指标、认证信息等，为业务函数提供支持
#[derive(Clone)]
struct AppState {
    config: FaaSConfig,//应用程序的配置，用于识别是否开启Basic Auth等
    metrics: HttpMetrics,//用于监视http请求的持续时间和总数
    credentials: Option<HashMap<String, String>>, //当有认证信息的时候，获取认证信息
}

//serve 把处理程序headlers load到正确路由规范。这个函数是阻塞的。
async fn serve() -> std::io::Result<()> {
    let config = FaaSConfig::new(); //加载配置，用于识别是否开启Basic Auth等
    let registry = Registry::new();
    let metrics = metrics::HttpMetrics::new(); //metrics监视http请求的持续时间和总数

    // 用于存储应用程序状态的结构体
    let mut app_state = AppState {
        config: config.clone(),
        metrics: metrics.clone(),
        credentials: None,
    };

    // 如果启用了Basic Auth，从指定路径读取认证凭证并存储在应用程序状态中
    if config.enable_basic_auth {
        // 读取Basic Auth凭证
        let auth = auth::ReadBasicAuthFromDisk::new(&config.secret_mount_path, "users.txt", "passwords.txt");
        let credentials = auth.read_basic_auth().await; //这里的credentials是所有的账号密码
        app_state.credentials = Some(credentials);
        //TODO:handlers decorate with basic auth,尚未清楚是不是需要给所有的函数都加上
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone())) // 将app_state存储在web::Data中，以便在处理程序中访问
            .wrap(middleware::Logger::default()) // 记录请求日志
            .service(
            web::scope("/system")
                .service(
                web::resource("/functions")
                    .route(web::get().to(handlers::function_lister))
                    .route(web::post().to(handlers::deploy_function))
                    .route(web::delete().to(handlers::delete_function))
                    .route(web::put().to(handlers::update_function))
                )
                .service(
                web::resource("/function/{name}")
                    .route(web::get().to(handlers::function_status))
                )
                .service(
                web::resource("/scale-function/{name}")
                    .route(web::post().to(handlers::scale_function))
                )
                .service(
                web::resource("/info")
                    .route(web::get().to(handlers::info))
                )
                .service(
                web::resource("/secrets")
                    .route(web::get().to(handlers::secrets))
                    .route(web::post().to(handlers::secrets))
                    .route(web::put().to(handlers::secrets))
                    .route(web::delete().to(handlers::secrets))
                )
                .service(
                web::resource("/logs")
                    .route(web::get().to(handlers::logs))
                )
                .service(
                web::resource("/namespaces")
                    .route(web::get().to(handlers::list_namespaces))
                    .route(web::post().to(handlers::mutate_namespace))
                )
            )
            .service(
            web::scope("/function")
                .service(
                web::resource("/{name}")
                    .route(web::get().to(handlers::function_proxy))
                    .route(web::post().to(handlers::function_proxy))
                )
                .service(
                web::resource("/{name}/{params:.*}")
                    .route(web::get().to(handlers::function_proxy))
                    .route(web::post().to(handlers::function_proxy))
                )
            )
            .route("/metrics", web::get().to(handlers::telemetry))
            .route("/healthz", web::get().to(handlers::health))
    })
    .bind(("0.0.0.0", config.tcp_port.unwrap_or(8080)))?
    .run()
    .await
}


//当上下文完成的时候关闭服务器
//无法关闭时候写进log,并且返回错误