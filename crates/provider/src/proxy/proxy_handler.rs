use crate::handlers::invoke_resolver::InvokeResolver;
use crate::proxy::builder::create_proxy_request;

use actix_web::{
    HttpRequest, HttpResponse,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorMethodNotAllowed},
    http::Method,
    web,
};

// 主要参考源码的响应设置
pub async fn proxy_handler(
    req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => proxy_request(&req, payload).await,
        _ => Err(ErrorMethodNotAllowed("Method not allowed")),
    }
}

//根据原始请求，解析url，构建转发请求并转发，获取响应
async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    let function_name = req.match_info().get("name").unwrap_or("");
    if function_name.is_empty() {
        return Err(ErrorBadRequest("Function name is required"));
    }

    let function_addr = InvokeResolver::resolve_function_url(function_name).await?;

    let proxy_req = create_proxy_request(req, &function_addr, payload);

    // Handle the error conversion explicitly
    let proxy_resp = match proxy_req.await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("Proxy request failed: {}", e);
            return Err(ErrorInternalServerError(format!(
                "Proxy request failed: {}",
                e
            )));
        }
    };

    // Now create an HttpResponse from the proxy response
    let mut client_resp = HttpResponse::build(proxy_resp.status());

    // Stream the response body
    Ok(client_resp.streaming(proxy_resp))
}
