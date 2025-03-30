use crate::consts::DEFAULT_FUNCTION_NAMESPACE;
use crate::handlers::function_get::get_function;
use actix_web::{Error, error::ErrorInternalServerError};
use log;
use url::Url;

pub struct InvokeResolver {
    client: service::Service,
}

impl InvokeResolver {
    pub async fn new(client: service::Service) -> Self {
        Self { client }
    }

    pub async fn resolve(&self, function_name: &str) -> Result<Url, Error> {
        //根据函数名和containerd获取函数ip，
        //从函数名称中提取命名空间。如果函数名称中包含 .，则将其后的部分作为命名空间；否则使用默认命名空间

        let mut actual_function_name = function_name;
        let namespace = get_namespace_or_default(function_name, DEFAULT_FUNCTION_NAMESPACE);
        if function_name.contains('.') {
            actual_function_name = function_name.trim_end_matches(&format!(".{}", namespace));
        }

        let function = match get_function(&self.client, actual_function_name, &namespace).await {
            Ok(function) => function,
            Err(e) => {
                log::error!("Failed to get function:{}", e);
                return Err(ErrorInternalServerError("Failed to get function"));
            }
        };

        let ip = function.ip;
        let port = 8080;

        let urlstr = format!("http://{}:{}", ip, port);
        match Url::parse(&urlstr) {
            Ok(url) => Ok(url),
            Err(e) => {
                log::error!("Failed to resolve url:{}", e);
                Err(ErrorInternalServerError("Failed to resolve URL"))
            }
        }
    }
}

fn get_namespace_or_default(function_name: &str, default_namespace: &str) -> String {
    let mut namespace = default_namespace.to_string();
    if function_name.contains('.') {
        if let Some(index) = function_name.rfind('.') {
            namespace = function_name[index + 1..].to_string();
        }
    }
    namespace
}
