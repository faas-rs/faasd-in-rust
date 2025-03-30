pub mod proxy_handler;

use crate::{handlers::invoke_resolver::InvokeResolver, types::config::FaaSConfig};
use actix_web::{HttpRequest, web};
pub struct ProxyHandlerInfo {
    req: HttpRequest,
    payload: web::Payload,
    config: FaaSConfig,
    resolver: Option<InvokeResolver>,
}
