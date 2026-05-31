use bollard::container::{RemoveContainerOptions, StopContainerOptions};
use gateway::types::{DeleteError, Query};
use crate::provider::ContainerdProvider;

impl ContainerdProvider {
    pub async fn function_delete(&self, q: Query) -> Result<(), DeleteError> {
        let ns = q.namespace.as_deref().unwrap_or("openfaas-fn");
        let name = format!("faasdrs-{}-{}", ns, q.function_name);
        let span = tracing::info_span!("delete", container = name);
        let _guard = span.enter();

        let _ = self.docker.stop_container(&name, None::<StopContainerOptions>).await;
        if let Err(e) = self.docker.remove_container(&name, Some(RemoveContainerOptions { force: true, ..Default::default() })).await {
            if !e.to_string().contains("No such container") && !e.to_string().contains("not found") {
                return Err(DeleteError::Internal(e.to_string()));
            }
        }
        self.cache.remove(&name).ok();
        tracing::info!("deleted");
        Ok(())
    }
}
