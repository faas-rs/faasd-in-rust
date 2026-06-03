//! Thin sled wrapper storing deploy metadata as JSON blobs.
//! netns is the authority for mutual exclusion and IP resolution;
//! sled is a write-through performance cache used only by list() and resolve().

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

/// Tracks the lifecycle state of a function beyond what netns alone captures.
///
/// `Clean` → function is healthy (netns may or may not be present — resolve
/// checks netns as fast path regardless of dirty state).
///
/// In-progress states (`Deploying`, `Deleting`, `Repairing`) are transient
/// locks: resolve returns `Busy` (503) so the caller retries. On crash,
/// startup scan promotes these to `Broken`.
///
/// `Broken(reason)` is terminal: resolve returns `Unavailable` (502).
/// Only a new deploy/delete can clear it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirtyState {
    #[default]
    Clean,
    Deploying,
    Deleting,
    Repairing,
    Broken(String),
}

/// Deployment metadata stored in sled as a JSON blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployMeta {
    pub image: String,
    pub created_at: u64,
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub dirty: DirtyState,
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

    /// Set the dirty state on a record, creating it if absent.
    pub fn mark_dirty(
        &self,
        endpoint: &Endpoint,
        state: DirtyState,
    ) -> Result<(), CacheStoreError> {
        let mut meta = self.get(endpoint)?.unwrap_or_else(|| DeployMeta {
            image: String::new(),
            created_at: 0,
            labels: HashMap::new(),
            dirty: DirtyState::Clean,
        });
        meta.dirty = state;
        self.insert(endpoint, &meta)
    }

    /// Clear the dirty flag — sets state to Clean.
    pub fn clear_dirty(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        if let Some(mut meta) = self.get(endpoint)? {
            meta.dirty = DirtyState::Clean;
            self.insert(endpoint, &meta)?;
        }
        Ok(())
    }

    /// Check whether an endpoint has a non-Clean dirty state.
    pub fn is_dirty(&self, endpoint: &Endpoint) -> Result<bool, CacheStoreError> {
        match self.get(endpoint)? {
            Some(meta) => Ok(!matches!(meta.dirty, DirtyState::Clean)),
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
            dirty: DirtyState::Clean,
        };
        let json = serde_json::to_vec(&meta).unwrap();
        let roundtripped: DeployMeta = serde_json::from_slice(&json).unwrap();
        assert_eq!(meta, roundtripped);
    }

    #[test]
    fn test_dirty_state_serde() {
        // Verify all variants serialize/deserialize
        let variants = vec![
            DirtyState::Clean,
            DirtyState::Deploying,
            DirtyState::Deleting,
            DirtyState::Repairing,
            DirtyState::Broken("test reason".into()),
        ];
        for v in variants {
            let json = serde_json::to_vec(&v).unwrap();
            let rt: DirtyState = serde_json::from_slice(&json).unwrap();
            assert_eq!(v, rt);
        }
    }
}
