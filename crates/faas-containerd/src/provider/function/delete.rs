use crate::provider::ContainerdProvider;
use gateway::handlers::function::DeleteError;
use gateway::types::function::Query;

impl ContainerdProvider {
    pub(crate) async fn _delete(&self, function: Query) -> Result<(), DeleteError> {
        let container = self
            .ctr_instance_map
            .lock()
            .await
            .remove(&function)
            .ok_or(DeleteError::NotFound("container not found".to_string()))?;

        // delete the container
        container.delete().await.map_err(|e| {
            log::error!("Failed to delete container: {:?}", e);
            DeleteError::Internal(e.to_string())
        })?;

        Ok(())
    }
}
