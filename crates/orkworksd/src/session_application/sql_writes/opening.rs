//! Private SQL recommendation opening; production format routing stays gated.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;
use crate::taskmaster::store::{RecommendationStore, StoreError as RecommendationError};
use crate::workspace_runtime::WorkspaceIdentity;

/// Minted only after this opening's SQL/root/lease and application checks.
/// Deliberately not Clone/Default/Deserialize or a cleanup-admission proof.
pub(crate) struct RecommendationOpenAdmission {
    _private: (),
}

/// An owned candidate, not adoption into AppState or production format selection.
/// A future qualified startup router must supply its own adoption boundary.
pub(crate) struct SqlRecommendationWorkspace {
    pub(super) store: Arc<SessionStore>,
    pub(super) recommendations: RecommendationStore,
    pub(super) lease: Arc<WorkspaceLease>,
    _identity: Arc<WorkspaceIdentity>,
}

struct Before {
    workspace: RecommendationDeliveryWorkspace,
    reader: Arc<metadata::session_reader::SessionReader>,
    recommendations: RecommendationStore,
    root: PathBuf,
    lease: Option<Arc<WorkspaceLease>>,
}
impl Before {
    fn capture(ws: &WorkspaceState) -> Self {
        Self {
            workspace: RecommendationDeliveryWorkspace::capture(ws),
            reader: ws.metadata.session_reader(),
            recommendations: ws.recommendation_store.clone(),
            root: ws.metadata.root_path(),
            lease: ws.lease.clone(),
        }
    }
    fn matches(&self, ws: &WorkspaceState) -> bool {
        self.workspace.matches(ws)
            && Arc::ptr_eq(&self.reader, &ws.metadata.session_reader())
            && self.recommendations.same_owner(&ws.recommendation_store)
    }
}
#[cfg(test)]
#[derive(Default)]
struct Gates {
    before_admit: ReplayGate,
    admitted: ReplayGate,
}

impl SqlRecommendationWorkspace {
    pub(crate) async fn open(
        state: Arc<AppState>,
        root: PathBuf,
        workspace: String,
        lease: Arc<WorkspaceLease>,
    ) -> Result<Self, StoreError> {
        Self::open_inner(
            state,
            root,
            workspace,
            lease,
            #[cfg(test)]
            None,
        )
        .await
    }
    async fn open_inner(
        state: Arc<AppState>,
        root: PathBuf,
        workspace: String,
        lease: Arc<WorkspaceLease>,
        #[cfg(test)] gates: Option<Arc<Gates>>,
    ) -> Result<Self, StoreError> {
        let before = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .map(Before::capture);
        let (sent, received) = oneshot::channel();
        // This task, not its waiter, owns the lease and in-progress worker.
        tokio::spawn(async move {
            let bound_lease = lease.clone();
            let expected = before
                .as_ref()
                .map(|b| (b.root.clone(), b.workspace.path.clone(), b.lease.clone()));
            let selected_workspace = workspace.clone();
            let binding = tokio::task::spawn_blocking(move || {
                let identity = WorkspaceIdentity::resolve(&root).map_err(StoreError::Io)?;
                if let Some((expected_root, expected_workspace, expected_lease)) = expected {
                    if expected_workspace.to_string_lossy() != selected_workspace
                        || !expected_lease
                            .as_ref()
                            .is_some_and(|lease| Arc::ptr_eq(lease, &bound_lease))
                        || WorkspaceIdentity::resolve(&expected_root).map_err(StoreError::Io)?
                            != identity
                    {
                        return Err(StoreError::Conflict);
                    }
                }
                let canonical = identity.canonical_path().to_path_buf();
                let binding =
                    RecommendationStore::bind_cleanup_recovery(canonical.clone(), bound_lease)
                        .map_err(opening_error)?;
                Ok::<_, StoreError>((Arc::new(identity), canonical, binding))
            })
            .await
            .map_err(|_| StoreError::Unavailable)
            .and_then(|r| r);
            let (identity, root, binding) = match binding {
                Ok(binding) => binding,
                Err(error) => {
                    let _ = sent.send(Err(error));
                    return;
                }
            };
            if sent.is_closed() {
                return;
            }
            let store =
                match SessionStore::open_leased(root, workspace.clone(), lease.clone()).await {
                    Ok(store) => Arc::new(store),
                    Err(error) => {
                        let _ = sent.send(Err(error));
                        return;
                    }
                };
            let result = async {
                let selected = store.opening_identity().await?;
                #[cfg(test)]
                if let Some(gates) = &gates {
                    let gates = gates.clone();
                    tokio::task::spawn_blocking(move || enter_replay_gate(&gates.before_admit))
                        .await
                        .map_err(|_| StoreError::Unavailable)?;
                }
                if sent.is_closed() {
                    return Err(StoreError::Cancelled);
                }
                if store.opening_identity().await? != selected {
                    return Err(StoreError::Conflict);
                }
                let checking_identity = identity.clone();
                let candidate = tokio::task::spawn_blocking(move || {
                    checking_identity.revalidate().map_err(StoreError::Io)?;
                    binding.claim_sql_open().map_err(opening_error)
                })
                .await
                .map_err(|_| StoreError::Unavailable)??;
                let recommendations = {
                    let _projection = state.projection_lock.lock().unwrap();
                    let current = state.workspace.lock().unwrap();
                    let unchanged = match (&before, current.as_ref()) {
                        (None, None) => true,
                        (Some(before), Some(ws)) => before.matches(ws),
                        _ => false,
                    };
                    if !unchanged
                        || sent.is_closed()
                        || !recommendation_deliveries_in_flight_for_workspace(Path::new(&workspace))
                            .is_empty()
                    {
                        return Err(StoreError::Conflict);
                    }
                    candidate.admit_open(RecommendationOpenAdmission { _private: () })
                };
                #[cfg(test)]
                if let Some(gates) = &gates {
                    let gates = gates.clone();
                    tokio::task::spawn_blocking(move || enter_replay_gate(&gates.admitted))
                        .await
                        .map_err(|_| StoreError::Unavailable)?;
                }
                Ok(Self {
                    store: store.clone(),
                    recommendations,
                    lease,
                    _identity: identity,
                })
            }
            .await;
            match result {
                Ok(opened) => {
                    if let Err(Ok(opened)) = sent.send(Ok(opened)) {
                        let _ = opened.store.close().await;
                    }
                }
                Err(error) => {
                    let _ = store.close().await;
                    let _ = sent.send(Err(error));
                }
            }
        });
        received.await.map_err(|_| StoreError::Unavailable)?
    }
}
fn opening_error(error: RecommendationError) -> StoreError {
    match error {
        RecommendationError::Busy => StoreError::Busy,
        RecommendationError::Io(error) => StoreError::Io(error),
        _ => StoreError::Unavailable,
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests;
