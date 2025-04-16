use futures::StreamExt;
use std::time::Duration;

use actix_web::{Error, HttpRequest, HttpResponse, Responder, http::Method, web};
use reqwest::{Client, RequestBuilder, redirect};
use url::Url;

use crate::{handlers::invoke_resolver::InvokeResolver, types::config::FaaSConfig};

pub async fn proxy_handler(
    config: web::Data<FaaSConfig>,
    req: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let proxy_client = new_proxy_client_from_config(config.as_ref()).await;
    log::info!("proxy_client : {:?}", proxy_client);

    match *req.method() {
        Method::POST
        | Method::PUT
        | Method::DELETE
        | Method::GET
        | Method::PATCH
        | Method::HEAD
        | Method::OPTIONS => match proxy_request(&req, payload, &proxy_client).await {
            Ok(resp) => resp,
            Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        },
        _ => HttpResponse::MethodNotAllowed().body("method not allowed"),
    }
}
//构建client
async fn new_proxy_client_from_config(config: &FaaSConfig) -> Client {
    new_proxy_client(
        config.get_read_timeout(),
        /*config.get_max_idle_conns(),*/ config.get_max_idle_conns_per_host(),
    )
    .await
}

//根据FaasConfig参数来设置Client
async fn new_proxy_client(
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

//根据原始请求，解析url，构建转发请求并转发，获取响应
async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
    proxy_client: &Client,
) -> Result<HttpResponse, Error> {
    let function_name = req.match_info().get("name").unwrap_or("");
    if function_name.is_empty() {
        return Ok(HttpResponse::BadRequest().body("provide function name in path"));
    }

    let function_addr = match InvokeResolver::resolve(function_name).await {
        Ok(function_addr) => function_addr,
        Err(e) => return Ok(HttpResponse::BadRequest().body(e.to_string())),
    };

    let proxy_req = build_proxy_request(req, &function_addr, proxy_client, payload).await?;

    match proxy_req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let mut client_resp = HttpResponse::build(status);

            for (name, value) in resp.headers().iter() {
                client_resp.insert_header((name.clone(), value.clone()));
            }

            let body = resp.bytes().await.unwrap();

            Ok(client_resp.body(body))
        }
        Err(e) => Ok(HttpResponse::BadGateway().body(e.to_string())),
    }
}

//根据URL和原始请求来构建转发请求，并对请求头进行处理
async fn build_proxy_request(
    req: &HttpRequest,
    base_url: &Url,
    proxy_client: &Client,
    mut payload: web::Payload,
) -> Result<RequestBuilder, Error> {
    let origin_url = base_url.join(req.uri().path()).unwrap();
    let remaining_segments = origin_url.path_segments().unwrap().skip(2);
    let rest_path = remaining_segments.collect::<Vec<_>>().join("/");
    let url = base_url.join(&rest_path).unwrap();
    let mut proxy_req = proxy_client
        .request(req.method().clone(), url)
        .headers(req.headers().clone().into());

    if req.headers().get("X-Forwarded-Host").is_none() {
        if let Some(host) = req.headers().get("Host") {
            proxy_req = proxy_req.header("X-Forwarded-Host", host);
        }
    }

    if req.headers().get("X-Forwarded-For").is_none() {
        if let Some(remote_addr) = req.peer_addr() {
            proxy_req = proxy_req.header("X-Forwarded-For", remote_addr.to_string());
        }
    }

    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }
    let body_bytes = body.freeze();
    let proxy_req = proxy_req.body(body_bytes);

    Ok(proxy_req)
}
