pub mod proxy;

use actix_web::{ web,HttpRequest};
use crate::{
     types::config::FaaSConfig,
     handlers::invoke_resolver::InvokeResolver,
};
pub struct ProxyHandlerInfo{
    req:HttpRequest,
    payload:web::Payload,
    config:FaaSConfig,
    resolver:Option<InvokeResolver>
}
