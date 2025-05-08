use actix_web::{HttpRequest, web};

use awc::http::Uri;
use url::Url;
//根据URL和原始请求来构建转发请求，并对请求头进行处理
pub fn create_proxy_request(
    req: &HttpRequest,
    base_url: &Url,
    payload: web::Payload,
) -> awc::SendClientRequest {
    let proxy_client = awc::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .finish();
    let origin_url = base_url.join(req.uri().path()).unwrap();
    let remaining_segments = origin_url.path_segments().unwrap().skip(2);
    let rest_path = remaining_segments.collect::<Vec<_>>().join("/");
    let url = base_url.join(&rest_path).unwrap();

    let uri = url.as_str().parse::<Uri>().unwrap();

    let mut proxy_req = proxy_client.request(req.method().clone(), uri);

    for header in req.headers() {
        proxy_req = proxy_req.insert_header(header);
    }

    if req.headers().get("X-Forwarded-Host").is_none() {
        if let Some(host) = req.headers().get("Host") {
            proxy_req = proxy_req.insert_header(("X-Forwarded-Host", host));
        }
    }

    if req.headers().get("X-Forwarded-For").is_none() {
        if let Some(remote_addr) = req.peer_addr() {
            proxy_req = proxy_req.insert_header(("X-Forwarded-For", remote_addr.to_string()));
        }
    }

    proxy_req.send_stream(payload)
}
