use std::time::Duration;

const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_MAX_IDLE_CONNS: usize = 1024;

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
}
#[derive(Debug, Clone)]
pub struct FaaSConfig {
    pub tcp_port: Option<u16>,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub enable_health: bool,
    pub enable_basic_auth: bool,
    pub secret_mount_path: String,
    pub max_idle_conns: usize,
    pub max_idle_conns_per_host: usize,
    pub jwt_config: JwtConfig,
}

impl Default for FaaSConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl FaaSConfig {
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let access_token_ttl_seconds = std::env::var("ACCESS_TOKEN_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string()) // 默认1小时
            .parse::<i64>()
            .expect("ACCESS_TOKEN_TTL_SECONDS must be an integer");
        let refresh_token_ttl_seconds = std::env::var("REFRESH_TOKEN_TTL_SECONDS")
            .unwrap_or_else(|_| "604800".to_string()) // 默认7天
            .parse::<i64>()
            .expect("REFRESH_TOKEN_TTL_SECONDS must be an integer");
        Self {
            tcp_port: None,
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
            enable_health: false,
            enable_basic_auth: false,
            secret_mount_path: String::from("/var/openfaas/secrets"),
            max_idle_conns: 0,
            max_idle_conns_per_host: 10,
            jwt_config: JwtConfig {
                secret: jwt_secret,
                access_token_ttl_seconds,
                refresh_token_ttl_seconds,
            },
        }
    }
    pub fn get_read_timeout(&self) -> Duration {
        if self.read_timeout <= Duration::from_secs(0) {
            DEFAULT_READ_TIMEOUT
        } else {
            self.read_timeout
        }
    }

    pub fn get_max_idle_conns(&self) -> usize {
        if self.max_idle_conns < 1 {
            DEFAULT_MAX_IDLE_CONNS
        } else {
            self.max_idle_conns
        }
    }

    pub fn get_max_idle_conns_per_host(&self) -> usize {
        if self.max_idle_conns_per_host < 1 {
            self.get_max_idle_conns()
        } else {
            self.max_idle_conns_per_host
        }
    }
}
