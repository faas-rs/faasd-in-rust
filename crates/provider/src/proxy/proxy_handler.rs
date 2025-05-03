use crate::handlers::invoke_resolver::InvokeResolver;
use crate::proxy::builder::build_proxy_request;
use crate::proxy::client::new_proxy_client_from_config;
use crate::types::config::FaaSConfig;
use actix_web::{
    Error, HttpRequest, HttpResponse,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorMethodNotAllowed},
    http::Method,
    web,
};

// 主要参考源码的响应设置
pub async fn proxy_handler(
    config: web::Data<FaaSConfig>,
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, Error> {
    let proxy_client = new_proxy_client_from_config(config.as_ref()).await;
    log::info!("proxy_client : {:?}", proxy_client);

    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => proxy_request(&req, payload, &proxy_client).await,
        _ => Err(ErrorMethodNotAllowed("method not allowed")),
    }
}

//根据原始请求，解析url，构建转发请求并转发，获取响应
async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
    proxy_client: &reqwest::Client,
) -> Result<HttpResponse, Error> {
    let function_name = req.match_info().get("name").unwrap_or("");
    if function_name.is_empty() {
        return Err(ErrorBadRequest("function name is required"));
    }

    let function_addr = InvokeResolver::resolve_function_url(function_name).await?;

    let proxy_req = build_proxy_request(req, &function_addr, proxy_client, payload).await?;

    match proxy_req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let mut client_resp = HttpResponse::build(status);

            for (name, value) in resp.headers().iter() {
                client_resp.insert_header((name.clone(), value.clone()));
            }

            let body = resp.bytes().await.unwrap();

            Ok(client_resp.body(body))
        }
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
