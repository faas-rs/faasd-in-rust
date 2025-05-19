use crate::handlers::proxy::proxy;
use actix_web::{
    App, HttpRequest, HttpResponse, Responder, http,
    test::{self},
    web::{self, Bytes},
};

#[actix_web::test]
#[ignore]
async fn test_proxy_handler_success() {
    todo!()
}

#[actix_web::test]
async fn test_path_parsing() {
    let test_cases = vec![
        ("simple_name_match", "/function/echo", "echo", "", 200),
        (
            "simple_name_match",
            "/function/echo.faasd-in-rs-fn",
            "echo.faasd-in-rs-fn",
            "",
            200,
        ),
        (
            "simple_name_match_with_trailing_slash",
            "/function/echo/",
            "echo",
            "",
            200,
        ),
        (
            "name_match_with_additional_path_values",
            "/function/echo/subPath/extras",
            "echo",
            "subPath/extras",
            200,
        ),
        (
            "name_match_with_additional_path_values_and_querystring",
            "/function/echo/subPath/extras?query=true",
            "echo",
            "subPath/extras",
            200,
        ),
        ("not_found_if_no_name", "/function/", "", "", 404),
    ];

    let app = test::init_service(
        App::new()
            .route("/function/{name}", web::get().to(var_handler))
            .route("/function/{name}/", web::get().to(var_handler))
            .route("/function/{name}/{params:.*}", web::get().to(var_handler)),
    )
    .await;

    for (name, path, function_name, extra_path, status_code) in test_cases {
        let req = test::TestRequest::get().uri(path).to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), status_code, "Test case: {}", name);

        if status_code == 200 {
            let body = test::read_body(resp).await;
            let expected_body = format!("name: {} params: {}", function_name, extra_path);
            assert_eq!(body, expected_body.as_bytes(), "Test case: {}", name);
        }
    }
}

#[actix_web::test]
async fn test_invalid_method() {
    let app = test::init_service(
        App::new().route("/function/{name}{path:/?.*}", web::to(proxy)),
    )
    .await;

    let req = test::TestRequest::with_uri("/function/test-service/path")
        .method(http::Method::from_bytes(b"INVALID").unwrap())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::METHOD_NOT_ALLOWED);
}

#[actix_web::test]
async fn test_empty_func_name() {
    let app = test::init_service(
        App::new().route("/function{name:/?}{path:/?.*}", web::to(proxy)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/function")
        .insert_header((http::header::CONTENT_TYPE, "application/json"))
        .set_payload(Bytes::from_static(b"{\"key\":\"value\"}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
}

async fn var_handler(req: HttpRequest) -> impl Responder {
    let vars = req.match_info();
    HttpResponse::Ok().body(format!(
        "name: {} params: {}",
        vars.get("name").unwrap_or(""),
        vars.get("params").unwrap_or("")
    ))
}
