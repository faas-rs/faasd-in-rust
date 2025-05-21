use std::str::FromStr;

use actix_http::Method;
use actix_web::{HttpRequest, HttpResponse, error::ErrorMethodNotAllowed, web};

use crate::{provider::Provider, proxy::proxy_handler::proxy_request, types::function::Query};

pub const PROXY_DISPATCH_PATH: &str = "/{any:.+}";

pub struct ProxyQuery {
    pub query: Query,
    pub path: String,
}

impl FromStr for ProxyQuery {
    type Err = ();
    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let (identifier, rest_path) = if let Some((identifier, rest_path)) = path.split_once('/') {
            match rest_path {
                "" => (identifier, "/".to_owned()),
                _ => (identifier, "/".to_owned() + rest_path),
            }
        } else {
            (path, "".to_owned())
        };
        let (service, namespace) = identifier
            .rsplit_once('.')
            .map(|(s, n)| (s.to_string(), Some(n.to_string())))
            .unwrap_or((identifier.to_string(), None));
        Ok(ProxyQuery {
            query: Query { service, namespace },
            path: rest_path,
        })
    }
}

// 主要参考源码的响应设置
pub async fn proxy<P: Provider>(
    req: HttpRequest,
    payload: web::Payload,
    provider: web::Data<P>,
    any: web::Path<String>,
) -> actix_web::Result<HttpResponse> {
    let meta = ProxyQuery::from_str(&any).map_err(|_| {
        log::error!("Failed to parse path: {}", any);
        ErrorMethodNotAllowed("Invalid path")
    })?;
    let function = meta.query;
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
                .map_err(|e| ErrorMethodNotAllowed(format!("Invalid function name {e}")))?;
            log::trace!("upstream: {:?}", upstream);
            proxy_request(&req, payload, upstream, &meta.path).await
        }
        _ => Err(ErrorMethodNotAllowed("Method not allowed")),
    }
}
