use std::collections::HashMap;

use gateway::{handlers::namespace::NamespaceError, types::namespace::Namespace};

use crate::{
    impls::{
        backend,
        namespace::{NamespaceServiceError, get_namespace_without_uuid},
    },
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
                    get_namespace_without_uuid(&namespace)
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
                get_namespace_without_uuid(&namespace)
            )));
        }
        let ns = exist.unwrap();
        let name = get_namespace_without_uuid(&ns.name);
        Ok(Namespace {
            name: Some(name),
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
            let name = get_namespace_without_uuid(&ns.name);
            ns_list_result.push(Namespace {
                name: Some(name),
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
                NamespaceServiceError::NotFound => NamespaceError::NotFound(format!(
                    "namespace {} not found",
                    get_namespace_without_uuid(&namespace)
                )),
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
                NamespaceServiceError::NotFound => NamespaceError::NotFound(format!(
                    "namespace {} not found",
                    get_namespace_without_uuid(&namespace)
                )),
                _ => NamespaceError::Internal(e.to_string()),
            })
    }
}
