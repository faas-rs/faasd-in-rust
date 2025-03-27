use super::IAmHandler;
use crate::types::{self, CreateContainerInfo};
use actix_web::HttpResponse;
use service::Service;
use std::sync::Arc;

pub struct DeployHandler {
    pub config: types::function_deployment::FunctionDeployment,
    service: Arc<Service>,
}

impl IAmHandler for DeployHandler {
    type Input = CreateContainerInfo;
    // type Output = String;

    async fn execute(&self, input: Self::Input) -> impl actix_web::Responder {
        let cid = input.container_id.clone();
        let image = input.image.clone();
        let ns = input.ns.clone();
        self.service
            .create_container(&image, &cid, &ns)
            .await
            .unwrap();
        HttpResponse::Ok().json(format!("Container {} created successfully!", cid))
    }
}
