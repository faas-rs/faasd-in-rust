use crate::impls::cni::{self, Endpoint};
use crate::impls::{backend, function::ContainerStaticMetadata, oci_image::ImageError};
use crate::provider::ContainerdProvider;
use crate::state::{CacheStore, DeployMeta};
use gateway::types::{DeployError, Deployment};
use netns_rs::NetNs;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use asupersync::Cx;
use asupersync::combinator::bracket::{BracketError, bracket};

/// Guard holding the netns lock and resources needed for cleanup.
/// Cloned for bracket's release path; `committed` prevents
/// double-cleanup when deploy succeeds.
struct DeployGuard {
    cache: CacheStore,
    endpoint: Endpoint,
    image: String,
    committed: Arc<AtomicBool>,
}

impl Clone for DeployGuard {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            endpoint: self.endpoint.clone(),
            image: self.image.clone(),
            committed: self.committed.clone(),
        }
    }
}

impl ContainerdProvider {
    /// Idempotent deploy with netns-based mutual exclusion and cancel-safe cleanup.
    ///
    /// Uses asupersync's `bracket` combinator:
    ///   acquire: `NetNs::new(endpoint)` — kernel-level lock
    ///   use:     pull → container → CNI(setup in existing netns) → snapshot → task
    ///   commit:  read IP from netns → sled.insert(DeployMeta)
    ///   release: if not committed → `cleanup_containerd_resources` + `ns.remove()`
    ///
    /// On cancel, panic, or failure the release phase always runs —
    /// the netns lock can never leak beyond this function.
    pub async fn deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let cx = Cx::current();
        let cx = cx.as_ref().ok_or(DeployError::Internal("no Cx".into()))?;

        let metadata = ContainerStaticMetadata::from(config);
        let endpoint = metadata.endpoint.clone();
        let image = metadata.image.clone();

        let committed = Arc::new(AtomicBool::new(false));

        let guard = DeployGuard {
            cache: self.cache.clone(),
            endpoint: endpoint.clone(),
            image: image.clone(),
            committed: committed.clone(),
        };

        let result: Result<(), BracketError<DeployError>> = bracket(
            // ── acquire: netns as lock ──────────────────────────
            {
                let g = guard.clone();
                async move {
                    // Fast-path: if netns already exists → conflict
                    if NetNs::get(g.endpoint.to_string()).is_ok() {
                        return Err(DeployError::Conflict(
                            "function is being deployed or deleted".into(),
                        ));
                    }
                    // Kernel-atomic: NetNs::new creates netns; EEXIST if raced
                    NetNs::new(g.endpoint.to_string()).map_err(|e| {
                        DeployError::Internal(format!("Failed to acquire netns lock: {e}"))
                    })?;
                    log::info!("Deploying function: {:?}", g.endpoint);
                    Ok(g)
                }
            },
            // ── use: run deploy, commit on success ──────────────
            {
                let cf = committed.clone();
                move |g: DeployGuard| {
                    Box::pin(async move {
                        let ip = do_deploy_impl(cx, &g.endpoint, &g.image).await?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |d| d.as_millis() as u64);
                        let meta = DeployMeta {
                            ip,
                            image: g.image.clone(),
                            created_at: now,
                            labels: HashMap::new(),
                            dirty: None,
                        };
                        g.cache.insert(&g.endpoint, &meta).map_err(|e| {
                            DeployError::Internal(format!("Cache write failed: {e}"))
                        })?;
                        cf.store(true, Ordering::Release);
                        log::info!("Function {} deployed at {}", g.endpoint, ip);
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
                    crate::provider::function::delete::cleanup_containerd_resources(
                        &g.cache,
                        &g.endpoint,
                    )
                    .await;
                    // Release netns lock
                    if let Ok(ns) = NetNs::get(g.endpoint.to_string()) {
                        ns.remove().ok();
                    }
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
/// The netns already exists (created by bracket acquire phase).
async fn do_deploy_impl(
    cx: &Cx,
    endpoint: &Endpoint,
    image: &str,
) -> Result<std::net::IpAddr, DeployError> {
    // ── Step 1: Pull image ────────────────────────────────────
    cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
    cx.trace("deploy:pulling");
    backend()
        .prepare_image(image, &endpoint.namespace, false)
        .await
        .map_err(|e| match &e {
            ImageError::ImageNotFound(msg) => DeployError::Invalid(msg.clone()),
            _ => DeployError::Internal(e.to_string()),
        })?;
    log::trace!("Image '{}' ready", image);

    // ── Step 2: Create container ────────────────────────────────
    cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
    cx.trace("deploy:creating");
    let metadata = ContainerStaticMetadata {
        image: image.to_string(),
        endpoint: endpoint.clone(),
    };
    if !backend().container_exists(endpoint).await {
        backend().create_container(&metadata).await.map_err(|e| {
            log::error!("Failed to create container: {:?}", e);
            DeployError::Internal(e.to_string())
        })?;
        log::trace!("Container created");
    } else {
        log::trace!("Container already exists");
    }

    // ── Step 3: CNI network (netns already exists from lock) ──
    cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
    cx.trace("deploy:networking");
    let ip = cni::cni_impl::setup_cni_network(endpoint).map_err(|e| {
        log::error!("CNI failed: {}", e);
        DeployError::Internal(e.msg)
    })?;

    // ── Step 4: Snapshot ─────────────────────────────────────────
    cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
    cx.trace("deploy:snapshoting");
    if !backend().snapshot_exists(endpoint).await {
        backend().prepare_snapshot(&metadata).await.map_err(|e| {
            log::error!("Snapshot failed: {:?}", e);
            DeployError::Internal(e.to_string())
        })?;
    }

    // ── Step 5: Task ─────────────────────────────────────────────
    cx.checkpoint().map_err(|_| DeployError::Cancelled)?;
    cx.trace("deploy:starting");
    if !backend().task_exists(endpoint).await {
        let mounts = backend().prepare_snapshot(&metadata).await.map_err(|e| {
            log::error!("Mounts failed: {:?}", e);
            DeployError::Internal(e.to_string())
        })?;
        backend().new_task(mounts, endpoint).await.map_err(|e| {
            log::error!("Task failed: {:?}", e);
            DeployError::Internal(e.to_string())
        })?;
    }

    Ok(ip)
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
