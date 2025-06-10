use crate::models::error::DbError;
use crate::oauth::error::AuthError;
use crate::oauth::jwt_utils::{generate_access_token, validate_access_token};
use crate::oauth::services::{self, UserService};
use crate::types::config::FaaSConfig; // 确保 JwtConfig 被正确导入
use actix_web::HttpMessage;
use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, dev::ServiceRequest};
use actix_web::{HttpResponse, web};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize, Debug, Clone)]
//注册结构体
pub struct RegisterPayload {
    pub username: String,
    pub password: String,
}
#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
    token_type: String,
}
pub async fn register_user_handler(
    pool: web::Data<Pool<AsyncPgConnection>>,
    payload: web::Json<RegisterPayload>,
) -> Result<HttpResponse, Error> {
    log::info!("Registering user: {}", payload.username);
    let mut conn = pool.get().await.map_err(|e| {
        log::error!("Failed to get connection from pool: {}", e);
        DbError::DieselError(diesel::result::Error::NotFound) // 这里可以根据实际情况调整错误类型
    })?;
    match UserService::register_user(
        payload.username.clone(),
        payload.password.clone(),
        &mut conn,
    )
    .await
    {
        Ok(user) => {
            log::info!(
                "User {} registered successfully with uid: {}",
                user.username,
                user.uid
            );
            Ok(HttpResponse::Created().json(serde_json::json!({
                "message": "User created successfully",
                "user_id": user.uid.to_string()
            })))
        }
        Err(AuthError::AlreadyExists(_)) => {
            log::warn!(
                "User registration failed: username {} already exists",
                payload.username
            );
            Ok(HttpResponse::Conflict().json(serde_json::json!({
                "error": "User already exists",
            })))
        }
        Err(e) => {
            log::error!("User registration failed for {}: {:?}", payload.username, e);
            Err(e.into())
        }
    }
}

// 登录请求体
pub async fn login_handler(
    pool: web::Data<Pool<AsyncPgConnection>>,
    config: web::Data<FaaSConfig>, // FaaSConfig 包含 JwtConfig
    payload: web::Json<LoginPayload>,
) -> Result<HttpResponse, Error> {
    log::info!("Attempting login for user: {}", payload.username);
    let mut conn = pool.get().await.map_err(|e| {
        log::error!("Failed to get DB connection: {}", e);
        DbError::DieselError(diesel::result::Error::NotFound)
    })?;
    match UserService::find_user_by_username(&payload.username, &mut conn).await {
        Ok(user) => {
            if services::verify_password(&user.password_hash, &payload.password)? {
                log::debug!("Password verified for user: {}", user.username);
                let token = generate_access_token(user.uid, &config.jwt_config)?;
                log::info!("Token generated for user: {}", user.username);
                Ok(HttpResponse::Ok().json(LoginResponse {
                    token,
                    token_type: "Bearer".to_string(),
                }))
            } else {
                log::warn!("Invalid password attempt for user: {}", payload.username);
                Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid username or password"
                })))
            }
        }
        Err(DbError::NotFound) => {
            log::warn!("User not found: {}", payload.username);
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid username or password"
            })))
        }
        Err(e) => {
            log::error!("Login failed for {}: {:?}", payload.username, e);
            Err(e.into())
        }
    }
}

pub async fn protected_endpoint(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    let token = credentials.token();
    let config = req.app_data::<web::Data<FaaSConfig>>().unwrap();
    match validate_access_token(token, &config.jwt_config) {
        Ok(claims) => {
            log::info!("Access granted to user with ID: {}", claims.sub);
            req.extensions_mut().insert(claims);
            Ok(req)
        }
        Err(AuthError::TokenExpired) => {
            log::warn!("Token expired");
            Err(ErrorUnauthorized("Token expired"))
        }
        Err(AuthError::InvalidToken) => {
            log::warn!("Invalid token");
            Err(ErrorUnauthorized("Invalid token"))
        }
        Err(_) => {
            log::error!("Unexpected authentication");
            Err(ErrorUnauthorized("Unexpected authentication error"))
        }
    }
}
