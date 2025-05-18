use std::collections::HashMap;
use std::time::SystemTime;

use crate::provider::Provider;
use crate::types::function::{Delete, Deployment, Query};
use actix_web::ResponseError;
use actix_web::{HttpResponse, web};
use derive_more::derive::{Display, Error};
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

pub async fn update<P: Provider>(
    provider: web::Data<P>,
    info: web::Json<Deployment>,
) -> Result<HttpResponse, DeployError> {
    (*provider)
        .update(info.0)
        .await
        .map(|()| HttpResponse::Accepted().finish())
}

pub async fn list<P: Provider>(provider: web::Data<P>) -> HttpResponse {
    let functions = (*provider).list().await;
    HttpResponse::Ok().json(functions)
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

#[derive(Debug, Deserialize)]
pub struct StatusParam {
    namespace: Option<String>,
}

pub async fn status<P: Provider>(
    provider: web::Data<P>,
    name: web::Path<String>,
    info: web::Query<StatusParam>,
) -> Result<HttpResponse, ResolveError> {
    let query = Query {
        service: name.into_inner(),
        namespace: info.namespace.clone(),
    };
    let status = (*provider).status(query).await?;
    Ok(HttpResponse::Ok().json(status))
}

#[derive(Debug, Display, Error)]
pub enum DeployError {
    Invalid,
    InternalError,
}

#[derive(Debug, Display, Error)]
pub enum DeleteError {
    Invalid,
    NotFound,
    Internal,
}

#[derive(Debug, Display, Error)]
pub enum ResolveError {
    NotFound,
    Invalid,
    Internal,
}

impl ResponseError for DeployError {}
impl ResponseError for DeleteError {}
impl ResponseError for ResolveError {}
