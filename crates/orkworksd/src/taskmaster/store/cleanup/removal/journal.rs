//! Blocking source-only v2 deletion protocol; never selected by production startup.
use super::*;
mod sql_codec;

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
impl PreparedRemoval {
    pub(crate) fn publish(self) -> JournalReceipt {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let result = protocol::publish(&self).map_err(StoreError::Io);
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let result = Err(unsupported());
        JournalReceipt {
            result,
            _operation: self.inventory._operation,
            _binding: self.inventory._binding,
        }
    }
}
/// Unadmitted memory-only claim. Dropping it does not change owner mode.
pub(crate) struct SqlCleanupClaim {
    operation: Arc<operation::Operation>,
    binding: BoundCleanup,
}
/// Admitted SQL work; not constructible from an identity or journal bytes.
pub(crate) struct SqlCleanup(SqlCleanupClaim);
impl SqlCleanupClaim {
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
            let mut removal = inventory.prepare_session_removal(identity.session_id())?;
            removal.uncommitted = sql_codec::encode(identity, &removal.entries, false)?;
            removal.committed = sql_codec::encode(identity, &removal.entries, true)?;
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
impl BoundCleanup {
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
    // Trusted store-layer fixture only; production must hand off checked admission.
    #[cfg(test)]
    fn remove_for_sql(
        &self,
        identity: &crate::session_store::SqlDeletionIdentity,
    ) -> Result<JournalReceipt, StoreError> {
        let claim = self.claim_sql_cleanup()?;
        let admitted = claim
            .admit(crate::session_application::sql_writes::trusted_cleanup_admission_fixture());
        Ok(admitted.remove(identity))
    }
    pub(crate) fn recover_removals(&self) -> Result<JournalReceipt, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let operation = self.0.store.try_operation()?;
            let result = protocol::recover(&self.0).map_err(StoreError::Io);
            Ok(JournalReceipt {
                result,
                _operation: Arc::new(operation),
                _binding: self.clone(),
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        Err(unsupported())
    }
}
impl RecommendationStore {
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
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported() -> StoreError {
    StoreError::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "recommendation deletion journal is not qualified on this platform",
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod protocol;

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod sql_tests;
