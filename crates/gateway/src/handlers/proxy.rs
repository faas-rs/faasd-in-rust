use std::str::FromStr;

use crate::bootstrap::FunctionPathParams;
use actix_http::Method;
use actix_web::{HttpRequest, HttpResponse, error::ErrorMethodNotAllowed, web};

use crate::{provider::Provider, proxy::proxy_handler::proxy_request, types::function::Query};

// 主要参考源码的响应设置
pub async fn proxy<P: Provider>(
    req: HttpRequest,
    payload: web::Payload,
    provider: web::Data<P>,
    query: web::Path<FunctionPathParams>,
) -> actix_web::Result<HttpResponse> {
    let function = Query::from_str(&query.service_and_optional_namespace)
        .map_err(|_| ErrorMethodNotAllowed("Invalid function name"))?;
    log::trace!("proxy query: {:?}", function);
    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => {
            let upstream = provider
                .resolve(function)
                .await
                .map_err(|_| ErrorMethodNotAllowed("Invalid function name"))?;
            proxy_request(&req, payload, upstream, &query.rest_path).await
        }
        _ => Err(ErrorMethodNotAllowed("Method not allowed")),
    }
}
