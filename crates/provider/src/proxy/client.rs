use std::time::Duration;

use reqwest::{Client, redirect};

use crate::types::config::FaaSConfig;

//构建client
pub async fn new_proxy_client_from_config(config: &FaaSConfig) -> Client {
    new_proxy_client(
        config.get_read_timeout(),
        /*config.get_max_idle_conns(),*/ config.get_max_idle_conns_per_host(),
    )
    .await
}

//根据FaasConfig参数来设置Client
pub async fn new_proxy_client(
    timeout: Duration,
    //max_idle_conns: usize,
    max_idle_conns_per_host: usize,
) -> Client {
    Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout)
        .pool_max_idle_per_host(max_idle_conns_per_host)
        .pool_idle_timeout(Duration::from_millis(120))
        .tcp_keepalive(120 * Duration::from_secs(1))
        .redirect(redirect::Policy::none())
        .tcp_nodelay(true)
        .build()
        .expect("Failed to create client")
}
