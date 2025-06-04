// 引入 bb8 连接池相关的类型，以及 diesel-async 的异步 PostgreSQL 连接和管理器
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::{self, Pool, PooledConnection};

// 定义数据库连接池的类型别名，方便使用
// DbPool 代表一个 bb8 连接池，它管理着 AsyncPgConnection 类型的连接
pub type DbPool = Pool<AsyncPgConnection>;

// 定义从连接池中获取到的单个数据库连接的类型别名
// PooledConnection<'static, AsyncPgConnection> 表示一个从池中借出的连接
// 'static 生命周期在这里通常意味着连接的生命周期由池来管理，而不是与某个特定的短期作用域绑定
pub type DbConn = PooledConnection<'static, AsyncPgConnection>; // 注意：这里的 'static 可能需要根据实际使用场景调整，
// 如果连接的生命周期与某个请求或特定作用域绑定，
// 可能需要更具体的生命周期参数。
// 对于 bb8，通常 PooledConnection<'a, M>，'a 是池的生命周期。
// 但作为类型别名，'static 可能是为了简化，假设池是 'static 的。

// 异步函数，用于创建数据库连接池
pub async fn create_pool(database_url: &str) -> DbPool {
    // 使用提供的数据库 URL 创建一个异步 Diesel 连接管理器
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    // 使用 bb8 的 Pool::builder() 来构建连接池
    bb8::Pool::builder()
        .build(config) // 应用连接管理器配置
        .await // 因为构建过程可能是异步的
        .expect("Failed to create DB pool") // 如果创建失败，则 panic
}
