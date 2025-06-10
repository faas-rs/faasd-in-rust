// src/services.rs
use crate::models::error::DbError;
use crate::models::{NewUser, User}; // 引入 models 中的 User 和 NewUser
use crate::oauth::error::AuthError;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::PooledConnection;
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    // 返回 Result 类型
    if password.is_empty() {
        log::error!("Password cannot be empty");
        return Err(AuthError::PasswordHashingError(
            "Password cannot be empty".to_string(),
        ));
    }
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    // password.as_bytes() 将 &str 转换为 &[u8]
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hashed_password_obj) => Ok(hashed_password_obj.to_string()),
        Err(e) => {
            log::error!("Password hashing failed: {}", e);
            // 将 argon2 的错误转换为你的应用错误类型
            Err(AuthError::PasswordHashingError(e.to_string()))
        }
    }
}
pub fn verify_password(
    hashed_password_from_db: &str,
    password_to_verify: &str,
) -> Result<bool, AuthError> {
    // 从数据库中存储的哈希字符串（PHC string format）解析出 PasswordHash 对象
    // 这个对象包含了哈希值本身以及哈希时使用的参数（如 salt, version, algorithm等）
    let parsed_hash = match PasswordHash::new(hashed_password_from_db) {
        Ok(h) => h,
        Err(e) => {
            log::error!(
                "Failed to parse stored password hash: {}. Hash string: '{}'",
                e,
                hashed_password_from_db
            );
            // 如果存储的哈希格式不正确，这通常是一个严重的问题，但也应视为验证失败
            // 返回一个特定的错误或通用的密码哈希错误
            return Err(AuthError::PasswordHashingError(format!(
                "Invalid stored hash format: {}",
                e
            )));
        }
    };

    // 使用 Argon2 实例来验证密码。
    // 重要：verify_password 会使用 parsed_hash 中包含的参数（salt, version等）进行验证，
    // 而不是 Argon2::default() 中配置的默认参数。这是正确的行为，确保了即使将来更改了默认参数，
    // 旧的哈希仍然可以被正确验证。
    match Argon2::default().verify_password(password_to_verify.as_bytes(), &parsed_hash) {
        Ok(_) => {
            // 验证成功（密码匹配）
            log::debug!("Password verification successful.");
            Ok(true)
        }
        Err(argon2::password_hash::Error::Password) => {
            // 密码不匹配，这是 verify_password 预期的“错误”类型
            log::debug!("Password verification failed: incorrect password.");
            Ok(false)
        }
        Err(e) => {
            // 其他类型的错误，例如参数不匹配（不太可能发生，因为参数来自 parsed_hash）
            // 或者其他内部错误。
            log::error!(
                "Password verification failed with unexpected error: {:?}",
                e
            );
            Err(AuthError::PasswordHashingError(format!(
                "Verification process error: {}",
                e
            )))
        }
    }
}

pub struct UserService;

impl UserService {
    pub async fn register_user(
        username_val: String,
        plain_password: String,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, AuthError> {
        log::info!("Attempting to register user with username:{}", username_val);
        let hashed_password = match hash_password(&plain_password) {
            Ok(phc) => phc,
            Err(e) => {
                log::error!(
                    "Password hashing failed during registration for user {}: {:?}",
                    username_val,
                    e
                );
                return Err(e);
            }
        };
        log::debug!("Password hashed successfully for user: {}", username_val);

        let new_user_data = NewUser {
            username: username_val.clone(),
            password_hash: hashed_password,
        };

        // 调用模型层的 create 方法
        match new_user_data.create(conn).await {
            Ok(user) => {
                log::info!(
                    "User {} registered successfully with uid: {}",
                    user.username,
                    user.uid
                );
                Ok(user)
            }
            Err(e) => {
                log::error!("Failed to register user {}: {:?}", username_val, e);
                Err(AuthError::AlreadyExists(e.to_string()))
            }
        }
    }
    // 创建用户
    pub async fn create_user(
        new_user: &NewUser,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, DbError> {
        new_user.create(conn).await
    }

    // 根据用户名查询用户
    pub async fn find_user_by_username(
        username: &str,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, DbError> {
        User::find_by_username(username, conn).await
    }

    // 根据 UUID 查询用户
    pub async fn find_user_by_uuid(
        uuid: uuid::Uuid,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, DbError> {
        User::find_by_uuid(uuid, conn).await
    }

    // 更新用户名
    pub async fn update_username(
        input_uid: uuid::Uuid,
        new_username: &str,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, DbError> {
        User::update_username_by_uid(input_uid, new_username, conn).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::schema::users::dsl::users; // 导入表定义
    use crate::models::*;
    use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
    use diesel_async::{AsyncPgConnection, RunQueryDsl};
    use std::env;
    async fn setup_test_db() -> Result<Pool<AsyncPgConnection>, DbError> {
        let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://dragonos:vitus@localhost/diesel_demo_db_dragonos".to_string()
        });
        let pool = db::create_pool(&database_url).await?;
        Ok(pool)
    }
    async fn clear_users_table(conn: &mut PooledConnection<'_, AsyncPgConnection>) {
        diesel::delete(users)
            .execute(conn)
            .await
            .expect("Failed to clear users table");
    }
    #[tokio::test]
    async fn test_register_user_success() {
        let pool = setup_test_db().await.expect("failed to set up test"); // 这里能用 ?
        let mut conn = pool
            .get()
            .await
            .map_err(|e| DbError::PoolError(e.to_string()))
            .expect("fialed to get connection");

        // 清理测试数据
        clear_users_table(&mut conn).await;

        let username = "test_user";
        let password = "test_password";

        // 调用注册方法
        let result =
            UserService::register_user(username.to_string(), password.to_string(), &mut conn).await;

        // 验证结果
        assert!(result.is_ok(), "User registration failed");
        let user = result.unwrap();
        assert_eq!(user.username, username, "Username mismatch");
        assert!(
            !user.password_hash.is_empty(),
            "Password hash should not be empty"
        );

        // 验证数据库中是否插入了用户
        let db_user = UserService::find_user_by_username(username, &mut conn)
            .await
            .expect("Failed to query user");
        assert_eq!(db_user.uid, user.uid, "UID mismatch");
        assert_eq!(db_user.username, user.username, "Username mismatch");
        clear_users_table(&mut conn).await;
    }

    #[tokio::test]
    async fn test_register_user_duplicate_username() {
        let pool = setup_test_db().await.expect("failed to set up test");
        let mut conn = pool
            .get()
            .await
            .map_err(|e| DbError::PoolError(e.to_string()))
            .expect("fialed to get connection");

        // 清理测试数据
        clear_users_table(&mut conn).await;

        let username = "duplicate_user";
        let password = "test_password";

        // 第一次注册
        let _ =
            UserService::register_user(username.to_string(), password.to_string(), &mut conn).await;

        // 第二次注册，应该失败
        let result =
            UserService::register_user(username.to_string(), password.to_string(), &mut conn).await;

        // 验证结果
        assert!(result.is_err(), "Duplicate username should fail");
        // let error = result.err().unwrap();
        // assert!(
        //     matches!(error, AuthError::AlreadyExists(_)),
        //     "Expected AuthError for duplicate username"
        // );
        // clear_users_table(&mut conn).await;
    }

    #[tokio::test]
    async fn test_register_user_password_hash_failure() {
        let pool = setup_test_db().await.expect("failed to set up test");
        let mut conn = pool
            .get()
            .await
            .map_err(|e| DbError::PoolError(e.to_string()))
            .expect("fialed to get connection");
        // 清理测试数据
        clear_users_table(&mut conn).await;

        let username = "hash_failure_user";
        let password = ""; //空密码会哈希失败

        // 调用注册方法
        let result =
            UserService::register_user(username.to_string(), password.to_string(), &mut conn).await;

        // 验证结果
        assert!(result.is_err(), "Password hashing failure should fail");
        let error = result.err().unwrap();
        assert!(
            matches!(error, AuthError::PasswordHashingError(_)),
            "Expected PasswordHashingError"
        );
        clear_users_table(&mut conn).await;
    }
}
