pub mod provider;
pub mod types;

pub use provider::Provider;
pub use types::*;

use std::sync::Arc;

use asupersync::http::h1::listener::Http1Listener;
use asupersync::http::h1::client::Http1Client;
use asupersync::http::h1::types::{Method, Request, Response, StatusCode};
use asupersync::runtime::RuntimeHandle;
use asupersync::net::TcpStream;
use serde::Deserialize;

/// Start the gateway HTTP server dispatching to the given provider.
pub async fn serve<P: Provider>(
    provider: Arc<P>,
    port: u16,
    handle: &RuntimeHandle,
) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    log::info!("Starting gateway on {addr}");

    let listener = Http1Listener::bind(addr, move |req| {
        let p = provider.clone();
        async move { dispatch(p, req).await }
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?;

    listener
        .run(handle)
        .await
        .map(|_| ())
        .map_err(|e| std::io::Error::other(e.to_string()))
}

// ── Route dispatch ──────────────────────────────────────────────────────

async fn dispatch<P: Provider>(provider: Arc<P>, req: Request) -> Response {
    let method = &req.method;
    let uri = &req.uri;
    let path = uri.split('?').next().unwrap_or(uri);

    match (method, path) {
        (Method::Post, "/system/functions") => handle_deploy(provider, &req.body).await,
        (Method::Put, "/system/functions") => handle_update(provider, &req.body).await,
        (Method::Delete, "/system/functions") => handle_delete(provider, &req.body).await,
        (Method::Get, "/system/functions") => handle_list(provider, uri).await,
        (Method::Get, p) if p.starts_with("/system/function/") => {
            let name = p.strip_prefix("/system/function/").unwrap_or("");
            handle_status(provider, name, uri).await
        }
        (Method::Get, "/system/namespaces") => handle_namespace_list(provider).await,
        (Method::Get, p) if p.starts_with("/system/namespace/") => {
            let ns = p.strip_prefix("/system/namespace/").unwrap_or("");
            handle_get_namespace(provider, ns).await
        }
        (Method::Post, p) if p.starts_with("/system/namespace/") => {
            let ns = p.strip_prefix("/system/namespace/").unwrap_or("");
            handle_create_namespace(provider, ns, &req.body).await
        }
        (Method::Put, p) if p.starts_with("/system/namespace/") => {
            let ns = p.strip_prefix("/system/namespace/").unwrap_or("");
            handle_update_namespace(provider, ns, &req.body).await
        }
        (Method::Delete, p) if p.starts_with("/system/namespace/") => {
            let ns = p.strip_prefix("/system/namespace/").unwrap_or("");
            handle_delete_namespace(provider, ns).await
        }
        // ── Function invocation proxy (ANY method) ──────────────────────
        (_, p) if p.starts_with("/function/") => {
            let trimmed = p.strip_prefix("/function/").unwrap_or("");
            let (name, subpath) = match trimmed.find('/') {
                Some(i) => (&trimmed[..i], &trimmed[i..]),
                None => (trimmed, "/"),
            };
            handle_proxy(provider, name, subpath, &req).await
        }
        (Method::Get, "/health") => ok_json(serde_json::json!({"status": "ok"})),
        _ => json_response(StatusCode(404), &serde_json::json!({"error": "not found"})),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn ok_json(data: serde_json::Value) -> Response {
    let body = serde_json::to_vec(&data).unwrap_or_default();
    Response::new(200, "OK", body).with_header("content-type", "application/json")
}

fn json_response(status: StatusCode, data: &serde_json::Value) -> Response {
    let body = serde_json::to_vec(data).unwrap_or_default();
    Response::new(status.0, "OK", body).with_header("content-type", "application/json")
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    json_response(status, &serde_json::json!({"error": msg}))
}

fn parse_body<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Response> {
    serde_json::from_slice(body)
        .map_err(|e| error_response(StatusCode(400), &format!("invalid JSON: {e}")))
}

#[derive(Deserialize)]
struct NamespaceBody {
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn handle_deploy<P: Provider>(p: Arc<P>, body: &[u8]) -> Response {
    let d = match parse_body::<Deployment>(body) {
        Ok(d) => d,
        Err(e) => return e,
    };

    match p.deploy(d).await {
        Ok(()) => json_response(StatusCode(202), &serde_json::json!({"status": "accepted"})),
        Err(DeployError::Conflict(msg)) => error_response(StatusCode(409), &msg),
        Err(DeployError::Cancelled) => error_response(StatusCode(500), "deploy cancelled"),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_update<P: Provider>(p: Arc<P>, body: &[u8]) -> Response {
    let d = match parse_body::<Deployment>(body) {
        Ok(d) => d,
        Err(e) => return e,
    };
    match p.update(d).await {
        Ok(()) => json_response(StatusCode(202), &serde_json::json!({"status": "accepted"})),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_delete<P: Provider>(p: Arc<P>, body: &[u8]) -> Response {
    let d: Delete = match parse_body(body) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let q = Query {
        function_name: d.function_name,
        namespace: None,
        cache_miss: false,
    };
    match p.delete(q).await {
        Ok(()) => json_response(StatusCode(202), &serde_json::json!({"status": "accepted"})),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_list<P: Provider>(p: Arc<P>, uri: &str) -> Response {
    let ns = parse_query_param(uri, "namespace").unwrap_or_default();
    match p.list(ns).await {
        Ok(fns) => ok_json(serde_json::to_value(fns).unwrap_or_default()),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_status<P: Provider>(p: Arc<P>, name: &str, uri: &str) -> Response {
    let ns = parse_query_param(uri, "namespace");
    let q = Query {
        function_name: name.to_string(),
        namespace: ns,
        cache_miss: false,
    };
    match p.status(q).await {
        Ok(s) => ok_json(serde_json::to_value(s).unwrap_or_default()),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_namespace_list<P: Provider>(p: Arc<P>) -> Response {
    match p.namespace_list().await {
        Ok(ns) => ok_json(serde_json::to_value(ns).unwrap_or_default()),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_get_namespace<P: Provider>(p: Arc<P>, ns: &str) -> Response {
    match p.get_namespace(ns.to_string()).await {
        Ok(n) => ok_json(serde_json::to_value(n).unwrap_or_default()),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_create_namespace<P: Provider>(p: Arc<P>, ns: &str, body: &[u8]) -> Response {
    let nb: NamespaceBody = match parse_body(body) {
        Ok(n) => n,
        Err(e) => return e,
    };
    match p.create_namespace(ns.to_string(), nb.labels).await {
        Ok(()) => json_response(StatusCode(201), &serde_json::json!({"status": "created"})),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_update_namespace<P: Provider>(p: Arc<P>, ns: &str, body: &[u8]) -> Response {
    let nb: NamespaceBody = match parse_body(body) {
        Ok(n) => n,
        Err(e) => return e,
    };
    match p.update_namespace(ns.to_string(), nb.labels).await {
        Ok(()) => json_response(StatusCode(202), &serde_json::json!({"status": "accepted"})),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

async fn handle_delete_namespace<P: Provider>(p: Arc<P>, ns: &str) -> Response {
    match p.delete_namespace(ns.to_string()).await {
        Ok(()) => json_response(StatusCode(202), &serde_json::json!({"status": "accepted"})),
        Err(e) => error_response(StatusCode(500), &e.to_string()),
    }
}

fn parse_query_param(uri: &str, key: &str) -> Option<String> {
    let query = uri.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? == key {
            return parts.next().map(url_decode);
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                16,
            )
        {
            out.push(hex as char);
            i += 3;
            continue;
        }
        out.push(if bytes[i] == b'+' {
            ' '
        } else {
            bytes[i] as char
        });
        i += 1;
    }
    out
}

// ── Function invocation proxy ───────────────────────────────────────────

/// Proxy an incoming function invocation to the resolved container.
///
/// Two-level resolve:
/// 1. Fast path (`cache_miss=false`): sled cache → proxy request
/// 2. Retry  (`cache_miss=true`):  netns ground truth → proxy request
async fn handle_proxy<P: Provider>(
    p: Arc<P>,
    function_name: &str,
    subpath: &str,
    req: &Request,
) -> Response {
    let fast = Query {
        function_name: function_name.to_string(),
        namespace: None,
        cache_miss: false,
    };

    if let Ok(uri) = p.resolve(fast).await
        && let Ok(r) = proxy_request(uri, subpath, req).await
    {
        return r;
    }

    // Retry: ground truth via netns
    let ground_truth = Query {
        function_name: function_name.to_string(),
        namespace: None,
        cache_miss: true,
    };

    match p.resolve(ground_truth).await {
        Ok(uri) => match proxy_request(uri, subpath, req).await {
            Ok(r) => r,
            Err(status) => error_response(status, "proxy connection failed after retry"),
        },
        Err(e) => error_response(StatusCode(502), &format!("resolve failed: {e}")),
    }
}

/// Forward a request to the container at `uri` + `subpath`.
///
/// Returns `Ok(Response)` on success, or `Err(StatusCode)` on connection failure.
async fn proxy_request(
    uri: http::Uri,
    subpath: &str,
    req: &Request,
) -> Result<Response, StatusCode> {
    let host = uri.authority()
        .filter(|a| !a.host().is_empty())
        .ok_or(StatusCode(502))?;
    let port = host.port_u16().unwrap_or(8080);
    let addr = format!("{}:{}", host.host(), port);

    let stream = TcpStream::connect(addr)
        .await
        .map_err(|_| StatusCode(502))?;

    // Build proxy request with forwarded method, path, headers, and body
    let proxy_req = Request::builder(req.method.clone(), subpath)
        .headers(
            req.headers
                .iter()
                .filter(|(name, _)| {
                    // Drop hop-by-hop headers
                    let lower = name.to_ascii_lowercase();
                    lower != "connection"
                        && lower != "keep-alive"
                        && lower != "transfer-encoding"
                        && lower != "te"
                        && lower != "host"
                })
                .cloned(),
        )
        .body(req.body.clone())
        .build();

    let resp = Http1Client::request(stream, proxy_req)
        .await
        .map_err(|_| StatusCode(502))?;

    Ok(Response {
        version: resp.version,
        status: resp.status,
        reason: resp.reason,
        headers: resp.headers,
        body: resp.body,
        trailers: resp.trailers,
    })
}
