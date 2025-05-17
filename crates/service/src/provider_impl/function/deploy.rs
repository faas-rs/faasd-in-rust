use crate::impls::{self, BACKEND, function::ContainerStaticMetadata};
use crate::provider_impl::CtrdProvider;
use provider::handlers::function::DeployError;

impl CtrdProvider {
    pub(crate) async fn _deploy(
        &self,
        config: provider::types::function::Deployment,
    ) -> Result<(), DeployError> {
        let metadata = ContainerStaticMetadata::from(config);

        // not going to check the conflict of namespace, should be handled by containerd backend
        BACKEND
            .prepare_image(&metadata.image, &metadata.namespace, true)
            .await
            .map_err(|img_err| {
                use impls::oci_image::ImageError;
                log::error!("Image '{}' fetch failed: {}", &metadata.image, img_err);
                match img_err {
                    ImageError::ImageNotFound(_) => DeployError::Invalid,
                    _ => DeployError::InternalError,
                }
            })?;
        log::trace!("Image '{}' fetch ok", &metadata.image);

        let container = impls::function::FunctionInstance::new(metadata.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to create container: {:?}", e);
                DeployError::InternalError
            })?;

        let old = self
            .ctr_instance_map
            .lock()
            .unwrap()
            .insert(metadata.into(), container);

        if old.is_some() {
            log::warn!("Container {:?} already exists but not failed", old);
        }

        Ok(())
    }
}
