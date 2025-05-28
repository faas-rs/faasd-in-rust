use crate::{provider::Provider, types::namespace::Namespace};
use actix_http::{Method, StatusCode};
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use derive_more::Display;

#[derive(Debug, Display)]
pub enum NamespaceError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("AlreadyExists: {}", _0)]
    AlreadyExists(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("MethodNotAllowed: {}", _0)]
    MethodNotAllowed(String),
}

impl ResponseError for NamespaceError {
    fn status_code(&self) -> StatusCode {
        match self {
            NamespaceError::Invalid(_) => StatusCode::BAD_REQUEST,
            NamespaceError::AlreadyExists(_) => StatusCode::BAD_REQUEST,
            NamespaceError::NotFound(_) => StatusCode::NOT_FOUND,
            NamespaceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            NamespaceError::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
        }
    }
}

pub async fn mut_namespace<P: Provider>(
    req: HttpRequest,
    provider: web::Data<P>,
    info: Option<web::Json<Namespace>>,
) -> Result<HttpResponse, NamespaceError> {
    let namespace = req.match_info().get("namespace");
    if namespace.is_none() {
        return Err(NamespaceError::Invalid("namespace is required".to_string()));
    }
    let namespace = namespace.unwrap();
    let labels;
    match *req.method() {
        Method::POST => {
            match info {
                Some(info) => {
                    labels = info.0.labels;
                }
                None => {
                    return Err(NamespaceError::Invalid(
                        "Request body is required".to_string(),
                    ));
                }
            }
            (*provider)
                .create_namespace(namespace.to_string(), labels)
                .await
                .map(|_| {
                    HttpResponse::Created()
                        .body(format!("namespace {} was created successfully", namespace))
                })
        }
        Method::DELETE => (*provider)
            .delete_namespace(namespace.to_string())
            .await
            .map(|_| {
                HttpResponse::Accepted()
                    .body(format!("namespace {} was deleted successfully", namespace))
            }),
        Method::PUT => {
            match info {
                Some(info) => {
                    labels = info.0.labels;
                }
                None => {
                    return Err(NamespaceError::Invalid(
                        "Request body is required".to_string(),
                    ));
                }
            }
            (*provider)
                .update_namespace(namespace.to_string(), labels)
                .await
                .map(|_| {
                    HttpResponse::Accepted()
                        .body(format!("namespace {} was updated successfully", namespace))
                })
        }
        Method::GET => (*provider)
            .get_namespace(namespace.to_string())
            .await
            .map(|ns| HttpResponse::Ok().json(ns)),
        _ => Err(NamespaceError::MethodNotAllowed(
            "Method not allowed".to_string(),
        )),
    }
}

pub async fn namespace_list<P: Provider>(
    provider: web::Data<P>,
) -> Result<HttpResponse, NamespaceError> {
    (*provider)
        .namespace_list()
        .await
        .map(|ns_list| HttpResponse::Ok().json(ns_list))
}
