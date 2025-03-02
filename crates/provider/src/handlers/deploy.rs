use crate::types;

use super::Handler;

pub struct DeployHandler {
    pub config: types::function_deployment::FunctionDeployment,
}

impl Handler for DeployHandler {
    fn get_handler(&self, req: actix_web::HttpRequest) -> impl actix_web::Responder {}
}
