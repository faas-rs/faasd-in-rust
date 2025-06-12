// 引入 bb8 连接池相关的类型，以及 diesel-async 的异步 PostgreSQL 连接和管理器
use super::error::DbError;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::{self, Pool};
// 定义数据库连接池的类型别名，方便使用
// DbPool 代表一个 bb8 连接池，它管理着 AsyncPgConnection 类型的连接
pub type DbPool = Pool<AsyncPgConnection>;
pub async fn create_pool(database_url: &str) -> Result<DbPool, DbError> {
    // 使用提供的数据库 URL 创建一个异步 Diesel 连接管理器
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    // 使用 bb8 的 Pool::builder() 来构建连接池
    bb8::Pool::builder()
        .build(config) // 应用连接管理器配置
        .await // 因为构建过程可能是异步的
        .map_err(|e| {
            log::error!("Failed to create database connection pool: {}", e);
            DbError::PoolError(e.to_string()) // 返回错误
        })
}
