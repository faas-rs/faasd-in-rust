use std::net::IpAddr;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::impls::cni::Endpoint;

bitflags! {
    /// Tracks which resources have been successfully created for an instance.
    /// Cleanup uses only these bits — idempotent and crash-safe.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct ResourceSet: u8 {
        const IMAGE     = 0b00001;
        const CONTAINER = 0b00010;
        const NETNS     = 0b00100;
        const SNAPSHOT  = 0b01000;
        const TASK      = 0b10000;
    }
}

impl ResourceSet {
    /// All resources that could exist for a fully-deployed instance.
    pub const ALL: Self = Self::all();

    /// No resources.
    pub const NONE: Self = Self::empty();
}

/// The 12-state persistent lifecycle model.
///
/// Every state transition is durably written to sled **before** the
/// corresponding containerd API call that mutates the world.  If the
/// process crashes, the reconciliation loop can observe the recorded
/// state and repair or complete the operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum InstanceState {
    /// No resources exist. Sled record may be absent or explicitly marked Absent.
    Absent,
    /// Image pull/verification in progress.
    Pulling,
    /// Container object created in containerd; no netns/snapshot/task yet.
    Creating,
    /// CNI netns + bridge configured; IP assigned; no snapshot/task yet.
    Networking,
    /// Overlayfs snapshot prepared; no task yet.
    Snapshoting,
    /// Task created and started; awaiting running confirmation.
    Starting,
    /// Fully deployed and serving traffic.
    Active,
    /// Graceful shutdown: draining in-flight requests.
    Draining,
    /// SIGTERM sent; waiting for task exit (with timeout → SIGKILL).
    Stopping,
    /// Removing resources in reverse order.  Partial failure retains the
    /// record so the next reconciliation pass can resume.
    CleaningUp,
    /// Permanent failure at `inner` state.  Resources that were created
    /// before the error are tracked in `InstanceRecord::resources`.
    Error(Box<InstanceState>),
}

impl InstanceState {
    /// Returns the set of resources that **must** exist for this state.
    ///
    /// For `CleaningUp` and `Error` the answer depends on what was
    /// actually created, so the method returns [`ResourceSet::NONE`];
    /// consult `InstanceRecord::resources` instead.
    pub fn expected_resources(&self) -> ResourceSet {
        match self {
            InstanceState::Absent => ResourceSet::NONE,
            InstanceState::Pulling => ResourceSet::IMAGE,
            InstanceState::Creating => ResourceSet::IMAGE,
            InstanceState::Networking => ResourceSet::IMAGE | ResourceSet::CONTAINER,
            InstanceState::Snapshoting => ResourceSet::IMAGE | ResourceSet::CONTAINER | ResourceSet::NETNS,
            InstanceState::Starting | InstanceState::Active | InstanceState::Draining | InstanceState::Stopping => {
                ResourceSet::ALL
            }
            InstanceState::CleaningUp | InstanceState::Error(_) => ResourceSet::NONE,
        }
    }

    /// Whether this state is terminal (no forward progress possible without
    /// external intervention such as delete or reconciliation).
    pub fn is_terminal(&self) -> bool {
        matches!(self, InstanceState::Error(_) | InstanceState::Absent)
    }

    /// Whether this state represents an active, serving instance.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            InstanceState::Active | InstanceState::Draining | InstanceState::Stopping
        )
    }

    /// Whether this state is part of the deploy (forward) path.
    pub fn is_deploying(&self) -> bool {
        matches!(
            self,
            InstanceState::Pulling
                | InstanceState::Creating
                | InstanceState::Networking
                | InstanceState::Snapshoting
                | InstanceState::Starting
        )
    }

    /// Whether this state is part of the delete (backward) path.
    pub fn is_deleting(&self) -> bool {
        matches!(
            self,
            InstanceState::Draining
                | InstanceState::Stopping
                | InstanceState::CleaningUp
        )
    }
}

impl std::fmt::Display for InstanceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceState::Absent => write!(f, "Absent"),
            InstanceState::Pulling => write!(f, "Pulling"),
            InstanceState::Creating => write!(f, "Creating"),
            InstanceState::Networking => write!(f, "Networking"),
            InstanceState::Snapshoting => write!(f, "Snapshoting"),
            InstanceState::Starting => write!(f, "Starting"),
            InstanceState::Active => write!(f, "Active"),
            InstanceState::Draining => write!(f, "Draining"),
            InstanceState::Stopping => write!(f, "Stopping"),
            InstanceState::CleaningUp => write!(f, "CleaningUp"),
            InstanceState::Error(inner) => write!(f, "Error({inner})"),
        }
    }
}

/// The durable record stored in sled for every function instance.
///
/// The sled key is `endpoint.to_string()` (`{namespace}-{function_name}`).
///
/// **Forward-compatibility constraints** (for deferred Phases 5-7):
/// - `previous_version_key` is unused in the MVP but required for atomic update.
/// - `ip_address` is stored here so Phase 6 can move resolve away from CNI file checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceRecord {
    pub state: InstanceState,
    /// Ground-truth of which resources actually exist.  Cleanup reads this,
    /// not `state.expected_resources()`.
    pub resources: ResourceSet,
    pub endpoint: Endpoint,
    /// The IP assigned by CNI.  `None` until Networking succeeds.
    pub ip_address: Option<IpAddr>,
    /// Reserved for Phase 5 (atomic update).  When present, this key points
    /// to the previous version of the function that is still serving traffic.
    pub previous_version_key: Option<String>,
    /// Human-readable reason when `state == Error(...)`.
    pub error_reason: Option<String>,
    /// Unix timestamp (milliseconds) of first creation.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) of last mutation.
    pub updated_at: u64,
}

impl InstanceRecord {
    /// Create a new record in the `Absent` state.
    pub fn new(endpoint: Endpoint) -> Self {
        let now = now_millis();
        Self {
            state: InstanceState::Absent,
            resources: ResourceSet::NONE,
            endpoint,
            ip_address: None,
            previous_version_key: None,
            error_reason: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new state, updating the timestamp.
    ///
    /// **Does not** automatically update `resources` — the caller is
    /// responsible for setting the correct `ResourceSet` bits after each
    /// containerd API call succeeds.
    pub fn with_state(mut self, state: InstanceState) -> Self {
        self.state = state;
        self.updated_at = now_millis();
        self
    }

    /// Set resources and update timestamp.
    pub fn with_resources(mut self, resources: ResourceSet) -> Self {
        self.resources = resources;
        self.updated_at = now_millis();
        self
    }

    /// Set the IP address and update timestamp.
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self.updated_at = now_millis();
        self
    }

    /// Mark a permanent error, preserving existing resources.
    pub fn with_error(mut self, reason: String) -> Self {
        self.error_reason = Some(reason);
        self.updated_at = now_millis();
        self
    }

    /// Clear the error reason (used when retrying from Error state).
    pub fn clear_error(mut self) -> Self {
        self.error_reason = None;
        self.updated_at = now_millis();
        self
    }

    /// Validate that `resources` is consistent with `state`.
    ///
    /// Returns `Ok(())` when `resources` contains at least the bits
    /// `state.expected_resources()`.  Extra bits are allowed — they mean
    /// a state write was lost (reconciliation will repair).
    ///
    /// `CleaningUp` and `Error` always pass because their resources are
    /// whatever was actually created.
    pub fn validate(&self) -> Result<(), StateError> {
        let expected = self.state.expected_resources();
        if self.state == InstanceState::CleaningUp || matches!(self.state, InstanceState::Error(_)) {
            return Ok(());
        }
        let missing = expected & !self.resources;
        if missing.is_empty() {
            Ok(())
        } else {
            Err(StateError::Inconsistent {
                state: self.state.clone(),
                expected,
                actual: self.resources,
                missing,
            })
        }
    }

    /// Returns the sled key for this record (`endpoint.to_string()`).
    pub fn key(&self) -> String {
        self.endpoint.to_string()
    }
}

/// Errors from the state store or record manipulation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("sled error: {0}")]
    Sled(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("inconsistent record: state={state} expected={expected:?} actual={actual:?} missing={missing:?}")]
    Inconsistent {
        state: InstanceState,
        expected: ResourceSet,
        actual: ResourceSet,
        missing: ResourceSet,
    },
}

impl From<sled::Error> for StateError {
    fn from(e: sled::Error) -> Self {
        StateError::Sled(e.to_string())
    }
}

impl From<serde_json::Error> for StateError {
    fn from(e: serde_json::Error) -> Self {
        StateError::Serialization(e.to_string())
    }
}

/// Compare-and-swap conflict: the current value in sled does not match
/// the expected old value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasConflict {
    pub current: Option<InstanceRecord>,
}

/// Error from a compare-and-swap operation.
///
/// Distinguishes CAS conflicts (the expected old value did not match the
/// stored value) from storage/serialization errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CasError {
    #[error("compare-and-swap conflict")]
    Conflict(CasConflict),
    #[error("state store error: {0}")]
    Store(#[from] StateError),
}

/// Wrapper around `sled::Db` providing typed access to [`InstanceRecord`]s.
#[derive(Clone)]
pub struct StateStore {
    db: sled::Db,
}

impl StateStore {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }

    /// Read a record by endpoint.  Returns `None` when the record does not
    /// exist (which semantically means the instance is in `Absent` state).
    pub fn get(&self, endpoint: &Endpoint) -> Result<Option<InstanceRecord>, StateError> {
        match self.db.get(endpoint.to_string())? {
            Some(ivec) => {
                let record: InstanceRecord = serde_json::from_slice(&ivec)?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Unconditional insert or overwrite.
    pub fn put(&self, record: &InstanceRecord) -> Result<(), StateError> {
        let bytes = serde_json::to_vec(record)?;
        self.db.insert(record.key(), bytes)?;
        Ok(())
    }

    /// Atomically compare-and-swap a record.
    ///
    /// * `old == None`  → key must not exist.
    /// * `old == Some`  → key must contain exactly that record.
    /// * `new == None`  → delete the key.
    /// * `new == Some`  → write the new record.
    ///
    /// This is the serialization primitive: concurrent deploy/delete of the
    /// same function race on this CAS; exactly one wins.
    pub fn cas(
        &self,
        endpoint: &Endpoint,
        old: Option<&InstanceRecord>,
        new: Option<&InstanceRecord>,
    ) -> Result<(), CasError> {
        let key = endpoint.to_string();
        let old_bytes: Option<Vec<u8>> = old
            .map(|r| serde_json::to_vec(r).map_err(StateError::from))
            .transpose()
            .map_err(CasError::Store)?;
        let new_bytes: Option<Vec<u8>> = new
            .map(|r| serde_json::to_vec(r).map_err(StateError::from))
            .transpose()
            .map_err(CasError::Store)?;

        let result = self
            .db
            .compare_and_swap(&key, old_bytes.as_deref(), new_bytes.as_deref())
            .map_err(|e| CasError::Store(StateError::Sled(e.to_string())))?;

        match result {
            Ok(()) => Ok(()),
            Err(conflict) => {
                let current = conflict
                    .current
                    .map(|b| serde_json::from_slice(&b).map_err(StateError::from))
                    .transpose()
                    .map_err(CasError::Store)?;
                Err(CasError::Conflict(CasConflict { current }))
            }
        }
    }

    /// Remove a record unconditionally.  Idempotent (removing a non-existent
    /// key is not an error).
    pub fn remove(&self, endpoint: &Endpoint) -> Result<(), StateError> {
        self.db.remove(endpoint.to_string())?;
        Ok(())
    }

    /// Iterate all records whose state is **not** `Absent`.
    ///
    /// Records in `Absent` state are skipped because they represent either
    /// deleted instances or records that should have been removed.
    pub fn iter_non_absent(
        &self,
    ) -> impl Iterator<Item = Result<InstanceRecord, StateError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((_key, ivec)) => match serde_json::from_slice::<InstanceRecord>(&ivec) {
                Ok(record) => {
                    if record.state == InstanceState::Absent {
                        None
                    } else {
                        Some(Ok(record))
                    }
                }
                Err(e) => Some(Err(StateError::Serialization(e.to_string()))),
            },
            Err(e) => Some(Err(StateError::Sled(e.to_string()))),
        })
    }

    /// Iterate **all** records, including Absent.
    pub fn iter_all(&self) -> impl Iterator<Item = Result<InstanceRecord, StateError>> + '_ {
        self.db.iter().filter_map(|res| match res {
            Ok((_key, ivec)) => match serde_json::from_slice::<InstanceRecord>(&ivec) {
                Ok(record) => Some(Ok(record)),
                Err(e) => Some(Err(StateError::Serialization(e.to_string()))),
            },
            Err(e) => Some(Err(StateError::Sled(e.to_string()))),
        })
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_set_arithmetic() {
        let r = ResourceSet::IMAGE | ResourceSet::CONTAINER;
        assert!(r.contains(ResourceSet::IMAGE));
        assert!(r.contains(ResourceSet::CONTAINER));
        assert!(!r.contains(ResourceSet::NETNS));

        let all = ResourceSet::ALL;
        assert_eq!(
            all,
            ResourceSet::IMAGE
                | ResourceSet::CONTAINER
                | ResourceSet::NETNS
                | ResourceSet::SNAPSHOT
                | ResourceSet::TASK
        );
    }

    #[test]
    fn state_expected_resources() {
        assert_eq!(InstanceState::Absent.expected_resources(), ResourceSet::NONE);
        assert_eq!(InstanceState::Pulling.expected_resources(), ResourceSet::IMAGE);
        assert_eq!(
            InstanceState::Creating.expected_resources(),
            ResourceSet::IMAGE
        );
        assert_eq!(
            InstanceState::Networking.expected_resources(),
            ResourceSet::IMAGE | ResourceSet::CONTAINER
        );
        assert_eq!(
            InstanceState::Snapshoting.expected_resources(),
            ResourceSet::IMAGE | ResourceSet::CONTAINER | ResourceSet::NETNS
        );
        assert_eq!(
            InstanceState::Starting.expected_resources(),
            ResourceSet::ALL
        );
        assert_eq!(InstanceState::Active.expected_resources(), ResourceSet::ALL);
        assert_eq!(
            InstanceState::CleaningUp.expected_resources(),
            ResourceSet::NONE
        );
        assert_eq!(
            InstanceState::Error(Box::new(InstanceState::Pulling)).expected_resources(),
            ResourceSet::NONE
        );
    }

    #[test]
    fn record_validate_ok() {
        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Creating)
            .with_resources(ResourceSet::IMAGE | ResourceSet::CONTAINER);
        assert!(record.validate().is_ok());
    }

    #[test]
    fn record_validate_missing_resources() {
        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Networking)
            .with_resources(ResourceSet::IMAGE); // missing CONTAINER
        let err = record.validate().unwrap_err();
        assert!(
            matches!(
                err,
                StateError::Inconsistent {
                    state: InstanceState::Networking,
                    missing,
                    ..
                } if missing == ResourceSet::CONTAINER
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn record_validate_extra_resources_ok() {
        // Extra resources are allowed — they indicate a lost state write
        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Pulling)
            .with_resources(ResourceSet::IMAGE | ResourceSet::CONTAINER);
        assert!(record.validate().is_ok());
    }

    #[test]
    fn record_serde_roundtrip() {
        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint)
            .with_state(InstanceState::Networking)
            .with_resources(ResourceSet::IMAGE | ResourceSet::CONTAINER | ResourceSet::NETNS)
            .with_ip(IpAddr::from([10, 66, 0, 5]));

        let bytes = serde_json::to_vec(&record).unwrap();
        let decoded: InstanceRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn state_store_get_put_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let endpoint = Endpoint::new("hello", "default");
        assert!(store.get(&endpoint).unwrap().is_none());

        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Active)
            .with_resources(ResourceSet::ALL)
            .with_ip(IpAddr::from([10, 66, 0, 5]));

        store.put(&record).unwrap();
        let fetched = store.get(&endpoint).unwrap().unwrap();
        assert_eq!(fetched.state, InstanceState::Active);
        assert_eq!(fetched.resources, ResourceSet::ALL);

        store.remove(&endpoint).unwrap();
        assert!(store.get(&endpoint).unwrap().is_none());
    }

    #[test]
    fn state_store_cas_success() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let endpoint = Endpoint::new("hello", "default");
        let old = InstanceRecord::new(endpoint.clone());
        let new = old
            .clone()
            .with_state(InstanceState::Pulling)
            .with_resources(ResourceSet::IMAGE);

        store.cas(&endpoint, None, Some(&old)).unwrap();
        store.cas(&endpoint, Some(&old), Some(&new)).unwrap();

        let fetched = store.get(&endpoint).unwrap().unwrap();
        assert_eq!(fetched.state, InstanceState::Pulling);
    }

    #[test]
    fn state_store_cas_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let endpoint = Endpoint::new("hello", "default");
        let old = InstanceRecord::new(endpoint.clone());
        let other = old
            .clone()
            .with_state(InstanceState::Active)
            .with_resources(ResourceSet::ALL);
        store.put(&other).unwrap();

        let new = old
            .clone()
            .with_state(InstanceState::Pulling)
            .with_resources(ResourceSet::IMAGE);
        let err = store.cas(&endpoint, Some(&old), Some(&new)).unwrap_err();
        match err {
            CasError::Conflict(c) => {
                assert!(c.current.is_some());
                assert_eq!(c.current.unwrap().state, InstanceState::Active);
            }
            other => panic!("expected CasError::Conflict, got {other:?}"),
        }
    }

    #[test]
    fn state_store_iter_non_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let e1 = Endpoint::new("a", "ns");
        let e2 = Endpoint::new("b", "ns");
        let e3 = Endpoint::new("c", "ns");

        store
            .put(
                &InstanceRecord::new(e1.clone())
                    .with_state(InstanceState::Active)
                    .with_resources(ResourceSet::ALL),
            )
            .unwrap();
        store
            .put(
                &InstanceRecord::new(e2.clone())
                    .with_state(InstanceState::Absent)
                    .with_resources(ResourceSet::NONE),
            )
            .unwrap();
        store
            .put(
                &InstanceRecord::new(e3.clone())
                    .with_state(InstanceState::CleaningUp)
                    .with_resources(ResourceSet::CONTAINER),
            )
            .unwrap();

        let mut states: Vec<_> = store
            .iter_non_absent()
            .map(|r| r.unwrap().state)
            .collect();
        states.sort_by_key(|s| s.to_string());
        assert_eq!(states, vec![InstanceState::Active, InstanceState::CleaningUp]);
    }

    #[test]
    fn state_store_cas_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Active)
            .with_resources(ResourceSet::ALL);
        store.put(&record).unwrap();
        assert!(store.get(&endpoint).unwrap().is_some());

        // CAS with new = None deletes the record.
        store.cas(&endpoint, Some(&record), None).unwrap();
        assert!(store.get(&endpoint).unwrap().is_none());
    }

    #[test]
    fn state_store_remove_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let store = StateStore::new(db);

        let endpoint = Endpoint::new("ghost", "default");
        // Removing a non-existent key should succeed (idempotent).
        store.remove(&endpoint).unwrap();
    }

    #[test]
    fn record_validate_networking_missing_netns() {
        // Networking state only guarantees IMAGE | CONTAINER (CNI not yet done).
        // Providing IMAGE alone (missing CONTAINER) must fail validation.
        let endpoint = Endpoint::new("hello", "default");
        let record = InstanceRecord::new(endpoint.clone())
            .with_state(InstanceState::Networking)
            .with_resources(ResourceSet::IMAGE); // missing CONTAINER
        let err = record.validate().unwrap_err();
        assert!(
            matches!(
                err,
                StateError::Inconsistent {
                    state: InstanceState::Networking,
                    missing,
                    ..
                } if missing == ResourceSet::CONTAINER
            ),
            "unexpected error: {err}"
        );
    }
}
