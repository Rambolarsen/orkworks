use super::sql_observation_origin_tests::ending;
use super::sql_reservation_tests::fixture;
use super::*;
use crate::session_application::sql_writes::lifecycle::{
    hold_observation, ObservationStage, SqlCandidateResult,
};
use crate::workflow_observations::RecordOutcome;
use std::time::Duration;
const ID: &str = "sql-runtime";
fn candidate() -> peon::PeonWorkflowObservation {
    serde_json::from_value(serde_json::json!({"kind":"obstacle","description":"blocked","evidence":"real error","problemArea":"build","reportedImpact":"low","confidence":0.8})).unwrap()
}
fn output() -> Vec<String> {
    vec!["real error".into()]
}
#[tokio::test]
async fn captured_batch_keeps_indices_grounding_and_original_duplicate_receipts() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let mut ungrounded = candidate();
    ungrounded.evidence = "invented".into();
    let mut invalid = candidate();
    invalid.confidence = 2.0;
    let candidates = vec![candidate(), ungrounded, invalid, candidate(), candidate()];
    let result = captured
        .persist_workflow_observations(7, candidates.clone(), output())
        .await
        .unwrap();
    assert!(!result.detached);
    assert!(matches!(
        &result.candidates[1],
        SqlCandidateResult::Ungrounded
    ));
    assert!(matches!(
        &result.candidates[2],
        SqlCandidateResult::Recorded(Err(_))
    ));
    for (index, sequence) in [(0, 1), (3, 2), (4, 3)] {
        assert!(
            matches!(&result.candidates[index],SqlCandidateResult::Recorded(Ok(RecordOutcome::Accepted(v)))if v.sequence==sequence)
        );
    }
    let before = std::fs::read(
        store
            .location()
            .0
            .join("workflow-observations/sql-runtime.ndjson"),
    )
    .unwrap();
    let retry = captured
        .persist_workflow_observations(7, candidates, output())
        .await
        .unwrap();
    for index in [0, 3, 4] {
        assert!(matches!(
            &retry.candidates[index],
            SqlCandidateResult::Recorded(Ok(RecordOutcome::Duplicate { .. }))
        ));
    }
    assert_eq!(
        std::fs::read(
            store
                .location()
                .0
                .join("workflow-observations/sql-runtime.ndjson")
        )
        .unwrap(),
        before
    );
    let original = captured.source().clone();
    let snapshot = captured.open_observations().await.unwrap();
    for record in snapshot.snapshot().records() {
        let source = record.origin().unwrap();
        assert_eq!(source.database_epoch(), original.database_epoch());
        assert_eq!(source.created_at(), original.created_at());
    }
    drop(snapshot);
    store.close().await.unwrap();
}
#[tokio::test]
async fn changed_original_sql_scope_or_ending_refuses_before_counter_effect() {
    let _guard = diagnostic_test_guard();
    for sql in ["UPDATE store_header SET epoch='00000000-0000-4000-8000-000000000001'", "UPDATE sessions SET created_at='replacement'", "UPDATE sessions SET metadata=json_set(metadata,'$.lifecyclePhase','ended')", "UPDATE sessions SET harness='different',metadata=json_set(metadata,'$.harnessId','different')", "UPDATE sessions SET deleted=1"] {
        let (_dir,state,store,writes,_,_,_,_)=fixture().await;
        let harness=store.get(ID.into()).await.unwrap().unwrap()["harnessId"].as_str().unwrap().to_owned();
        state.sessions.lock().unwrap().get_mut(ID).unwrap().info.harness=Some(harness);
        let captured=ending(&state,&writes).await.capture_sql_observation_origin().await.unwrap();
        let conn=rusqlite::Connection::open(store.location().0.join("session-store/v1.sqlite")).unwrap();conn.execute_batch(sql).unwrap();
        assert!(captured.persist_workflow_observations(7,vec![candidate()],output()).await.is_err(),"{sql}");
        assert!(!store.location().0.join("workflow-observations/sequence").exists());store.close().await.unwrap();
    }
}
#[tokio::test]
async fn pending_batch_caller_loss_never_admits_or_writes() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let (reached, release) = hold_observation(&writes, ObservationStage::BeforeAdmit);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(7, vec![candidate()], output())
            .await
    });
    reached.await.unwrap();
    job.abort();
    let _ = job.await;
    release.send(()).unwrap();
    let lane = store.runtime_lane(ID).unwrap();
    let _settled = tokio::time::timeout(Duration::from_secs(3), lane.lock_owned())
        .await
        .unwrap();
    assert!(!store
        .location()
        .0
        .join("workflow-observations/sequence")
        .exists());
    store.close().await.unwrap();
}
#[tokio::test]
async fn admitted_batch_caller_loss_finishes_all_original_candidates_after_deadline() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let (reached, release) = hold_observation(&writes, ObservationStage::AfterCounter);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(7, vec![candidate(); 5], output())
            .await
    });
    reached.await.unwrap();
    job.abort();
    let _ = job.await;
    let lease = {
        let mut ws = state.workspace.lock().unwrap();
        Arc::downgrade(&ws.as_mut().unwrap().lease.take().unwrap())
    };
    assert!(lease.upgrade().is_some());
    tokio::time::sleep(Duration::from_millis(2050)).await;
    let lane = store.runtime_lane(ID).unwrap();
    assert!(lane.clone().try_lock_owned().is_err());
    release.send(()).unwrap();
    let _settled = tokio::time::timeout(Duration::from_secs(3), lane.lock_owned())
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(store.location().0.join("workflow-observations/sequence")).unwrap(),
        b"5"
    );
    let raw = std::fs::read_to_string(
        store
            .location()
            .0
            .join("workflow-observations/sql-runtime.ndjson"),
    )
    .unwrap();
    assert_eq!(raw.lines().count(), 5);
    store.close().await.unwrap();
}

#[tokio::test]
async fn admitted_batch_delivers_definitive_receipts_after_original_deadline() {
    let _guard = diagnostic_test_guard();
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let (reached, release) = hold_observation(&writes, ObservationStage::AfterCounter);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(7, vec![candidate(); 5], output())
            .await
    });
    reached.await.unwrap();
    tokio::time::sleep(Duration::from_millis(2050)).await;
    assert!(!job.is_finished());
    release.send(()).unwrap();
    let receipt = job.await.unwrap().unwrap();
    assert!(!receipt.detached);
    assert_eq!(receipt.candidates.len(), 5);
    assert!(receipt.candidates.iter().all(|r| matches!(
        r,
        SqlCandidateResult::Recorded(Ok(RecordOutcome::Accepted(_)))
    )));
    store.close().await.unwrap();
}
#[tokio::test]
async fn batch_keeps_full_slot_after_caller_loss_until_actual_settlement() {
    const CHILD: &str = "ORKWORKS_SQL_BATCH_SLOT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output=tokio::task::spawn_blocking(||std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","runtime::peon_runtime::sql_observation_batch_tests::batch_keeps_full_slot_after_caller_loss_until_actual_settlement","--nocapture"])
            .env(CHILD,"1").output().unwrap()).await.unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (_dir, state, store, writes, _, _, _, _) = fixture().await;
    let captured = ending(&state, &writes)
        .await
        .capture_sql_observation_origin()
        .await
        .unwrap();
    let held: Vec<_> = (0..31)
        .map(|_| crate::session_application::sql_writes::reserve_observation_read().unwrap())
        .collect();
    let (reached, release) = hold_observation(&writes, ObservationStage::AfterCounter);
    let job = tokio::spawn(async move {
        captured
            .persist_workflow_observations(7, vec![candidate(); 5], output())
            .await
    });
    reached.await.unwrap();
    job.abort();
    let _ = job.await;
    assert!(matches!(
        crate::session_application::sql_writes::reserve_observation_read(),
        Err(crate::session_store::StoreError::QueueFull)
    ));
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(slot) = crate::session_application::sql_writes::reserve_observation_read() {
                drop(slot);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(store.location().0.join("workflow-observations/sequence")).unwrap(),
        b"5"
    );
    drop(held);
    store.close().await.unwrap();
}
