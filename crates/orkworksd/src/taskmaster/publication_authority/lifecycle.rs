//! Close-only facade. Production has no handle factory or attachment operation.
use super::{Authority, Binding, Scope};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct RuntimeRevocation {
    owner: Arc<Authority>,
    binding: Binding,
    #[cfg(test)]
    attempt: Arc<std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>>,
}
impl RuntimeRevocation {
    pub(crate) fn close(&self) {
        #[cfg(test)]
        testing::observe(&self.attempt);
        let _ = self.owner.close_binding(&self.binding);
    }
}
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceRevocation {
    owner: Arc<Authority>,
    scope: Scope,
    #[cfg(test)]
    attempt: Arc<std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>>,
}
impl WorkspaceRevocation {
    pub(crate) fn close(&self) {
        #[cfg(test)]
        testing::observe(&self.attempt);
        let _ = self.owner.close_scope(&self.scope);
    }
}
#[derive(Debug)]
pub(crate) struct TaskRevocationGuard(pub(crate) Option<RuntimeRevocation>);
impl TaskRevocationGuard {
    pub(crate) fn close(&self) {
        if let Some(handle) = &self.0 {
            handle.close();
        }
    }
}
impl Drop for TaskRevocationGuard {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::super::*;
    use super::*;
    pub(super) fn observe(attempt: &std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>) {
        // Test scheduling receipt happens BEFORE the authority latch, never inside it.
        if let Some(tx) = attempt.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
    pub(crate) struct PendingClaim {
        owner: Arc<Authority>,
        claim: ClaimedEvaluation,
    }
    impl PendingClaim {
        pub(crate) fn settle(self) {
            self.owner
                .settle(&self.claim, Disposition::Committed)
                .unwrap();
        }
    }
    pub(crate) struct Fixture {
        pub(crate) runtime: RuntimeRevocation,
        pub(crate) workspace: WorkspaceRevocation,
    }
    impl Fixture {
        pub(crate) fn new() -> Self {
            let f = Self::unissued();
            f.runtime
                .owner
                .register_source_binding(&f.runtime.binding)
                .unwrap();
            f
        }
        pub(crate) fn unissued() -> Self {
            let owner = Arc::new(Authority::from_generation([37; 32]));
            let node = Arc::new(ScopeState {
                owner: Arc::downgrade(&owner.inner),
                closed: AtomicBool::new(false),
            });
            let scope = Scope {
                node: node.clone(),
                workspace: node,
                parents: [None, None],
            };
            owner.select_workspace(&scope).unwrap();
            let binding = Binding {
                data: Arc::new(BindingData {
                    scope: scope.clone(),
                    phase: AtomicU8::new(UNISSUED),
                    source: Arc::from(b"synthetic lifecycle source".as_slice()),
                }),
            };
            let attempt = Arc::new(std::sync::Mutex::new(None));
            Self {
                runtime: RuntimeRevocation {
                    owner: owner.clone(),
                    binding,
                    attempt: attempt.clone(),
                },
                workspace: WorkspaceRevocation {
                    owner,
                    scope,
                    attempt,
                },
            }
        }
        pub(crate) fn observe_close(&self) -> std::sync::mpsc::Receiver<()> {
            let (tx, rx) = std::sync::mpsc::channel();
            *self.runtime.attempt.lock().unwrap() = Some(tx);
            rx
        }
        pub(crate) fn hold_latch(&self) -> impl Drop + '_ {
            self.runtime.owner.inner.state.lock().unwrap()
        }
        pub(crate) fn try_register(&self) -> bool {
            self.runtime
                .owner
                .register_source_binding(&self.runtime.binding)
                .is_ok()
        }
        pub(crate) fn claim(&self) -> PendingClaim {
            let claim = self
                .runtime
                .owner
                .try_claim(ReadyEvaluation {
                    binding: self.runtime.binding.clone(),
                    intent: Arc::from(b"exact intent".as_slice()),
                })
                .unwrap();
            PendingClaim {
                owner: self.runtime.owner.clone(),
                claim,
            }
        }
        pub(crate) fn can_respond(&self) -> bool {
            self.runtime
                .owner
                .commit_response(ReadyResponse {
                    binding: self.runtime.binding.clone(),
                    bytes: Arc::from(b"bounded response".as_slice()),
                })
                .is_ok()
        }
        pub(crate) fn poison(&self) {
            let owner = self.runtime.owner.clone();
            let _ = std::thread::spawn(move || {
                let _guard = owner.inner.state.lock().unwrap();
                panic!("injected authority poison")
            })
            .join();
        }
    }
}
