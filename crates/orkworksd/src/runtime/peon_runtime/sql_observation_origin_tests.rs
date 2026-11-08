use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::{
    lifecycle::{CapturedEnding, LifecycleWrite},
    SessionWrites,
};
use crate::session_store::StoreError;
use std::time::Duration;
const ID: &str = "sql-runtime";
async fn ending(state: &Arc<AppState>, writes: &Arc<SessionWrites>) -> CapturedEnding {
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    writes
        .lifecycle(
            ID.into(),
            runtime.clone(),
            LifecycleWrite::Transition {
                status: "killed".into(),
            },
        )
        .await
        .unwrap();
    writes
        .capture_ending_history(state, ID.into(), runtime)
        .await
        .unwrap()
}
#[tokio::test]
async fn checked_observation_open_revokes_existing_legacy_clones_without_writing() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let legacy = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    let writer = legacy.capture_writer(ID.into(), "2026-01-01T00:00:00Z".into());
    let original = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    assert_eq!(original.source().session_id(), ID);
    assert_eq!(original.source().created_at(), "2026-01-01T00:00:00Z");
    assert!(legacy.workspace_observations().is_ok());
    let opened = original.open_observations().await.unwrap();
    assert!(opened.snapshot().records().is_empty());
    assert!(legacy.workspace_observations().is_err());
    assert!(writer
        .record_batch(&crate::workflow_observations::FileClaim::new(), 1, vec![])
        .is_err());
    assert!(!store
        .location()
        .0
        .join("workflow-observations/sequence")
        .exists());
    drop(opened);
    assert!(legacy.workspace_observations().is_err());
    store.close().await.unwrap();
}
#[tokio::test]
async fn changed_sql_scope_before_file_admission_leaves_legacy_available() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let legacy = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    let original = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let conn =
        rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();
    conn.execute(
        "UPDATE store_header SET epoch=?1",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    assert!(matches!(
        original.open_observations().await,
        Err(StoreError::Conflict)
    ));
    assert!(legacy.workspace_observations().is_ok());
    store.close().await.unwrap();
}

#[tokio::test]
async fn replaced_root_before_origin_capture_cannot_be_adopted() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes).await;
    let root = store.location().0.to_owned();
    let moved = root.with_extension("original-root");
    std::fs::rename(&root, &moved).unwrap();
    std::fs::create_dir_all(root.join("workflow-observations")).unwrap();
    assert!(captured.capture_sql_observation_origin().await.is_err());
    assert!(std::fs::read_dir(root.join("workflow-observations"))
        .unwrap()
        .next()
        .is_none());
    store.close().await.unwrap();
}

#[tokio::test]
async fn completed_read_releases_session_lane_while_snapshot_remains_available() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let original = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let opened = original.open_observations().await.unwrap();
    let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
    let outcome = tokio::time::timeout(
        Duration::from_millis(300),
        writes.lifecycle(
            ID.into(),
            runtime,
            LifecycleWrite::Transition {
                status: "ended".into(),
            },
        ),
    )
    .await;
    assert!(opened.snapshot().records().is_empty());
    drop(opened);
    store.close().await.unwrap();
    assert!(
        outcome.is_ok(),
        "completed read retained the session mutation lane"
    );
    assert!(outcome.unwrap().is_ok());
}

#[tokio::test]
async fn origin_capture_respects_full_file_job_budget() {
    const CHILD: &str = "ORKWORKS_ORIGIN_CAPTURE_BUDGET_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output=tokio::task::spawn_blocking(||std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","runtime::peon_runtime::sql_observation_origin_tests::origin_capture_respects_full_file_job_budget","--nocapture"]).env(CHILD,"1").output().unwrap()).await.unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes).await;
    let held: Vec<_> = (0..32)
        .map(|_| crate::session_application::sql_writes::reserve_observation_read().unwrap())
        .collect();
    let result = captured.capture_sql_observation_origin().await;
    drop(held);
    store.close().await.unwrap();
    assert!(
        matches!(result, Err(StoreError::QueueFull)),
        "root checks escaped the existing file-job budget"
    );
}

#[tokio::test]
async fn replaced_lease_inode_before_origin_capture_is_not_current_authority() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes).await;
    let lock = store.location().0.join(".sidecar.lock");
    std::fs::remove_file(&lock).unwrap();
    std::fs::write(&lock, b"replacement").unwrap();
    let result = captured.capture_sql_observation_origin().await;
    store.close().await.unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read(lock).unwrap(), b"replacement");
}
#[tokio::test]
async fn replaced_workflow_directory_before_admission_is_not_adopted() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let legacy = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    let original = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let root = store.location().0;
    let directory = root.join("workflow-observations");
    std::fs::rename(&directory, root.join("original-workflow")).unwrap();
    std::fs::create_dir(&directory).unwrap();
    let result = original.open_observations().await;
    store.close().await.unwrap();
    assert!(result.is_err());
    assert!(legacy.workspace_observations().is_ok());
    assert!(std::fs::read_dir(directory).unwrap().next().is_none());
}

#[tokio::test]
async fn checked_sql_observation_workspace_reopens_after_all_original_owners_are_gone() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let root = store.location().0.to_owned();
    let workspace = store.location().1.to_owned();
    let original = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let opened = original.open_observations().await.unwrap();
    drop(opened);
    store.close().await.unwrap();
    drop(writes);
    drop(store);
    state.workspace.lock().unwrap().take();
    state.sessions.lock().unwrap().clear();
    assert!(crate::workflow_observations::WorkflowObservationStore::open(root.clone()).is_err());
    let lease = Arc::new(crate::workspace_runtime::WorkspaceLease::acquire(&root).unwrap());
    let reopened =
        crate::session_application::sql_writes::observation_opening::SqlObservationWorkspace::open(
            state,
            root.clone(),
            workspace,
            lease,
        )
        .await
        .unwrap();
    assert!(reopened.snapshot().records().is_empty());
    assert!(!root.join("workflow-observations/sequence").exists());
    assert!(crate::workflow_observations::WorkflowObservationStore::open(root).is_err());
    reopened.close().await.unwrap();
}

#[tokio::test]
async fn replacements_during_last_open_wait_refuse_before_admission() {
    use crate::session_application::sql_writes::lifecycle::{hold_observation, ObservationStage};
    let _guard = diagnostic_test_guard();
    for change in [
        "epoch",
        "creation",
        "live-job",
        "root",
        "lease-inode",
        "workflow-directory",
        "runtime",
        "workflow-owner",
        "workspace",
        "application-lease",
        "worker-close",
    ] {
        let (_dir, state, store, writes, _, _, _, _) = fixture().await;
        let legacy = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .workflow_observations
            .clone();
        let captured = ending(&state, &writes)
            .await
            .capture_sql_observation_origin()
            .await
            .unwrap();
        let (reached, release) = hold_observation(&writes, ObservationStage::BeforeAdmit);
        let job = tokio::spawn(async move { captured.open_observations().await });
        tokio::time::timeout(Duration::from_secs(3), reached)
            .await
            .unwrap()
            .unwrap();
        let root = store.location().0.to_owned();
        let extra = tempfile::tempdir().unwrap();
        match change {
            "epoch" | "creation" | "live-job" => {
                let conn =
                    rusqlite::Connection::open(root.join("session-store/v1.sqlite")).unwrap();
                match change {
                    "epoch" => {
                        conn.execute(
                            "UPDATE store_header SET epoch=?1",
                            [uuid::Uuid::new_v4().to_string()],
                        )
                        .unwrap();
                    }
                    "creation" => {
                        conn.execute("UPDATE sessions SET created_at='reused' WHERE id=?1", [ID])
                            .unwrap();
                    }
                    _ => {
                        conn.execute(
                            "INSERT INTO deletion_jobs VALUES(?1,'2026-01-01T00:00:00Z',0)",
                            [ID],
                        )
                        .unwrap();
                    }
                }
            }
            "root" => {
                std::fs::rename(&root, root.with_extension("moved")).unwrap();
                std::fs::create_dir_all(root.join("workflow-observations")).unwrap();
            }
            "lease-inode" => {
                std::fs::remove_file(root.join(".sidecar.lock")).unwrap();
                std::fs::write(root.join(".sidecar.lock"), b"replacement").unwrap();
            }
            "workflow-directory" => {
                std::fs::rename(
                    root.join("workflow-observations"),
                    root.join("original-workflow"),
                )
                .unwrap();
                std::fs::create_dir(root.join("workflow-observations")).unwrap();
            }
            "runtime" => {
                state.sessions.lock().unwrap().get_mut(ID).unwrap().runtime =
                    crate::runtime::session_runtime::SessionRuntime::detached(24, 80);
            }
            "workflow-owner" => {
                state
                    .workspace
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .workflow_observations =
                    crate::workflow_observations::WorkflowObservationStore::open(
                        extra.path().into(),
                    )
                    .unwrap();
            }
            "workspace" => {
                state.workspace.lock().unwrap().as_mut().unwrap().path = extra.path().into();
            }
            "application-lease" => {
                state.workspace.lock().unwrap().as_mut().unwrap().lease = Some(Arc::new(
                    crate::workspace_runtime::WorkspaceLease::acquire(extra.path()).unwrap(),
                ));
            }
            "worker-close" => {
                store.close().await.unwrap();
            }
            _ => unreachable!(),
        }
        release.send(()).unwrap();
        let result = job.await.unwrap();
        let refused = result.is_err();
        drop(result);
        store.close().await.unwrap();
        assert!(
            refused,
            "adopted {change} during the original admission wait"
        );
        assert!(
            legacy.workspace_observations().is_ok(),
            "revoked legacy before admitting {change}"
        );
        assert!(!root.join("workflow-observations/sequence").exists());
    }
}

#[tokio::test]
async fn cancellation_and_deadline_keep_original_file_capacity_until_actual_settlement() {
    const CHILD: &str = "ORKWORKS_ORIGIN_SETTLEMENT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output=tokio::task::spawn_blocking(||std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","runtime::peon_runtime::sql_observation_origin_tests::cancellation_and_deadline_keep_original_file_capacity_until_actual_settlement","--nocapture"]).env(CHILD,"1").output().unwrap()).await.unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    use crate::session_application::sql_writes::{
        lifecycle::{hold_observation, ObservationStage},
        reserve_observation_read,
    };
    for stage in ["root", "before", "admitted"] {
        for timeout in [false, true] {
            let (_dir, state, store, writes, _, _, _, _) = fixture().await;
            let legacy = state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .workflow_observations
                .clone();
            let captured = ending(&state, &writes).await;
            let original = if stage != "root" {
                Some(captured.capture_sql_observation_origin().await.unwrap())
            } else {
                drop(captured);
                None
            };
            let held: Vec<_> = (0..31)
                .map(|_| reserve_observation_read().unwrap())
                .collect();
            let gate = match stage {
                "root" => ObservationStage::Root,
                "before" => ObservationStage::BeforeAdmit,
                _ => ObservationStage::Admitted,
            };
            let (reached, release) = hold_observation(&writes, gate);
            let job = if let Some(original) = original {
                tokio::spawn(async move { original.open_observations().await.map(|_| ()) })
            } else {
                let runtime = state.sessions.lock().unwrap()[ID].runtime.identity();
                let captured = writes
                    .capture_ending_history(&state, ID.into(), runtime)
                    .await
                    .unwrap();
                tokio::spawn(
                    async move { captured.capture_sql_observation_origin().await.map(|_| ()) },
                )
            };
            tokio::time::timeout(Duration::from_secs(3), reached)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                reserve_observation_read(),
                Err(StoreError::QueueFull)
            ));
            if timeout {
                assert!(matches!(job.await.unwrap(), Err(StoreError::Expired)));
            } else {
                job.abort();
                assert!(job.await.unwrap_err().is_cancelled());
            }
            assert!(
                matches!(reserve_observation_read(), Err(StoreError::QueueFull)),
                "{stage} released resources before native settlement"
            );
            if stage == "admitted" {
                assert!(legacy.workspace_observations().is_err());
            }
            release.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    match reserve_observation_read() {
                        Ok(capacity) => {
                            drop(capacity);
                            break;
                        }
                        Err(StoreError::QueueFull) => {
                            tokio::time::sleep(Duration::from_millis(5)).await
                        }
                        Err(error) => panic!("{error}"),
                    }
                }
            })
            .await
            .unwrap();
            if stage == "admitted" {
                assert!(legacy.workspace_observations().is_err());
            } else {
                assert!(legacy.workspace_observations().is_ok());
            }
            assert!(!store
                .location()
                .0
                .join("workflow-observations/sequence")
                .exists());
            drop(held);
            store.close().await.unwrap();
        }
    }
}

#[tokio::test]
async fn completed_snapshot_is_not_delivered_after_its_original_deadline() {
    use std::future::Future;
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let legacy = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let mut read = Box::pin(captured.open_observations());
    {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(read.as_mut().poll(&mut context).is_pending());
    }
    let lane = store.runtime_lane(ID).unwrap();
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if legacy.workspace_observations().is_err() && lane.try_lock().is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    // Actual file work completed; deliberately stop polling its buffered delivery.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let result = read.await;
    let expired = matches!(result, Err(StoreError::Expired));
    drop(result);
    store.close().await.unwrap();
    assert!(expired, "late polling delivered an expired snapshot");
}

#[tokio::test]
async fn original_capture_deadline_includes_waiting_for_application_memory_checks() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes).await;
    let projection = state.projection_lock.lock().unwrap();
    let mut job = tokio::spawn(async move { captured.capture_sql_observation_origin().await });
    let outcome = tokio::time::timeout(Duration::from_millis(2300), &mut job).await;
    let bounded = outcome.is_ok();
    drop(projection);
    if !bounded {
        let _ = job.await;
    }
    store.close().await.unwrap();
    assert!(
        bounded,
        "capture exceeded its original deadline waiting for application guards"
    );
    assert!(matches!(
        outcome.unwrap().unwrap(),
        Err(StoreError::Expired)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_open_deadline_starts_before_initial_application_lock_wait() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, _writes, _, _, _, _) = fixture().await;
    let root = store.location().0.to_owned();
    let workspace = store.location().1.to_owned();
    let lease = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .lease
        .clone()
        .unwrap();
    let workspace_guard = state.workspace.lock().unwrap();
    let opening_state = state.clone();
    let mut job = tokio::spawn(async move {
        crate::session_application::sql_writes::observation_opening::SqlObservationWorkspace::open(
            opening_state,
            root,
            workspace,
            lease,
        )
        .await
    });
    let outcome = tokio::time::timeout(Duration::from_millis(2300), &mut job).await;
    let bounded = outcome.is_ok();
    drop(workspace_guard);
    if !bounded {
        if let Ok(Ok(opened)) = job.await {
            opened.close().await.unwrap();
        }
    }
    store.close().await.unwrap();
    assert!(
        bounded,
        "workspace lock waited before the original deadline began"
    );
    assert!(matches!(
        outcome.unwrap().unwrap(),
        Err(StoreError::Expired)
    ));
}
#[tokio::test]
async fn app_lock_deadline_expiry_releases_candidate_and_lane_before_the_lock_is_released() {
    use crate::session_application::sql_writes::lifecycle::{hold_observation, ObservationStage};
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let legacy = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .workflow_observations
        .clone();
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let lane = store.runtime_lane(ID).unwrap();
    let (reached, release) = hold_observation(&writes, ObservationStage::BeforeAdmit);
    let job = tokio::spawn(async move { captured.open_observations().await });
    reached.await.unwrap();
    let projection = state.projection_lock.lock().unwrap();
    release.send(()).unwrap();
    assert!(matches!(job.await.unwrap(), Err(StoreError::Expired)));
    // The caller's timer may win a few scheduler ticks before the finite job exits.
    let released = tokio::time::timeout(Duration::from_millis(100), async {
        loop {
            if legacy.workspace_observations().is_ok() && lane.try_lock().is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .is_ok();
    drop(projection);
    store.close().await.unwrap();
    assert!(
        released,
        "detached candidate/lane stayed blocked on an application mutex"
    );
}
