use crate::provider::Provider;
use crate::types::function::{Delete, Deployment, Query};
use actix_http::StatusCode;
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
    let service = info.0.service.clone();
    (*provider).deploy(info.0).await.map(|()| {
        HttpResponse::Accepted().body(format!("function {} was created successfully", service))
    })
}

pub async fn update<P: Provider>(
    provider: web::Data<P>,
    info: web::Json<Deployment>,
) -> Result<HttpResponse, UpdateError> {
    let service = info.0.service.clone();
    (*provider).update(info.0).await.map(|()| {
        HttpResponse::Accepted().body(format!("function {} was updated successfully", service))
    })
}

pub async fn delete<P: Provider>(
    provider: web::Data<P>,
    info: web::Json<Delete>,
) -> Result<HttpResponse, DeleteError> {
    let service = info.0.function_name.clone();
    let query = Query {
        service: service.clone(),
        namespace: Some(info.0.namespace),
    };
    (*provider)
        .delete(query)
        .await
        .map(|()| HttpResponse::Ok().body(format!("function {} was deleted successfully", service)))
}

#[derive(Debug, Deserialize)]
pub struct ListParam {
    namespace: String,
}

pub async fn list<P: Provider>(
    provider: web::Data<P>,
    info: web::Query<ListParam>,
) -> Result<HttpResponse, ListError> {
    (*provider)
        .list(info.namespace.clone())
        .await
        .map(|functions| HttpResponse::Ok().json(functions))
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
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    InternalError(String),
}

#[derive(Debug, Display)]
pub enum DeleteError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Internal: {}", _0)]
    Internal(String),
}

#[derive(Debug, Display)]
pub enum ResolveError {
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    Internal(String),
}

#[derive(Debug, Display)]
pub enum ListError {
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
}

#[derive(Debug, Display)]
pub enum UpdateError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
}

impl ResponseError for DeployError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeployError::Invalid(_) => StatusCode::BAD_REQUEST,
            DeployError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteError::Invalid(_) => StatusCode::BAD_REQUEST,
            DeleteError::NotFound(_) => StatusCode::NOT_FOUND,
            DeleteError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for ResolveError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResolveError::NotFound(_) => StatusCode::NOT_FOUND,
            ResolveError::Invalid(_) => StatusCode::BAD_REQUEST,
            ResolveError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ResponseError for ListError {
    fn status_code(&self) -> StatusCode {
        match self {
            ListError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ListError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

impl ResponseError for UpdateError {
    fn status_code(&self) -> StatusCode {
        match self {
            UpdateError::Invalid(_) => StatusCode::BAD_REQUEST,
            UpdateError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            UpdateError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}
