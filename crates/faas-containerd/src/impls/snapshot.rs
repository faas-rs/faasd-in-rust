use containerd_client::{services::v1::snapshots::MountsRequest, types::Mount, with_namespace};
use tonic::Request;

use crate::impls::error::ContainerdError;

use super::ContainerdService;

impl ContainerdService {
    pub(super) async fn get_mounts(
        &self,
        cid: &str,
        ns: &str,
    ) -> Result<Vec<Mount>, ContainerdError> {
        let mut sc = self.client.snapshots();
        let req = MountsRequest {
            snapshotter: "overlayfs".to_string(),
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
}
