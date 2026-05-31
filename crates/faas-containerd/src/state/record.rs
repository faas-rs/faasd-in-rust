//! Minimal sled cache: only stores IP addresses for resolve().
//! containerd is the authoritative state machine.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

/// The only data we cache in sled: the IP assigned by CNI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheRecord {
    pub ip: IpAddr,
    /// Unix timestamp (milliseconds) of creation.
    pub created_at: u64,
}

/// Lightweight sled wrapper for [`CacheRecord`].
#[derive(Clone)]
pub struct CacheStore {
    db: sled::Db,
}

impl CacheStore {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    pub fn get(&self, endpoint: &Endpoint) -> Result<Option<CacheRecord>, CacheStoreError> {
        match self.db.get(endpoint.to_string())? {
            Some(ivec) => {
                let record: CacheRecord = serde_json::from_slice(&ivec)?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    pub fn put(&self, endpoint: &Endpoint, record: &CacheRecord) -> Result<(), CacheStoreError> {
        let bytes = serde_json::to_vec(record)?;
        self.db.insert(endpoint.to_string(), bytes)?;
        Ok(())
    }

    pub fn remove(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        self.db.remove(endpoint.to_string())?;
        Ok(())
    }

    /// Iterate all cached records.
    pub fn iter_all(&self) -> impl Iterator<Item = Result<(Endpoint, CacheRecord), CacheStoreError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((key, ivec)) => {
                let key_str = String::from_utf8_lossy(&key).to_string();
                match serde_json::from_slice::<CacheRecord>(&ivec) {
                    Ok(record) => {
                        // Reconstruct Endpoint from key: "faasdrs-{namespace}-{function_name}"
                        let remainder = key_str.strip_prefix("faasdrs-")?;
                        let dash_pos = remainder.find('-')?;
                        let ns = &remainder[..dash_pos];
                        let fn_name = &remainder[dash_pos + 1..];
                        Some(Ok((Endpoint::new(fn_name, ns), record)))
                    }
                    Err(e) => Some(Err(CacheStoreError::Serialization(e.to_string()))),
                }
            }
            Err(e) => Some(Err(CacheStoreError::Sled(e.to_string()))),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CacheStoreError {
    #[error("sled: {0}")]
    Sled(String),
    #[error("serialization: {0}")]
    Serialization(String),
}

impl From<sled::Error> for CacheStoreError {
    fn from(e: sled::Error) -> Self {
        CacheStoreError::Sled(e.to_string())
    }
}

impl From<serde_json::Error> for CacheStoreError {
    fn from(e: serde_json::Error) -> Self {
        CacheStoreError::Serialization(e.to_string())
    }
}

