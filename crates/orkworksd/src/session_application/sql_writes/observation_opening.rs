//! Private checked SQL observation opening; no AppState adoption or legacy recovery.
#![cfg_attr(not(test), allow(dead_code))]
use super::*;
use crate::workflow_observations::{
    sql_origin::{ScanRequest, Snapshot},
    FileClaim, SqlCandidate, SqlOwner,
};
use crate::workspace_runtime::WorkspaceIdentity;
use sha2::{Digest, Sha256};
struct Before {
    workspace: RecommendationDeliveryWorkspace,
    reader: Arc<metadata::session_reader::SessionReader>,
    workflow: crate::workflow_observations::WorkflowObservationStore,
    root: PathBuf,
    lease: Option<Arc<WorkspaceLease>>,
}
impl Before {
    fn capture(ws: &WorkspaceState) -> Self {
        Self {
            workspace: RecommendationDeliveryWorkspace::capture(ws),
            reader: ws.metadata.session_reader(),
            workflow: ws.workflow_observations.clone(),
            root: ws.metadata.root_path(),
            lease: ws.lease.clone(),
        }
    }
    fn matches(&self, ws: &WorkspaceState) -> bool {
        self.workspace.matches(ws)
            && Arc::ptr_eq(&self.reader, &ws.metadata.session_reader())
            && self.workflow.instance_id() == ws.workflow_observations.instance_id()
    }
}
/// Data owner only; it grants no origin capture, producer write, deletion or application adoption.
#[must_use]
pub(crate) struct SqlObservationWorkspace {
    snapshot: Snapshot,
    store: Arc<SessionStore>,
    _owner: SqlOwner,
    _lease: Arc<WorkspaceLease>,
    _root: Arc<WorkspaceIdentity>,
}
struct Cancel(Arc<FileClaim>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
impl SqlObservationWorkspace {
    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub(crate) async fn close(self) -> Result<(), StoreError> {
        self.store.close().await
    }
    pub(crate) async fn open(
        state: Arc<AppState>,
        root: PathBuf,
        workspace: String,
        lease: Arc<WorkspaceLease>,
    ) -> Result<Self, StoreError> {
        let claim = Arc::new(FileClaim::new());
        let _cancel = Cancel(claim.clone());
        let scan = ScanRequest::capture(claim.deadline)?;
        let before = loop {
            claim.pending()?;
            match try_before(&state) {
                Ok(before) => break before,
                Err(StoreError::Busy) => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await
                }
                Err(error) => return Err(error),
            }
        };
        let (sent, mut received) = oneshot::channel();
        let job_claim = claim.clone();
        tokio::spawn(async move {
            let checking_root = root.clone();
            let checking_lease = lease.clone();
            let expected = before
                .as_ref()
                .map(|b| (b.root.clone(), b.workspace.path.clone(), b.lease.clone()));
            let selected_workspace = workspace.clone();
            let binding = tokio::task::spawn_blocking(move || {
                let identity = WorkspaceIdentity::resolve(&checking_root)?;
                if let Some((expected_root, expected_workspace, expected_lease)) = expected {
                    if expected_workspace.to_string_lossy() != selected_workspace
                        || !expected_lease
                            .as_ref()
                            .is_some_and(|l| Arc::ptr_eq(l, &checking_lease))
                        || WorkspaceIdentity::resolve(&expected_root)? != identity
                    {
                        return Err(StoreError::Conflict);
                    }
                }
                validate_root_lease(&identity, &checking_lease)?;
                Ok::<_, StoreError>(Arc::new(identity))
            })
            .await
            .map_err(|_| StoreError::Unavailable)
            .and_then(|r| r);
            let identity = match binding {
                Ok(identity) => identity,
                Err(error) => {
                    let _ = sent.send(Err(error));
                    return;
                }
            };
            if sent.is_closed() || std::time::Instant::now() >= job_claim.deadline {
                let _ = sent.send(Err(StoreError::Expired));
                return;
            }
            // The task owns scan capacity and lease throughout startup, including caller loss.
            let store = match SessionStore::open_leased(
                identity.canonical_path().into(),
                workspace.clone(),
                lease.clone(),
            )
            .await
            {
                Ok(store) => Arc::new(store),
                Err(error) => {
                    let _ = sent.send(Err(error));
                    return;
                }
            };
            let result = async {
                let opening = store.validate_observation_opening().await?;
                if *store.observation_root_identity()? != *identity {
                    return Err(StoreError::Conflict);
                }
                let checking = identity.clone();
                let checking_lease = lease.clone();
                let handle = tokio::runtime::Handle::current();
                let checked_store = store.clone();
                let opened = tokio::task::spawn_blocking(move || {
                    if std::time::Instant::now() >= job_claim.deadline {
                        return Err(StoreError::Expired);
                    }
                    validate_root_lease(&checking, &checking_lease)?;
                    let mut candidate =
                        SqlCandidate::bind(checking.canonical_path(), &checking_lease)?;
                    // Bind again to the same original worker header before file admission.
                    // No file mutex is held; candidate is only a memory admission claim.
                    handle.block_on(checked_store.validate_observation_opening())?;
                    validate_root_lease(&checking, &checking_lease)?;
                    candidate.revalidate(checking.canonical_path(), &checking_lease)?;
                    loop {
                        job_claim.pending()?;
                        match try_admit(&state, &before, &mut candidate, &job_claim) {
                            Ok(()) => break,
                            Err(StoreError::Busy) => {
                                std::thread::sleep(std::time::Duration::from_millis(2))
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    let snapshot = candidate.read_scope(
                        scan,
                        opening.epoch(),
                        &hex::encode(Sha256::digest(workspace.as_bytes())),
                    )?;
                    if std::time::Instant::now() >= job_claim.deadline {
                        return Err(StoreError::Expired);
                    }
                    Ok::<_, StoreError>((snapshot, candidate.settle()?))
                })
                .await
                .map_err(|_| StoreError::Unavailable)??;
                Ok::<_, StoreError>(Self {
                    snapshot: opened.0,
                    store: store.clone(),
                    _owner: opened.1,
                    _lease: lease,
                    _root: identity,
                })
            }
            .await;
            match result {
                Ok(opened) => {
                    if let Err(Ok(opened)) = sent.send(Ok(opened)) {
                        let _ = opened.close().await;
                    }
                }
                Err(error) => {
                    let _ = store.close().await;
                    let _ = sent.send(Err(error));
                }
            }
        });
        tokio::select! {biased;result=&mut received=>{let result=result.map_err(|_|StoreError::Unavailable)?;if std::time::Instant::now()>=claim.deadline{if let Ok(opened)=result{opened.close().await?;}Err(StoreError::Expired)}else{result}},_=tokio::time::sleep_until(claim.deadline.into())=>{claim.expire();Err(StoreError::Expired)}}
    }
}
pub(super) fn try_observation_lock<T>(
    lock: &std::sync::Mutex<T>,
) -> Result<std::sync::MutexGuard<'_, T>, StoreError> {
    lock.try_lock().map_err(|error| match error {
        std::sync::TryLockError::WouldBlock => StoreError::Busy,
        std::sync::TryLockError::Poisoned(_) => StoreError::Unavailable,
    })
}
fn try_before(state: &AppState) -> Result<Option<Before>, StoreError> {
    Ok(try_observation_lock(&state.workspace)?
        .as_ref()
        .map(Before::capture))
}
fn try_admit(
    state: &AppState,
    before: &Option<Before>,
    candidate: &mut SqlCandidate,
    claim: &FileClaim,
) -> Result<(), StoreError> {
    let _projection = try_observation_lock(&state.projection_lock)?;
    let ws = try_observation_lock(&state.workspace)?;
    let unchanged = match (before, ws.as_ref()) {
        (None, None) => true,
        (Some(before), Some(ws)) => before.matches(ws),
        _ => false,
    };
    if !unchanged {
        return Err(StoreError::Conflict);
    }
    candidate.admit(ObservationOpenAdmission { _private: () }, claim)
}
pub(super) fn validate_root_lease(
    identity: &WorkspaceIdentity,
    lease: &WorkspaceLease,
) -> Result<(), StoreError> {
    identity.revalidate()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(identity.canonical_path())?;
        lease.validate_directory(&directory)?;
        identity.revalidate()?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = lease;
        Err(StoreError::Unsupported)
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn checked_workspace_open_revokes_independent_legacy_instances_and_never_adopts_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let (root, workspace, lease, first) = {
            let mut ws = state.workspace.lock().unwrap();
            let ws = ws.as_mut().unwrap();
            let root = ws.metadata.root_path();
            let lease = Arc::new(WorkspaceLease::acquire(&root).unwrap());
            ws.lease = Some(lease.clone());
            (
                root,
                ws.path.to_string_lossy().into_owned(),
                lease,
                ws.workflow_observations.clone(),
            )
        };
        let second =
            crate::workflow_observations::WorkflowObservationStore::open(root.clone()).unwrap();
        let writer = second.capture_writer("s".into(), "creation".into());
        let store = SessionStore::initialize(root.clone(), workspace.clone())
            .await
            .unwrap();
        let opened = SqlObservationWorkspace::open(state.clone(), root.clone(), workspace, lease)
            .await
            .unwrap();
        assert!(opened.snapshot().records().is_empty());
        assert!(first.workspace_observations().is_err());
        assert!(second.workspace_observations().is_err());
        assert!(writer.record_batch(&FileClaim::new(), 1, vec![]).is_err());
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .workflow_observations
                .instance_id(),
            first.instance_id()
        );
        assert!(!root.join("workflow-observations/sequence").exists());
        opened.close().await.unwrap();
        store.close().await.unwrap();
        assert!(first.workspace_observations().is_err());
        assert!(second.workspace_observations().is_err());
    }
    #[tokio::test]
    async fn checked_workspace_open_does_not_create_a_missing_database_or_accept_a_foreign_lease() {
        let state_root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(state_root.path());
        state.workspace.lock().unwrap().take();
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let foreign = Arc::new(WorkspaceLease::acquire(other.path()).unwrap());
        assert!(SqlObservationWorkspace::open(
            state.clone(),
            root.path().into(),
            "/workspace".into(),
            foreign
        )
        .await
        .is_err());
        assert!(!root.path().join("session-store").exists());
        assert!(!root.path().join("workflow-observations").exists());
        let lease = Arc::new(WorkspaceLease::acquire(root.path()).unwrap());
        assert!(SqlObservationWorkspace::open(
            state,
            root.path().into(),
            "/workspace".into(),
            lease
        )
        .await
        .is_err());
        assert!(!root.path().join("session-store").exists());
        assert!(!root.path().join("workflow-observations").exists());
    }
}
