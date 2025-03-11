use actix_web::HttpResponse;

pub struct FunctionLister {
    service: std::sync::Arc<service::Service>,
}

impl super::IAmHandler for FunctionLister {
    type Input = String;
    // type Output = Vec<String>;

    async fn execute(&self, input: Self::Input) -> impl actix_web::Responder {
        // faasd进来的第一步是验证命名空间的标签是否具有某个值，也就是验证是否为true，确保命名空间有效
        // 但是这里省略，因为好像标签为空？

        let containers = self
            .service
            .get_container_list(input.as_str())
            .await
            .unwrap();

        for container in containers.iter() {}

        HttpResponse::Ok().json("函数列表")
    }
}
