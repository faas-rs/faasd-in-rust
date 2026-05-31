use crate::impls::cni;
use crate::impls::{backend, function::ContainerStaticMetadata, oci_image::ImageError};
use crate::provider::ContainerdProvider;
use crate::state::CacheRecord;
use gateway::types::{DeployError, Deployment};

impl ContainerdProvider {
    /// Idempotent deploy: each step queries containerd first, creates only
    /// if the resource does not already exist.  `cx.checkpoint()` between
    /// steps enables cancellation at any point without resource leaks.
    ///
    /// Naming convention `faasdrs-{namespace}-{function_name}` isolates
    /// faasd resources from other containerd workloads.
    pub async fn deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let cx = asupersync::Cx::current();
        let cx = cx.as_ref().ok_or(DeployError::Internal("no Cx".into()))?;

        let metadata = ContainerStaticMetadata::from(config);
        let endpoint = &metadata.endpoint;
        log::info!("Deploying function: {:?}", endpoint);

        // ── Step 1: Pull image ──────────────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:pulling");

        backend()
            .prepare_image(&metadata.image, &endpoint.namespace, false /* idempotent */)
            .await
            .map_err(|e| match &e {
                ImageError::ImageNotFound(msg) => DeployError::Invalid(msg.clone()),
                _ => DeployError::Internal(e.to_string()),
            })?;
        log::trace!("Image '{}' ready", &metadata.image);

        // ── Step 2: Create container ─────────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:creating");

        if !backend().container_exists(endpoint).await {
            backend().create_container(&metadata).await.map_err(|e| {
                log::error!("Failed to create container: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            log::trace!("Container created");
        } else {
            log::trace!("Container already exists, skipping create");
        }

        // ── Step 3: Create CNI network ───────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:networking");

        let (ip, _netns) = cni::cni_impl::create_cni_network(cx, endpoint).map_err(|e| {
            log::error!("Failed to create CNI network: {}", e);
            DeployError::Internal(e.msg)
        })?;
        let ip_addr = ip.address();
        log::trace!("CNI network created with IP: {:?}", ip_addr);

        // ── Step 4: Prepare snapshot ──────────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:snapshoting");

        if !backend().snapshot_exists(endpoint).await {
            backend().prepare_snapshot(&metadata).await.map_err(|e| {
                log::error!("Failed to prepare snapshot: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            log::trace!("Snapshot prepared");
        } else {
            log::trace!("Snapshot already exists, skipping prepare");
        }

        // ── Step 5: Create and start task ─────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:starting");

        if !backend().task_exists(endpoint).await {
            // Need mounts from snapshot for task creation
            let mounts = backend().prepare_snapshot(&metadata).await.map_err(|e| {
                log::error!("Failed to get mounts: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            backend().new_task(mounts, endpoint).await.map_err(|e| {
                log::error!("Failed to create task: {:?}", e);
                DeployError::Internal(e.to_string())
            })?;
            log::trace!("Task created and started");
        } else {
            log::trace!("Task already exists, skipping create");
        }

        // ── Step 6: Cache IP ──────────────────────────────────────────
        cx.checkpoint()
            .map_err(|_| DeployError::Cancelled)?;
        cx.trace("deploy:caching");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.cache
            .put(endpoint, &CacheRecord {
                ip: ip_addr,
                created_at: now,
            })
            .map_err(|e| DeployError::Internal(e.to_string()))?;

        log::info!(
            "Function {} deployed successfully at {}",
            endpoint,
            ip_addr
        );
        Ok(())
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
