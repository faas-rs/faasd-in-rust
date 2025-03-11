use super::IAmHandler;
use actix_web::{HttpResponse, Responder};
use service::Service;
use std::sync::Arc;

pub struct NamespaceLister {
    service: Arc<Service>,
}

impl IAmHandler for NamespaceLister {
    type Input = ();
    // type Output = Vec<String>;
    async fn execute(&self, _input: Self::Input) -> impl Responder {
        let ns_list = self.service.list_namespaces().await.unwrap();
        HttpResponse::Ok().json(ns_list)
    }
}
