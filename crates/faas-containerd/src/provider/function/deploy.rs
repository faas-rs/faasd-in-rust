use crate::impls::cni::{self, Endpoint};
use crate::impls::{backend, function::ContainerStaticMetadata, oci_image::ImageError};
use crate::provider::ContainerdProvider;
use crate::state::{CacheRecord, CacheStore};
use gateway::types::{DeployError, Deployment};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use asupersync::Cx;
use asupersync::combinator::bracket::{BracketError, bracket};

/// Guard holding resources needed for deploy cleanup.
/// Cloned for bracket's release path; `committed` prevents
/// double-cleanup when deploy succeeds within the use phase.
struct DeployGuard {
    cache: CacheStore,
    endpoint: Endpoint,
    committed: Arc<AtomicBool>,
}

impl Clone for DeployGuard {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            endpoint: self.endpoint.clone(),
            committed: self.committed.clone(),
        }
    }
}

impl ContainerdProvider {
    /// Idempotent deploy with CAS serialization and cancel-safe cleanup.
    ///
    /// Uses asupersync's `bracket` combinator:
    ///   acquire: `try_acquire_deploy` (CAS None→InFlight)
    ///   use:     `do_deploy_impl` → on success, `commit_deploy` (CAS InFlight→Cached)
    ///   release: if not committed → `release_deploy` + `cleanup_containerd_resources`
    ///
    /// On cancel, panic, or failure the release phase always runs —
    /// InFlight can never leak beyond this function.
    pub async fn deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let cx = Cx::current();
        let cx = cx.as_ref().ok_or(DeployError::Internal("no Cx".into()))?;

        let metadata = ContainerStaticMetadata::from(config);
        let endpoint = metadata.endpoint.clone();

        let committed = Arc::new(AtomicBool::new(false));

        let guard = DeployGuard {
            cache: self.cache.clone(),
            endpoint: endpoint.clone(),
            committed: committed.clone(),
        };

        let result: Result<(), BracketError<DeployError>> = bracket(
            // ── acquire: CAS lock ────────────────────────────────
            {
                let g = guard.clone();
                async move {
                    if !g
                        .cache
                        .try_acquire_deploy(&g.endpoint)
                        .map_err(|e| DeployError::Internal(e.to_string()))?
                    {
                        return Err(DeployError::Conflict(
                            "function is being deployed or deleted".into(),
                        ));
                    }
                    log::info!("Deploying function: {:?}", g.endpoint);
                    Ok(g)
                }
            },
            // ── use: run deploy, commit on success ──────────────
            {
                let cf = committed.clone();
                move |g: DeployGuard| {
                    Box::pin(async move {
                        let ip = do_deploy_impl(cx, &metadata).await?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |d| d.as_millis() as u64);
                        if g.cache
                            .commit_deploy(
                                &g.endpoint,
                                &CacheRecord {
                                    ip,
                                    created_at: now,
                                },
                            )
                            .unwrap_or(false)
                        {
                            cf.store(true, Ordering::Release);
                            log::info!("Function {} deployed at {}", g.endpoint, ip);
                        }
                        Ok(())
                    })
                }
            },
            // ── release: cleanup if not committed ───────────────
            |g: DeployGuard| {
                Box::pin(async move {
                    if g.committed.load(Ordering::Acquire) {
                        return;
                    }
                    log::warn!("Deploy failed for {}, cleaning up", g.endpoint);
                    g.cache.release_deploy(&g.endpoint).ok();
                    crate::provider::function::delete::cleanup_containerd_resources(
                        &g.cache,
                        &g.endpoint,
                    )
                    .await;
                })
            },
        )
        .await;

        result.map_err(|e| match e {
            BracketError::Inner(e) => e,
            BracketError::PolledAfterCompletion => {
                DeployError::Internal("bracket polled after completion".into())
            }
        })
    }
}

/// Deploy implementation.  Idempotent ensure: each step checks
/// containerd for existing resources before creating.  Per-step
/// checkpoints enable cancel-safe interruption.
async fn do_deploy_impl(
    cx: &Cx,
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
