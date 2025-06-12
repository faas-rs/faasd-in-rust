pub mod db;
pub mod error;
pub mod schema;
use crate::models::error::DbError;
use crate::models::schema::users; // 导入表定义
use crate::models::schema::users::dsl::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use diesel_async::pooled_connection::bb8::PooledConnection;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
// --- User 结构体 ---
#[derive(Queryable, Selectable, Identifiable, Debug, Serialize, Deserialize)]
#[diesel(table_name = users)] // 使用导入的 users 表
#[diesel(primary_key(uid))] // 指定主键，用于 Identifiable
pub struct User {
    pub uid: uuid::Uuid,
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)] // 避免序列化密码哈希
    pub password_hash: String,
    pub created_at: NaiveDateTime,
}

impl User {
    pub async fn find_by_username(
        input_username_val: &str, // 变量名稍作修改以示区分
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<Self, DbError> {
        users // 使用导入的 dsl `users` (表名)
            .filter(username.eq(input_username_val)) // 使用导入的 dsl `username` (列名)
            .select(User::as_select()) // User 需要 derive(Selectable)
            .first(conn)
            .await
            .map_err(DbError::from)
    }

    pub async fn find_by_uuid(
        input_uuid_val: Uuid, // 变量名稍作修改
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<Self, DbError> {
        users
            .filter(uid.eq(input_uuid_val)) // 使用导入的 dsl `uid` (列名)
            .select(User::as_select())
            .first(conn)
            .await
            .map_err(DbError::from)
    }

    // 更新指定 uid 用户的用户名
    pub async fn update_username_by_uid(
        target_uid: Uuid,
        new_username_val: &str,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<Self, DbError> {
        // 返回更新后的 User
        // 1. 检查新用户名是否已被其他用户占用
        let new_username_taken = users
            .filter(username.eq(new_username_val))
            .filter(uid.ne(target_uid)) // 确保不是当前用户自己
            .select(diesel::dsl::count_star())
            .get_result::<i64>(conn)
            .await;

        match new_username_taken {
            Ok(count) if count > 0 => return Err(DbError::Conflict), // 新用户名已被占用
            Err(diesel::result::Error::NotFound) => {}               // 正常，新用户名可用
            Err(e) => return Err(DbError::from(e)),                  // 其他数据库错误
            _ => {}                                                  // count is 0
        }

        // 2. 执行更新
        diesel::update(users.find(target_uid)) // users.find(pk) 需要 User derive Identifiable
            .set(username.eq(new_username_val))
            .get_result(conn) // 返回更新后的 User
            .await
            .map_err(DbError::from)
    }

    // 删除用户
    pub async fn delete_by_uuid(
        target_uid: Uuid,
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<usize, DbError> {
        // 返回删除的行数
        diesel::delete(users.find(target_uid))
            .execute(conn)
            .await
            .map_err(DbError::from)
    }
}

// --- NewUser 结构体 ---
// 用于创建新用户，通常不包含数据库生成的字段如 uid (如果DB生成) 和 created_at (如果DB生成)
#[derive(Insertable, Debug, Deserialize, Clone)] // 必须有 Insertable, Deserialize 用于从请求体转换
#[diesel(table_name = users)]
pub struct NewUser {
    // 如果 uid 是数据库通过 DEFAULT gen_random_uuid() 生成的，这里就不应该有 uid 字段
    // pub uid: uuid::Uuid, // 移除，如果数据库负责生成
    pub username: String,
    pub password_hash: String,
    // 如果 created_at 是数据库通过 DEFAULT NOW() 生成的，这里就不应该有 created_at 字段
    // pub created_at: NaiveDateTime, // 移除，如果数据库负责生成
}

impl NewUser {
    pub async fn create(
        &self, // self 是 NewUser 的实例
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<User, DbError> {
        // 返回创建的 User 记录
        diesel::insert_into(users)
            .values(self) // NewUser 必须 derive(Insertable)
            .get_result(conn) // 将插入结果转换为 User 类型
            .await
            .map_err(DbError::from)
    }
}

// --- UpdateUserPayload 结构体 (可选，用于部分更新) ---
#[derive(AsChangeset, Deserialize, Debug, Default)]
#[diesel(table_name = users)]
pub struct UpdateUserPayload {
    pub username: Option<String>,
    pub password_hash: Option<String>,
    // uid 和 created_at 通常不通过这种方式更新
}

// 可以在 User impl 中添加一个通用的更新方法
impl User {
    pub async fn update_by_uid(
        target_uid: Uuid,
        payload: &UpdateUserPayload, // 使用这个结构体进行部分更新
        conn: &mut PooledConnection<'_, AsyncPgConnection>,
    ) -> Result<Self, DbError> {
        diesel::update(users.find(target_uid))
            .set(payload) // UpdateUserPayload 需要 derive(AsChangeset)
            .get_result(conn)
            .await
            .map_err(DbError::from)
    }
}
