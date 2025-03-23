pub mod deploy;
pub mod function_list;
pub mod namespace_list;
use lazy_static::lazy_static;
use std::collections::HashMap;

use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::{Serialize, de::DeserializeOwned};

pub async fn function_lister(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("函数列表")
}

pub async fn deploy_function(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("部署函数")
}

pub async fn delete_function(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("删除函数")
}

pub async fn update_function(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("更新函数")
}

pub async fn function_status(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("函数状态")
}

pub async fn scale_function(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("扩展函数")
}

pub async fn info(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("信息")
}

pub async fn secrets(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("秘密")
}

pub async fn logs(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("日志")
}

pub async fn list_namespaces(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("命名空间列表")
}

pub async fn mutate_namespace(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("变更命名空间")
}

pub async fn function_proxy(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("函数代理")
}

pub async fn telemetry(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("遥测")
}

pub async fn health(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("健康检查")
}

// lazy_static! {
//     pub static ref HANDLERS: HashMap<String, Box<dyn IAmHandler>> = {
//         let mut map = HashMap::new();
//         map.insert(
//             "function_list".to_string(),
//             Box::new(function_list::FunctionLister),
//         );
//         map.insert(
//             "namespace_list".to_string(),
//             Box::new(namespace_list::NamespaceLister),
//         );
//         map
//     };
// }

#[derive(Debug, thiserror::Error)]
pub struct FaasError {
    message: String,
    error_type: FaasErrorType,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug)]
pub enum FaasErrorType {
    ContainerFailure,
    Timeout,
    InternalError,
}

impl std::fmt::Display for FaasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.error_type, self.message)
    }
}

// 实现从常见错误类型转换
impl From<std::io::Error> for FaasError {
    fn from(err: std::io::Error) -> Self {
        FaasError {
            message: format!("IO error: {}", err),
            error_type: FaasErrorType::InternalError,
            source: Some(Box::new(err)),
        }
    }
}

pub trait IAmHandler {
    type Input: DeserializeOwned + Send + 'static;
    // type Output: Serialize + Send + 'static;

    /// 获取Handler元数据（函数名、超时时间等）
    // fn metadata(&self) -> HandlerMeta;

    /// 执行核心逻辑
    fn execute(
        &self,
        input: Self::Input,
    ) -> impl std::future::Future<Output = impl Responder> + Send;
}
