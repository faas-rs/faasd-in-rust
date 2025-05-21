use crate::impls::cni;
use crate::impls::{self, backend, function::ContainerStaticMetadata};
use crate::provider::ContainerdProvider;
use gateway::handlers::function::DeployError;
use gateway::types::function::Deployment;
use scopeguard::{ScopeGuard, guard};

impl ContainerdProvider {
    pub(crate) async fn _deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let metadata = ContainerStaticMetadata::from(config);
        log::trace!("Deploying function: {:?}", metadata);

        // not going to check the conflict of namespace, should be handled by containerd backend
        backend()
            .prepare_image(&metadata.image, &metadata.endpoint.namespace, true)
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

        let mounts = backend().prepare_snapshot(&metadata).await.map_err(|e| {
            log::error!("Failed to prepare snapshot: {:?}", e);
            DeployError::InternalError(e.to_string())
        })?;

        let snapshot_defer = scopeguard::guard((), |()| {
            log::trace!("Cleaning up snapshot");
            let endpoint = metadata.endpoint.clone();
            tokio::spawn(async move { backend().remove_snapshot(&endpoint).await });
        });

        // let network = CNIEndpoint::new(&metadata.container_id, &metadata.namespace)?;
        let (ip, netns) = cni::cni_impl::create_cni_network(&metadata.endpoint).map_err(|e| {
            log::error!("Failed to create CNI network: {}", e);
            DeployError::InternalError(e.to_string())
        })?;

        let netns_defer = guard(netns, |ns| ns.remove().unwrap());

        let _ = backend().create_container(&metadata).await.map_err(|e| {
            log::error!("Failed to create container: {:?}", e);
            DeployError::InternalError(e.to_string())
        })?;

        let container_defer = scopeguard::guard((), |()| {
            let endpoint = metadata.endpoint.clone();
            tokio::spawn(async move { backend().delete_container(&endpoint).await });
        });

        // TODO: Use ostree-ext
        // let img_conf = BACKEND.get().unwrap().get_runtime_config(&metadata.image).unwrap();

        backend().new_task(mounts, &metadata.endpoint).await?;

        let task_defer = scopeguard::guard((), |()| {
            let endpoint = metadata.endpoint.clone();
            tokio::spawn(async move { backend().kill_task_with_timeout(&endpoint).await });
        });

        use std::net::IpAddr::*;

        match ip.address() {
            V4(addr) => {
                if let Err(err) = self
                    .database
                    .insert(metadata.endpoint.to_string(), &addr.octets())
                {
                    log::error!("Failed to insert into database: {:?}", err);
                    return Err(DeployError::InternalError(err.to_string()));
                }
            }
            V6(addr) => {
                if let Err(err) = self
                    .database
                    .insert(metadata.endpoint.to_string(), &addr.octets())
                {
                    log::error!("Failed to insert into database: {:?}", err);
                    return Err(DeployError::InternalError(err.to_string()));
                }
            }
        }

        log::info!("container was created successfully: {}", metadata.endpoint);
        ScopeGuard::into_inner(snapshot_defer);
        ScopeGuard::into_inner(netns_defer);
        ScopeGuard::into_inner(container_defer);
        ScopeGuard::into_inner(task_defer);
        Ok(())
    }
}
