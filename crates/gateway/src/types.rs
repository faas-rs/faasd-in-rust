use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub function_name: String,
    pub image: String,
    pub namespace: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    pub limits: Option<Resources>,
    pub requests: Option<Resources>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    pub memory: Option<String>,
    pub cpu: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub memory: Option<f64>,
    pub cpu: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Status {
    pub name: String,
    pub image: String,
    pub namespace: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    pub created_at: String,
    pub available_replicas: u64,
    pub invocation_count: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub function_name: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delete {
    pub function_name: String,
    pub namespace: Option<String>,
}

// ---------------------------------------------------------------------------
// Resolve response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ResolveResponse {
    pub url: String,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DeployError {
    Invalid(String),
    Internal(String),
    Conflict(String),
    Cancelled,
}

impl std::fmt::Display for DeployError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeployError::Invalid(msg) => write!(f, "invalid deployment: {msg}"),
            DeployError::Internal(msg) => write!(f, "internal error: {msg}"),
            DeployError::Conflict(msg) => write!(f, "conflict: {msg}"),
            DeployError::Cancelled => write!(f, "deployment cancelled"),
        }
    }
}

impl std::error::Error for DeployError {}

#[derive(Debug)]
pub enum DeleteError {
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::NotFound(msg) => write!(f, "not found: {msg}"),
            DeleteError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for DeleteError {}

#[derive(Debug)]
pub enum ResolveError {
    NotFound(String),
    Invalid(String),
    Internal(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound(msg) => write!(f, "not found: {msg}"),
            ResolveError::Invalid(msg) => write!(f, "invalid: {msg}"),
            ResolveError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ResolveError {}

#[derive(Debug)]
pub enum ListError {
    Internal(String),
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ListError {}

#[derive(Debug)]
pub enum UpdateError {
    Invalid(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Invalid(msg) => write!(f, "invalid update: {msg}"),
            UpdateError::NotFound(msg) => write!(f, "not found: {msg}"),
            UpdateError::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for UpdateError {}
