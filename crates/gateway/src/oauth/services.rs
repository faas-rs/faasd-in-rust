// src/services.rs
use crate::models::{User, NewUser};  // 引入 models 中的 User 和 NewUser
use crate::models::Error;
use diesel_async::pooled_connection::bb8::PooledConnection;
use diesel_async::AsyncPgConnection;
pub struct UserService;

impl UserService {
    // 创建用户
    pub async fn create_user(
        new_user: &NewUser,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, Error> {
        new_user.create(conn).await
    }

    // 根据用户名查询用户
    pub async fn find_user_by_username(
        username: &str,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, Error> {
        User::find_by_username(username, conn).await
    }

    // 根据 UUID 查询用户
    pub async fn find_user_by_uuid(
        uuid: uuid::Uuid,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, Error> {
        User::find_by_uuid(uuid, conn).await
    }

    // 更新用户名
    pub async fn update_username(
        input_uid: uuid::Uuid,
        new_username: &str,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, Error> {
        User::update_username_by_uid(input_uid, new_username, conn).await
    }
}
