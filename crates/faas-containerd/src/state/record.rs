//! Minimal sled cache with CAS-based serialization for deploy/delete.
//! containerd is the authoritative state machine; sled provides mutual exclusion
//! and a small IP cache for resolve().

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

/// Sled value states.  Key absent = no operation in flight, no cached IP.
///
/// State machine:
///   Absent → InFlight → Cached(ip)       (deploy success)
///   Absent → InFlight → Absent            (deploy cancelled before commit)
///   Cached → Absent                       (delete)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheTag {
    /// Deploy in progress — serialization point.  Blocks concurrent deploy and delete.
    InFlight,
    /// Deploy completed.  Carries the IP for resolve().
    Cached(CacheRecord),
}

/// IP cache entry stored when deploy succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheRecord {
    pub ip: IpAddr,
    pub created_at: u64,
}

// ── Wire format ───────────────────────────────────────────────────────
// Key absent → semantically Absent
// Key = [0]        → InFlight
// Key = [1, …]     → Cached; remaining bytes are JSON

const TAG_INFLIGHT: u8 = 0;
const TAG_CACHED: u8 = 1;

fn decode_tag(bytes: &[u8]) -> Option<CacheTag> {
    match *bytes.first()? {
        TAG_INFLIGHT => Some(CacheTag::InFlight),
        TAG_CACHED => {
            let record: CacheRecord = serde_json::from_slice(&bytes[1..]).ok()?;
            Some(CacheTag::Cached(record))
        }
        _ => None,
    }
}

/// Lightweight sled wrapper with CAS-based serialization.
#[derive(Clone)]
pub struct CacheStore {
    db: sled::Db,
}

impl CacheStore {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    /// Read the current tag.  Absent key = None.
    pub fn get_tag(&self, endpoint: &Endpoint) -> Result<Option<CacheTag>, CacheStoreError> {
        match self.db.get(endpoint.to_string())? {
            Some(ivec) => Ok(decode_tag(&ivec)),
            None => Ok(None),
        }
    }

    /// Read cached IP.  Returns None if not Cached.
    pub fn get_ip(&self, endpoint: &Endpoint) -> Result<Option<CacheRecord>, CacheStoreError> {
        match self.get_tag(endpoint)? {
            Some(CacheTag::Cached(r)) => Ok(Some(r)),
            _ => Ok(None),
        }
    }

    // ── CAS primitives ────────────────────────────────────────────

    /// Acquire the deploy lock: CAS from Absent (None) → InFlight.
    /// Returns Ok(true) if acquired, Ok(false) if key already exists.
    pub fn try_acquire_deploy(&self, endpoint: &Endpoint) -> Result<bool, CacheStoreError> {
        match self.db.compare_and_swap(
            endpoint.to_string(),
            None::<&[u8]>,
            Some(&[TAG_INFLIGHT][..]),
        )? {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Commit deploy: CAS from InFlight → Cached(record).
    /// Returns Ok(true) if committed, Ok(false) on CAS conflict.
    pub fn commit_deploy(
        &self,
        endpoint: &Endpoint,
        record: &CacheRecord,
    ) -> Result<bool, CacheStoreError> {
        let mut val = vec![TAG_CACHED];
        serde_json::to_writer(&mut val, record).map_err(|e| {
            CacheStoreError::Sled(e.to_string())
        })?;

        match self.db.compare_and_swap(
            endpoint.to_string(),
            Some(&[TAG_INFLIGHT][..]),
            Some(val.as_slice()),
        )? {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Release deploy lock on cancel: remove the InFlight key.
    /// Idempotent — removing a non-existent key is not an error.
    pub fn release_deploy(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        self.db.remove(endpoint.to_string())?;
        Ok(())
    }

    /// Remove a cached record (delete path).  Idempotent.
    pub fn remove(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        self.db.remove(endpoint.to_string())?;
        Ok(())
    }

    /// Iterate all Cached records.
    pub fn iter_cached(
        &self,
    ) -> impl Iterator<Item = Result<(Endpoint, CacheRecord), CacheStoreError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((key, ivec)) => {
                let key_str = String::from_utf8_lossy(&key).to_string();
                match decode_tag(&ivec) {
                    Some(CacheTag::Cached(record)) => {
                        let remainder = key_str.strip_prefix("faasdrs-")?;
                        let dash_pos = remainder.find('-')?;
                        let ns = &remainder[..dash_pos];
                        let fn_name = &remainder[dash_pos + 1..];
                        Some(Ok((Endpoint::new(fn_name, ns), record)))
                    }
                    _ => None, // skip InFlight or corrupt
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
}

impl From<sled::Error> for CacheStoreError {
    fn from(e: sled::Error) -> Self {
        CacheStoreError::Sled(e.to_string())
    }
}
