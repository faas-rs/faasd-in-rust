use gateway::{
    handlers::function::{DeleteError, DeployError, UpdateError},
    types::function::{Deployment, Query},
};

use crate::provider::ContainerdProvider;

impl ContainerdProvider {
    pub(crate) async fn _update(&self, param: Deployment) -> Result<(), UpdateError> {
        let function = Query {
            service: param.service.clone(),
            namespace: param.namespace.clone(),
        };
        self._delete(function).await.map_err(|e| {
            log::error!("failed to delete function when update because {:?}", e);
            match e {
                DeleteError::NotFound(e) => UpdateError::NotFound(e.to_string()),
                DeleteError::Internal(e) => UpdateError::Internal(e.to_string()),
                _ => UpdateError::Internal(e.to_string()),
            }
        })?;
        self._deploy(param).await.map_err(|e| {
            log::error!("failed to deploy function when update because {:?}", e);
            match e {
                DeployError::Invalid(e) => UpdateError::Invalid(e.to_string()),
                DeployError::InternalError(e) => UpdateError::Internal(e.to_string()),
            }
        })?;

        Ok(())
    }
}
