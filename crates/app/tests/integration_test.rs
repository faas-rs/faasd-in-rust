use actix_web::{App, web};
use provider::{
    handlers::{delete::delete_handler, deploy::deploy_handler},
    proxy::proxy_handler::proxy_handler,
    types::config::FaaSConfig,
};
use service::containerd_manager::ContainerdManager;

mod integration_tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use serde_json::json;

    #[actix_web::test]
    #[ignore]
    async fn test_handlers_in_order() {
        dotenv::dotenv().ok();
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
        let socket_path = std::env::var("SOCKET_PATH")
            .unwrap_or_else(|_| "/run/containerd/containerd.sock".to_string());
        ContainerdManager::init(&socket_path).await;

        let faas_config = FaaSConfig::new();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(faas_config))
                .route("/system/functions", web::post().to(deploy_handler))
                .route("/system/functions", web::delete().to(delete_handler))
                .route("/function/{name}{path:/?.*}", web::to(proxy_handler)),
        )
        .await;

        // test proxy no-found-function in namespace 'default'
        let req = test::TestRequest::get()
            .uri("/function/test-no-found-function")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let response_body = test::read_body(resp).await;
        let response_str = std::str::from_utf8(&response_body).unwrap();
        assert!(response_str.contains("Failed to get function"));

        // test delete no-found-function in namespace 'default'
        let req = test::TestRequest::delete()
            .uri("/system/functions")
            .set_json(json!({"function_name": "test-no-found-function"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let response_body = test::read_body(resp).await;
        let response_str = std::str::from_utf8(&response_body).unwrap();
        assert!(
            response_str
                .contains("Function 'test-no-found-function' not found in namespace 'default'")
        );

        // test deploy in namespace 'default'
        let req = test::TestRequest::post()
            .uri("/system/functions")
            .set_json(json!({
                "function_name": "test-function",
                "image": "docker.io/library/nginx:alpine"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::ACCEPTED,
            "check whether the container has been existed"
        );

        let response_body = test::read_body(resp).await;
        let response_str = std::str::from_utf8(&response_body).unwrap();
        log::info!("{}", response_str);
        assert!(response_str.contains("Function test-function deployment initiated successfully."));

        // test proxy in namespace 'default'
        let req = test::TestRequest::get()
            .uri("/function/test-function")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let response_body = test::read_body(resp).await;
        let response_str = std::str::from_utf8(&response_body).unwrap();
        assert!(response_str.contains("Welcome to nginx!"));

        // test delete in namespace 'default'
        let req = test::TestRequest::delete()
            .uri("/system/functions")
            .set_json(json!({"function_name": "test-function"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let response_body = test::read_body(resp).await;
        let response_str = std::str::from_utf8(&response_body).unwrap();
        assert!(response_str.contains("Function test-function deleted successfully."));
    }
}
