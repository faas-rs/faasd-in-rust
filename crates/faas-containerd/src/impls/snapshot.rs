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
                ContainerdError::DeleteContainerError(e.to_string())
            })?
            .into_inner()
            .mounts;

        Ok(mounts)
    }

    pub async fn prepare_snapshot(
        &self,
        container: &ContainerStaticMetadata,
    ) -> Result<Vec<Mount>, ContainerdError> {
        let cid = container.endpoint.to_string();
        let ns = &container.endpoint.namespace;
        let parent_snapshot = self.get_parent_snapshot(&container.image, ns).await?;
        self.do_prepare_snapshot(&cid, ns, parent_snapshot).await
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
        use containerd_client::services::v1::snapshots::ListSnapshotsRequest;

        let mut sc = self.client.snapshots();
        let ls_req = ListSnapshotsRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            filters: vec!["parent==".to_string()],
        };

        let mut stream = sc
            .list(with_namespace!(ls_req, namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to list snapshots: {}", e);
                ContainerdError::GetParentSnapshotError(e.to_string())
            })?
            .into_inner();

        let mut infos: Vec<containerd_client::services::v1::snapshots::Info> = Vec::new();
        while let Some(msg) = stream
            .message()
            .await
            .map_err(|e| ContainerdError::GetParentSnapshotError(e.to_string()))?
        {
            infos.extend(msg.info);
        }

        // 如果已经有 parent==空 的快照，直接使用
        for info in &infos {
            log::debug!("Found parent snapshot: {:?}", info);
            if info.parent.is_empty() {
                return Ok(info.name.clone());
            }
        }

        // Fallback: try removing all existing snapshots and create fresh
        for info in &infos {
            let rm_req = RemoveSnapshotRequest {
                snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
                key: info.name.clone(),
            };
            if let Err(e) = sc.remove(with_namespace!(rm_req, namespace)).await {
                log::warn!("Failed to remove old snapshot {}: {}", info.name, e);
            }
        }

        // 强制创建新的空 parent snapshot
        let img_ref = container_image_dist_ref::ImgRef::new(image_name)
            .map_err(|e| ContainerdError::GetParentSnapshotError(format!("{:?}", e)))?;
        let parent_key = format!("{}-rootfs", img_ref.name().to_str());
        let prepare_req = PrepareSnapshotRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: parent_key.clone(),
            parent: String::new(),
            ..Default::default()
        };
        sc.prepare(with_namespace!(prepare_req, namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to prepare parent snapshot: {}", e);
                ContainerdError::GetParentSnapshotError(e.to_string())
            })?;

        Ok(parent_key)
    }

    pub async fn remove_snapshot(&self, endpoint: &Endpoint) -> Result<(), ContainerdError> {
        let mut sc = self.client.snapshots();
        let req = RemoveSnapshotRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: endpoint.to_string(),
        };
        sc.remove(with_namespace!(req, endpoint.namespace))
            .await
            .map_err(|e| {
                log::error!("Failed to remove snapshot: {}", e);
                ContainerdError::DeleteContainerError(e.to_string())
            })?;

        Ok(())
    }

    /// Check whether a snapshot exists in containerd.
    pub async fn snapshot_exists(&self, endpoint: &Endpoint) -> bool {
        let mut sc = self.client.snapshots();
        let req = MountsRequest {
            snapshotter: crate::consts::DEFAULT_SNAPSHOTTER.to_string(),
            key: endpoint.to_string(),
        };
        sc.mounts(with_namespace!(req, endpoint.namespace))
            .await
            .is_ok()
    }
}
