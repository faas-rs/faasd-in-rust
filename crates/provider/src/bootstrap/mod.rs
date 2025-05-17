use actix_web::{App, HttpServer, middleware, web};
use std::{collections::HashMap, sync::Arc};
use tokio::signal::unix::{SignalKind, signal};

use crate::{handlers, metrics::HttpMetrics, provider::Provider, types::config::FaaSConfig};

//应用程序状态，存储共享的数据，如配置、指标、认证信息等，为业务函数提供支持
#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    config: FaaSConfig,   //应用程序的配置，用于识别是否开启Basic Auth等
    metrics: HttpMetrics, //用于监视http请求的持续时间和总数
    credentials: Option<HashMap<String, String>>, //当有认证信息的时候，获取认证信息
}

// this is a blocking serve function
pub fn serve<P: Provider>(provider: Arc<P>) -> std::io::Result<()> {
    let config = FaaSConfig::new();
    // let _registry = Registry::new();
    let metrics = HttpMetrics::new();

    let port = config.tcp_port.unwrap_or(8080);

    // 用于存储应用程序状态的结构体
    let app_state = web::Data::new(AppState {
        config,
        metrics,
        credentials: None,
    });

    let task_tracker = provider.task_tracker(); // 获取任务跟踪器

    let provider = web::Data::from(provider);

    // 如果启用了Basic Auth，从指定路径读取认证凭证并存储在应用程序状态中
    // TODO: Authentication Logic

    actix_web::rt::System::new().block_on(async {
        let server = HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .app_data(provider.clone())
                .wrap(middleware::Logger::default())
                .service(
                    web::scope("/system")
                        .service(
                            web::resource("/functions")
                                // .route(web::get().to(handlers::function_list::function_list_handler))
                                .route(web::post().to(handlers::function::deploy::<P>))
                                .route(web::delete().to(handlers::function::delete::<P>)), // .route(web::put().to(handlers::update_function)),
                        )
                        //         .service(
                        //             web::resource("/function/{name}")
                        //                 .route(web::get().to(handlers::function_status)),
                        //         )
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
                        .service(web::scope("/function").service(
                            web::resource("/{name}").route(web::to(handlers::proxy::proxy::<P>)),
                        )),
                )
            // .route("/metrics", web::get().to(handlers::telemetry))
            // .route("/healthz", web::get().to(handlers::health))
        })
        .bind(("0.0.0.0", port))?
        .run();

        let server_handle = server.handle();

        let server_task = actix_web::rt::spawn(server);

        let shutdown = actix_web::rt::spawn(async move {
            // listen for shutdown signals
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigquit = signal(SignalKind::quit()).unwrap();
            tokio::select! {
                _ = sigint.recv() => println!("SIGINT received, starting graceful shutdown..."),
                _ = sigterm.recv() => println!("SIGTERM received, starting graceful shutdown..."),
                _ = sigquit.recv() => println!("SIGQUIT received, starting graceful shutdown..."),
            }

            //先停止服务器防止close后还有delete 请求
            server_handle.stop(true).await;

            // 关闭任务跟踪器
            task_tracker.close();
            task_tracker.wait().await;
        });

        let _ = tokio::try_join!(server_task, shutdown).expect("unable to join tasks");

        Ok(())
    })
}

//当上下文完成的时候关闭服务器
//无法关闭时候写进log,并且返回错误
