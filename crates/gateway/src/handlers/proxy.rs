use actix_web::{HttpRequest, HttpResponse, error::ErrorMethodNotAllowed, web};
use awc::http::Method;
use serde::Deserialize;

use crate::{provider::Provider, proxy::proxy_handler::proxy_request, types::function::Query};

#[derive(Deserialize, Debug)]
pub struct ProxyPathParams {
    pub service_and_optional_namespace: String,
    pub rest_path: String,
}

// 主要参考源码的响应设置
pub async fn proxy<P: Provider>(
    req: HttpRequest,
    payload: web::Payload,
    provider: web::Data<P>,
    query: web::Path<ProxyPathParams>,
) -> actix_web::Result<HttpResponse> {
    let service_with_option_namesapce = query.service_and_optional_namespace.clone();
    let query = Query::from(service_with_option_namesapce.as_str());
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
