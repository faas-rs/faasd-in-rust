use crate::provider::Provider;
use crate::types::function::{Delete, Deployment, Query};
use actix_web::ResponseError;
use actix_web::{HttpResponse, web};
use derive_more::derive::Display;
use serde::Deserialize;

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

// TODO: 为 Errors 添加错误信息

#[derive(Debug, Display)]
pub enum DeployError {
    Invalid,
    InternalError,
}

#[derive(Debug, Display)]
pub enum DeleteError {
    Invalid,
    NotFound,
    Internal,
}

#[derive(Debug, Display)]
pub enum ResolveError {
    NotFound,
    Invalid,
    Internal,
}

impl ResponseError for DeployError {
    fn status_code(&self) -> awc::http::StatusCode {
        match self {
            DeployError::Invalid => awc::http::StatusCode::BAD_REQUEST,
            DeployError::InternalError => awc::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> awc::http::StatusCode {
        match self {
            DeleteError::Invalid => awc::http::StatusCode::BAD_REQUEST,
            DeleteError::NotFound => awc::http::StatusCode::NOT_FOUND,
            DeleteError::Internal => awc::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for ResolveError {
    fn status_code(&self) -> awc::http::StatusCode {
        match self {
            ResolveError::NotFound => awc::http::StatusCode::NOT_FOUND,
            ResolveError::Invalid => awc::http::StatusCode::BAD_REQUEST,
            ResolveError::Internal => awc::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
