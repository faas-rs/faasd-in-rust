use gateway::types::{DeleteError, DeployError, Deployment, Query, UpdateError};

use crate::provider::ContainerdProvider;

impl ContainerdProvider {
    pub async fn update(&self, param: Deployment) -> Result<(), UpdateError> {
        let function = Query {
            function_name: param.function_name.clone(),
            namespace: param.namespace.clone(),
        };
        self.delete(function).await.map_err(|e| {
            log::error!("failed to delete function when update because {:?}", e);
            match e {
                DeleteError::NotFound(e) => UpdateError::NotFound(e.to_string()),
                DeleteError::Internal(e) => UpdateError::Internal(e.to_string()),
                _ => UpdateError::Internal(e.to_string()),
            }
        })?;
        self.deploy(param).await.map_err(|e| {
            log::error!("failed to deploy function when update because {:?}", e);
            match e {
                DeployError::Invalid(e) => UpdateError::Invalid(e.to_string()),
                DeployError::Internal(e) => UpdateError::Internal(e.to_string()),
            }
        })?;

        Ok(())
    }
}
