use gateway::types::function;

use crate::consts;

use super::cni::Endpoint;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ContainerStaticMetadata {
    pub image: String,
    pub endpoint: Endpoint,
}

impl From<function::Deployment> for ContainerStaticMetadata {
    fn from(info: function::Deployment) -> Self {
        let ns = info.namespace.unwrap();
        let ns_parts = ns.rsplit_once('-');
        let (uuid, actual_ns) = ns_parts
            .map(|(uuid, ns)| {
                if ns.is_empty() {
                    (uuid, None)
                } else {
                    (uuid, Some(ns))
                }
            })
            .unwrap();
        ContainerStaticMetadata {
            image: info.image,
            endpoint: Endpoint::new(
                &info.service,
                &format!(
                    "{}-{}",
                    uuid,
                    actual_ns.unwrap_or(consts::DEFAULT_FUNCTION_NAMESPACE)
                ),
            ),
        }
    }
}

// /// A function is a container instance with correct cni connected
// #[derive(Debug)]
// pub struct FunctionInstance {
//     container: containerd_client::services::v1::Container,
//     namespace: String,
//     // ip addr inside cni
//     // network: CNIEndpoint,
//     // port: Vec<u16>, default use 8080
//     // manager: Weak<crate::provider::ContainerdProvider>,
// }

// impl FunctionInstance {
//     pub async fn new(metadata: ContainerStaticMetadata) -> Result<Self, ContainerdError> {

//         Ok(Self {
//             container,
//             namespace: metadata.namespace,
//             // network,
//         })
//     }

//     pub async fn delete(&self) -> Result<(), ContainerdError> {
//         let container_id = self.container.id.clone();
//         let namespace = self.namespace.clone();

//         let kill_err = backend()
//             .kill_task_with_timeout(&container_id, &namespace)
//             .await
//             .map_err(|e| {
//                 log::error!("Failed to kill task: {:?}", e);
//                 e
//             });

//         let del_ctr_err = backend()
//             .delete_container(&container_id, &namespace)
//             .await
//             .map_err(|e| {
//                 log::error!("Failed to delete container: {:?}", e);
//                 e
//             });

//         let rm_snap_err = backend()
//             .remove_snapshot(&container_id, &namespace)
//             .await
//             .map_err(|e| {
//                 log::error!("Failed to remove snapshot: {:?}", e);
//                 e
//             });
//         if kill_err.is_ok() && del_ctr_err.is_ok() && rm_snap_err.is_ok() {
//             Ok(())
//         } else {
//             Err(ContainerdError::DeleteContainerError(format!(
//                 "{:?}, {:?}, {:?}",
//                 kill_err, del_ctr_err, rm_snap_err
//             )))
//         }
//     }

//     pub fn address(&self) -> IpAddr {
//         self.network.address()
//     }
// }
