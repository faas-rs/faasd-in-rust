use crate::provider_impl::CtrdProvider;
use provider::handlers::function::DeleteError;
use provider::types::function::Query;

impl CtrdProvider {
    pub(crate) async fn _delete(&self, function: Query) -> Result<(), DeleteError> {
        let container = self
            .ctr_instance_map
            .lock()
            .unwrap()
            .remove(&function)
            .ok_or(DeleteError::NotFound)?;

        // delete the container
        container.delete().await.map_err(|e| {
            log::error!("Failed to delete container: {:?}", e);
            DeleteError::Internal
        })?;

        Ok(())
    }
}
