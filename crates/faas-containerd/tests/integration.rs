use actix_web::App;
use actix_web::http::StatusCode;
use actix_web::test;
use gateway::bootstrap::config_app;
use serde_json::json;
use std::thread::sleep;
use std::time::Duration;

#[actix_web::test]
#[ignore]
async fn test_handlers_in_order() {
    let provider = faas_containerd::provider::ContainerdProvider::new();

    let app = test::init_service(App::new().configure(config_app(provider))).await;

    // test proxy no-found-function in namespace 'default'
    let req = test::TestRequest::get()
        .uri("/function/test-no-found-function")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
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
        response_str.contains("Function 'test-no-found-function' not found in namespace 'default'")
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

    sleep(Duration::from_secs(2));
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
