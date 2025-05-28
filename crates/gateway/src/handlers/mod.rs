pub mod function;
pub mod namespace;
pub mod proxy;

#[derive(Debug, thiserror::Error)]
pub struct FaasError {
    message: String,
    error_type: FaasErrorType,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug)]
pub enum FaasErrorType {
    ContainerFailure,
    Timeout,
    InternalError,
}

impl std::fmt::Display for FaasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.error_type, self.message)
    }
}

// 实现从常见错误类型转换
impl From<std::io::Error> for FaasError {
    fn from(err: std::io::Error) -> Self {
        FaasError {
            message: format!("IO error: {}", err),
            error_type: FaasErrorType::InternalError,
            source: Some(Box::new(err)),
        }
    }
}
