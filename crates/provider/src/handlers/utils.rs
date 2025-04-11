use crate::handlers::function_get::FunctionError;
use actix_web::{Error, HttpResponse, ResponseError};
use derive_more::Display;

pub fn map_service_error(e: Box<dyn std::error::Error>) -> Error {
    eprintln!("Service error: {}", e);
    actix_web::error::ErrorInternalServerError(format!("Operationfailed: {}", e))
}

//枚举错误类型，并非所有被调用的函数的错误类型都能实现调用函数的错误特征
#[derive(Debug, Display)]
pub enum CustomError {
    #[display("GrpcError: {}", _0)]
    GrpcError(tonic::Status),
    #[display("OtherError: {}", _0)]
    OtherError(String),
    #[display("ActixError: {}", _0)]
    ActixError(actix_web::Error),
    #[display("FunctionError: {}", _0)]
    FunctionError(FunctionError),
}

impl ResponseError for CustomError {
    fn error_response(&self) -> HttpResponse {
        match self {
            CustomError::GrpcError(status) => {
                // Customize the HTTP response based on the gRPC status
                match status.code() {
                    tonic::Code::NotFound => {
                        HttpResponse::NotFound().body(status.message().to_string())
                    }
                    tonic::Code::PermissionDenied => {
                        HttpResponse::Forbidden().body(status.message().to_string())
                    }
                    _ => HttpResponse::InternalServerError().body(status.message().to_string()),
                }
            }
            CustomError::OtherError(message) => {
                HttpResponse::InternalServerError().body(message.clone())
            }
            CustomError::ActixError(err) => err.error_response(),
            CustomError::FunctionError(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
        }
    }
}

impl From<actix_web::Error> for CustomError {
    fn from(err: actix_web::Error) -> Self {
        CustomError::ActixError(err)
    }
}

impl From<tonic::Status> for CustomError {
    fn from(err: tonic::Status) -> Self {
        CustomError::GrpcError(err)
    }
}

impl From<FunctionError> for CustomError {
    fn from(err: FunctionError) -> Self {
        CustomError::FunctionError(err)
    }
}
