use actix_web::{HttpRequest, HttpResponse, error::ErrorMethodNotAllowed, web};
use awc::http::Method;

use crate::{provider::Provider, proxy::proxy_handler::proxy_request, types::function::Query};

// 主要参考源码的响应设置
pub async fn proxy<P: Provider>(
    req: HttpRequest,
    payload: web::Payload,
    provider: web::Data<P>,
    query: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let query = Query::from(query.into_inner().as_str());
    log::trace!("proxy query: {:?}", query);
    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => {
            let upstream = provider
                .resolve(query)
                .await
                .map_err(|_| ErrorMethodNotAllowed("Invalid function name"))?;
            log::trace!("proxy upstream: {:?}", upstream);
            proxy_request(&req, payload, upstream).await
        }
        _ => Err(ErrorMethodNotAllowed("Method not allowed")),
    }
}
