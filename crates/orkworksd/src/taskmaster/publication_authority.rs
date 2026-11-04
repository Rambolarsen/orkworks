//! Dormant in-memory evaluation authority mechanics; no source admission or I/O.
//! Binding/readiness factories are test-only. No runtime or storage caller.
#![allow(dead_code)]
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex, MutexGuard, Weak,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Error {
    Closed,
    Unissued,
    Foreign,
    Busy,
    Conflict,
    Unavailable,
    Exhausted,
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct ClaimIdentity {
    sidecar_generation: [u8; 32],
    sequence: u64,
}
#[derive(Debug)]
struct Authority {
    inner: Arc<Inner>,
}
#[derive(Debug)]
struct Inner {
    generation: [u8; 32],
    state: Mutex<State>,
}
#[derive(Debug)]
struct State {
    closed: bool,
    last_sequence: u64,
    workspace: Option<Arc<ScopeState>>,
    pending: Option<Pending>,
}
#[derive(Debug)]
struct ScopeState {
    owner: Weak<Inner>,
    closed: AtomicBool,
}
#[derive(Clone, Debug)]
struct Scope {
    node: Arc<ScopeState>,
    workspace: Arc<ScopeState>,
    parents: [Option<Arc<ScopeState>>; 2],
}
#[derive(Clone, Debug)]
struct Binding {
    data: Arc<BindingData>,
}
#[derive(Debug)]
struct BindingData {
    scope: Scope,
    phase: AtomicU8,
    source: Arc<[u8]>,
}
#[derive(Debug)]
struct ReadyEvaluation {
    binding: Binding,
    intent: Arc<[u8]>,
}
#[derive(Debug)]
struct ClaimedEvaluation {
    // The worker claim retains its owner even after the request facade is dropped.
    owner: Arc<Inner>,
    identity: ClaimIdentity,
    binding: Binding,
    intent: Arc<[u8]>,
}
#[derive(Debug)]
struct Pending {
    identity: ClaimIdentity,
    binding: Binding,
    intent: Arc<[u8]>,
    uncertain: bool,
}
#[derive(Clone, Copy)]
enum Disposition {
    Committed,
    RolledBack,
    Unknown,
}
#[derive(Debug)]
struct ReadyResponse {
    binding: Binding,
    bytes: Arc<[u8]>,
}
#[derive(Debug)]
struct CommittedResponse {
    ready: ReadyResponse,
}
impl CommittedResponse {
    fn bytes(&self) -> &[u8] {
        &self.ready.bytes
    }
}
impl ClaimedEvaluation {
    fn identity(&self) -> ClaimIdentity {
        self.identity
    }
    fn intent(&self) -> &[u8] {
        &self.intent
    }
}
const UNISSUED: u8 = 0;
const OPEN: u8 = 1;
const CLOSED: u8 = 2;

impl Authority {
    fn from_generation(generation: [u8; 32]) -> Self {
        Self {
            inner: Arc::new(Inner {
                generation,
                state: Mutex::new(State {
                    closed: false,
                    last_sequence: 0,
                    workspace: None,
                    pending: None,
                }),
            }),
        }
    }
    /// OS entropy is obtained before constructing or locking the owner.
    fn new() -> Result<Self> {
        let mut generation = [0; 32];
        let entropy = getrandom::fill(&mut generation)
            .map(|()| generation)
            .map_err(|_| ());
        Self::from_entropy(entropy)
    }
    fn from_entropy(entropy: std::result::Result<[u8; 32], ()>) -> Result<Self> {
        entropy
            .map(Self::from_generation)
            .map_err(|()| Error::Unavailable)
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>> {
        // Poison is permanently unavailable; never recover/reset authority.
        self.inner.state.lock().map_err(|_| Error::Unavailable)
    }
    fn owns(&self, owner: &Weak<Inner>) -> bool {
        std::ptr::eq(owner.as_ptr(), Arc::as_ptr(&self.inner))
    }
    fn owns_scope(&self, scope: &Scope) -> Result<()> {
        if !self.owns(&scope.node.owner)
            || !self.owns(&scope.workspace.owner)
            || scope.parents.iter().flatten().any(|p| !self.owns(&p.owner))
        {
            return Err(Error::Foreign);
        }
        Ok(())
    }
    fn check_scope(&self, state: &State, scope: &Scope) -> Result<()> {
        self.owns_scope(scope)?;
        // All phase/close atomic accesses are protected by this owner's mutex.
        // Relaxed flags provide shared storage, not a separate lock-free protocol.
        if state.closed
            || scope.node.closed.load(Ordering::Relaxed)
            || scope.workspace.closed.load(Ordering::Relaxed)
            || scope
                .parents
                .iter()
                .flatten()
                .any(|p| p.closed.load(Ordering::Relaxed))
            || !state
                .workspace
                .as_ref()
                .is_some_and(|w| Arc::ptr_eq(w, &scope.workspace))
        {
            return Err(Error::Closed);
        }
        Ok(())
    }
    fn check_binding(&self, state: &State, binding: &Binding) -> Result<()> {
        self.check_scope(state, &binding.data.scope)?;
        match binding.data.phase.load(Ordering::Relaxed) {
            OPEN => Ok(()),
            UNISSUED => Err(Error::Unissued),
            CLOSED => Err(Error::Closed),
            _ => Err(Error::Unavailable),
        }
    }
    fn register_source_binding(&self, binding: &Binding) -> Result<()> {
        let state = self.lock()?;
        self.check_scope(&state, &binding.data.scope)?;
        match binding.data.phase.load(Ordering::Relaxed) {
            UNISSUED => {
                binding.data.phase.store(OPEN, Ordering::Relaxed);
                Ok(())
            }
            OPEN => Err(Error::Conflict),
            CLOSED => Err(Error::Closed),
            _ => Err(Error::Unavailable),
        }
    }
    fn close_binding(&self, binding: &Binding) -> Result<()> {
        let _state = self.lock()?;
        self.owns_scope(&binding.data.scope)?;
        match binding.data.phase.load(Ordering::Relaxed) {
            UNISSUED | OPEN | CLOSED => {
                binding.data.phase.store(CLOSED, Ordering::Relaxed);
                Ok(())
            }
            _ => Err(Error::Unavailable),
        }
    }
    fn close_scope(&self, scope: &Scope) -> Result<()> {
        let _state = self.lock()?;
        self.owns_scope(scope)?;
        scope.node.closed.store(true, Ordering::Relaxed);
        Ok(())
    }
    fn close(&self) -> Result<()> {
        self.lock()?.closed = true;
        Ok(())
    }
    fn select_workspace(&self, scope: &Scope) -> Result<()> {
        let retired = {
            let mut state = self.lock()?;
            self.owns_scope(scope)?;
            if !Arc::ptr_eq(&scope.node, &scope.workspace)
                || scope.parents.iter().any(Option::is_some)
            {
                return Err(Error::Conflict);
            }
            if state.closed || scope.node.closed.load(Ordering::Relaxed) {
                return Err(Error::Closed);
            }
            if state
                .workspace
                .as_ref()
                .is_some_and(|w| Arc::ptr_eq(w, &scope.node))
            {
                return Ok(());
            }
            let old = state.workspace.replace(scope.node.clone());
            if let Some(old) = &old {
                old.closed.store(true, Ordering::Relaxed);
            }
            old
        };
        // In particular, last-reference destruction is never under the latch.
        drop(retired);
        Ok(())
    }
    fn try_claim(&self, ready: ReadyEvaluation) -> Result<ClaimedEvaluation> {
        let mut state = self.lock()?;
        self.check_binding(&state, &ready.binding)?;
        if state.pending.is_some() {
            return Err(Error::Busy);
        }
        let sequence = state.last_sequence.checked_add(1).ok_or(Error::Exhausted)?;
        let identity = ClaimIdentity {
            sidecar_generation: self.inner.generation,
            sequence,
        };
        // Only Arc references and scalar values are installed; buffers were prepared outside.
        state.pending = Some(Pending {
            identity,
            binding: ready.binding.clone(),
            intent: ready.intent.clone(),
            uncertain: false,
        });
        state.last_sequence = sequence;
        Ok(ClaimedEvaluation {
            owner: self.inner.clone(),
            identity,
            binding: ready.binding,
            intent: ready.intent,
        })
    }
    fn settle(&self, claim: &ClaimedEvaluation, outcome: Disposition) -> Result<()> {
        let retired = {
            let mut state = self.lock()?;
            if !Arc::ptr_eq(&self.inner, &claim.owner) {
                return Err(Error::Foreign);
            }
            let pending = state.pending.as_mut().ok_or(Error::Conflict)?;
            if pending.identity != claim.identity
                || !Arc::ptr_eq(&pending.binding.data, &claim.binding.data)
                || !Arc::ptr_eq(&pending.intent, &claim.intent)
            {
                return Err(Error::Conflict);
            }
            match outcome {
                Disposition::Unknown => {
                    pending.uncertain = true;
                    None
                }
                Disposition::Committed | Disposition::RolledBack => state.pending.take(),
            }
        };
        // Claim drop is deliberately inert; only this exact definitive settlement retires.
        drop(retired);
        Ok(())
    }
    fn commit_response(&self, response: ReadyResponse) -> Result<CommittedResponse> {
        let state = self.lock()?;
        self.check_binding(&state, &response.binding)?;
        if state.pending.as_ref().is_some_and(|p| {
            p.uncertain
                || !Arc::ptr_eq(
                    &p.binding.data.scope.workspace,
                    &response.binding.data.scope.workspace,
                )
        }) {
            return Err(Error::Unavailable);
        }
        // Keep the whole prepared response alive; no buffer/context is destroyed under guard.
        Ok(CommittedResponse { ready: response })
    }
}
#[cfg(test)]
mod tests;
