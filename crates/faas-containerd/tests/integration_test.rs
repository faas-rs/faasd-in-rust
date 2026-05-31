use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use axum::body::Body;
use gateway::provider::Provider;
use gateway::types::{
    DeleteError, DeployError, Deployment, ListError, Query, ResolveError, Status, UpdateError,
};
use http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

/// Mock provider for integration testing the axum router.
struct MockProvider {
    functions: Mutex<HashMap<String, Status>>,
}

impl MockProvider {
    fn new() -> Self {
        Self {
            functions: Mutex::new(HashMap::new()),
        }
    }

    fn key(function_name: &str, namespace: &str) -> String {
        format!("{}/{}", namespace, function_name)
    }
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    async fn deploy(&self, deployment: Deployment) -> Result<(), DeployError> {
        let key = Self::key(&deployment.function_name, &deployment.namespace);
        let mut funcs = self.functions.lock().unwrap();
        if funcs.contains_key(&key) {
            return Err(DeployError::Conflict(format!(
                "function {} already exists",
                deployment.function_name
            )));
        }
        funcs.insert(
            key,
            Status {
                name: deployment.function_name.clone(),
                image: deployment.image.clone(),
                namespace: deployment.namespace.clone(),
                labels: deployment.labels.clone(),
                annotations: deployment.annotations.clone(),
                env_vars: deployment.env_vars.clone(),
                created_at: String::new(),
                available_replicas: 1,
                invocation_count: 0,
                status: "Ready".into(),
            },
        );
        Ok(())
    }

    async fn delete(&self, query: Query) -> Result<(), DeleteError> {
        let namespace = query.namespace.as_deref().unwrap_or("default");
        let key = Self::key(&query.function_name, namespace);
        let mut funcs = self.functions.lock().unwrap();
        if funcs.remove(&key).is_none() {
            return Err(DeleteError::NotFound(format!(
                "function {} not found",
                query.function_name
            )));
        }
        Ok(())
    }

    async fn resolve(&self, query: Query) -> Result<http::Uri, ResolveError> {
        let namespace = query.namespace.as_deref().unwrap_or("default");
        let key = Self::key(&query.function_name, namespace);
        let funcs = self.functions.lock().unwrap();
        if funcs.contains_key(&key) {
            Ok("http://127.0.0.1:8080".parse().unwrap())
        } else {
            Err(ResolveError::NotFound(format!(
                "function {} not found",
                query.function_name
            )))
        }
    }

    async fn list(&self, namespace: String) -> Result<Vec<Status>, ListError> {
        let funcs = self.functions.lock().unwrap();
        let result: Vec<Status> = funcs
            .values()
            .filter(|s| s.namespace == namespace)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn update(&self, deployment: Deployment) -> Result<(), UpdateError> {
        let key = Self::key(&deployment.function_name, &deployment.namespace);
        let mut funcs = self.functions.lock().unwrap();
        if !funcs.contains_key(&key) {
            return Err(UpdateError::NotFound(format!(
                "function {} not found",
                deployment.function_name
            )));
        }
        funcs.insert(
            key,
            Status {
                name: deployment.function_name.clone(),
                image: deployment.image.clone(),
                namespace: deployment.namespace.clone(),
                labels: deployment.labels.clone(),
                annotations: deployment.annotations.clone(),
                env_vars: deployment.env_vars.clone(),
                created_at: String::new(),
                available_replicas: 1,
                invocation_count: 0,
                status: "Ready".into(),
            },
        );
        Ok(())
    }

    async fn status(&self, query: Query) -> Result<Status, ResolveError> {
        let namespace = query.namespace.as_deref().unwrap_or("default");
        let key = Self::key(&query.function_name, namespace);
        let funcs = self.functions.lock().unwrap();
        funcs.get(&key).cloned().ok_or_else(|| {
            ResolveError::NotFound(format!("function {} not found", query.function_name))
        })
    }
}

fn make_app() -> (axum::Router, Arc<MockProvider>) {
    let provider = Arc::new(MockProvider::new());
    let app = gateway::app(provider.clone());
    (app, provider)
}

#[tokio::test]
async fn deploy_and_status() {
    let (app, _provider) = make_app();

    // Deploy a function
    let req = Request::post("/deploy")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "function_name": "test-fn",
                "image": "test/image:latest",
                "namespace": "test-ns"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Check status
    let req = Request::get("/status/test-fn?namespace=test-ns")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn deploy_conflict() {
    let (app, _provider) = make_app();

    let body = json!({
        "function_name": "dup-fn",
        "image": "test/image:latest",
        "namespace": "test-ns"
    })
    .to_string();

    // First deploy
    let req = Request::post("/deploy")
        .header("content-type", "application/json")
        .body(Body::from(body.clone()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Second deploy (conflict)
    let req = Request::post("/deploy")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn delete_not_found() {
    let (app, _provider) = make_app();

    let req = Request::post("/delete")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "function_name": "nonexistent",
                "namespace": "test-ns"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_success() {
    let (app, _provider) = make_app();

    // Deploy first
    let body = json!({
        "function_name": "del-fn",
        "image": "test/image:latest",
        "namespace": "test-ns"
    })
    .to_string();
    let req = Request::post("/deploy")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Delete
    let req = Request::post("/delete")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "function_name": "del-fn",
                "namespace": "test-ns"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_empty_namespace() {
    let (app, _provider) = make_app();

    let req = Request::get("/list?namespace=empty-ns")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_not_found() {
    let (app, _provider) = make_app();

    let req = Request::post("/update")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "function_name": "nonexistent",
                "image": "test/image:v2",
                "namespace": "test-ns"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn status_not_found() {
    let (app, _provider) = make_app();

    let req = Request::get("/status/nonexistent?namespace=test-ns")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn resolve_not_found() {
    let (app, _provider) = make_app();

    let req = Request::get("/resolve/nonexistent?namespace=test-ns")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn resolve_success() {
    let (app, _provider) = make_app();

    // Deploy first
    let req = Request::post("/deploy")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "function_name": "resolve-fn",
                "image": "test/image:latest",
                "namespace": "test-ns"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Resolve
    let req = Request::get("/resolve/resolve-fn?namespace=test-ns")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
