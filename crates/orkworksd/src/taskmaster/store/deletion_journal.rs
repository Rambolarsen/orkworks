//! Bounded, FD-anchored recommendation deletion journal and SQL cleanup capability seam.
//! Application admission remains separate (`session_application::sql_writes`).

use super::{operation, RecommendationStore, StoreError};
use crate::workspace_runtime::WorkspaceLease;
use std::{path::PathBuf, sync::Arc};

#[cfg(any(target_os = "macos", target_os = "linux"))]
use super::{
    deletion_graph_ids, hash_bytes, recommendation_filename, references_session, sql_codec,
    transaction_backup_path, validate_graph_records, validate_manifest_entries,
    validate_recommendation, Recommendation, RollupTransactionEntry, RollupTransactionManifest,
};
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use std::io;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{CStr, CString},
    fs, io,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
};

// Existing anchored enumeration budget: 1,024 leaves plus one known scaffold.
pub(super) const MAX_DIRECTORY_ENTRIES: usize = 1025;

#[cfg(any(target_os = "macos", target_os = "linux"))]
const ENTRIES: usize = 1024;
#[cfg(any(target_os = "macos", target_os = "linux"))]
const RECORD_BYTES: usize = 1024 * 1024;
#[cfg(any(target_os = "macos", target_os = "linux"))]
const TOTAL_BYTES: usize = 32 * RECORD_BYTES;

pub(crate) struct CleanupIntent {
    store: RecommendationStore,
}

#[derive(Clone)]
pub(crate) struct BoundCleanup(Arc<Binding>);

struct Binding {
    store: RecommendationStore,
    _lease: Arc<WorkspaceLease>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    root: fs::File,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    records: Option<fs::File>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[must_use]
pub(crate) struct CleanupInventory {
    records: Vec<Recommendation>,
    leaves: BTreeMap<String, CapturedLeaf>,
    _operation: Arc<operation::Operation>,
    _binding: BoundCleanup,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct CapturedLeaf {
    bytes: Vec<u8>,
    identity: (u64, u64),
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) struct PreparedRemoval {
    inventory: CleanupInventory,
    entries: Vec<RollupTransactionEntry>,
    uncommitted: Vec<u8>,
    committed: Vec<u8>,
}

/// Borrowed observations; copying these values does not copy the owner's authority.
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
pub(crate) struct RemovalTarget<'a> {
    pub(crate) id: &'a str,
    pub(crate) filename: &'a str,
    pub(crate) backup: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) identity: (u64, u64),
}

#[must_use]
pub(crate) struct JournalReceipt {
    result: Result<(), StoreError>,
    // Field order releases the operation before the workspace binding/lease.
    _operation: Arc<operation::Operation>,
    _binding: BoundCleanup,
}

impl JournalReceipt {
    pub(crate) fn result(&self) -> &Result<(), StoreError> {
        &self.result
    }
}

/// Unadmitted memory-only claim. Dropping it does not change owner mode.
pub(crate) struct SqlCleanupClaim {
    operation: Arc<operation::Operation>,
    binding: BoundCleanup,
}

/// Admitted SQL work; not constructible from an identity or journal bytes.
pub(crate) struct SqlCleanup(SqlCleanupClaim);

impl RecommendationStore {
    // Shared canonical-root operation identity, not a filesystem freshness check.
    pub(crate) fn same_owner(&self, other: &Self) -> bool {
        self.dir == other.dir && Arc::ptr_eq(&self.owner, &other.owner)
    }

    // Memory only: capture while the caller owns its workspace identity.
    pub(crate) fn capture_cleanup(&self) -> CleanupIntent {
        CleanupIntent {
            store: self.clone(),
        }
    }

    /// Existing-root recovery binding only: no migration, legacy recovery, or mkdir.
    pub(crate) fn bind_cleanup_recovery(
        root: PathBuf,
        lease: Arc<WorkspaceLease>,
    ) -> Result<BoundCleanup, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let root = fs::canonicalize(root).map_err(StoreError::Io)?;
            let store = Self {
                owner: operation::for_root(&root)?,
                dir: root.join("recommendations"),
            };
            store.capture_cleanup().bind(lease)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (root, lease);
            Err(unsupported())
        }
    }
}

impl CleanupIntent {
    pub(crate) fn bind(self, lease: Arc<WorkspaceLease>) -> Result<BoundCleanup, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            unix::bind(self.store, lease).map_err(StoreError::Io)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (self.store, lease);
            Err(StoreError::Io(io::Error::new(
                io::ErrorKind::Unsupported,
                "recommendation cleanup preflight is not qualified on this platform",
            )))
        }
    }
}

impl BoundCleanup {
    // Blocking-only. Caller releases application guards before inspection.
    #[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
    pub(crate) fn inspect(&self) -> Result<CleanupInventory, StoreError> {
        self.inspect_mode(false)
    }

    // Legacy successful graph transactions retain an empty journal parent.
    // Permit only that scaffold; pending journals still need separate authority.
    #[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
    pub(crate) fn prepare_removal(&self, session_id: &str) -> Result<PreparedRemoval, StoreError> {
        self.inspect_mode(true)?.prepare_session_removal(session_id)
    }

    #[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
    fn inspect_mode(&self, allow_empty_journal: bool) -> Result<CleanupInventory, StoreError> {
        let operation = self.0.store.try_operation()?;
        let (records, leaves) = unix::inspect(&self.0, allow_empty_journal)?;
        Ok(CleanupInventory {
            records,
            leaves,
            _binding: self.clone(),
            _operation: Arc::new(operation),
        })
    }

    /// Memory-only, nonblocking. Current application authority must be checked
    /// before admit; no recovery/inspection occurs while holding app guards.
    pub(crate) fn claim_sql_cleanup(&self) -> Result<SqlCleanupClaim, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            Ok(SqlCleanupClaim {
                operation: Arc::new(self.0.store.owner.claim_sql()?),
                binding: self.clone(),
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Err(unsupported())
    }

    /// Read-only original binding validation under the retained operation.
    pub(crate) fn claim_sql_open(&self) -> Result<SqlCleanupClaim, StoreError> {
        let claim = self.claim_sql_cleanup()?;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        unix::current(&self.0).map_err(StoreError::Io)?;
        Ok(claim)
    }

    // Trusted store-layer fixture only; production must hand off checked admission.
    #[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
    fn remove_for_sql(
        &self,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> Result<JournalReceipt, StoreError> {
        let claim = self.claim_sql_cleanup()?;
        let admitted = claim
            .admit(crate::session_application::sql_writes::trusted_cleanup_admission_fixture());
        Ok(admitted.remove(identity))
    }

    #[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
    pub(crate) fn recover_removals(&self) -> Result<JournalReceipt, StoreError> {
        let operation = self.0.store.try_operation()?;
        let result = protocol::recover(&self.0).map_err(StoreError::Io);
        Ok(JournalReceipt {
            result,
            _operation: Arc::new(operation),
            _binding: self.clone(),
        })
    }
}

impl SqlCleanupClaim {
    /// Opening grants no exact-job cleanup authority and performs no file I/O.
    pub(crate) fn admit_open(
        self,
        _proof: crate::session_application::sql_writes::RecommendationOpenAdmission,
    ) -> RecommendationStore {
        self.operation.revoke_legacy();
        self.binding.0.store.clone()
    }

    /// Consume the opaque proof minted by checked application admission; callers
    /// cannot replace workspace/lease/delivery/cancellation checks with a callback.
    pub(crate) fn admit(
        self,
        _admission: crate::session_application::sql_writes::RecommendationCleanupAdmission,
    ) -> SqlCleanup {
        self.operation.revoke_legacy();
        SqlCleanup(self)
    }
}

impl SqlCleanup {
    pub(crate) fn remove(
        self,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> JournalReceipt {
        let SqlCleanupClaim { operation, binding } = self.0;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let result = (|| {
            protocol::recover_sql(&binding.0, identity).map_err(StoreError::Io)?;
            #[cfg(test)]
            {
                sql_tests::after_recovery();
                crate::taskmaster::store::sql_cleanup_test_gate::recovered(&binding.0.store.dir);
            }
            let (records, leaves) = unix::inspect(&binding.0, true)?;
            let inventory = CleanupInventory {
                records,
                leaves,
                _operation: operation.clone(),
                _binding: binding.clone(),
            };
            let removal =
                inventory.prepare_removal_for(identity.session_id(), Kind::Sql(identity))?;
            protocol::publish_sql(&removal, identity).map_err(StoreError::Io)
        })();
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let result = {
            let _ = identity;
            Err(unsupported())
        };
        JournalReceipt {
            result,
            _operation: operation,
            _binding: binding,
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl CleanupInventory {
    // Copies remain snapshots, never a deletion or source-freshness capability.
    #[cfg(test)]
    pub(crate) fn records(&self) -> &[Recommendation] {
        &self.records
    }

    // Pure legacy v2 preparation for journal/removal tests.
    #[cfg(test)]
    pub(crate) fn prepare_session_removal(
        self,
        session_id: &str,
    ) -> Result<PreparedRemoval, StoreError> {
        self.prepare_removal_for(session_id, Kind::Legacy)
    }

    // Pure preparation from the bounded inventory parameterized directly by journal kind.
    fn prepare_removal_for(
        self,
        session_id: &str,
        kind: Kind<'_>,
    ) -> Result<PreparedRemoval, StoreError> {
        let seeds = self
            .records
            .iter()
            .filter(|record| references_session(record, session_id))
            .map(|record| record.id.clone())
            .collect();
        let ids = deletion_graph_ids(&self.records, seeds);
        let mut entries = Vec::new();
        for record in &self.records {
            if !ids.contains(&record.id) {
                continue;
            }
            let leaf = self.leaves.get(&record.id).ok_or_else(|| {
                StoreError::Recovery("removal inventory lacks original leaf proof".into())
            })?;
            entries.push(RollupTransactionEntry {
                id: record.id.clone(),
                target: recommendation_filename(&record.id),
                staged: None,
                backup: Some(transaction_backup_path(&record.id)),
                old_sha256: Some(hash_bytes(&leaf.bytes)),
                old_exists: true,
                new_sha256: None,
                new_exists: false,
            });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        validate_manifest_entries(&entries)?;
        let (uncommitted, committed) = kind.encode_pair(&entries)?;
        Ok(PreparedRemoval {
            inventory: self,
            entries,
            uncommitted,
            committed,
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl PreparedRemoval {
    pub(crate) fn manifest_bytes(&self, committed: bool) -> &[u8] {
        if committed {
            &self.committed
        } else {
            &self.uncommitted
        }
    }

    #[cfg(test)]
    pub(crate) fn targets(&self) -> impl Iterator<Item = RemovalTarget<'_>> {
        self.entries.iter().map(|entry| {
            // All entries are deletion-only and validated before construction;
            // the inventory and entries are private and immutable afterwards.
            let leaf = self
                .inventory
                .leaves
                .get(&entry.id)
                .expect("prepared removal retains each original leaf");
            RemovalTarget {
                id: &entry.id,
                filename: &entry.target,
                backup: entry
                    .backup
                    .as_deref()
                    .expect("prepared deletion has a backup"),
                bytes: &leaf.bytes,
                identity: leaf.identity,
            }
        })
    }

    #[cfg(test)]
    pub(crate) fn publish(self) -> JournalReceipt {
        let result = protocol::publish(&self).map_err(StoreError::Io);
        JournalReceipt {
            result,
            _operation: self.inventory._operation,
            _binding: self.inventory._binding,
        }
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
fn bounded_manifest(manifest: &RollupTransactionManifest) -> Result<Vec<u8>, StoreError> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(StoreError::Json)?;
    if bytes.len() > RECORD_BYTES {
        return Err(StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "removal exceeds manifest byte bound",
        )));
    }
    Ok(bytes)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported() -> StoreError {
    StoreError::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "recommendation deletion journal is not qualified on this platform",
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Clone, Copy)]
enum Kind<'a> {
    #[cfg(test)]
    Legacy,
    Sql(&'a crate::session_store::SqlDeletionIdentity),
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Kind<'_> {
    fn encode_pair(
        self,
        entries: &[RollupTransactionEntry],
    ) -> Result<(Vec<u8>, Vec<u8>), StoreError> {
        match self {
            #[cfg(test)]
            Self::Legacy => {
                let mut manifest = RollupTransactionManifest {
                    version: super::ROLLUP_TRANSACTION_VERSION,
                    committed: false,
                    entries: entries.to_vec(),
                };
                let uncommitted = bounded_manifest(&manifest)?;
                manifest.committed = true;
                let committed = bounded_manifest(&manifest)?;
                Ok((uncommitted, committed))
            }
            Self::Sql(identity) => Ok((
                sql_codec::encode(identity, entries, false)?,
                sql_codec::encode(identity, entries, true)?,
            )),
        }
    }

    fn name(self) -> String {
        match self {
            #[cfg(test)]
            Self::Legacy => format!("tx-{}", uuid::Uuid::new_v4()),
            Self::Sql(identity) => identity.journal_name(),
        }
    }

    fn check_name(self, text: &str) -> io::Result<()> {
        match self {
            Self::Sql(identity) if text == identity.journal_name() => Ok(()),
            Self::Sql(_) => Err(unix::invalid(
                "pending journal belongs to another SQL deletion",
            )),
            #[cfg(test)]
            Self::Legacy => {
                let id = text
                    .strip_prefix("tx-")
                    .ok_or_else(|| unix::invalid("invalid transaction name"))?;
                if uuid::Uuid::parse_str(id)
                    .map(|u| u.to_string())
                    .ok()
                    .as_deref()
                    != Some(id)
                {
                    return Err(unix::invalid("noncanonical transaction identity"));
                }
                Ok(())
            }
        }
    }

    fn parse(self, bytes: &[u8], name: &str) -> io::Result<RollupTransactionManifest> {
        match self {
            Self::Sql(identity) => {
                let decoded = sql_codec::decode(bytes, identity, name)
                    .map_err(|e| unix::invalid(&e.to_string()))?;
                Ok(RollupTransactionManifest {
                    version: 3,
                    committed: decoded.committed,
                    entries: decoded.entries,
                })
            }
            #[cfg(test)]
            Self::Legacy => {
                let m: RollupTransactionManifest =
                    crate::harness::definition::parse_strict_json(bytes, RECORD_BYTES)
                        .map_err(|_| unix::invalid("invalid deletion manifest"))?;
                if m.version != 2
                    || m.entries.len() > ENTRIES
                    || m.entries.iter().any(|e| {
                        !e.old_exists
                            || e.new_exists
                            || e.staged.is_some()
                            || e.new_sha256.is_some()
                    })
                {
                    return Err(unix::invalid("unsupported deletion manifest"));
                }
                validate_manifest_entries(&m.entries).map_err(|e| unix::invalid(&e.to_string()))?;
                Ok(m)
            }
        }
    }

    fn encode(self, manifest: &RollupTransactionManifest) -> io::Result<Vec<u8>> {
        match self {
            #[cfg(test)]
            Self::Legacy => bounded_manifest(manifest),
            Self::Sql(identity) => {
                sql_codec::encode(identity, &manifest.entries, manifest.committed)
            }
        }
        .map_err(|e| unix::invalid(&e.to_string()))
    }

    fn selection(
        self,
        records: &[Recommendation],
        entries: &BTreeMap<&str, &RollupTransactionEntry>,
        require_complete: bool,
    ) -> io::Result<()> {
        match self {
            Self::Sql(identity) => {
                let seeds = records
                    .iter()
                    .filter(|r| references_session(r, identity.session_id()))
                    .map(|r| r.id.clone())
                    .collect();
                let expected = deletion_graph_ids(records, seeds);
                let selected: BTreeSet<_> = entries.values().map(|e| e.id.clone()).collect();
                // Committed recovery may observe extra independent references from
                // before producer fencing; the retained fresh pass removes them.
                if !selected.is_subset(&expected) || (require_complete && expected != selected) {
                    return Err(unix::invalid(
                        "SQL journal targets do not match the session graph",
                    ));
                }
                Ok(())
            }
            #[cfg(test)]
            Self::Legacy => Ok(()),
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod unix {
    use super::*;

    pub(super) fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }

    pub(super) fn open(parent: &fs::File, name: &CStr, directory: bool) -> io::Result<fs::File> {
        // SAFETY: live parent FD and a fixed or enumerated single C basename.
        // No-follow rejects aliases; nonblocking avoids waiting on a FIFO.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC
                    | if directory { libc::O_DIRECTORY } else { 0 },
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful openat returned one new owned descriptor.
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }

    pub(super) fn identity(file: &fs::File) -> io::Result<(u64, u64)> {
        let m = file.metadata()?;
        Ok((m.dev(), m.ino()))
    }

    pub(super) fn entry(parent: &fs::File, name: &CStr) -> io::Result<Option<(u64, u64)>> {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: live FD, valid basename and writable stat output; never follows aliases.
        if unsafe {
            libc::fstatat(
                parent.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } < 0
        {
            let e = io::Error::last_os_error();
            return if e.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            };
        }
        // SAFETY: fstatat succeeded and initialized stat.
        let stat = unsafe { stat.assume_init() };
        Ok(Some((stat.st_dev as u64, stat.st_ino as u64)))
    }

    pub(super) fn bind(
        store: RecommendationStore,
        lease: Arc<WorkspaceLease>,
    ) -> io::Result<BoundCleanup> {
        let root = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(
                store
                    .dir
                    .parent()
                    .ok_or_else(|| invalid("recommendation root unavailable"))?,
            )?;
        lease.validate_directory(&root)?;
        let records = match open(&root, c"recommendations", true) {
            Ok(file) => Some(file),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(BoundCleanup(Arc::new(Binding {
            store,
            _lease: lease,
            root,
            records,
        })))
    }

    pub(super) fn current(binding: &Binding) -> io::Result<()> {
        binding._lease.validate_directory(&binding.root)?;
        let expected = binding.records.as_ref().map(identity).transpose()?;
        if entry(&binding.root, c"recommendations")? != expected {
            return Err(invalid("recommendation directory changed after binding"));
        }
        Ok(())
    }

    pub(super) fn inspect(
        binding: &Binding,
        allow_empty_journal: bool,
    ) -> Result<(Vec<Recommendation>, BTreeMap<String, CapturedLeaf>), StoreError> {
        let (records, leaves) = read_records(binding, allow_empty_journal)?;
        if allow_empty_journal {
            if let Some(directory) = &binding.records {
                match open(directory, c".rollup-transactions", true) {
                    Ok(journal) => {
                        if !names(&journal).map_err(StoreError::Io)?.is_empty()
                            || entry(directory, c".rollup-transactions").map_err(StoreError::Io)?
                                != Some(identity(&journal).map_err(StoreError::Io)?)
                        {
                            return Err(StoreError::Recovery(
                                "pending recommendation journal requires identity-bound recovery"
                                    .into(),
                            ));
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(e) => return Err(StoreError::Io(e)),
                }
            }
        }
        validate_graph_records(&records)?;
        Ok((records, leaves))
    }

    pub(super) fn read_records(
        binding: &Binding,
        allow_journal: bool,
    ) -> Result<(Vec<Recommendation>, BTreeMap<String, CapturedLeaf>), StoreError> {
        current(binding).map_err(StoreError::Io)?;
        let Some(directory) = &binding.records else {
            return Ok((Vec::new(), BTreeMap::new()));
        };
        let names = names(directory).map_err(StoreError::Io)?;
        let journal = names.iter().any(|n| n == c".rollup-transactions");
        if names.len() > ENTRIES + usize::from(allow_journal && journal) {
            return Err(StoreError::Io(invalid(
                "recommendations exceed entry bound",
            )));
        }
        let mut records = Vec::new();
        let mut leaves = BTreeMap::new();
        let mut total = 0usize;
        for name in names {
            if allow_journal && name == c".rollup-transactions" {
                continue;
            }
            if !name.to_bytes().ends_with(b".json") {
                return Err(StoreError::Recovery(
                    "recommendation preflight requires recovered canonical record entries".into(),
                ));
            }
            let file = open(directory, &name, false).map_err(StoreError::Io)?;
            let metadata = file.metadata().map_err(StoreError::Io)?;
            if !metadata.is_file() || metadata.nlink() != 1 {
                return Err(StoreError::Io(invalid(
                    "recommendation is not an unaliased regular file",
                )));
            }
            if metadata.len() > RECORD_BYTES as u64 {
                return Err(StoreError::Io(invalid(
                    "recommendation exceeds record byte bound",
                )));
            }
            let mut bytes = Vec::new();
            (&file)
                .take((RECORD_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(StoreError::Io)?;
            if bytes.len() > RECORD_BYTES {
                return Err(StoreError::Io(invalid(
                    "recommendation exceeds record byte bound",
                )));
            }
            total = total
                .checked_add(bytes.len())
                .filter(|n| *n <= TOTAL_BYTES)
                .ok_or_else(|| {
                    StoreError::Io(invalid("recommendations exceed aggregate byte bound"))
                })?;
            if entry(directory, &name).map_err(StoreError::Io)?
                != Some((metadata.dev(), metadata.ino()))
            {
                return Err(StoreError::Io(invalid(
                    "recommendation entry changed during preflight",
                )));
            }
            let record: Recommendation =
                crate::harness::definition::parse_strict_json(&bytes, RECORD_BYTES).map_err(
                    |_| StoreError::Io(invalid("invalid or ambiguous recommendation JSON")),
                )?;
            validate_recommendation(&record)?;
            if name.to_bytes() != recommendation_filename(&record.id).as_bytes() {
                return Err(StoreError::Io(invalid(
                    "recommendation filename does not match its identity",
                )));
            }
            leaves.insert(
                record.id.clone(),
                CapturedLeaf {
                    bytes,
                    identity: (metadata.dev(), metadata.ino()),
                },
            );
            records.push(record);
        }
        current(binding).map_err(StoreError::Io)?;
        Ok((records, leaves))
    }

    pub(super) fn names(directory: &fs::File) -> io::Result<Vec<CString>> {
        // A fresh open description avoids sharing directory offsets across inspections.
        let file = open(directory, c".", true)?;
        let fd = file.into_raw_fd();
        // SAFETY: newly owned directory FD; fdopendir takes ownership on success.
        let raw = unsafe { libc::fdopendir(fd) };
        if raw.is_null() {
            let e = io::Error::last_os_error();
            // SAFETY: fdopendir failed and did not take ownership of fd.
            unsafe { libc::close(fd) };
            return Err(e);
        }
        struct Stream(*mut libc::DIR);
        impl Drop for Stream {
            fn drop(&mut self) {
                // SAFETY: this stream uniquely owns a successful fdopendir result.
                unsafe { libc::closedir(self.0) };
            }
        }
        let stream = Stream(raw);
        let mut names = Vec::new();
        loop {
            // SAFETY: errno is thread-local; reset to distinguish EOF from error.
            #[cfg(target_os = "macos")]
            unsafe {
                *libc::__error() = 0
            };
            #[cfg(target_os = "linux")]
            unsafe {
                *libc::__errno_location() = 0
            };
            // SAFETY: stream remains live, and each name is copied before the next call.
            let raw = unsafe { libc::readdir(stream.0) };
            if raw.is_null() {
                let e = io::Error::last_os_error();
                if e.raw_os_error() != Some(0) {
                    return Err(e);
                }
                break;
            }
            // SAFETY: successful readdir provides a NUL-terminated name in this dirent.
            let name = unsafe { CStr::from_ptr((*raw).d_name.as_ptr()) };
            if name == c"." || name == c".." {
                continue;
            }
            if names.len() == MAX_DIRECTORY_ENTRIES {
                return Err(invalid("recommendations exceed entry bound"));
            }
            names.push(name.to_owned());
        }
        Ok(names)
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod protocol {
    use super::unix::{self, entry, identity, invalid, names, open};
    use super::*;

    const JOURNAL: &CStr = c".rollup-transactions";
    const MANIFEST: &CStr = c"manifest.json";
    const TEMP: &CStr = c"manifest.json.tmp";

    fn basename(text: &str) -> io::Result<CString> {
        if text.is_empty() || text == "." || text == ".." || text.contains('/') {
            return Err(invalid("invalid journal basename"));
        }
        CString::new(text).map_err(|_| invalid("invalid journal basename"))
    }

    fn check(parent: &fs::File, name: &CStr, file: &fs::File) -> io::Result<()> {
        if entry(parent, name)? != Some(identity(file)?) {
            return Err(invalid("journal entry identity changed"));
        }
        Ok(())
    }

    struct Leaf {
        name: CString,
        bytes: Vec<u8>,
        file: fs::File,
    }

    impl Leaf {
        fn verify(&self, parent: &fs::File) -> io::Result<()> {
            let current = read(parent, &self.name)?;
            if identity(&current.file)? != identity(&self.file)? || current.bytes != self.bytes {
                return Err(invalid("journal leaf changed"));
            }
            Ok(())
        }
    }

    fn read(parent: &fs::File, name: &CStr) -> io::Result<Leaf> {
        let file = open(parent, name, false)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > RECORD_BYTES as u64 {
            return Err(invalid("journal leaf is aliased, nonregular or oversized"));
        }
        let mut bytes = Vec::new();
        (&file)
            .take((RECORD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > RECORD_BYTES {
            return Err(invalid("journal leaf exceeds byte bound"));
        }
        check(parent, name, &file)?;
        Ok(Leaf {
            name: name.to_owned(),
            bytes,
            file,
        })
    }

    fn optional_read(parent: &fs::File, name: &CStr) -> io::Result<Option<Leaf>> {
        match read(parent, name) {
            Ok(leaf) => Ok(Some(leaf)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn mkdir(parent: &fs::File, name: &CStr) -> io::Result<fs::File> {
        // SAFETY: live parent descriptor and a validated single basename; private mode.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
            return Err(io::Error::last_os_error());
        }
        checkpoint()?;
        let file = open(parent, name, true)?;
        check(parent, name, &file)?;
        Ok(file)
    }

    fn write(parent: &fs::File, name: &CStr, bytes: &[u8]) -> io::Result<Leaf> {
        // SAFETY: live parent and validated basename; O_EXCL prevents overwriting an entry.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY
                    | libc::O_CREAT
                    | libc::O_EXCL
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful openat returned a new uniquely owned descriptor.
        let mut file = unsafe { fs::File::from_raw_fd(fd) };
        checkpoint()?;
        file.write_all(bytes)?;
        checkpoint()?;
        sync(&file)?;
        check(parent, name, &file)?;
        let leaf = read(parent, name)?;
        if identity(&leaf.file)? != identity(&file)? || leaf.bytes != bytes {
            return Err(invalid("journal write changed"));
        }
        Ok(leaf)
    }

    fn remove(parent: &fs::File, name: &CStr, file: &fs::File, directory: bool) -> io::Result<()> {
        check(parent, name, file)?;
        // SAFETY: live descriptors, validated basename; directories must be empty.
        if unsafe {
            libc::unlinkat(
                parent.as_raw_fd(),
                name.as_ptr(),
                if directory { libc::AT_REMOVEDIR } else { 0 },
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        checkpoint()
    }

    fn sync(file: &fs::File) -> io::Result<()> {
        file.sync_all()?;
        checkpoint()
    }

    fn checkpoint() -> io::Result<()> {
        #[cfg(test)]
        super::JOURNAL_FAILURE.with(|failure| match failure.get() {
            Some(0) => {
                failure.set(None);
                Err(io::Error::other("injected journal boundary failure"))
            }
            Some(n) => {
                failure.set(Some(n - 1));
                Ok(())
            }
            None => Ok(()),
        })?;
        Ok(())
    }

    fn graph(records: &[Recommendation]) -> io::Result<()> {
        validate_graph_records(records).map_err(|e| invalid(&e.to_string()))
    }

    fn records(
        binding: &Binding,
    ) -> io::Result<(Vec<Recommendation>, BTreeMap<String, CapturedLeaf>)> {
        unix::read_records(binding, true).map_err(|e| invalid(&e.to_string()))
    }

    fn current(binding: &Binding) -> io::Result<()> {
        unix::current(binding)?;
        let root_path = binding
            .store
            .dir
            .parent()
            .ok_or_else(|| invalid("missing metadata root"))?;
        // A renamed ancestor bridged by a symlink preserves inode identity but would
        // allow a new canonical path to acquire a different path-keyed operation owner.
        if fs::canonicalize(root_path)?.as_path() != root_path {
            return Err(invalid("canonical metadata root changed after binding"));
        }
        let root = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root_path)?;
        if identity(&root)? != identity(&binding.root)? {
            return Err(invalid("metadata root changed after binding"));
        }
        Ok(())
    }

    fn sync_records(binding: &Binding) -> io::Result<()> {
        current(binding)?;
        if let Some(dir) = &binding.records {
            sync(dir)?;
        }
        sync(&binding.root)
    }

    struct Transaction {
        parent: fs::File,
        name: CString,
        directory: fs::File,
        backups: Option<fs::File>,
        staged: Option<fs::File>,
    }

    impl Transaction {
        fn current(&self, binding: &Binding) -> io::Result<()> {
            current(binding)?;
            let dir = binding
                .records
                .as_ref()
                .ok_or_else(|| invalid("missing records directory"))?;
            check(dir, JOURNAL, &self.parent)?;
            check(&self.parent, &self.name, &self.directory)?;
            for (name, child) in [(c"backups", &self.backups), (c"staged", &self.staged)] {
                match child {
                    Some(file) => check(&self.directory, name, file)?,
                    None if entry(&self.directory, name)?.is_some() => {
                        return Err(invalid("journal directory appeared"))
                    }
                    None => (),
                }
            }
            Ok(())
        }

        fn sync(&self, binding: &Binding) -> io::Result<()> {
            self.current(binding)?;
            sync(&self.directory)?;
            sync(&self.parent)?;
            sync_records(binding)
        }
    }

    fn optional_directory(parent: &fs::File, name: &CStr) -> io::Result<Option<fs::File>> {
        match open(parent, name, true) {
            Ok(file) => {
                check(parent, name, &file)?;
                Ok(Some(file))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn manifest(
        tx: &Transaction,
        bytes: &[u8],
        previous: Option<&Leaf>,
        binding: &Binding,
    ) -> io::Result<Leaf> {
        tx.current(binding)?;
        let temp = write(&tx.directory, TEMP, bytes)?;
        tx.current(binding)?;
        temp.verify(&tx.directory)?;
        match previous {
            Some(leaf) => leaf.verify(&tx.directory)?,
            None if entry(&tx.directory, MANIFEST)?.is_some() => {
                return Err(invalid("manifest appeared"))
            }
            None => (),
        }
        // SAFETY: live pinned directory descriptor and fixed, single basenames.
        if unsafe {
            libc::renameat(
                tx.directory.as_raw_fd(),
                TEMP.as_ptr(),
                tx.directory.as_raw_fd(),
                MANIFEST.as_ptr(),
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        checkpoint()?;
        tx.sync(binding)?;
        read(&tx.directory, MANIFEST)
    }

    #[cfg(test)]
    pub(super) fn publish(removal: &PreparedRemoval) -> io::Result<()> {
        publish_kind(removal, Kind::Legacy)
    }

    pub(super) fn publish_sql(
        removal: &PreparedRemoval,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> io::Result<()> {
        publish_kind(removal, Kind::Sql(identity))
    }

    fn publish_kind(removal: &PreparedRemoval, kind: Kind<'_>) -> io::Result<()> {
        let binding = &removal.inventory._binding.0;
        current(binding)?;
        let (current_records, leaves) = records(binding)?;
        graph(&current_records)?;
        if leaves.len() != removal.inventory.leaves.len()
            || leaves.iter().any(|(id, leaf)| {
                !removal
                    .inventory
                    .leaves
                    .get(id)
                    .is_some_and(|old| old.identity == leaf.identity && old.bytes == leaf.bytes)
            })
        {
            return Err(invalid(
                "recommendation inventory changed before publication",
            ));
        }
        let existing_parent = match &binding.records {
            Some(dir) => optional_directory(dir, JOURNAL)?,
            None => None,
        };
        if let Some(parent) = &existing_parent {
            if !names(parent)?.is_empty() {
                return Err(invalid("recommendation recovery required"));
            }
        }
        if removal.entries.is_empty() {
            return sync_records(binding);
        }
        let dir = binding
            .records
            .as_ref()
            .ok_or_else(|| invalid("missing records directory"))?;
        current(binding)?;
        let parent = match existing_parent {
            Some(parent) => parent,
            None => mkdir(dir, JOURNAL)?,
        };
        let name = basename(&kind.name())?;
        let directory = mkdir(&parent, &name)?;
        if matches!(kind, Kind::Sql(_)) {
            // Establish the bound transaction name durably before any backup write.
            sync(&directory)?;
            sync(&parent)?;
            sync_records(binding)?;
        }
        let backups = Some(mkdir(&directory, c"backups")?);
        let staged = Some(mkdir(&directory, c"staged")?);
        let tx = Transaction {
            parent,
            name,
            directory,
            backups,
            staged,
        };
        for entry in &removal.entries {
            let leaf = removal
                .inventory
                .leaves
                .get(&entry.id)
                .expect("prepared removal retains each original leaf");
            tx.current(binding)?;
            write(
                tx.backups.as_ref().expect("new backups directory"),
                &basename(&entry.target)?,
                &leaf.bytes,
            )?;
        }
        sync(tx.backups.as_ref().expect("new backups directory"))?;
        sync(tx.staged.as_ref().expect("new staged directory"))?;
        tx.sync(binding)?;
        let uncommitted = manifest(&tx, removal.manifest_bytes(false), None, binding)?;
        manifest(
            &tx,
            removal.manifest_bytes(true),
            Some(&uncommitted),
            binding,
        )?;
        // Recovery performs complete content/graph validation before its first unlink.
        // The same held operation permit spans publication, recovery and the receipt.
        recover_expected(binding, Some(&removal.inventory.leaves), kind)
    }

    #[cfg(test)]
    pub(super) fn recover(binding: &Binding) -> io::Result<()> {
        recover_expected(binding, None, Kind::Legacy)
    }

    pub(super) fn recover_sql(
        binding: &Binding,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> io::Result<()> {
        recover_expected(binding, None, Kind::Sql(identity))
    }

    fn recover_expected(
        binding: &Binding,
        expected: Option<&BTreeMap<String, CapturedLeaf>>,
        kind: Kind<'_>,
    ) -> io::Result<()> {
        current(binding)?;
        let (live, leaves) = records(binding)?;
        if let Some(expected) = expected {
            if leaves.len() != expected.len()
                || leaves.iter().any(|(id, leaf)| {
                    !expected
                        .get(id)
                        .is_some_and(|old| old.identity == leaf.identity && old.bytes == leaf.bytes)
                })
            {
                return Err(invalid(
                    "captured inventory changed during journal publication",
                ));
            }
        }
        let Some(dir) = &binding.records else {
            return sync_records(binding);
        };
        let Some(parent) = optional_directory(dir, JOURNAL)? else {
            graph(&live)?;
            return sync_records(binding);
        };
        let pending = names(&parent)?;
        if pending.is_empty() {
            graph(&live)?;
            current(binding)?;
            remove(dir, JOURNAL, &parent, true)?;
            return sync_records(binding);
        }
        if pending.len() != 1 {
            return Err(invalid("multiple pending recommendation journals"));
        }
        let name = pending.into_iter().next().expect("one transaction");
        let text = name
            .to_str()
            .map_err(|_| invalid("invalid transaction name"))?;
        kind.check_name(text)?;
        let directory = open(&parent, &name, true)?;
        let mut tx = Transaction {
            backups: optional_directory(&directory, c"backups")?,
            staged: optional_directory(&directory, c"staged")?,
            parent,
            name,
            directory,
        };
        tx.current(binding)?;
        let entries = names(&tx.directory)?;
        if entries
            .iter()
            .any(|n| ![MANIFEST, TEMP, c"backups", c"staged"].contains(&n.as_c_str()))
        {
            return Err(invalid("unknown journal entry"));
        }
        if let Some(staged) = &tx.staged {
            if !names(staged)?.is_empty() {
                return Err(invalid("deletion journal contains staged replacements"));
            }
        }
        let manifest_leaf = optional_read(&tx.directory, MANIFEST)?;
        let temp = optional_read(&tx.directory, TEMP)?;
        let text = tx
            .name
            .to_str()
            .map_err(|_| invalid("invalid transaction name"))?;
        let parsed: Option<RollupTransactionManifest> = manifest_leaf
            .as_ref()
            .map(|leaf| kind.parse(&leaf.bytes, text))
            .transpose()?;
        let committed = parsed.as_ref().is_some_and(|m| m.committed);
        let selected: BTreeMap<&str, &RollupTransactionEntry> = parsed
            .as_ref()
            .map(|m| m.entries.iter().map(|e| (e.target.as_str(), e)).collect())
            .unwrap_or_default();
        let all_absent = committed && selected.values().all(|e| !leaves.contains_key(&e.id));
        let mut backups = BTreeMap::new();
        let mut total = 0usize;
        if let Some(directory) = &tx.backups {
            for name in names(directory)? {
                let leaf = read(directory, &name)?;
                total = total
                    .checked_add(leaf.bytes.len())
                    .filter(|n| *n <= TOTAL_BYTES)
                    .ok_or_else(|| invalid("backup aggregate bound exceeded"))?;
                let filename = name
                    .to_str()
                    .map_err(|_| invalid("invalid backup filename"))?;
                if committed {
                    let e = selected
                        .get(filename)
                        .ok_or_else(|| invalid("unknown backup"))?;
                    if e.old_sha256.as_deref() != Some(hash_bytes(&leaf.bytes).as_str()) {
                        return Err(invalid("backup hash mismatch"));
                    }
                    let record = parse_record(&leaf.bytes, &e.id)?;
                    backups.insert(e.id.clone(), (capture_backup(leaf)?, Some(record)));
                } else {
                    let matching = leaves
                        .iter()
                        .find(|(id, _)| recommendation_filename(id) == filename);
                    let Some((id, original)) = matching else {
                        return Err(invalid("draft backup has no original"));
                    };
                    if parsed.is_some() && !selected.contains_key(filename) {
                        return Err(invalid("unknown uncommitted backup"));
                    }
                    if !original.bytes.starts_with(&leaf.bytes) {
                        return Err(invalid("draft backup differs from original"));
                    }
                    backups.insert(id.clone(), (capture_backup(leaf)?, None));
                }
            }
        }
        if let Some(temp) = &temp {
            if committed {
                return Err(invalid(
                    "committed journal has an unexpected temporary manifest",
                ));
            }
            let next = if let Some(manifest) = &parsed {
                let mut next = manifest.clone();
                next.committed = true;
                next
            } else {
                // Before the first manifest rename, the complete backup set determines
                // the sole manifest this writer could have been staging.
                if backups.is_empty() {
                    return Err(invalid("temporary manifest has no completed backup"));
                }
                let mut entries = Vec::new();
                for (id, (backup, _)) in &backups {
                    if leaves[id].bytes != backup.bytes {
                        return Err(invalid("temporary manifest precedes completed backups"));
                    }
                    entries.push(RollupTransactionEntry {
                        id: id.clone(),
                        target: recommendation_filename(id),
                        staged: None,
                        backup: Some(transaction_backup_path(id)),
                        old_sha256: Some(hash_bytes(&backup.bytes)),
                        old_exists: true,
                        new_sha256: None,
                        new_exists: false,
                    });
                }
                RollupTransactionManifest {
                    version: 2,
                    committed: false,
                    entries,
                }
            };
            if parsed.is_none() {
                // A short first prefix cannot prove backup completeness. SQL's
                // selected job identifies the full expected graph before discard.
                let expected = next
                    .entries
                    .iter()
                    .map(|e| (e.target.as_str(), e))
                    .collect();
                kind.selection(&live, &expected, true)?;
            }
            let bytes = kind.encode(&next)?;
            if !bytes.starts_with(&temp.bytes) {
                return Err(invalid("temporary manifest differs from expected write"));
            }
        }
        // Validate the entire graph and every target before the first effect.
        if committed {
            let mut old: BTreeMap<String, Recommendation> =
                live.iter().map(|r| (r.id.clone(), r.clone())).collect();
            let complete_backups = selected.values().all(|e| backups.contains_key(&e.id));
            // Complete SQL evidence must still be validated when the targets are
            // already gone. Missing backups are allowed only after all targets are
            // absent, so interrupted backup cleanup remains resumable.
            if !all_absent || (matches!(kind, Kind::Sql(_)) && complete_backups) {
                for e in selected.values() {
                    let (_, record) = backups
                        .get(&e.id)
                        .ok_or_else(|| invalid("missing committed backup"))?;
                    if let Some(leaf) = leaves.get(&e.id) {
                        if e.old_sha256.as_deref() != Some(hash_bytes(&leaf.bytes).as_str()) {
                            return Err(invalid("target changed since journal capture"));
                        }
                    }
                    old.insert(
                        e.id.clone(),
                        record.as_ref().expect("parsed committed backup").clone(),
                    );
                }
                if old.len() > ENTRIES
                    || old
                        .values()
                        .map(|r| {
                            leaves
                                .get(&r.id)
                                .map(|l| l.bytes.len())
                                .unwrap_or_else(|| backups[&r.id].0.bytes.len())
                        })
                        .sum::<usize>()
                        > TOTAL_BYTES
                {
                    return Err(invalid("reconstructed graph exceeds bounds"));
                }
                let old = old.into_values().collect::<Vec<_>>();
                graph(&old)?;
                // An empty journal must prove there is no selected work. A
                // nonempty committed pass may precede additional independent refs.
                kind.selection(&old, &selected, selected.is_empty())?;
            }
            let remaining: Vec<_> = live
                .iter()
                .filter(|r| !selected.values().any(|e| e.id == r.id))
                .cloned()
                .collect();
            graph(&remaining)?;
        } else {
            graph(&live)?;
            if parsed.is_some() {
                kind.selection(&live, &selected, true)?;
            }
            for e in selected.values() {
                if !leaves
                    .get(&e.id)
                    .is_some_and(|l| e.old_sha256.as_deref() == Some(hash_bytes(&l.bytes).as_str()))
                {
                    return Err(invalid("uncommitted target changed"));
                }
            }
        }
        if committed {
            // Repeat durability barrier even after a prior crash following manifest rename.
            if let Some(directory) = &tx.backups {
                for (id, (captured, _)) in &backups {
                    let leaf = reopen_backup(directory, id, captured)?;
                    sync(&leaf.file)?;
                }
                sync(directory)?;
            }
            tx.sync(binding)?;
            for e in selected.values() {
                if let Some(captured) = leaves.get(&e.id) {
                    tx.current(binding)?;
                    if let Some(manifest) = &manifest_leaf {
                        manifest.verify(&tx.directory)?;
                    }
                    let leaf = read(dir, &basename(&e.target)?)?;
                    if identity(&leaf.file)? != captured.identity || leaf.bytes != captured.bytes {
                        return Err(invalid("target changed during recovery"));
                    }
                    leaf.verify(dir)?;
                    remove(dir, &leaf.name, &leaf.file, false)?;
                }
            }
        }
        sync_records(binding)?;
        graph(&records(binding)?.0)?;
        // The first draft temp needs the complete backup set for validation. Its
        // removal must be durable before any of that evidence can be discarded.
        if let Some(temp) = temp {
            tx.current(binding)?;
            temp.verify(&tx.directory)?;
            remove(&tx.directory, TEMP, &temp.file, false)?;
            sync(&tx.directory)?;
        }
        // Known leaf removals only. Manifest remains until all backup cleanup is durable.
        if let Some(directory) = &tx.backups {
            for (id, (captured, _)) in &backups {
                tx.current(binding)?;
                let leaf = reopen_backup(directory, id, captured)?;
                leaf.verify(directory)?;
                remove(directory, &leaf.name, &leaf.file, false)?;
            }
            sync(directory)?;
        }
        for name in [c"backups", c"staged"] {
            tx.current(binding)?;
            let child = if name == c"backups" {
                tx.backups.take()
            } else {
                tx.staged.take()
            };
            if let Some(file) = child {
                remove(&tx.directory, name, &file, true)?;
                sync(&tx.directory)?;
            }
        }
        if let Some(manifest) = manifest_leaf {
            tx.current(binding)?;
            manifest.verify(&tx.directory)?;
            remove(&tx.directory, MANIFEST, &manifest.file, false)?;
        }
        tx.sync(binding)?;
        remove(&tx.parent, &tx.name, &tx.directory, true)?;
        sync(&tx.parent)?;
        current(binding)?;
        remove(dir, JOURNAL, &tx.parent, true)?;
        sync_records(binding)
    }

    fn parse_record(bytes: &[u8], id: &str) -> io::Result<Recommendation> {
        let r: Recommendation = crate::harness::definition::parse_strict_json(bytes, RECORD_BYTES)
            .map_err(|_| invalid("invalid backup JSON"))?;
        validate_recommendation(&r).map_err(|e| invalid(&e.to_string()))?;
        if r.id != id {
            return Err(invalid("backup identity mismatch"));
        }
        Ok(r)
    }

    // Inventory bytes and observed identities are bounded; open descriptors are
    // constant in the number of records. Every later effect reopens one leaf and
    // checks both its current inode and bytes against the observation.
    fn capture_backup(leaf: Leaf) -> io::Result<CapturedLeaf> {
        Ok(CapturedLeaf {
            identity: identity(&leaf.file)?,
            bytes: leaf.bytes,
        })
    }

    fn reopen_backup(directory: &fs::File, id: &str, captured: &CapturedLeaf) -> io::Result<Leaf> {
        let leaf = read(directory, &basename(&recommendation_filename(id))?)?;
        if identity(&leaf.file)? != captured.identity || leaf.bytes != captured.bytes {
            return Err(invalid("backup changed during recovery"));
        }
        Ok(leaf)
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
thread_local! {
    pub(super) static JOURNAL_FAILURE: std::cell::Cell<Option<usize>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
#[path = "sql_journal_tests.rs"]
mod sql_tests;
