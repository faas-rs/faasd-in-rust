use crate::handlers::invoke_resolver::InvokeResolver;
use crate::proxy::builder::build_proxy_request;
use crate::proxy::client::new_proxy_client_from_config;
use crate::types::config::FaaSConfig;
use actix_web::{Error, HttpRequest, HttpResponse, Responder, http::Method, web};

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
            Err(e) => HttpResponse::from_error(e),
        },
        _ => HttpResponse::MethodNotAllowed().body("method not allowed"),
    }
}

//根据原始请求，解析url，构建转发请求并转发，获取响应
async fn proxy_request(
    req: &HttpRequest,
    payload: web::Payload,
    proxy_client: &reqwest::Client,
) -> Result<HttpResponse, Error> {
    let function_name = req.match_info().get("name").unwrap_or("");
    if function_name.is_empty() {
        return Ok(HttpResponse::BadRequest().body("provide function name in path"));
    }

    let function_addr = match InvokeResolver::resolve_function_url(function_name).await {
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
