use std::{collections::HashMap, str::FromStr};

use derive_more::Display;
use serde::{Deserialize, Serialize};

// ── Error types (from gateway::handlers::function) ──────────────

#[derive(Debug, Display)]
pub enum DeployError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("Conflict: {}", _0)]
    Conflict(String),
    #[display("Cancelled")]
    Cancelled,
}

#[derive(Debug, Display)]
pub enum DeleteError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Internal: {}", _0)]
    Internal(String),
}

#[derive(Debug, Display)]
pub enum ResolveError {
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("Busy: {}", _0)]
    Busy(String),
    #[display("Unavailable: {}", _0)]
    Unavailable(String),
}

#[derive(Debug, Display)]
pub enum ListError {
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
}

#[derive(Debug, Display)]
pub enum UpdateError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
}

// ── Error type (from gateway::handlers::namespace) ──────────────

#[derive(Debug, Display)]
pub enum NamespaceError {
    #[display("Invalid: {}", _0)]
    Invalid(String),
    #[display("AlreadyExists: {}", _0)]
    AlreadyExists(String),
    #[display("NotFound: {}", _0)]
    NotFound(String),
    #[display("Internal: {}", _0)]
    Internal(String),
    #[display("MethodNotAllowed: {}", _0)]
    MethodNotAllowed(String),
}

// ── Domain types (from gateway::types::function) ────────────────

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    /// Service is the name of the function deployment
    pub function_name: String,

    /// Image is a fully-qualified container image
    pub image: String,

    /// Namespace for the function, if supported by the faas-provider
    pub namespace: Option<String>,

    /// EnvProcess overrides the fprocess environment variable and can be used
    /// with the watchdog
    pub env_process: Option<String>,

    /// EnvVars can be provided to set environment variables for the function runtime.
    pub env_vars: Option<HashMap<String, String>>,

    /// Constraints are specific to the faas-provider.
    pub constraints: Option<Vec<String>>,

    /// Secrets list of secrets to be made available to function
    pub secrets: Option<Vec<String>>,

    /// Labels are metadata for functions which may be used by the
    /// faas-provider or the gateway
    pub labels: Option<HashMap<String, String>>,

    /// Annotations are metadata for functions which may be used by the
    /// faas-provider or the gateway
    pub annotations: Option<HashMap<String, String>>,

    /// Limits for function
    pub limits: Option<Resources>,

    /// Requests of resources requested by function
    pub requests: Option<Resources>,

    /// ReadOnlyRootFilesystem removes write-access from the root filesystem
    /// mount-point.
    #[serde(default = "default_read_only_root_filesystem")]
    pub read_only_root_filesystem: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Resources {
    /// The amount of memory that is allocated for the function
    pub memory: Option<String>,

    /// The amount of CPU that is allocated for the function
    pub cpu: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// CPU usage increase since the last measurement, equivalent to Kubernetes' concept of millicores
    pub cpu: Option<f64>,

    /// Total memory usage in bytes
    pub total_memory_bytes: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// The name of the function
    pub function_name: String,

    /// The fully qualified docker image name of the function
    pub image: String,

    /// The namespace of the function
    pub namespace: Option<String>,

    /// Process for watchdog to fork
    pub env_process: Option<String>,

    /// Environment variables for the function runtime
    pub env_vars: Option<HashMap<String, String>>,

    /// Constraints are specific to OpenFaaS Provider
    pub constraints: Option<Vec<String>>,

    /// An array of names of secrets that are made available to the function
    pub secrets: Option<Vec<String>>,

    /// A map of labels for making scheduling or routing decisions
    pub labels: Option<HashMap<String, String>>,

    /// A map of annotations for management, orchestration, events, and build tasks
    pub annotations: Option<HashMap<String, String>>,

    /// Limits for function resources
    pub limits: Option<Resources>,

    /// Requests for function resources
    pub requests: Option<Resources>,

    /// Removes write-access from the root filesystem mount-point
    #[serde(default = "default_read_only_root_filesystem")]
    pub read_only_root_filesystem: bool,

    /// The amount of invocations for the specified function
    pub invocation_count: Option<i32>,

    /// Desired amount of replicas
    pub replicas: Option<i32>,

    /// The current available amount of replicas
    pub available_replicas: Option<i32>,

    /// The time read back from the faas backend's data store for when the function or its container was created
    pub created_at: Option<String>,

    /// Usage statistics for the function
    pub usage: Option<Usage>,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct Query {
    /// Name of deployed function
    pub function_name: String,

    /// Namespace of deployed function
    pub namespace: Option<String>,
}

/// TODO: 其实应该是 try from, 排除非法的函数名
impl FromStr for Query {
    type Err = ();

    fn from_str(function_name: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(index) = function_name.rfind('.') {
            Self {
                function_name: function_name[..index].to_string(),
                namespace: Some(function_name[index + 1..].to_string()),
            }
        } else {
            Self {
                function_name: function_name.to_string(),
                namespace: Some("default".to_string()),
            }
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Delete {
    /// Name of deployed function
    pub function_name: String,
    pub namespace: String,
}

pub const fn default_read_only_root_filesystem() -> bool {
    false
}

// ── Domain type (from gateway::types::namespace) ────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct Namespace {
    pub name: Option<String>,
    pub labels: HashMap<String, String>,
}
