use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sled::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRecord {
    pub ip: IpAddr,
    pub created_at: u64,
}

/// CAS-based deploy lock + IP cache backed by sled.
#[derive(Clone)]
pub struct CacheStore {
    db: Db,
}

// ── Key helpers ──────────────────────────────────────────────────────────────

fn lock_key(name: &str) -> Vec<u8> {
    format!("{name}:deploy_lock").into_bytes()
}

fn meta_key(name: &str) -> Vec<u8> {
    format!("{name}:meta").into_bytes()
}

impl CacheStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Try to acquire the deploy lock.  Returns `true` if acquired.
    pub fn try_acquire_deploy(&self, key: &str) -> Result<bool, sled::Error> {
        let k = lock_key(key);
        match self.db.compare_and_swap(k, None::<&[u8]>, Some(b"1"))? {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Commit a deploy result and release the lock atomically.
    pub fn commit_deploy(&self, key: &str, record: &CacheRecord) -> Result<(), sled::Error> {
        let meta = serde_json::to_vec(record).unwrap_or_default();
        self.db.insert(meta_key(key), meta)?;
        self.db.remove(lock_key(key))?;
        Ok(())
    }

    /// Release the deploy lock without committing (deploy failed/cancelled).
    pub fn release_deploy(&self, key: &str) -> Result<(), sled::Error> {
        self.db.remove(lock_key(key))?;
        Ok(())
    }

    /// Look up a cached IP address.
    pub fn get_ip(&self, key: &str) -> Result<Option<CacheRecord>, sled::Error> {
        match self.db.get(meta_key(key))? {
            Some(v) => match serde_json::from_slice::<CacheRecord>(&v) {
                Ok(r) => Ok(Some(r)),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Remove a cached record and release the lock.
    pub fn remove(&self, key: &str) -> Result<(), sled::Error> {
        self.db.remove(meta_key(key))?;
        self.db.remove(lock_key(key))?;
        Ok(())
    }
}
