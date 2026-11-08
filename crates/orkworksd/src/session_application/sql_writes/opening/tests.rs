use super::*;
use crate::session_store::SessionStore;
use crate::taskmaster::store::{RecommendationStore, StoreError as RecommendationError};
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;

#[tokio::test]
async fn sql_open_revokes_before_return_even_without_pending_jobs_and_has_no_file_effects() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("metadata");
    let legacy = RecommendationStore::open(root.clone()).unwrap();
    let original = root.join("recommendations/preserve");
    std::fs::write(&original, b"invalid legacy graph must not be read").unwrap();
    let sql = SessionStore::initialize(root.clone(), "workspace".into())
        .await
        .unwrap();
    sql.close().await.unwrap();
    drop(sql);
    drop(legacy);
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    state.workspace.lock().unwrap().take();
    let lease = Arc::new(WorkspaceLease::acquire(&root).unwrap());
    let opened =
        SqlRecommendationWorkspace::open(state.clone(), root.clone(), "workspace".into(), lease)
            .await
            .unwrap();
    assert!(opened
        .store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    assert!(matches!(
        opened.recommendations.list(),
        Err(RecommendationError::SqlAuthorityRequired)
    ));
    assert!(matches!(
        RecommendationStore::open(root.clone()),
        Err(RecommendationError::SqlAuthorityRequired)
    ));
    assert_eq!(
        std::fs::read(original).unwrap(),
        b"invalid legacy graph must not be read"
    );
    assert!(state.workspace.lock().unwrap().is_none());
    opened.store.close().await.unwrap();
}

async fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    RecommendationStore,
    Arc<WorkspaceLease>,
    PathBuf,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let (root, workspace, legacy, lease) = {
        let mut guard = state.workspace.lock().unwrap();
        let ws = guard.as_mut().unwrap();
        let root = ws.metadata.root_path();
        let lease = Arc::new(WorkspaceLease::acquire(&root).unwrap());
        ws.lease = Some(lease.clone());
        (
            root,
            ws.path.to_string_lossy().into_owned(),
            ws.recommendation_store.clone(),
            lease,
        )
    };
    let sql = SessionStore::initialize(root.clone(), workspace.clone())
        .await
        .unwrap();
    sql.close().await.unwrap();
    (dir, state, legacy, lease, root, workspace)
}

#[tokio::test]
async fn sql_open_rejects_foreign_workspace_lease_and_missing_database_without_revoking() {
    let (_dir, state, legacy, lease, root, workspace) = fixture().await;
    let error = SqlRecommendationWorkspace::open(
        state.clone(),
        root.clone(),
        "foreign".into(),
        lease.clone(),
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(error, StoreError::Conflict));
    assert!(legacy.list().unwrap().is_empty());
    let other = tempfile::tempdir().unwrap();
    let other_sql = SessionStore::initialize(other.path().into(), workspace.clone())
        .await
        .unwrap();
    other_sql.close().await.unwrap();
    drop(other_sql);
    let foreign_lease = Arc::new(WorkspaceLease::acquire(other.path()).unwrap());
    assert!(SqlRecommendationWorkspace::open(
        state.clone(),
        other.path().into(),
        workspace.clone(),
        foreign_lease.clone()
    )
    .await
    .is_err());
    let old = state.workspace.lock().unwrap().take();
    assert!(SqlRecommendationWorkspace::open(
        state.clone(),
        root.clone(),
        workspace.clone(),
        foreign_lease
    )
    .await
    .is_err());
    *state.workspace.lock().unwrap() = old;
    assert!(legacy.list().unwrap().is_empty());
    std::fs::remove_file(root.join("session-store/v1.sqlite")).unwrap();
    assert!(
        SqlRecommendationWorkspace::open(state, root.clone(), workspace, lease)
            .await
            .is_err()
    );
    assert!(!root.join("session-store/v1.sqlite").exists());
    assert!(legacy.list().unwrap().is_empty());
}

#[tokio::test]
async fn sql_open_detects_epoch_and_workspace_replacement_before_admission() {
    for change in ["epoch", "workspace", "root", "recommendations"] {
        let (dir, state, legacy, lease, root, workspace) = fixture().await;
        let gates = Arc::new(Gates::default());
        let mut held = super::super::delete_replay_tests::hold(&gates.before_admit);
        let opening = tokio::spawn(SqlRecommendationWorkspace::open_inner(
            state.clone(),
            root.clone(),
            workspace,
            lease,
            Some(gates),
        ));
        held.entered().await;
        assert!(state.projection_lock.try_lock().is_ok());
        assert!(state.sessions.try_lock().is_ok());
        assert!(state.workspace.try_lock().is_ok());
        if change == "epoch" {
            let sql = rusqlite::Connection::open(root.join("session-store/v1.sqlite")).unwrap();
            sql.execute(
                "UPDATE store_header SET epoch=?1",
                [uuid::Uuid::new_v4().to_string()],
            )
            .unwrap();
        } else if change == "workspace" {
            crate::test_support::swap_workspace(&state, &dir.path().join("replacement"));
        } else {
            let target = if change == "root" {
                root.clone()
            } else {
                root.join("recommendations")
            };
            std::fs::rename(&target, dir.path().join("retained-original")).unwrap();
            std::fs::create_dir(&target).unwrap();
        }
        drop(held);
        assert!(
            matches!(
                opening.await.unwrap(),
                Err(StoreError::Conflict | StoreError::Io(_))
            ),
            "{change}"
        );
        if change == "root" {
            assert!(
                matches!(legacy.list(), Err(RecommendationError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound)
            );
        } else {
            assert!(legacy.list().unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn sql_open_cancelled_before_admission_keeps_legacy_but_after_admission_stays_revoked() {
    for admitted in [false, true] {
        let (_dir, state, legacy, lease, root, workspace) = fixture().await;
        let gates = Arc::new(Gates::default());
        let mut held = super::super::delete_replay_tests::hold(if admitted {
            &gates.admitted
        } else {
            &gates.before_admit
        });
        let opening = tokio::spawn(SqlRecommendationWorkspace::open_inner(
            state.clone(),
            root,
            workspace,
            lease.clone(),
            Some(gates),
        ));
        held.entered().await;
        opening.abort();
        assert!(opening.await.err().unwrap().is_cancelled());
        if admitted {
            assert!(matches!(
                legacy.list(),
                Err(RecommendationError::SqlAuthorityRequired)
            ));
        } else {
            assert!(legacy.list().unwrap().is_empty());
        }
        assert!(Arc::strong_count(&lease) > 2); // retained task/worker plus workspace and test
        drop(held);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while Arc::strong_count(&lease) != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(state.workspace.lock().unwrap().is_some());
    }
}

#[tokio::test]
async fn sql_open_busy_and_in_flight_delivery_leave_legacy_available() {
    let (_dir, state, legacy, lease, root, workspace) = fixture().await;
    let bound = legacy.capture_cleanup().bind(lease.clone()).unwrap();
    let busy = bound.claim_sql_cleanup().unwrap();
    assert!(matches!(
        SqlRecommendationWorkspace::open(
            state.clone(),
            root.clone(),
            workspace.clone(),
            lease.clone()
        )
        .await,
        Err(StoreError::Busy)
    ));
    drop(busy);
    assert!(legacy.list().unwrap().is_empty());
    let delivery = crate::session_application::RecommendationDeliveryGuard::new(
        Path::new(&workspace),
        "unrelated",
    );
    assert!(matches!(
        SqlRecommendationWorkspace::open(state, root, workspace, lease).await,
        Err(StoreError::Conflict)
    ));
    drop(delivery);
    assert!(legacy.list().unwrap().is_empty());
}

#[tokio::test]
async fn sql_open_all_progress_bits_remain_sql_owned_without_finalizing() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = super::super::delete_replay_tests::deleted().await;
    use super::super::organization_tests::{root, CREATED, ID};
    store
        .deletion_progress(ID.into(), CREATED.into(), 31, false)
        .await
        .unwrap();
    let path = root(&state);
    let old = state.workspace.lock().unwrap().take().unwrap();
    let workspace = old.path.to_string_lossy().into_owned();
    store.close().await.unwrap();
    drop(writes);
    drop(store);
    drop(old);
    let lease = Arc::new(WorkspaceLease::acquire(&path).unwrap());
    let opened = SqlRecommendationWorkspace::open(state, path, workspace, lease)
        .await
        .unwrap();
    assert!(matches!(
        opened.recommendations.list(),
        Err(RecommendationError::SqlAuthorityRequired)
    ));
    let jobs = opened
        .store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].progress, 31);
    opened.store.close().await.unwrap();
}
