use crate::impls::{self, backend, function::ContainerStaticMetadata};
use crate::provider::ContainerdProvider;
use gateway::handlers::function::DeployError;
use gateway::types::function::Deployment;

impl ContainerdProvider {
    pub(crate) async fn _deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let metadata = ContainerStaticMetadata::from(config);

        // not going to check the conflict of namespace, should be handled by containerd backend
        backend()
            .prepare_image(&metadata.image, &metadata.namespace, true)
            .await
            .map_err(|img_err| {
                use impls::oci_image::ImageError;
                log::error!("Image '{}' fetch failed: {}", &metadata.image, img_err);
                match img_err {
                    ImageError::ImageNotFound(e) => DeployError::Invalid(e.to_string()),
                    _ => DeployError::InternalError(img_err.to_string()),
                }
            })?;
        log::trace!("Image '{}' fetch ok", &metadata.image);

        let container = impls::function::FunctionInstance::new(metadata.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to create container: {:?}", e);
                DeployError::InternalError(e.to_string())
            })?;
        log::info!(
            "container was created successfully: {}",
            metadata.container_id.clone()
        );

        let old = self
            .ctr_instance_map
            .lock()
            .await
            .insert(metadata.into(), container);

        if old.is_some() {
            log::warn!("Container {:?} already exists but not failed", old);
        }

        Ok(())
    }
}
