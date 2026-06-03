use crate::impls::cni;
use crate::impls::{backend, function::ContainerStaticMetadata, oci_image::ImageError};
use crate::provider::ContainerdProvider;
use crate::state::CacheRecord;
use gateway::types::{DeployError, Deployment};

impl ContainerdProvider {
    /// Idempotent deploy with CAS serialization.
    ///
    /// `try_acquire_deploy` → CAS None→InFlight, serialising against
    /// concurrent deploys and deletes.  On any error (including cancellation),
    /// the deploy lock is released and containerd resources are cleaned up
    /// before returning.
    pub async fn deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let cx = asupersync::Cx::current();
        let cx = cx.as_ref().ok_or(DeployError::Internal("no Cx".into()))?;

        let metadata = ContainerStaticMetadata::from(config);
        let endpoint = &metadata.endpoint;

        // ── Acquire deploy lock ────────────────────────────────────
        if !self
            .cache
            .try_acquire_deploy(endpoint)
            .map_err(|e| DeployError::Internal(e.to_string()))?
        {
            return Err(DeployError::Conflict(
                "function is being deployed or deleted".into(),
            ));
        }
        log::info!("Deploying function: {:?}", endpoint);

        // Run deploy with auto-cleanup on any failure or cancel.
        let result = self.do_deploy_impl(cx, &metadata).await;

        match &result {
            Ok(ip_addr) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let _ = self.cache.commit_deploy(
                    endpoint,
                    &CacheRecord {
                        ip: *ip_addr,
                        created_at: now,
                    },
                );
                log::info!("Function {} deployed at {}", endpoint, ip_addr);
            }
            Err(_) => {
                log::warn!("Deploy failed for {}, cleaning up", endpoint);
                self.cache.release_deploy(endpoint).ok();
                self.cleanup_containerd_resources(endpoint).await;
            }
        }

        result.map(|_| ())
    }

    async fn do_deploy_impl(
        &self,
        cx: &asupersync::Cx,
        metadata: &ContainerStaticMetadata,
    ) -> Result<std::net::IpAddr, DeployError> {
        let endpoint = &metadata.endpoint;

        // ── Step 1: Pull image ────────────────────────────────────
        cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:pulling");
        backend()
            .prepare_image(&metadata.image, &endpoint.namespace, false)
            .await
            .map_err(|e| match &e {
                ImageError::ImageNotFound(msg) => DeployError::Invalid(msg.clone()),
                _ => DeployError::Internal(e.to_string()),
            })?;
        log::trace!("Image '{}' ready", metadata.image);

        // ── Step 2: Create container ────────────────────────────────
        cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:creating");
        if !backend().container_exists(endpoint).await {
            backend().create_container(metadata).await.map_err(|e| {
                log::error!("Failed to create container: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            log::trace!("Container created");
        } else {
            log::trace!("Container already exists");
        }

        // ── Step 3: CNI network ─────────────────────────────────────
        cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:networking");
        let (ip, _netns) = cni::cni_impl::create_cni_network(cx, endpoint).map_err(|e| {
            log::error!("CNI failed: {}", e);
            DeployError::Internal(e.msg)
        })?;
        let ip_addr = ip.address();

        // ── Step 4: Snapshot ─────────────────────────────────────────
        cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:snapshoting");
        if !backend().snapshot_exists(endpoint).await {
            backend().prepare_snapshot(metadata).await.map_err(|e| {
                log::error!("Snapshot failed: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
        }

        // ── Step 5: Task ─────────────────────────────────────────────
        cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:starting");
        if !backend().task_exists(endpoint).await {
            let mounts = backend().prepare_snapshot(metadata).await.map_err(|e| {
                log::error!("Mounts failed: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            backend().new_task(mounts, endpoint).await.map_err(|e| {
                log::error!("Task failed: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
        }

        Ok(ip_addr)
    }
}

#[cfg(test)]
mod tests {
    use crate::impls::cni::Endpoint;

    #[test]
    fn test_endpoint_display_includes_prefix() {
        let endpoint = Endpoint::new("hello", "default");
        let s = endpoint.to_string();
        assert!(s.starts_with("faasdrs-"));
        assert_eq!(s, "faasdrs-default-hello");
    }
}
