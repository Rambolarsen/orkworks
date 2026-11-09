//! Private retained scope, not root registration or input authority.
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
use crate::workspace_runtime::WorkspaceLease;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, OwnedSemaphorePermit, Semaphore};

#[derive(Clone, Copy)]
pub(crate) struct ScopeLimits {
    pub scopes: usize,
    pub bytes_per_scope: usize,
    pub backing_bytes: usize,
    pub lease_refs: usize,
    pub sender_refs: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeError {
    InvalidLimits,
    Capacity,
    TooLarge,
    Unavailable,
    Stale,
}

/// Fixed reservations include captures and rejected inputs still held by callers.
pub(crate) struct ScopeBudget {
    slots: Arc<Semaphore>,
    bytes: usize,
}
#[derive(Debug)]
pub(crate) struct ScopeReservation {
    _permit: OwnedSemaphorePermit,
    bytes: usize,
}
#[derive(Debug)]
pub(crate) struct CaptureRefusal {
    pub reason: ScopeError,
    pub reservation: ScopeReservation,
}

/// Immutable attachment. Never cloned into observers or public diagnostics.
pub(crate) struct OriginalInputScope {
    path: PathBuf,
    lease: Arc<WorkspaceLease>,
    session: String,
    identity: RuntimeIdentity,
    control: Sender<RuntimeCommand>,
    _reservation: ScopeReservation,
}
impl ScopeBudget {
    pub(crate) fn new(limits: ScopeLimits) -> Result<Self, ScopeError> {
        let total = limits
            .scopes
            .checked_mul(limits.bytes_per_scope)
            .ok_or(ScopeError::InvalidLimits)?;
        if limits.scopes == 0
            || limits.scopes > Semaphore::MAX_PERMITS
            || limits.bytes_per_scope < std::mem::size_of::<OriginalInputScope>()
            || total > limits.backing_bytes
            || limits.scopes > limits.lease_refs
            || limits.scopes > limits.sender_refs
        {
            return Err(ScopeError::InvalidLimits);
        }
        Ok(Self {
            slots: Arc::new(Semaphore::new(limits.scopes)),
            bytes: limits.bytes_per_scope,
        })
    }
    pub(crate) fn reserve(&self) -> Result<ScopeReservation, ScopeError> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| ScopeError::Capacity)?;
        Ok(ScopeReservation {
            _permit: permit,
            bytes: self.bytes,
        })
    }
}
impl ScopeReservation {
    pub(crate) fn refuse(self, reason: ScopeError) -> CaptureRefusal {
        CaptureRefusal {
            reason,
            reservation: self,
        }
    }
    pub(crate) fn capture(
        self,
        path: &Path,
        lease: &Arc<WorkspaceLease>,
        session: &str,
        identity: &RuntimeIdentity,
        control: &Sender<RuntimeCommand>,
    ) -> Result<OriginalInputScope, CaptureRefusal> {
        // Check minimum encoded storage before allocating; actual capacities after
        // each reserve remain authoritative. Allocator overhead is not qualified.
        let build = || {
            let fixed = std::mem::size_of::<OriginalInputScope>();
            let check = |path: usize, session: usize, runtime: usize| {
                fixed
                    .checked_add(path)
                    .and_then(|n| n.checked_add(session))
                    .and_then(|n| n.checked_add(runtime))
                    .filter(|n| *n <= self.bytes)
                    .map(|_| ())
                    .ok_or(ScopeError::TooLarge)
            };
            check(
                path.as_os_str().as_encoded_bytes().len(),
                session.len(),
                identity.runtime_instance_id.len(),
            )?;
            let mut owned_path = PathBuf::new();
            owned_path
                .try_reserve_exact(path.as_os_str().as_encoded_bytes().len())
                .map_err(|_| ScopeError::Unavailable)?;
            owned_path.push(path);
            check(
                owned_path.capacity(),
                session.len(),
                identity.runtime_instance_id.len(),
            )?;
            let mut owned_session = String::new();
            owned_session
                .try_reserve_exact(session.len())
                .map_err(|_| ScopeError::Unavailable)?;
            owned_session.push_str(session);
            check(
                owned_path.capacity(),
                owned_session.capacity(),
                identity.runtime_instance_id.len(),
            )?;
            let mut runtime = String::new();
            runtime
                .try_reserve_exact(identity.runtime_instance_id.len())
                .map_err(|_| ScopeError::Unavailable)?;
            runtime.push_str(&identity.runtime_instance_id);
            check(
                owned_path.capacity(),
                owned_session.capacity(),
                runtime.capacity(),
            )?;
            Ok((owned_path, owned_session, runtime))
        };
        let (path, session, runtime_instance_id) = match build() {
            Ok(parts) => parts,
            Err(reason) => return Err(self.refuse(reason)),
        };
        // Clone handles only after all fallible storage validation succeeds.
        Ok(OriginalInputScope {
            path,
            lease: lease.clone(),
            session,
            identity: RuntimeIdentity {
                runtime_instance_id,
                run_generation: identity.run_generation,
            },
            control: control.clone(),
            _reservation: self,
        })
    }
}
impl ScopeBudget {
    pub(super) fn owns(&self, scope: &OriginalInputScope) -> bool {
        Arc::ptr_eq(&self.slots, scope._reservation._permit.semaphore())
    }
}
impl OriginalInputScope {
    pub(super) fn same_binding(&self, other: &Self) -> bool {
        self.matches(
            &other.path,
            &other.lease,
            &other.session,
            &other.identity,
            &other.control,
        )
    }
    pub(crate) fn matches(
        &self,
        path: &Path,
        lease: &Arc<WorkspaceLease>,
        session: &str,
        identity: &RuntimeIdentity,
        control: &Sender<RuntimeCommand>,
    ) -> bool {
        self.path == path
            && Arc::ptr_eq(&self.lease, lease)
            && self.session == session
            && self.identity == *identity
            && self.control.same_channel(control)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
    use crate::workspace_runtime::WorkspaceLease;
    use std::path::Path;
    use std::sync::Arc;

    pub(crate) fn budget() -> ScopeBudget {
        ScopeBudget::new(ScopeLimits {
            scopes: 1,
            bytes_per_scope: 4096,
            backing_bytes: 4096,
            lease_refs: 1,
            sender_refs: 1,
        })
        .unwrap()
    }
    pub(crate) fn capture(root: &Path) -> OriginalInputScope {
        let lease = Arc::new(WorkspaceLease::acquire(root).unwrap());
        let (tx, _rx) = tokio::sync::mpsc::channel::<RuntimeCommand>(1);
        budget()
            .reserve()
            .unwrap()
            .capture(
                root,
                &lease,
                "session",
                &RuntimeIdentity {
                    runtime_instance_id: "runtime".into(),
                    run_generation: 1,
                },
                &tx,
            )
            .unwrap_or_else(|e| panic!("{:?}", e.reason))
    }

    #[test]
    fn finite_scope_reservations_cover_rejected_and_detached_inputs() {
        let budget = budget();
        let reservation = budget.reserve().unwrap();
        assert!(matches!(budget.reserve(), Err(ScopeError::Capacity)));
        let root = tempfile::tempdir().unwrap();
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let identity = RuntimeIdentity {
            runtime_instance_id: "runtime".into(),
            run_generation: 1,
        };
        let refusal =
            match reservation.capture(root.path(), &lease, &"x".repeat(8192), &identity, &tx) {
                Err(e) => e,
                Ok(_) => panic!("oversized scope captured"),
            };
        assert_eq!(refusal.reason, ScopeError::TooLarge);
        assert!(matches!(budget.reserve(), Err(ScopeError::Capacity)));
        drop(refusal.reservation);
        let scope = budget
            .reserve()
            .unwrap()
            .capture(root.path(), &lease, "session", &identity, &tx)
            .unwrap_or_else(|e| panic!("{:?}", e.reason));
        assert!(scope.matches(root.path(), &lease, "session", &identity, &tx));
        let (other_tx, _other_rx) = tokio::sync::mpsc::channel(1);
        assert!(!scope.matches(root.path(), &lease, "session", &identity, &other_tx));
        assert!(!scope.matches(root.path(), &lease, "other", &identity, &tx));
        let replacement = RuntimeIdentity {
            run_generation: 2,
            ..identity.clone()
        };
        assert!(!scope.matches(root.path(), &lease, "session", &replacement, &tx));
        drop(lease);
        assert!(WorkspaceLease::acquire(root.path()).is_err());
        assert!(matches!(budget.reserve(), Err(ScopeError::Capacity)));
        drop(scope);
        assert!(WorkspaceLease::acquire(root.path()).is_ok());
        assert!(budget.reserve().is_ok());
    }

    #[test]
    fn scope_limits_reject_overflow_and_unreserved_references() {
        for limits in [
            ScopeLimits {
                scopes: 0,
                bytes_per_scope: 1,
                backing_bytes: 1,
                lease_refs: 1,
                sender_refs: 1,
            },
            ScopeLimits {
                scopes: 2,
                bytes_per_scope: usize::MAX,
                backing_bytes: usize::MAX,
                lease_refs: 2,
                sender_refs: 2,
            },
            ScopeLimits {
                scopes: 2,
                bytes_per_scope: 4096,
                backing_bytes: 8192,
                lease_refs: 1,
                sender_refs: 2,
            },
            ScopeLimits {
                scopes: 1,
                bytes_per_scope: 4096,
                backing_bytes: 4095,
                lease_refs: 1,
                sender_refs: 1,
            },
            ScopeLimits {
                scopes: 1,
                bytes_per_scope: 4096,
                backing_bytes: 4096,
                lease_refs: 1,
                sender_refs: 0,
            },
        ] {
            assert!(matches!(
                ScopeBudget::new(limits),
                Err(ScopeError::InvalidLimits)
            ));
        }
    }
    #[cfg(unix)]
    #[test]
    fn scope_retains_non_utf8_path_without_lossy_identity_or_accounting() {
        use std::os::unix::ffi::OsStringExt;
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join(std::ffi::OsString::from_vec(vec![b'r', 0xff]));
        // Some Unix filesystems reject this filename. The primitive stores
        // supplied encoded intent; only the application adapter validates its
        // source target. Keep actual lease I/O on a valid directory.
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let identity = RuntimeIdentity {
            runtime_instance_id: "runtime".into(),
            run_generation: 1,
        };
        let scope = budget()
            .reserve()
            .unwrap()
            .capture(&path, &lease, "session", &identity, &tx)
            .unwrap_or_else(|e| panic!("{:?}", e.reason));
        assert_eq!(
            scope.path.as_os_str().as_encoded_bytes(),
            path.as_os_str().as_encoded_bytes()
        );
        assert!(scope.matches(&path, &lease, "session", &identity, &tx));
        drop(lease);
        assert!(WorkspaceLease::acquire(root.path()).is_err());
        drop(scope);
        assert!(WorkspaceLease::acquire(root.path()).is_ok());
    }

    #[test]
    fn exact_scope_byte_allowance_admits_and_one_less_refuses_without_handle_clones() {
        let root = tempfile::tempdir().unwrap();
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let identity = RuntimeIdentity {
            runtime_instance_id: "run".into(),
            run_generation: 1,
        };
        // Encoded path bytes + literal session/runtime bytes, plus inline representation.
        let bytes = std::mem::size_of::<OriginalInputScope>()
            + root.path().as_os_str().as_encoded_bytes().len()
            + 1
            + 3;
        for (cap, accepted) in [(bytes, true), (bytes - 1, false)] {
            let budget = ScopeBudget::new(ScopeLimits {
                scopes: 1,
                bytes_per_scope: cap,
                backing_bytes: cap,
                lease_refs: 1,
                sender_refs: 1,
            })
            .unwrap();
            let before = Arc::strong_count(&lease);
            match budget
                .reserve()
                .unwrap()
                .capture(root.path(), &lease, "s", &identity, &tx)
            {
                Ok(scope) => {
                    assert!(accepted);
                    assert_eq!(Arc::strong_count(&lease), before + 1);
                    drop(scope);
                }
                Err(refusal) => {
                    assert!(!accepted);
                    assert_eq!(refusal.reason, ScopeError::TooLarge);
                }
            }
            assert_eq!(Arc::strong_count(&lease), before);
            assert!(budget.reserve().is_ok());
        }
    }
}
