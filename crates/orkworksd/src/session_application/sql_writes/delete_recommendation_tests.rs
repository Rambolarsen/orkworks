use super::delete_replay_tests::{deleted, hold, progress};
use super::organization_tests::{root, CREATED, ID};
use super::*;
use crate::taskmaster::store::{RecommendationStore, StoreError as RecommendationError};
use crate::taskmaster::{
    Recommendation, RecommendationConfidence, RecommendationStatus, RecommendationType,
    TargetSurface, WorkflowImprovement, WorkflowObservationEvidence,
};
use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};

fn recommendations(state: &Arc<AppState>) -> RecommendationStore {
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .recommendation_store
        .clone()
}
fn seed(state: &Arc<AppState>) -> RecommendationStore {
    let store = recommendations(state);
    for record in [
        recommendation("a", ID),
        recommendation("b", "other"),
        recommendation("keep", "unrelated"),
    ] {
        store.put(&record).unwrap();
    }
    store
}
fn recommendation(id: &str, session_id: &str) -> Recommendation {
    let evidence = WorkflowObservationEvidence {
        observation_id: format!("observation-{id}"),
        sequence: 4,
        session_id: session_id.into(),
        kind: ObservationKind::Obstacle,
        description: "A recurring obstacle".into(),
        evidence: "The same command failed twice".into(),
        problem_area: None,
        reported_impact: Impact::High,
        source: ObservationSource::Peon,
        confidence: 0.9,
        observed_at: "2026-08-20T12:00:00Z".into(),
    };
    Recommendation {
        assessment: None,
        id: id.into(),
        workspace_id: "workspace-1".into(),
        chain_id: "chain-1".into(),
        chain_depth: 0,
        recommendation_type: RecommendationType::ImproveWorkflow,
        status: RecommendationStatus::Proposed,
        priority: Impact::High,
        title: "Improve workflow".into(),
        summary: "Make the obstacle easier to avoid".into(),
        reason: vec!["It recurred".into()],
        evidence: vec![evidence],
        repository_evidence: vec![],
        knowledge_evidence: vec![],
        source_session_ids: vec![session_id.into()],
        target_session_id: None,
        suggested_harness_id: None,
        suggested_model: None,
        suggested_working_directory: None,
        suggested_prompt: None,
        confidence: RecommendationConfidence::High,
        requires_approval: false,
        dedupe_key: "improve_workflow:v1:tooling:v1:obstacle:test".into(),
        created_at: "2026-08-20T12:00:00Z".into(),
        updated_at: "2026-08-20T12:00:00Z".into(),
        expires_at: None,
        workflow_improvement: WorkflowImprovement {
            proposed_improvement: "Remove the obstacle".into(),
            target_surface: TargetSurface::Tooling,
            observation_ids: vec!["observation-1".into()],
            recurrence_count: 1,
            affected_session_ids: vec![session_id.into()],
            impact: Impact::High,
            expected_benefit: "Less repeated failure".into(),
            supersedes_recommendation_id: None,
            dismissal_watermark: None,
        },
        completion_packet: None,
        audit: None,
        rollup_member_ids: Vec::new(),
        rollup_member_dedupe_keys: Vec::new(),
        rollup_generation: None,
        rolled_up_by: None,
        proposed_change: None,
    }
}

fn rollup_parent(id: &str, member_ids: &[&str]) -> Recommendation {
    let mut parent = recommendation(id, "parent-session");
    parent.title = "Combined improvement".into();
    parent.summary = "Combined summary".into();
    parent.rollup_member_ids = member_ids.iter().map(|id| (*id).into()).collect();
    parent.rollup_member_ids.sort();
    parent.rollup_member_dedupe_keys = parent
        .rollup_member_ids
        .iter()
        .map(|_| parent.dedupe_key.clone())
        .collect();
    parent.evidence.clear();
    parent.source_session_ids.clear();
    parent.workflow_improvement.observation_ids.clear();
    parent.workflow_improvement.affected_session_ids.clear();
    parent.workflow_improvement.recurrence_count = 0;
    parent.workflow_improvement.supersedes_recommendation_id = None;
    parent
}

#[tokio::test]
async fn sql_recommendation_cleanup_removes_full_graph_and_reopens_progress_without_legacy_changes()
{
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = recommendations(&state);
    let parent_id = crate::taskmaster::rollup::stable_rollup_id(&["a".into(), "b".into()]);
    let mut a = recommendation("a", ID);
    a.status = RecommendationStatus::RolledUp;
    a.rolled_up_by = Some(parent_id.clone());
    let mut b = recommendation("b", "other");
    b.status = RecommendationStatus::RolledUp;
    b.rolled_up_by = Some(parent_id.clone());
    let graph = [
        a,
        b,
        rollup_parent(&parent_id, &["a", "b"]),
        recommendation("keep", "unrelated"),
    ];
    let expected = graph.iter().map(|r| (r.id.clone(), None)).collect();
    files
        .apply_recommendation_graph_transaction(&expected, &graph)
        .unwrap();
    let legacy = std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap();
    let keep = std::fs::read(root(&state).join("recommendations/keep.json")).unwrap();
    store
        .deletion_progress(ID.into(), CREATED.into(), 15, false)
        .await
        .unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_ok(), "{:?}", receipt.files);
    assert!(matches!(receipt.progress, Some(Ok(31))));
    assert!(!receipt.detached);
    assert_eq!(
        files
            .list()
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["keep"]
    );
    assert_eq!(
        std::fs::read(root(&state).join("recommendations/keep.json")).unwrap(),
        keep
    );
    assert_eq!(
        std::fs::read(root(&state).join(format!("sessions/{ID}.json"))).unwrap(),
        legacy
    );
    assert!(root(&state).join(format!("events/{ID}.terminal")).exists());
    assert!(store.get(ID.into()).await.unwrap().is_none());
    let (path, key) = store.location();
    let (path, key) = (path.to_owned(), key.to_owned());
    store.close().await.unwrap();
    let reopened = Arc::new(SessionStore::open(path, key).await.unwrap());
    assert_eq!(progress(&reopened).await, 31);
    let writes = SessionWrites::bind(state, reopened.clone()).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(31))));
    assert_eq!(
        reopened
            .pending_deletions(None, 100)
            .await
            .unwrap()
            .jobs
            .len(),
        1
    );
    reopened.close().await.unwrap();
}
#[tokio::test]
async fn sql_recommendation_cleanup_holds_graph_owner_through_sql_failure_and_retries_noop() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_recommendations());
    gate.entered().await;
    assert!(matches!(files.list(), Err(RecommendationError::Busy)));
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let blocker = rusqlite::Connection::open(root(&state).join("session-store/v1.sqlite")).unwrap();
    blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    drop(gate);
    let receipt = action.await.unwrap().unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Err(StoreError::Busy))));
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(progress(&store).await, 0);
    assert_eq!(
        files
            .list()
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "keep"]
    );
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(matches!(receipt.progress, Some(Ok(16))));
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_recommendation_cleanup_never_recovers_another_jobs_pending_journal() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let removal = files
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap()
        .prepare_session_removal("other")
        .unwrap();
    let tx = root(&state)
        .join("recommendations/.rollup-transactions/tx-00000000-0000-4000-8000-000000000001");
    std::fs::create_dir_all(tx.join("backups")).unwrap();
    std::fs::create_dir(tx.join("staged")).unwrap();
    for target in removal.targets() {
        std::fs::write(tx.join(target.backup), target.bytes).unwrap();
    }
    let manifest = removal.manifest_bytes(true).to_vec();
    std::fs::write(tx.join("manifest.json"), &manifest).unwrap();
    drop(removal);
    let before = ["a", "b", "keep"]
        .map(|id| std::fs::read(root(&state).join(format!("recommendations/{id}.json"))).unwrap());
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 0);
    assert_eq!(
        ["a", "b", "keep"].map(|id| std::fs::read(
            root(&state).join(format!("recommendations/{id}.json"))
        )
        .unwrap()),
        before
    );
    assert_eq!(std::fs::read(tx.join("manifest.json")).unwrap(), manifest);
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_recommendation_cleanup_busy_owner_preserves_files_and_other_progress() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let inventory = files
        .capture_cleanup()
        .bind(lease)
        .unwrap()
        .inspect()
        .unwrap();
    store
        .deletion_progress(ID.into(), CREATED.into(), 3, false)
        .await
        .unwrap();
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let receipt = view.remove_recommendations().await.unwrap();
    assert_eq!(
        receipt.files.err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(receipt.progress.is_none());
    assert_eq!(progress(&store).await, 3);
    assert!(root(&state).join("recommendations/a.json").exists());
    drop(inventory);
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(19))));
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_recommendation_cleanup_rejects_changed_store_and_physical_directory() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    for physical in [false, true] {
        let (dir, state, store, writes) = deleted().await;
        seed(&state);
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let original = std::fs::read(root(&state).join("recommendations/a.json")).unwrap();
        let moved = root(&state).join("recommendations-original");
        if physical {
            std::fs::rename(root(&state).join("recommendations"), &moved).unwrap();
            std::fs::create_dir(root(&state).join("recommendations")).unwrap();
            let mut foreign = recommendation("a", ID);
            foreign.title = "Replacement-directory card".into();
            std::fs::write(
                root(&state).join("recommendations/a.json"),
                serde_json::to_vec(&foreign).unwrap(),
            )
            .unwrap();
        } else {
            let replacement =
                RecommendationStore::open(dir.path().join("other-recommendations")).unwrap();
            state
                .workspace
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .recommendation_store = replacement;
        }
        let result = view.remove_recommendations().await;
        if physical {
            let receipt = result.unwrap();
            assert!(receipt.files.is_err());
            assert!(receipt.progress.is_none());
            assert_eq!(std::fs::read(moved.join("a.json")).unwrap(), original);
            assert_eq!(
                std::fs::read(root(&state).join("recommendations/a.json")).unwrap(),
                serde_json::to_vec(&{
                    let mut foreign = recommendation("a", ID);
                    foreign.title = "Replacement-directory card".into();
                    foreign
                })
                .unwrap()
            );
        } else {
            assert!(matches!(result, Err(StoreError::Conflict)));
            assert_eq!(
                std::fs::read(root(&state).join("recommendations/a.json")).unwrap(),
                original
            );
        }
        assert_eq!(progress(&store).await, 0);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_recommendation_cleanup_binding_error_isolated_from_other_stages() {
    use std::os::unix::fs::symlink;
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = deleted().await;
    seed(&state);
    let original = root(&state).join("recommendations-original");
    std::fs::rename(root(&state).join("recommendations"), &original).unwrap();
    let foreign = dir.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    let foreign_bytes = serde_json::to_vec(&recommendation("a", ID)).unwrap();
    std::fs::write(foreign.join("a.json"), &foreign_bytes).unwrap();
    symlink(&foreign, root(&state).join("recommendations")).unwrap();
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_recommendations()
        .await
        .unwrap();
    assert!(receipt.files.is_err());
    assert!(receipt.progress.is_none());
    let receipt = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap()
        .remove_replay()
        .await
        .unwrap();
    assert!(receipt.files.is_ok());
    assert!(matches!(receipt.progress, Some(Ok(1))));
    assert_eq!(
        std::fs::read(foreign.join("a.json")).unwrap(),
        foreign_bytes
    );
    assert!(original.join("a.json").exists());
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_recommendation_cleanup_rejects_reused_sql_creation_before_file_effects() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    seed(&state);
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut replacement = store
        .deletion_record(ID.into(), CREATED.into())
        .await
        .unwrap()
        .unwrap()
        .document;
    replacement["createdAt"] = serde_json::json!("2026-06-01T00:00:00Z");
    store
        .deletion_progress(ID.into(), CREATED.into(), 0, true)
        .await
        .unwrap();
    store.create(replacement).await.unwrap();
    assert!(matches!(
        view.remove_recommendations().await,
        Err(StoreError::Missing)
    ));
    assert!(root(&state).join("recommendations/a.json").exists());
    assert!(store
        .pending_deletions(None, 100)
        .await
        .unwrap()
        .jobs
        .is_empty());
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_recommendation_cleanup_admitted_caller_loss_retains_original_lease_and_owner() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let owner = super::organization_tests::organization(&state);
    let weak = Arc::downgrade(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .lease
            .as_ref()
            .unwrap(),
    );
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_recommendations());
    gate.entered().await;
    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    assert!(weak.upgrade().is_some());
    assert!(owner.pending(ID));
    assert!(matches!(files.list(), Err(RecommendationError::Busy)));
    action.abort();
    assert!(action.await.unwrap_err().is_cancelled());
    drop(gate);
    super::organization_tests::pending(&owner, false).await;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while weak.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(progress(&store).await, 16);
    assert_eq!(
        files
            .list()
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "keep"]
    );
    assert_eq!(
        store.pending_deletions(None, 100).await.unwrap().jobs.len(),
        1
    );
    store.close().await.unwrap();
}
#[tokio::test(flavor = "current_thread")]
async fn sql_recommendation_cleanup_stall_keeps_guards_and_acknowledged_input_free() {
    let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes) = deleted().await;
    let files = seed(&state);
    let (input_id, mut commands) = super::organization_tests::input_runtime(&state, dir.path());
    let view = writes
        .capture_deletion(ID.into(), CREATED.into())
        .await
        .unwrap();
    let mut gate = hold(&writes.replay_removed);
    let action = tokio::spawn(view.remove_recommendations());
    gate.entered().await;
    assert!(matches!(files.list(), Err(RecommendationError::Busy)));
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    assert!(state.session_pids.try_lock().is_ok());
    let driver = tokio::spawn(async move {
        match commands.recv().await.unwrap() {
            crate::runtime::session_runtime::RuntimeCommand::Input {
                data,
                accepted: Some(accepted),
            } => {
                assert_eq!(data, "recommendation-input\r");
                let _ = accepted.send(Ok(()));
            }
            _ => panic!("expected acknowledged input"),
        }
    });
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            input_id,
            "recommendation-input\r".into(),
        ),
    )
    .await;
    drop(gate);
    driver.await.unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(matches!(
        action.await.unwrap().unwrap().progress,
        Some(Ok(16))
    ));
    store.close().await.unwrap();
}
#[test]
fn sql_recommendation_cleanup_completes_with_one_blocking_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let _serial = crate::runtime::peon_runtime::diagnostic_test_guard();
        let (_dir, state, store, writes) = deleted().await;
        seed(&state);
        let view = writes
            .capture_deletion(ID.into(), CREATED.into())
            .await
            .unwrap();
        let receipt = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            view.remove_recommendations(),
        )
        .await;
        store.close().await.unwrap();
        receipt
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    assert!(result.is_ok_and(|r| r.is_ok_and(|receipt| matches!(receipt.progress, Some(Ok(16))))));
}
