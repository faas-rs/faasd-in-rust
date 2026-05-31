pub mod provider;
pub mod types;

pub use provider::Provider;
pub use types::*;

use axum::{extract::{Path, State}, http::StatusCode, routing::{get, post}, Json, Router};
use axum::extract::Query as AxumQuery;
use std::collections::HashMap;
use std::sync::Arc;

pub fn app<P: Provider>(provider: Arc<P>) -> Router {
    Router::new()
        .route("/deploy", post(handle_deploy::<P>))
        .route("/delete", post(handle_delete::<P>))
        .route("/resolve/{name}", get(handle_resolve::<P>))
        .route("/list", get(handle_list::<P>))
        .route("/update", post(handle_update::<P>))
        .route("/status/{name}", get(handle_status::<P>))
        .with_state(provider)
}

// ---------------------------------------------------------------------------
// Handler helpers
// ---------------------------------------------------------------------------

fn namespace_from_query(params: &HashMap<String, String>) -> String {
    params
        .get("namespace")
        .cloned()
        .unwrap_or_else(|| "openfaas-fn".to_string())
}

fn error_json(msg: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": msg }))
}

// ---------------------------------------------------------------------------
// POST /deploy
// ---------------------------------------------------------------------------

async fn handle_deploy<P: Provider>(
    State(provider): State<Arc<P>>,
    Json(deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    provider
        .deploy(deployment)
        .await
        .map(|_| (StatusCode::ACCEPTED, Json(serde_json::json!({}))))
        .map_err(|e| {
            let (code, msg) = match &e {
                DeployError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                DeployError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
                DeployError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
                DeployError::Cancelled => {
                    (StatusCode::BAD_GATEWAY, "deployment cancelled".to_string())
                }
            };
            tracing::warn!(error = %e, "deploy failed");
            (code, error_json(&msg))
        })
}

// ---------------------------------------------------------------------------
// POST /delete
// ---------------------------------------------------------------------------

async fn handle_delete<P: Provider>(
    State(provider): State<Arc<P>>,
    Json(query): Json<Query>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    provider
        .delete(query)
        .await
        .map(|_| (StatusCode::OK, Json(serde_json::json!({}))))
        .map_err(|e| {
            let (code, msg) = match &e {
                DeleteError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                DeleteError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            };
            tracing::warn!(error = %e, "delete failed");
            (code, error_json(&msg))
        })
}

// ---------------------------------------------------------------------------
// GET /resolve/{name}
// ---------------------------------------------------------------------------

async fn handle_resolve<P: Provider>(
    Path(name): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    State(provider): State<Arc<P>>,
) -> Result<Json<ResolveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let namespace = namespace_from_query(&params);
    let query = Query {
        function_name: name,
        namespace: Some(namespace),
    };
    provider
        .resolve(query)
        .await
        .map(|uri| {
            Json(ResolveResponse {
                url: uri.to_string(),
            })
        })
        .map_err(|e| {
            let (code, msg) = match &e {
                ResolveError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                ResolveError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                ResolveError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            };
            tracing::warn!(error = %e, "resolve failed");
            (code, error_json(&msg))
        })
}

// ---------------------------------------------------------------------------
// GET /list
// ---------------------------------------------------------------------------

async fn handle_list<P: Provider>(
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    State(provider): State<Arc<P>>,
) -> Result<Json<Vec<Status>>, (StatusCode, Json<serde_json::Value>)> {
    let namespace = namespace_from_query(&params);
    provider
        .list(namespace)
        .await
        .map(Json)
        .map_err(|e| {
            let msg = match &e {
                ListError::Internal(msg) => msg.clone(),
            };
            tracing::warn!(error = %e, "list failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_json(&msg),
            )
        })
}

// ---------------------------------------------------------------------------
// POST /update
// ---------------------------------------------------------------------------

async fn handle_update<P: Provider>(
    State(provider): State<Arc<P>>,
    Json(deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    provider
        .update(deployment)
        .await
        .map(|_| (StatusCode::ACCEPTED, Json(serde_json::json!({}))))
        .map_err(|e| {
            let (code, msg) = match &e {
                UpdateError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                UpdateError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                UpdateError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            };
            tracing::warn!(error = %e, "update failed");
            (code, error_json(&msg))
        })
}

// ---------------------------------------------------------------------------
// GET /status/{name}
// ---------------------------------------------------------------------------

async fn handle_status<P: Provider>(
    Path(name): Path<String>,
    AxumQuery(params): AxumQuery<HashMap<String, String>>,
    State(provider): State<Arc<P>>,
) -> Result<Json<Status>, (StatusCode, Json<serde_json::Value>)> {
    let namespace = namespace_from_query(&params);
    let query = Query {
        function_name: name,
        namespace: Some(namespace),
    };
    provider
        .status(query)
        .await
        .map(Json)
        .map_err(|e| {
            let (code, msg) = match &e {
                ResolveError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
                ResolveError::Invalid(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
                ResolveError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            };
            tracing::warn!(error = %e, "status failed");
            (code, error_json(&msg))
        })
}
