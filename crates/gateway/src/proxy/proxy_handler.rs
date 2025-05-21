// use crate::handlers::invoke_resolver::InvokeResolver;
use crate::proxy::builder::create_proxy_request;

use actix_web::{HttpRequest, HttpResponse, error::ErrorInternalServerError, web};

pub async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
    upstream: actix_http::uri::Builder,
    path: &str,
) -> actix_web::Result<HttpResponse> {
    let uri = upstream.path_and_query(path).build().map_err(|e| {
        log::error!("Failed to build URI: {}", e);
        ErrorInternalServerError("Failed to build URI")
    })?;
    log::trace!("Proxying request to: {}", uri);
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
