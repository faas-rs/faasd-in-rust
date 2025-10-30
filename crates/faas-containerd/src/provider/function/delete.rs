use crate::impls::cni::Endpoint;
use crate::impls::{backend, cni, task::TaskError};
use crate::provider::ContainerdProvider;
use gateway::handlers::function::DeleteError;
use gateway::types::function::Query;

impl ContainerdProvider {
    pub(crate) async fn _delete(&self, function: Query) -> Result<(), DeleteError> {
        let endpoint: Endpoint = function.into();
        log::trace!("Deleting function: {:?}", endpoint);

        match backend().kill_task_with_timeout(&endpoint).await {
            Ok(_)=>{},
            Err(e)=> {
                match e {
                    TaskError::NotFound=> {}
                    _ => return Err(DeleteError::Internal(format!("kill task failed: {:?}", e)))
                }
            }
        };
        let del_ctr_err = backend().delete_container(&endpoint).await.map_err(|e| {
            log::error!("Failed to delete container: {:?}", e);
            e
        });

        let rm_snap_err = backend().remove_snapshot(&endpoint).await.map_err(|e| {
            log::error!("Failed to remove snapshot: {:?}", e);
            e
        });

        let del_net_err = cni::cni_impl::delete_cni_network(endpoint);

        if del_ctr_err.is_ok() && rm_snap_err.is_ok() && del_net_err.is_ok() {
            Ok(())
        } else {
            Err(DeleteError::Internal(format!(
                "{:?}, {:?}, {:?}",
                del_ctr_err, rm_snap_err, del_net_err
            )))
        }
    }
}
