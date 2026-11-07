//! Blocking source-only v2 deletion protocol; never selected by production startup.
use super::*;

#[must_use]
pub(crate) struct JournalReceipt {
    result: Result<(), StoreError>,
    // Field order releases the operation before the workspace binding/lease.
    _operation: operation::Operation,
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
impl BoundCleanup {
    pub(crate) fn recover_removals(&self) -> Result<JournalReceipt, StoreError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let operation = self.0.store.try_operation()?;
            let result = protocol::recover(&self.0).map_err(StoreError::Io);
            Ok(JournalReceipt {
                result,
                _operation: operation,
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
