use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;
#[derive(Debug)]
pub enum AuthError {
    /// 密码哈希处理错误
    PasswordHashingError(String),

    /// JWT 相关错误
    JwtError(String),

    /// Token 已过期
    TokenExpired,

    /// Token 无效
    InvalidToken,

    AlreadyExists(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::PasswordHashingError(e) => write!(f, "Password hashing error: {}", e),
            AuthError::JwtError(e) => write!(f, "JWT error: {}", e),
            AuthError::TokenExpired => write!(f, "Token expired"),
            AuthError::InvalidToken => write!(f, "Invalid token"),
            AuthError::AlreadyExists(e) => write!(f, "AlreadyExists: {}", e),
        }
    }
}
impl std::error::Error for AuthError {}
impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AuthError::PasswordHashingError(e) => {
                HttpResponse::InternalServerError().json(json!({"error": e}))
            }
            AuthError::JwtError(e) => HttpResponse::InternalServerError().json(json!({"error": e})),
            AuthError::TokenExpired => {
                HttpResponse::Unauthorized().json(json!({"error": "Token expired"}))
            }
            AuthError::InvalidToken => {
                HttpResponse::Unauthorized().json(json!({"error": "Invalid token"}))
            }
            AuthError::AlreadyExists(e) => HttpResponse::Conflict().json(json!({"error": e})),
        }
    }
}
