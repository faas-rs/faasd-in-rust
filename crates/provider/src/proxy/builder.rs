use actix_web::{Error, HttpRequest, web};
use futures::StreamExt;
use reqwest::{Client, RequestBuilder};
use url::Url;
//根据URL和原始请求来构建转发请求，并对请求头进行处理
pub async fn build_proxy_request(
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
