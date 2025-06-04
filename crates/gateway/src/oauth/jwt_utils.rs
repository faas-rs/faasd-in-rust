// filepath: /home/dragonos/Work/faasd-in-rust/crates/gateway/src/oauth/jwt_utils.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use chrono::{Utc, Duration};
use crate::models::Error as AppError; // 确保你的 Error 枚举中有 JwtError, TokenExpired, InvalidToken
use uuid::Uuid;
use crate::types::config::JwtConfig; // 引入你的 JwtConfig
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: Uuid, // Subject (user_id)
    pub exp: usize,  // Expiration time (timestamp)
    pub iat: usize,  // Issued at (timestamp）
}

pub fn generate_access_token(user_id: Uuid, jwt_config: &JwtConfig) -> Result<String, AppError> {
    let now = Utc::now();
    let expiration = now + Duration::seconds(jwt_config.access_token_ttl_seconds);
    let claims = AccessTokenClaims {
        sub: user_id,
        exp: expiration.timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_config.secret.as_ref()))
        .map_err(|e| {
            log::error!("Failed to generate access token: {}", e);
            AppError::JwtError(e.to_string())
        })
}

pub fn validate_access_token(token: &str, jwt_config: &JwtConfig) -> Result<AccessTokenClaims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256); // 确保算法与生成时一致
    validation.validate_exp = true;
     validation.leeway = 60; // 可选：允许60秒的 leeway
    decode::<AccessTokenClaims>(token, &DecodingKey::from_secret(jwt_config.secret.as_ref()), &validation)
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                log::debug!("Access token expired");
                AppError::TokenExpired
            }
            _ => {
                log::warn!("Invalid access token: {}", e);
                AppError::InvalidToken
            }
        })
}