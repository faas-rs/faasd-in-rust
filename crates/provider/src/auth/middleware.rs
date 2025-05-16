use crate::types::config::FaaSConfig;
use actix_web::body::BoxBody;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    error::ErrorInternalServerError,
    web,
};
use base64::prelude::*;
use futures_util::future::ready;
use futures_util::future::{FutureExt, LocalBoxFuture, Ready, ok};
use std::rc::Rc;

pub struct BasicAuth; //用于实现TransForm trait

impl<S> Transform<S, ServiceRequest> for BasicAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = BasicAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(BasicAuthMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct BasicAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S> Service<ServiceRequest> for BasicAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);

        // 从 app_data 获取 FaaSConfig
        let config = req.app_data::<web::Data<FaaSConfig>>();
        if config.is_none() {
            log::error!("FaaSConfig not found in app_data. Authentication cannot proceed.");

            return ready(Err(ErrorInternalServerError(
                "FaasConfig not found in app_data.",
            )))
            .boxed_local();
        }
        let config = config.unwrap().get_ref();
        let expected_username = config.basic_auth_username.clone();
        let expected_password = config.basic_auth_password.clone();

        let auth_header = req.headers().get("Authorization");

        if let Some(auth_val) = auth_header {
            if let Ok(auth_str) = auth_val.to_str() {
                if auth_str.starts_with("Basic ") {
                    let base64_credentials = &auth_str[6..];
                    if let Ok(decoded_bytes) = BASE64_STANDARD.decode(base64_credentials) {
                        if let Ok(credentials_str) = String::from_utf8(decoded_bytes) {
                            let mut parts = credentials_str.splitn(2, ':');
                            if let (Some(username), Some(password)) = (parts.next(), parts.next()) {
                                if username == expected_username && password == expected_password {
                                    log::info!("Authentication successful for user: {}", username);
                                    return Box::pin(async move { service.call(req).await });
                                }
                            }
                        }
                    }
                }
            }
        }

        log::warn!("Unauthorized access attempt to: {}", req.path());

        // 认证失败时
        let (http_req, _payload) = req.into_parts();
        let response = HttpResponse::Unauthorized()
            .insert_header(("WWW-Authenticate", "Basic realm=\"Restricted Area\""))
            .finish()
            .map_into_boxed_body(); // 统一用BoxBody
        ready(Ok(ServiceResponse::new(http_req, response))).boxed_local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::config::FaaSConfig;
    use actix_web::dev::ServiceFactory;
    use actix_web::{App, HttpResponse, test, web};
    fn get_app() -> App<
        impl ServiceFactory<
            ServiceRequest,
            Config = (),
            Response = ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        let config = FaaSConfig {
            basic_auth_password: "password".to_string(),
            basic_auth_username: "admin".to_string(),
            ..Default::default()
        };
        App::new()
            .app_data(web::Data::new(config))
            .wrap(BasicAuth)
            .route("/test", web::get().to(HttpResponse::Ok))
    }
    #[actix_web::test]
    async fn test_auth_success() {
        let app = test::init_service(get_app()).await;
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:password");
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Basic {}", credentials)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK); //200
    }
    #[actix_web::test]
    async fn test_auth_fail_wrong_password() {
        let app = test::init_service(get_app()).await;
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:wrong");
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Basic {}", credentials)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED); // 401
    }

    #[actix_web::test]
    async fn test_auth_fail_no_header() {
        let app = test::init_service(get_app()).await;
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED); // 401
    }
}
