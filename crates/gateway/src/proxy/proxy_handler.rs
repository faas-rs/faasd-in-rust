// use crate::handlers::invoke_resolver::InvokeResolver;
use crate::proxy::builder::create_proxy_request;

use actix_web::{HttpRequest, HttpResponse, error::ErrorInternalServerError, web};

pub async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
    upstream: actix_http::Uri,
    path: &str,
) -> actix_web::Result<HttpResponse> {
    let uri = if !path.is_empty() {
        let mut uri = http::uri::Parts::from(upstream);
        uri.path_and_query = Some(path.parse().map_err(|_| {
            log::error!("Failed to parse path: {}", path);
            ErrorInternalServerError("Invalid path")
        })?);
        actix_http::Uri::from_parts(uri).unwrap()
    } else {
        upstream
    };
    // Handle the error conversion explicitly
    let proxy_resp = create_proxy_request(req, uri, payload).await.map_err(|e| {
        log::error!("Failed to create proxy request: {}", e);
        ErrorInternalServerError("Failed to create proxy request")
    })?;

    // Now create an HttpResponse from the proxy response
    let mut client_resp = HttpResponse::build(proxy_resp.status());

    // Stream the response body
    Ok(client_resp.streaming(proxy_resp))
}
