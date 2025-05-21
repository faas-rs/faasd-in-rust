use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni};
use crate::provider::ContainerdProvider;
use gateway::handlers::function::DeleteError;
use gateway::types::function::Query;

impl ContainerdProvider {
    pub(crate) async fn _delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();

        let kill_err = backend()
            .kill_task_with_timeout(&endpoint)
            .await
            .map_err(|e| {
                log::error!("Failed to kill task: {:?}", e);
                e
            });

        let del_ctr_err = backend().delete_container(&endpoint).await.map_err(|e| {
            log::error!("Failed to delete container: {:?}", e);
            e
        });

        let rm_snap_err = backend().remove_snapshot(&endpoint).await.map_err(|e| {
            log::error!("Failed to remove snapshot: {:?}", e);
            e
        });

        let del_net_err = cni::cni_impl::delete_cni_network(endpoint);

        if kill_err.is_ok() && del_ctr_err.is_ok() && rm_snap_err.is_ok() {
            Ok(())
        } else {
            Err(DeleteError::Internal(format!(
                "{:?}, {:?}, {:?}, {:?}",
                kill_err, del_ctr_err, rm_snap_err, del_net_err
            )))
        }
    }
}
