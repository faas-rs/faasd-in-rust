use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDeployment {
    /// Service is the name of the function deployment
    pub service: String,

    /// Image is a fully-qualified container image
    pub image: String,

    /// Namespace for the function, if supported by the faas-provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// EnvProcess overrides the fprocess environment variable and can be used
    /// with the watchdog
    #[serde(rename = "envProcess", skip_serializing_if = "Option::is_none")]
    pub env_process: Option<String>,

    /// EnvVars can be provided to set environment variables for the function runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<HashMap<String, String>>,

    /// Constraints are specific to the faas-provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<String>>,

    /// Secrets list of secrets to be made available to function
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<String>>,

    /// Labels are metadata for functions which may be used by the
    /// faas-provider or the gateway
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,

    /// Annotations are metadata for functions which may be used by the
    /// faas-provider or the gateway
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,

    /// Limits for function
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<FunctionResources>,

    /// Requests of resources requested by function
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<FunctionResources>,

    /// ReadOnlyRootFilesystem removes write-access from the root filesystem
    /// mount-point.
    #[serde(rename = "readOnlyRootFilesystem", default)]
    pub read_only_root_filesystem: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionResources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
}
