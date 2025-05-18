use actix_web::{HttpRequest, HttpResponse, error::ErrorMethodNotAllowed, web};
use awc::http::Method;

use crate::{provider::Provider, proxy::proxy_handler::proxy_request};

// 主要参考源码的响应设置
pub async fn proxy<P: Provider>(
    req: HttpRequest,
    payload: web::Payload,
    provider: web::Data<P>,
    query: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => {
            let upstream = provider
                .resolve(query.as_ref().as_str().into())
                .await
                .map_err(|_| ErrorMethodNotAllowed("Invalid function name"))?;
            proxy_request(&req, payload, upstream).await
        }
        _ => Err(ErrorMethodNotAllowed("Method not allowed")),
    }
}
