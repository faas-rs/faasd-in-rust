use crate::impls::cni;
use crate::impls::{backend, function::ContainerStaticMetadata, oci_image::ImageError};
use crate::provider::ContainerdProvider;
use crate::state::{InstanceRecord, InstanceState, ResourceSet};
use gateway::types::{DeployError, Deployment};

impl ContainerdProvider {
    pub async fn deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let store = self.state_store.clone();
        let metadata = ContainerStaticMetadata::from(config);

        let endpoint = metadata.endpoint.clone();
        log::trace!("Deploying function: {:?}", metadata);

        // ── Step 0: Check for conflicts ──────────────────────────────
        let current = store.get(&endpoint).unwrap_or(None);
        if let Some(record) = &current {
            match &record.state {
                InstanceState::Absent => {}
                InstanceState::Error(_) => {
                    // Retry from Error: reuse the record but clear error
                    log::info!(
                        "Retrying deploy for {} from Error state",
                        endpoint
                    );
                }
                _ => {
                        return Err(DeployError::Internal(
                        "function already exists".to_string(),
                    ));
                }
                }
            }

        // ── Step 1: Pulling ──────────────────────────────────────────
        let record = current.clone().unwrap_or_else(|| InstanceRecord::new(endpoint.clone()));
        let pulling = record
            .clear_error()
            .with_state(InstanceState::Pulling)
            .with_resources(ResourceSet::IMAGE);
        store
            .cas(&endpoint, current.as_ref(), Some(&pulling))
            .map_err(|e| {
                DeployError::Internal(format!("CAS conflict: {}", e))
            })?;

        let image_result = backend()
            .prepare_image(&metadata.image,
                &metadata.endpoint.namespace,
                true,
            )
            .await;
        if let Err(e) = image_result {
            log::error!("Image '{}' fetch failed: {}", &metadata.image, e);
            let err = match &e {
                ImageError::ImageNotFound(msg) => {
                    DeployError::Invalid(msg.clone())
                }
                _ => DeployError::Internal(e.to_string()),
            };
            let err_record = pulling
                .with_state(InstanceState::Error(Box::new(
                    InstanceState::Pulling,
                )))
                .with_error(e.to_string());
            store.put(&err_record).ok();
            return Err(err);
        }
        log::trace!("Image '{}' fetch ok", &metadata.image);

        // ── Step 2: Creating ─────────────────────────────────────────
        let creating = pulling
            .with_state(InstanceState::Creating)
            .with_resources(ResourceSet::IMAGE | ResourceSet::CONTAINER);
        store.put(&creating).map_err(|e| {
            DeployError::Internal(e.to_string())
        })?;

        let ctr_result = backend().create_container(&metadata).await;
        if let Err(e) = ctr_result {
            log::error!("Failed to create container: {:?}", e);
            // Cleanup: container may have been partially created
            backend().delete_container(&endpoint).await.ok();
            let err_record = creating
                .with_state(InstanceState::Error(Box::new(
                    InstanceState::Creating,
                )))
                .with_error(e.to_string());
            store.put(&err_record).ok();
            return Err(DeployError::Internal(e.to_string()));
        }

        // ── Step 3: Networking ───────────────────────────────────────
        let networking = creating
            .with_state(InstanceState::Networking)
            .with_resources(
                ResourceSet::IMAGE | ResourceSet::CONTAINER,
            );
        store.put(&networking).map_err(|e| {
            DeployError::Internal(e.to_string())
        })?;

        let cx = asupersync::Cx::current();
        let (ip, netns) = match cni::cni_impl::create_cni_network(
            cx.as_ref().ok_or(DeployError::Internal("no Cx".into()))?,
            &metadata.endpoint,
        ) {
            Ok((ip, netns)) => (ip, netns),
            Err(e) => {
                log::error!("Failed to create CNI network: {}", e);
                // Cleanup: delete container
                backend().delete_container(&endpoint).await.ok();
                let err_record = networking
                    .with_state(InstanceState::Error(Box::new(
                        InstanceState::Networking,
                    )))
                    .with_error(e.msg.clone());
                store.put(&err_record).ok();
                return Err(DeployError::Internal(e.msg));
            }
        };
        let ip_addr = ip.address();
        log::trace!("CNI network created with IP: {:?}", ip_addr);

        // ── Step 4: Snapshoting ──────────────────────────────────────
        let snapshoting = networking
            .with_state(InstanceState::Snapshoting)
            .with_resources(
                ResourceSet::IMAGE
                    | ResourceSet::CONTAINER
                    | ResourceSet::NETNS,
            )
            .with_ip(ip_addr);
        store.put(&snapshoting).map_err(|e| {
            DeployError::Internal(e.to_string())
        })?;

        let mounts = match backend().prepare_snapshot(&metadata).await {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to prepare snapshot: {:?}", e);
                // Cleanup: CNI + container
                if let Some(cx) = asupersync::Cx::current() {
                    cni::cni_impl::delete_cni_network(&cx, &endpoint).ok();
                }
                netns.remove().ok();
                backend().delete_container(&endpoint).await.ok();
                let err_record = snapshoting
                    .with_state(InstanceState::Error(Box::new(
                        InstanceState::Snapshoting,
                    )))
                    .with_error(e.to_string());
                store.put(&err_record).ok();
                return Err(DeployError::Internal(e.to_string()));
            }
        };

        // ── Step 5: Starting ─────────────────────────────────────────
        let starting = snapshoting
            .with_state(InstanceState::Starting)
            .with_resources(
                ResourceSet::IMAGE
                    | ResourceSet::CONTAINER
                    | ResourceSet::NETNS
                    | ResourceSet::SNAPSHOT,
            );
        store.put(&starting).map_err(|e| {
            DeployError::Internal(e.to_string())
        })?;

        if let Err(e) = backend().new_task(mounts, &endpoint).await {
            log::error!("Failed to create task: {:?}", e);
            // Cleanup: snapshot + CNI + container
            backend().remove_snapshot(&endpoint).await.ok();
            if let Some(cx) = asupersync::Cx::current() {
                cni::cni_impl::delete_cni_network(&cx, &endpoint).ok();
            }
            netns.remove().ok();
            backend().delete_container(&endpoint).await.ok();
            let err_record = starting
                .with_state(InstanceState::Error(Box::new(
                    InstanceState::Starting,
                )))
                .with_error(e.to_string());
            store.put(&err_record).ok();
            return Err(DeployError::Internal(e.to_string()));
        }

        // ── Step 6: Active ───────────────────────────────────────────
        let active = starting
            .with_state(InstanceState::Active)
            .with_resources(ResourceSet::ALL);
        store.put(&active).map_err(|e| {
            DeployError::Internal(e.to_string())
        })?;

        log::info!(
            "container was created successfully: {}",
            metadata.endpoint
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::impls::cni::Endpoint;
    use crate::state::{InstanceRecord, InstanceState, ResourceSet};

    #[test]
    fn test_deploy_state_progression() {
        let endpoint = Endpoint::new("hello", "default");

        // Absent → Pulling
        let record = InstanceRecord::new(endpoint.clone());
        let pulling = record
            .with_state(InstanceState::Pulling)
            .with_resources(ResourceSet::IMAGE);
        assert_eq!(pulling.state, InstanceState::Pulling);
        assert_eq!(pulling.resources, ResourceSet::IMAGE);

        // Pulling → Creating
        let creating = pulling
            .with_state(InstanceState::Creating)
            .with_resources(ResourceSet::IMAGE | ResourceSet::CONTAINER);
        assert_eq!(creating.state, InstanceState::Creating);

        // Creating → Networking
        let networking = creating
            .with_state(InstanceState::Networking)
            .with_resources(
                ResourceSet::IMAGE | ResourceSet::CONTAINER,
            );
        assert_eq!(networking.state, InstanceState::Networking);

        // Networking → Snapshoting (with IP)
        let snapshoting = networking
            .with_state(InstanceState::Snapshoting)
            .with_resources(
                ResourceSet::IMAGE
                    | ResourceSet::CONTAINER
                    | ResourceSet::NETNS,
            )
            .with_ip(std::net::IpAddr::from([10, 66, 0, 5]));
        assert!(snapshoting.ip_address.is_some());

        // Snapshoting → Starting
        let starting = snapshoting
            .with_state(InstanceState::Starting)
            .with_resources(
                ResourceSet::IMAGE
                    | ResourceSet::CONTAINER
                    | ResourceSet::NETNS
                    | ResourceSet::SNAPSHOT,
            );
        assert_eq!(starting.state, InstanceState::Starting);

        // Starting → Active
        let active = starting
            .with_state(InstanceState::Active)
            .with_resources(ResourceSet::ALL);
        assert_eq!(active.state, InstanceState::Active);
        assert!(active.validate().is_ok());
    }
}
