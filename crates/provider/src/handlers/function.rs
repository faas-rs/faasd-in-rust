use std::collections::HashMap;
use std::time::SystemTime;

use crate::provider::Provider;
use crate::types::function::{Delete, Deployment, Query};
use actix_web::ResponseError;
use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub namespace: String,
    pub image: String,
    pub pid: u32,
    pub replicas: i32,
    pub address: String,
    pub labels: HashMap<String, String>,
    // pub annotations: HashMap<String, String>,
    // pub secrets: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub env_process: String,
    // pub memory_limit: i64,
    pub created_at: SystemTime,
}

// 参考响应状态 https://github.com/openfaas/faas/blob/7803ea1861f2a22adcbcfa8c79ed539bc6506d5b/api-docs/spec.openapi.yml#L121C1-L140C45
// 请求体反序列化失败，自动返回400错误
pub async fn deploy<P: Provider>(
    provider: web::Data<P>,
    info: web::Json<Deployment>,
) -> Result<HttpResponse, DeployError> {
    (*provider)
        .deploy(info.0)
        .await
        .map(|()| HttpResponse::Accepted().finish())
}

pub async fn delete<P: Provider>(
    provider: web::Data<P>,
    info: web::Json<Delete>,
) -> Result<HttpResponse, DeleteError> {
    let query = Query::from(info.0);
    (*provider)
        .delete(query)
        .await
        .map(|()| HttpResponse::Ok().finish())
}

#[derive(Debug)]
pub enum DeleteError {
    Invalid,
    NotFound,
    Internal,
}

#[derive(Debug)]
pub enum DeployError {
    Invalid,
    InternalError,
}

#[derive(Debug)]
pub enum ResolveError {
    NotFound,
    Invalid,
    Internal,
}

impl std::fmt::Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeployError::Invalid => write!(f, "Invalid function deployment request"),
            DeployError::InternalError => write!(f, "Internal error occurred"),
        }
    }
}

impl ResponseError for DeployError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            DeployError::Invalid => actix_web::http::StatusCode::BAD_REQUEST,
            DeployError::InternalError => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::Invalid => write!(f, "Invalid function deletion request"),
            DeleteError::NotFound => write!(f, "Function not found"),
            DeleteError::Internal => write!(f, "Internal error occurred"),
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            DeleteError::Invalid => actix_web::http::StatusCode::BAD_REQUEST,
            DeleteError::NotFound => actix_web::http::StatusCode::NOT_FOUND,
            DeleteError::Internal => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
