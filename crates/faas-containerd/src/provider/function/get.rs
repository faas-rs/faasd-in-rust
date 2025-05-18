use crate::provider::ContainerdProvider;

pub enum GetError {
    NotFound,
    InternalError,
}

impl ContainerdProvider {
    // pub async fn getfn(
    //     &self,
    //     query: function::Query,
    // ) -> Option<FunctionInstance> {
    //     let instance = self.ctr_instance_map
    //         .lock()
    //         .await
    //         .get(&query)
    //         .cloned();
    // }
}
