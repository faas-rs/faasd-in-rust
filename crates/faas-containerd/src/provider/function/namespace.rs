use std::collections::HashMap;

use gateway::{handlers::namespace::NamespaceError, types::namespace::Namespace};

use crate::{
    impls::{backend, namespace::NamespaceServiceError},
    provider::ContainerdProvider,
};

impl ContainerdProvider {
    pub(crate) async fn _create_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceError> {
        backend()
            .create_namespace(&namespace, labels)
            .await
            .map_err(|e| match e {
                NamespaceServiceError::AlreadyExists => NamespaceError::AlreadyExists(format!(
                    "namespace {} has been existed",
                    namespace
                )),
                _ => NamespaceError::Internal(e.to_string()),
            })
    }

    pub(crate) async fn _get_namespace(
        &self,
        namespace: String,
    ) -> Result<Namespace, NamespaceError> {
        let exist = backend()
            .namespace_exist(&namespace)
            .await
            .map_err(|e| NamespaceError::Internal(e.to_string()))?;
        if exist.is_none() {
            return Err(NamespaceError::NotFound(format!(
                "namespace {} not found",
                namespace
            )));
        }
        let ns = exist.unwrap();
        Ok(Namespace {
            name: Some(ns.name),
            labels: ns.labels,
        })
    }

    pub(crate) async fn _namespace_list(&self) -> Result<Vec<Namespace>, NamespaceError> {
        let ns_list = backend()
            .list_namespace()
            .await
            .map_err(|e| NamespaceError::Internal(e.to_string()))?;
        let mut ns_list_result = Vec::new();
        for ns in ns_list {
            ns_list_result.push(Namespace {
                name: Some(ns.name),
                labels: ns.labels,
            });
        }
        Ok(ns_list_result)
    }

    pub(crate) async fn _delete_namespace(&self, namespace: String) -> Result<(), NamespaceError> {
        backend()
            .delete_namespace(&namespace)
            .await
            .map_err(|e| match e {
                NamespaceServiceError::NotFound => {
                    NamespaceError::NotFound(format!("namespace {} not found", namespace))
                }
                _ => NamespaceError::Internal(e.to_string()),
            })
    }

    pub(crate) async fn _update_namespace(
        &self,
        namespace: String,
        labels: HashMap<String, String>,
    ) -> Result<(), NamespaceError> {
        backend()
            .update_namespace(&namespace, labels)
            .await
            .map_err(|e| match e {
                NamespaceServiceError::NotFound => {
                    NamespaceError::NotFound(format!("namespace {} not found", namespace))
                }
                _ => NamespaceError::Internal(e.to_string()),
            })
    }
}
