use containerd_client::{
    services::v1::snapshots::{MountsRequest, PrepareSnapshotRequest, RemoveSnapshotRequest},
    types::Mount,
    with_namespace,
};
use tonic::Request;

use crate::impls::error::ContainerdError;

use super::{ContainerdService, cni::Endpoint, function::ContainerStaticMetadata};

impl ContainerdService {
    #[allow(unused)]
    pub(super) async fn get_mounts(
        &self,
        cid: &str,
        ns: &str,
    ) -> Result<Vec<Mount>, ContainerdError> {
        let mut sc = self.client.snapshots();
        let req = MountsRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: cid.to_string(),
        };
        let mounts = sc
            .mounts(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to get mounts: {}", e);
                ContainerdError::CreateTaskError(e.to_string())
            })?
            .into_inner()
            .mounts;

        Ok(mounts)
    }

    pub async fn prepare_snapshot(
        &self,
        container: &ContainerStaticMetadata,
    ) -> Result<Vec<Mount>, ContainerdError> {
        let parent_snapshot = self
            .get_parent_snapshot(&container.image, &container.endpoint.namespace)
            .await?;
        self.do_prepare_snapshot(
            &container.endpoint.service,
            &container.endpoint.namespace,
            parent_snapshot,
        )
        .await
    }

    async fn do_prepare_snapshot(
        &self,
        cid: &str,
        ns: &str,
        parent_snapshot: String,
    ) -> Result<Vec<Mount>, ContainerdError> {
        let req = PrepareSnapshotRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: cid.to_string(),
            parent: parent_snapshot,
            ..Default::default()
        };
        let mut client = self.client.snapshots();
        let resp = client
            .prepare(with_namespace!(req, ns))
            .await
            .map_err(|e| {
                log::error!("Failed to prepare snapshot: {}", e);
                ContainerdError::CreateSnapshotError(e.to_string())
            })?;

        log::trace!("Prepare snapshot response: {:?}", resp);

        Ok(resp.into_inner().mounts)
    }

    async fn get_parent_snapshot(
        &self,
        image_name: &str,
        namespace: &str,
    ) -> Result<String, ContainerdError> {
        use sha2::Digest;
        let config = self
            .image_config(image_name, namespace)
            .await
            .map_err(|e| {
                log::error!("Failed to get image config: {}", e);
                ContainerdError::GetParentSnapshotError(e.to_string())
            })?;

        if config.rootfs().diff_ids().is_empty() {
            log::error!("Image config has no diff_ids for image: {}", image_name);
            return Err(ContainerdError::GetParentSnapshotError(
                "No diff_ids found in image config".to_string(),
            ));
        }

        let mut iter = config.rootfs().diff_ids().iter();
        let mut ret = iter
            .next()
            .map_or_else(String::new, |layer_digest| layer_digest.clone());

        for layer_digest in iter {
            let mut hasher = sha2::Sha256::new();
            hasher.update(ret.as_bytes());
            ret.push_str(&format!(",{}", layer_digest));
            hasher.update(" ");
            hasher.update(layer_digest);
            let digest = ::hex::encode(hasher.finalize());
            ret = format!("sha256:{digest}");
        }
        Ok(ret)
    }

    pub async fn remove_snapshot(&self, endpoint: &Endpoint) -> Result<(), ContainerdError> {
        let mut sc = self.client.snapshots();
        let req = RemoveSnapshotRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: endpoint.service.clone(),
        };
        sc.remove(with_namespace!(req, endpoint.namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to delete snapshot: {}", e);
                ContainerdError::DeleteContainerError(e.to_string())
            })?;

        Ok(())
    }
}
