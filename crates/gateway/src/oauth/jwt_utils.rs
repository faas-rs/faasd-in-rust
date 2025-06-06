//生成jsonwebtoken
use crate::models::Error as AppError; // 确保你的 Error 枚举中有 JwtError, TokenExpired, InvalidToken
use crate::types::config::JwtConfig; // 引入你的 JwtConfig
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessTokenClaims {
    pub sub: Uuid,  // users.user_id,use uuid as user identifier
    pub exp: usize, // Expiration time (timestamp),jwtconfig.access_token_ttl_seconds
    pub iat: usize, // Issued at (timestamp）
}

pub fn generate_access_token(user_id: Uuid, jwt_config: &JwtConfig) -> Result<String, AppError> {
    let now = Utc::now();
    let expiration = now + Duration::seconds(jwt_config.access_token_ttl_seconds);
    let claims = AccessTokenClaims {
        sub: user_id,
        exp: expiration.timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    //The default algorithm is HS256
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_config.secret.as_ref()),
    )
    .map_err(|e| {
        log::error!("Failed to generate access token: {}", e);
        AppError::JwtError(e.to_string())
    })
}
pub fn validate_access_token(
    token: &str,
    jwt_config: &JwtConfig,
) -> Result<AccessTokenClaims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256); // 确保算法与生成时一致
    validation.validate_exp = true;
    validation.leeway = 60; // 可选：允许60秒的 leeway，防止始终不对齐
    decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_ref()),
        &validation,
    )
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
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_generate_and_validate_token() {
        let jwt_config = JwtConfig {
            secret: "my_secret_key".to_string(),
            access_token_ttl_seconds: 3600,   // 1小时
            refresh_token_ttl_seconds: 86400, // 1天
        };

        let user_id = Uuid::new_v4();
        let token = generate_access_token(user_id, &jwt_config).expect("Failed to generate token");

        let claims = validate_access_token(&token, &jwt_config).expect("Failed to validate token");
        print!("\nGenerated Token: {}\n", token);
        print!("claims:{:?}", claims);
        assert_eq!(claims.sub, user_id);
        assert!(claims.exp > claims.iat); // 过期时间应该大于签发时间
    }

    #[test]
    fn test_expired_token() {
        let jwt_config = JwtConfig {
            secret: "my_secret_key".to_string(),
            access_token_ttl_seconds: -3600, // 过期时间设置为过去
            refresh_token_ttl_seconds: 86400,
        };

        let user_id = Uuid::new_v4();
        let token = generate_access_token(user_id, &jwt_config).expect("Failed to generate token");

        let result = validate_access_token(&token, &jwt_config);
        assert!(matches!(result, Err(AppError::TokenExpired)));
    }

    #[test]
    fn test_invalid_token() {
        let jwt_config = JwtConfig {
            secret: "my_secret_key".to_string(),
            access_token_ttl_seconds: 3600,
            refresh_token_ttl_seconds: 86400,
        };

        let invalid_token = "invalid.token.string";
        let result = validate_access_token(invalid_token, &jwt_config);
        assert!(matches!(result, Err(AppError::InvalidToken)));
    }
}
