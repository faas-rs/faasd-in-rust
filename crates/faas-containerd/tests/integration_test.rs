use actix_web::App;
use actix_web::http::StatusCode;
use actix_web::test;
use gateway::bootstrap::config_app;
use serde_json::json;

#[actix_web::test]
#[ignore]
async fn test_handlers_in_order() {
    dotenv::dotenv().ok();
    faas_containerd::init_backend().await;
    let provider = faas_containerd::provider::ContainerdProvider::new();
    let app = test::init_service(App::new().configure(config_app(provider))).await;

    // test proxy no-found-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::get()
        .uri("/function/test-no-found-function")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("Invalid function name"));

    // test update no-found-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::put()
        .uri("/system/functions")
        .set_json(json!({
            "service": "test-no-found-function",
            "image": "hub.scutosc.cn/dolzhuying/echo:latest",
            "namespace": "faasrs-test-namespace"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("NotFound: container not found"));

    // test delete no-found-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::delete()
        .uri("/system/functions")
        .set_json(json!({
            "functionName": "test-no-found-function",
            "namespace": "faasrs-test-namespace"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("NotFound: container not found"));

    // test deploy test-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::post()
        .uri("/system/functions")
        .set_json(json!({
            "service": "test-function",
            "image": "hub.scutosc.cn/dolzhuying/echo:latest",
            "namespace": "faasrs-test-namespace"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "check whether the container exists"
    );
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("function test-function was created successfully"));

    // test update test-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::put()
        .uri("/system/functions")
        .set_json(json!({
            "service": "test-function",
            "image": "hub.scutosc.cn/dolzhuying/echo:latest",
            "namespace": "faasrs-test-namespace"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("function test-function was updated successfully"));

    // test list
    let req = test::TestRequest::get()
        .uri("/system/functions?namespace=faasrs-test-namespace")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(response_str).unwrap();
    if let Some(arr) = response_json.as_array() {
        for item in arr {
            assert_eq!(item["name"], "test-function");
            assert_eq!(item["image"], "hub.scutosc.cn/dolzhuying/echo:latest");
            assert_eq!(item["namespace"], "faasrs-test-namespace");
        }
    }

    // test status test-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::get()
        .uri("/system/function/test-function?namespace=faasrs-test-namespace")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(response_str).unwrap();
    if let Some(arr) = response_json.as_array() {
        for item in arr {
            assert_eq!(item["name"], "test-function");
            assert_eq!(item["image"], "hub.scutosc.cn/dolzhuying/echo:latest");
            assert_eq!(item["namespace"], "faasrs-test-namespace");
        }
    }

    // test proxy test-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::get()
        .uri("/function/test-function.faasrs-test-namespace")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("Hello world!"));

    // test delete test-function in namespace 'faasrs-test-namespace'
    let req = test::TestRequest::delete()
        .uri("/system/functions")
        .set_json(json!({
            "functionName": "test-function",
            "namespace": "faasrs-test-namespace"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let response_body = test::read_body(resp).await;
    let response_str = std::str::from_utf8(&response_body).unwrap();
    assert!(response_str.contains("function test-function was deleted successfully"));
}
