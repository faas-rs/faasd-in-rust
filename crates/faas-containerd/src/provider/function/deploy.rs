use crate::provider::ContainerdProvider;
use crate::state::CacheRecord;
use bollard::container::{
    CreateContainerOptions, InspectContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use futures_util::TryStreamExt;
use gateway::types::{DeployError, Deployment};

impl ContainerdProvider {
    pub async fn function_deploy(&self, config: Deployment) -> Result<(), DeployError> {
        let ns = &config.namespace;
        let container_name = format!("faasdrs-{}-{}", ns, config.function_name);

        // Reject if already deployed — update path clears cache first via delete
        if self
            .cache
            .get_ip(&container_name)
            .map_err(|e| DeployError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(DeployError::Conflict("function already exists".into()));
        }

        // CAS lock
        if !self
            .cache
            .try_acquire_deploy(&container_name)
            .map_err(|e| DeployError::Internal(e.to_string()))?
        {
            return Err(DeployError::Conflict("deploy or delete in progress".into()));
        }

        let result = self.do_deploy(&container_name, &config).await;

        match &result {
            Ok(ip) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                self.cache
                    .commit_deploy(
                        &container_name,
                        &CacheRecord {
                            ip: *ip,
                            created_at: now,
                        },
                    )
                    .ok();
            }
            Err(_) => {
                self.cache.release_deploy(&container_name).ok();
                let _ = self
                    .docker
                    .remove_container(
                        &container_name,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            }
        }
        result.map(|_| ())
    }

    async fn do_deploy(&self, name: &str, c: &Deployment) -> Result<std::net::IpAddr, DeployError> {
        let span = tracing::info_span!("deploy", container = name);
        let _guard = span.enter();

        // Pull image
        let mut stream = self.docker.create_image(
            Some(CreateImageOptions {
                from_image: c.image.clone(),
                ..Default::default()
            }),
            None,
            None,
        );
        while stream
            .try_next()
            .await
            .map_err(|e| DeployError::Internal(e.to_string()))?
            .is_some()
        {}
        tracing::debug!("image pulled");

        // Create host config with network
        let host_config = bollard::models::HostConfig {
            network_mode: Some(self.network.clone()),
            readonly_rootfs: Some(true),
            tmpfs: Some(std::collections::HashMap::from([(
                "/tmp".into(),
                "rw,noexec,nosuid,size=64m".into(),
            )])),
            cap_drop: Some(vec!["ALL".into()]),
            ..Default::default()
        };
        let env_vars: Vec<String> = c.env_vars.iter().map(|(k, v)| format!("{k}={v}")).collect();

        let config = bollard::container::Config {
            image: Some(c.image.clone()),
            env: Some(env_vars),
            host_config: Some(host_config),
            labels: Some(c.labels.clone()),
            ..Default::default()
        };

        // Remove existing container if any — deploy is "ensure running with this config"
        if self.container_exists(name).await {
            self.docker
                .remove_container(
                    name,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
                .map_err(|e| DeployError::Internal(e.to_string()))?;
        }

        self.docker
            .create_container(
                Some(CreateContainerOptions {
                    name,
                    ..Default::default()
                }),
                config,
            )
            .await
            .map_err(|e| DeployError::Internal(e.to_string()))?;
        tracing::debug!("container created");

        // Start
        self.docker
            .start_container(name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DeployError::Internal(e.to_string()))?;
        tracing::debug!("container started");

        // Get IP
        let info = self
            .docker
            .inspect_container(name, None::<InspectContainerOptions>)
            .await
            .map_err(|e| DeployError::Internal(e.to_string()))?;
        let ip = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.networks.as_ref())
            .and_then(|nets| nets.get(&self.network))
            .and_then(|ep| ep.ip_address.as_ref())
            .and_then(|ip_str| ip_str.parse().ok())
            .ok_or(DeployError::Internal("no IP assigned".into()))?;

        tracing::info!(ip = %ip, "deployed");
        Ok(ip)
    }

    async fn container_exists(&self, name: &str) -> bool {
        self.docker
            .inspect_container(name, None::<InspectContainerOptions>)
            .await
            .is_ok()
    }
}
