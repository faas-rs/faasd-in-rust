//! Thin sled wrapper storing deploy metadata as JSON blobs.
//! netns is the authority for mutual exclusion and IP resolution;
//! sled is a write-through performance cache used only by list() and resolve().

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

/// Deployment metadata stored in sled as a JSON blob.
///
/// `dirty` is `Some(reason)` only when containerd resources exist but
/// netns does not and CNI reconstruction failed. This is the sole
/// remaining case that netns cannot represent on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployMeta {
    pub image: String,
    pub created_at: u64,
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub dirty: Option<String>,
}

/// Lightweight sled wrapper with JSON-based CRUD.
#[derive(Clone)]
pub struct CacheStore {
    db: sled::Db,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheStoreError {
    #[error("sled error: {0}")]
    Sled(String),
}

impl From<sled::Error> for CacheStoreError {
    fn from(e: sled::Error) -> Self {
        CacheStoreError::Sled(e.to_string())
    }
}

impl CacheStore {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    /// Get deploy metadata. Returns None if key absent or deserialization fails.
    pub fn get(&self, endpoint: &Endpoint) -> Result<Option<DeployMeta>, CacheStoreError> {
        match self.db.get(endpoint.to_string())? {
            Some(ivec) => match serde_json::from_slice::<DeployMeta>(&ivec) {
                Ok(meta) => Ok(Some(meta)),
                Err(e) => {
                    log::error!("Failed to deserialize DeployMeta for {endpoint}: {e}");
                    Ok(None)
                }
            },
            None => Ok(None),
        }
    }

    /// Write-through insert. Overwrites any existing record.
    pub fn insert(
        &self,
        endpoint: &Endpoint,
        meta: &DeployMeta,
    ) -> Result<(), CacheStoreError> {
        let val = serde_json::to_vec(meta)
            .map_err(|e| CacheStoreError::Sled(e.to_string()))?;
        self.db.insert(endpoint.to_string(), val.as_slice())?;
        Ok(())
    }

    /// Remove a record. Idempotent.
    pub fn remove(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        self.db.remove(endpoint.to_string())?;
        Ok(())
    }

    /// Mark an endpoint as dirty by setting the dirty field.
    pub fn mark_dirty(
        &self,
        endpoint: &Endpoint,
        reason: &str,
    ) -> Result<(), CacheStoreError> {
        let mut meta = self.get(endpoint)?.unwrap_or_else(|| DeployMeta {
            image: String::new(),
            created_at: 0,
            labels: HashMap::new(),
            dirty: None,
        });
        meta.dirty = Some(reason.to_string());
        self.insert(endpoint, &meta)
    }

    /// Clear the dirty flag on a record.
    pub fn clear_dirty(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        if let Some(mut meta) = self.get(endpoint)? {
            meta.dirty = None;
            self.insert(endpoint, &meta)?;
        }
        Ok(())
    }

    /// Check whether an endpoint has the dirty flag set.
    pub fn is_dirty(&self, endpoint: &Endpoint) -> Result<bool, CacheStoreError> {
        match self.get(endpoint)? {
            Some(meta) => Ok(meta.dirty.is_some()),
            None => Ok(false),
        }
    }

    /// Iterate all stored records.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = Result<(Endpoint, DeployMeta), CacheStoreError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((key, ivec)) => {
                let key_str = String::from_utf8_lossy(&key).to_string();
                let meta: DeployMeta = match serde_json::from_slice(&ivec) {
                    Ok(m) => m,
                    Err(_) => return None,
                };
                let remainder = key_str.strip_prefix("faasdrs-")?;
                let dash_pos = remainder.find('-')?;
                let ns = &remainder[..dash_pos];
                let fn_name = &remainder[dash_pos + 1..];
                Some(Ok((Endpoint::new(fn_name, ns), meta)))
            }
            Err(e) => Some(Err(CacheStoreError::Sled(e.to_string()))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_meta_serde_roundtrip() {
        let meta = DeployMeta {
            image: "alpine:latest".into(),
            created_at: 1717430000000,
            labels: {
                let mut m = HashMap::new();
                m.insert("env".into(), "prod".into());
                m
            },
            dirty: None,
        };
        let json = serde_json::to_vec(&meta).unwrap();
        let roundtripped: DeployMeta = serde_json::from_slice(&json).unwrap();
        assert_eq!(meta, roundtripped);
    }
}
