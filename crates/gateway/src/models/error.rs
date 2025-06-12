use actix_web::{HttpResponse, ResponseError};
use diesel::result::Error as DieselError;
use diesel_async::pooled_connection::bb8::RunError;
use serde_json::json;
use std::io;
#[derive(Debug)]
pub enum DbError {
    /// Diesel ORM 操作相关的错误
    DieselError(DieselError),

    /// 数据库连接池相关的错误
    PoolError(String),

    /// 数据未找到
    NotFound,

    /// 数据冲突（例如唯一约束冲突）
    Conflict,
    AlreadyExists(String),
}
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::DieselError(e) => write!(f, "Diesel error: {}", e),
            DbError::Conflict => write!(f, "Conflict error"),
            DbError::NotFound => write!(f, "Resource not found"),
            DbError::PoolError(e) => write!(f, "Database connection pool error: {}", e),
            DbError::AlreadyExists(e) => write!(f, "Resource already exists: {}", e),
        }
    }
}
impl From<RunError> for DbError {
    fn from(err: RunError) -> Self {
        DbError::PoolError(err.to_string())
    }
}

impl From<DbError> for io::Error {
    fn from(err: DbError) -> io::Error {
        // 这里可以把 DbError 转成 io::Error，通常用 io::ErrorKind::Other
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}
impl From<DieselError> for DbError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::NotFound => DbError::NotFound,
            DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                info,
            ) => DbError::AlreadyExists(
                info.details()
                    .unwrap_or("Resource already exists")
                    .to_string(),
            ),
            _ => DbError::DieselError(err),
        }
    }
}

impl ResponseError for DbError {
    fn error_response(&self) -> HttpResponse {
        match self {
            DbError::NotFound => {
                HttpResponse::NotFound().json(json!({"error": "Resource not found"}))
            }
            DbError::Conflict => HttpResponse::Conflict().json(json!({"error": "Conflict error"})),
            DbError::PoolError(e) => HttpResponse::InternalServerError().json(json!({"error": e})),
            DbError::DieselError(e) => {
                log::error!("Diesel error: {}", e);
                HttpResponse::InternalServerError().json(json!({"error": "Database error"}))
            }
            DbError::AlreadyExists(e) => HttpResponse::Conflict().json(json!({"error": e})),
        }
    }
}
