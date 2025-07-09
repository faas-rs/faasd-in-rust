// use actix_web::{
//     dev::{Service, ServiceRequest, ServiceResponse, Transform},
//     Error, HttpMessage,
// };
// use futures_util::future::{ok, Ready};
// use std::future::Future;
// use std::pin::Pin;
// use std::task::{Context, Poll};
// use crate::oauth::jwt_utils::validate_access_token;
// use crate::types::config::FaaSConfig;
// use crate::models::Error as AppError;
// pub struct BearerTokenMiddleware;

// impl<S, B> Transform<S, ServiceRequest> for BearerTokenMiddleware
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type Transform = BearerTokenMiddlewareService<S>;
//     type InitError = ();
//     type Future = Ready<Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         ok(BearerTokenMiddlewareService { service })
//     }
// }

// pub struct BearerTokenMiddlewareService<S> {
//     service: S,
// }

// impl<S, B> Service<ServiceRequest> for BearerTokenMiddlewareService<S>
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

//     fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.service.poll_ready(cx)
//     }

//     fn call(&self, req: ServiceRequest) -> Self::Future {
//         let mut extensions = req.extensions_mut();

//         // 从请求头中提取 Authorization
//         if let Some(auth_header) = req.headers().get("Authorization") {
//             if let Ok(auth_str) = auth_header.to_str() {
//                 if auth_str.starts_with("Bearer ") {
//                     let token = &auth_str["Bearer ".len()..];

//                     // 从应用状态中获取 FaaSConfig
//                     if let Some(config) = req.app_data::<FaaSConfig>() {
//                         match validate_access_token(token, &config.jwt_config) {
//                             Ok(claims) => {
//                                 // 将解析后的用户信息存储到 extensions 中
//                                 extensions.insert(claims);
//                             }
//                             Err(AppError::TokenExpired) => {
//                                 log::warn!("Token expired");
//                             }
//                             Err(e) => {
//                                 log::warn!("Invalid token: {:?}", e);
//                             }
//                         }
//                     }
//                 }
//             }
//         }

//         let fut = self.service.call(req);

//         Box::pin(async move {
//             let res = fut.await?;
//             Ok(res)
//         })
//     }
// }
