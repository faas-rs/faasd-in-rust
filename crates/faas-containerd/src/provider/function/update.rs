use gateway::types::{Deployment, UpdateError};
use crate::provider::ContainerdProvider;

impl ContainerdProvider {
    pub async fn function_update(&self, p: Deployment) -> Result<(), UpdateError> {
        let q = gateway::types::Query { function_name: p.function_name.clone(), namespace: Some(p.namespace.clone()) };
        self.function_delete(q).await.map_err(|e| match e {
            gateway::types::DeleteError::NotFound(_) => UpdateError::NotFound("not found".into()),
            gateway::types::DeleteError::Internal(s) => UpdateError::Internal(s),
        })?;
        self.function_deploy(p).await.map_err(|e| match e {
            gateway::types::DeployError::Invalid(s) => UpdateError::Invalid(s),
            gateway::types::DeployError::Conflict(s) => UpdateError::Internal(s),
            gateway::types::DeployError::Cancelled => UpdateError::Internal("cancelled".into()),
            gateway::types::DeployError::Internal(s) => UpdateError::Internal(s),
        })?;
        Ok(())
    }
}
