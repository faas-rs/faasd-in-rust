//! Minimal sled cache with CAS-based serialization for deploy/delete.
//! containerd is the authoritative state machine; sled provides mutual exclusion
//! and a small IP cache for resolve().

use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

/// Sled value states.  Key absent = no operation in flight, no cached IP.
///
/// State machine:
///   Absent → InFlight → Cached(ip)            (deploy success)
///   Absent → InFlight → Absent                 (deploy cancelled before commit)
///   Absent → InFlight → Dirty(reason)          (deploy step failed, cleanup hung)
///   Cached → Dirty(reason)                     (delete step hung)
///   Cached → Absent                            (delete complete)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheTag {
    /// Deploy in progress — serialization point.  Blocks concurrent deploy and delete.
    InFlight,
    /// Deploy completed. Carries the IP for resolve().
    Cached(CacheRecord),
    /// Operation timed out, cleanup couldn't recover. Leaves audit trail.
    Dirty(DirtyRecord),
}

/// IP cache entry stored when deploy succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheRecord {
    pub ip: IpAddr,
    pub created_at: u64,
}

/// Audit record stored when a deploy or delete step times out and
/// cleanup itself cannot complete. These are recovered at startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRecord {
    pub reason: String,
    pub dirty_at: u64,
    pub attempts: u32,
}

// ── Wire format ───────────────────────────────────────────────────────
// Key absent → semantically Absent
// Key = [0]        → InFlight
// Key = [1, …]     → Cached; remaining bytes are JSON (CacheRecord)
// Key = [2, …]     → Dirty; remaining bytes are JSON (DirtyRecord)

const TAG_INFLIGHT: u8 = 0;
const TAG_CACHED: u8 = 1;
const TAG_DIRTY: u8 = 2;

fn decode_tag(bytes: &[u8]) -> Option<CacheTag> {
    match *bytes.first()? {
        TAG_INFLIGHT => Some(CacheTag::InFlight),
        TAG_CACHED => {
            let record: CacheRecord = serde_json::from_slice(&bytes[1..]).ok()?;
            Some(CacheTag::Cached(record))
        }
        TAG_DIRTY => {
            let record: DirtyRecord = serde_json::from_slice(&bytes[1..]).ok()?;
            Some(CacheTag::Dirty(record))
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

    /// Check if this endpoint is in Dirty state.
    pub fn is_dirty(&self, endpoint: &Endpoint) -> Result<bool, CacheStoreError> {
        match self.get_tag(endpoint)? {
            Some(CacheTag::Dirty(_)) => Ok(true),
            _ => Ok(false),
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
        serde_json::to_writer(&mut val, record)
            .map_err(|e| CacheStoreError::Sled(e.to_string()))?;

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

    /// Mark an endpoint as Dirty with reason. Overwrites whatever was there.
    pub fn mark_dirty(&self, endpoint: &Endpoint, reason: &str) -> Result<(), CacheStoreError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let record = DirtyRecord {
            reason: reason.to_string(),
            dirty_at: now,
            attempts: 0,
        };
        let mut val = vec![TAG_DIRTY];
        serde_json::to_writer(&mut val, &record)
            .map_err(|e| CacheStoreError::Sled(e.to_string()))?;
        self.db.insert(endpoint.to_string(), val.as_slice())?;
        Ok(())
    }

    /// Increment retry attempts on a Dirty record. No-op if not Dirty.
    pub fn increment_dirty_attempts(&self, endpoint: &Endpoint) -> Result<(), CacheStoreError> {
        match self.get_tag(endpoint)? {
            Some(CacheTag::Dirty(mut record)) => {
                record.attempts = record.attempts.saturating_add(1);
                let mut val = vec![TAG_DIRTY];
                serde_json::to_writer(&mut val, &record)
                    .map_err(|e| CacheStoreError::Sled(e.to_string()))?;
                self.db.insert(endpoint.to_string(), val.as_slice())?;
                Ok(())
            }
            _ => Ok(()),
        }
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
                    _ => None,
                }
            }
            Err(e) => Some(Err(CacheStoreError::Sled(e.to_string()))),
        })
    }

    /// Iterate all Dirty records — used at startup for recovery scanning.
    pub fn iter_dirty(
        &self,
    ) -> impl Iterator<Item = Result<(Endpoint, DirtyRecord), CacheStoreError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((key, ivec)) => {
                let key_str = String::from_utf8_lossy(&key).to_string();
                match decode_tag(&ivec) {
                    Some(CacheTag::Dirty(record)) => {
                        let remainder = key_str.strip_prefix("faasdrs-")?;
                        let dash_pos = remainder.find('-')?;
                        let ns = &remainder[..dash_pos];
                        let fn_name = &remainder[dash_pos + 1..];
                        Some(Ok((Endpoint::new(fn_name, ns), record)))
                    }
                    _ => None,
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
