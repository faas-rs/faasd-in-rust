use crate::models::Error as AppError;
use crate::oauth::jwt_utils::{AccessTokenClaims, generate_access_token, validate_access_token};
use crate::oauth::services::{self, UserService};
use crate::types::config::FaaSConfig; // 确保 JwtConfig 被正确导入
use actix_web::HttpMessage;
use actix_web::error::ErrorUnauthorized;
use actix_web::{Error, dev::ServiceRequest};
use actix_web::{FromRequest, HttpRequest, HttpResponse, dev::Payload, web};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use serde::{Deserialize, Serialize};
use std::future::{Ready, ready}; // 用于认证错误// 确保导入 verify_password
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
        AppError::DieselError(diesel::result::Error::NotFound) // 这里可以根据实际情况调整错误类型
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
        AppError::DieselError(diesel::result::Error::NotFound)
    })?;
    let user = UserService::find_user_by_username(&payload.username, &mut conn).await?;

    if services::verify_password(&user.password_hash, &payload.password)? {
        log::debug!("Password verified for user: {}", user.username);
        // 使用 FaaSConfig 中的 jwt_config
        let token = generate_access_token(user.uid, &config.jwt_config)?;
        log::info!("Token generated for user: {}", user.username);
        Ok(HttpResponse::Ok().json(LoginResponse {
            token,
            token_type: "Bearer".to_string(),
        }))
    } else {
        log::warn!("Invalid password attempt for user: {}", payload.username);
        Err(AppError::InvalidToken.into())
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
            {
                let mut extensions = req.extensions_mut();
                extensions.insert(claims);
            }
            Ok(req)
        }
        Err(AppError::TokenExpired) => {
            log::warn!("Token expired");
            Err(actix_web::error::ErrorUnauthorized("Token expired"))
        }
        Err(AppError::InvalidToken) => {
            log::warn!("Invalid token");
            Err(actix_web::error::ErrorUnauthorized("Invalid token"))
        }
        Err(_) => {
            log::error!("Unexpected error");
            Err(actix_web::error::ErrorInternalServerError(
                "Unexpected error",
            ))
        }
    }
}
pub struct AuthenticatedUser {
    pub claims: AccessTokenClaims, // 包含 user_id (sub) 和其他声明
}
impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");
        let config = req.app_data::<web::Data<FaaSConfig>>();

        if auth_header.is_none() || config.is_none() {
            log::warn!("Missing Authorization header or FaaSConfig in app_data");
            return ready(Err(ErrorUnauthorized(
                "Missing Authorization header or server misconfiguration",
            )));
        }

        let config_data = config.unwrap(); // We checked it's Some

        let auth_str = match auth_header.unwrap().to_str() {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Invalid characters in Authorization header");
                return ready(Err(ErrorUnauthorized(
                    "Invalid Authorization header format",
                )));
            }
        };
        if !auth_str.starts_with("Bearer ") {
            log::warn!("Authorization header does not start with Bearer");
            return ready(Err(ErrorUnauthorized("Invalid token type")));
        }
        // 提取 Bearer 后的 token
        let token = &auth_str["Bearer ".len()..];

        match validate_access_token(token, &config_data.jwt_config) {
            Ok(claims) => {
                log::debug!("Token validated successfully for sub: {}", claims.sub);
                ready(Ok(AuthenticatedUser { claims }))
            }
            Err(AppError::TokenExpired) => {
                log::info!("Access token expired");
                ready(Err(ErrorUnauthorized("Token expired")))
            }
            Err(e) => {
                log::warn!("Invalid token: {:?}", e);
                ready(Err(ErrorUnauthorized("Invalid token")))
            }
        }
    }
}
