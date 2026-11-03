use super::tests::attention_test_handle;
use super::*;
const ID: &str = "ending-history";
struct Fixture {
    _dir: tempfile::TempDir,
    state: Arc<AppState>,
    generation: crate::runtime::session_runtime::RuntimeGeneration,
    sessions: PathBuf,
    events: PathBuf,
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(dir.path());
    let mut meta = crate::test_support::test_session_metadata(
        ID,
        "Ending",
        dir.path().display().to_string(),
        "running",
        "before",
        "before",
    );
    meta.lifecycle = "stopping".into();
    meta.lifecycle_phase = "ending".into();
    meta.pending_terminal_status = Some("killed".into());
    let (sessions, events) = {
        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().metadata;
        store.try_write_session(&meta).unwrap();
        (store.sessions_dir(), store.events_dir())
    };
    let mut handle = attention_test_handle(ID, dir.path());
    handle.info.created_at = meta.created_at;
    handle.info.harness = Some(meta.harness);
    handle.info.lifecycle_phase = "ending".into();
    handle.info.lifecycle = "stopping".into();
    handle.runtime.last_cols = 150;
    handle.runtime.last_rows = 50;
    handle.resume_in_progress = true;
    let generation = handle.runtime.run_generation();
    state.sessions.lock().unwrap().insert(ID.into(), handle);
    Fixture {
        _dir: dir,
        state,
        generation,
        sessions,
        events,
    }
}
fn snapshot() -> metadata::ObservedStatusSnapshotMetadata {
    metadata::ObservedStatusSnapshotMetadata {
        value: Some("blocked".into()),
        source: "peon".into(),
        confidence: Some(0.8),
        observed_at: Some("after".into()),
        attention_origin: None,
    }
}
#[test]
fn ending_history_failed_grid_does_not_publish_ended_and_can_recover() {
    let f = fixture();
    let grid = f.events.join(format!("{ID}.terminal-size"));
    std::fs::create_dir_all(&grid).unwrap();
    let record = f.sessions.join(format!("{ID}.json"));
    let before = std::fs::read(&record).unwrap();
    let app = SessionApplication::new(f.state.clone());
    assert!(
        !app.complete_session_ending(ID, f.generation, snapshot(), "ended"),
        "failed final grid still published ended"
    );
    assert_eq!(std::fs::read(&record).unwrap(), before);
    assert_eq!(
        f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ending"
    );
    assert!(!f.events.join(format!("{ID}.ndjson")).exists());
    std::fs::remove_dir(grid).unwrap();
    assert!(app.complete_session_ending(ID, f.generation, snapshot(), "ended"));
    assert_eq!(
        f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
}

#[tokio::test]
async fn ending_history_cannot_write_for_a_replaced_owner() {
    use ending_completion::testing::{hold, Stage};
    for change in [
        "workspace",
        "reader",
        "runtime",
        "control",
        "creation",
        "record_creation",
        "record_harness",
        "harness",
        "harness_id",
        "record_id",
        "lease",
    ] {
        let f = fixture();
        let mut pause = hold(&f.state, Stage::BeforeHistory);
        let state = f.state.clone();
        let generation = f.generation;
        let writer = tokio::task::spawn_blocking(move || {
            SessionApplication::new(state).complete_session_ending(
                ID,
                generation,
                snapshot(),
                "ended",
            )
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
            .await
            .unwrap();
        change_owner(&f, change);
        let before = std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap();
        drop(pause);
        assert!(
            !writer.await.unwrap(),
            "old finalizer published after {change}"
        );
        assert_eq!(
            std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap(),
            before,
            "old finalizer wrote after {change}"
        );
        assert!(
            !f.events.join(format!("{ID}.terminal-size")).exists(),
            "old finalizer wrote a grid after {change}"
        );
    }
}
fn change_owner(f: &Fixture, change: &str) {
    match change {
        "workspace" => crate::test_support::swap_workspace(&f.state, f._dir.path()),
        "reader" => {
            let mut ws = f.state.workspace.lock().unwrap();
            let ws = ws.as_mut().unwrap();
            ws.metadata = metadata::MetadataStore::new(&ws.metadata.root_path());
        }
        "lease" => {
            let mut ws = f.state.workspace.lock().unwrap();
            let ws = ws.as_mut().unwrap();
            ws.lease = Some(Arc::new(
                WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
            ));
        }
        "record_creation" | "record_harness" => {
            let ws = f.state.workspace.lock().unwrap();
            let store = &ws.as_ref().unwrap().metadata;
            let mut m = store.read_session(ID).unwrap();
            if change == "record_creation" {
                m.created_at = "replacement".into();
            } else {
                m.harness.push_str("-replacement");
            }
            store.try_write_session(&m).unwrap();
        }
        _ => {
            let mut sessions = f.state.sessions.lock().unwrap();
            let h = sessions.get_mut(ID).unwrap();
            match change {
                "runtime" => {
                    h.runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
                }
                "control" => h.runtime.control_tx = tokio::sync::mpsc::channel(2).0,
                "creation" => h.info.created_at = "replacement".into(),
                "harness" => h.info.harness = Some("replacement".into()),
                "harness_id" => h.info.harness_id = Some("replacement".into()),
                "record_id" => h.info.id = "replacement".into(),
                _ => unreachable!(),
            }
        }
    }
}
#[tokio::test]
async fn ending_history_committed_result_does_not_adopt_live_replacements() {
    use ending_completion::testing::{hold, Stage};
    for change in [
        "control",
        "runtime",
        "creation",
        "workspace",
        "reader",
        "harness",
        "harness_id",
        "record_id",
    ] {
        let f = fixture();
        let mut pause = hold(&f.state, Stage::BeforePublish);
        let state = f.state.clone();
        let generation = f.generation;
        let writer = tokio::task::spawn_blocking(move || {
            SessionApplication::new(state).complete_session_ending(
                ID,
                generation,
                snapshot(),
                "ended",
            )
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
            .await
            .unwrap();
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap())
                .unwrap();
        assert_eq!(record["lifecyclePhase"], "ended");
        assert_eq!(
            std::fs::read_to_string(f.events.join(format!("{ID}.terminal-size"))).unwrap(),
            "150x50"
        );
        assert!(f.events.join(format!("{ID}.ndjson")).exists());
        change_owner(&f, change);
        drop(pause);
        assert!(
            !writer.await.unwrap(),
            "committed result published after {change}"
        );
        assert_eq!(
            f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
            "ending"
        );
    }
}

#[test]
fn ending_history_reports_committed_metadata_when_event_append_fails() {
    use ending_completion::WriteResult as W;
    let f = fixture();
    let event = f.events.join(format!("{ID}.ndjson"));
    std::fs::create_dir_all(&event).unwrap();
    let app = SessionApplication::new(f.state.clone());
    let result = app.complete_session_ending_result(ID, f.generation, snapshot(), "ended");
    assert!(matches!(result.replay, W::Written));
    assert!(matches!(result.metadata, W::Written));
    assert!(matches!(result.event, W::Failed(_)));
    assert!(result.published);
    let record = f.sessions.join(format!("{ID}.json"));
    let committed = std::fs::read(&record).unwrap();
    assert_eq!(
        f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
    std::fs::remove_dir(&event).unwrap();
    let retry = app.complete_session_ending_result(ID, f.generation, snapshot(), "ended");
    assert!(!retry.published);
    assert!(matches!(retry.event, W::NotAttempted));
    assert_eq!(std::fs::read(record).unwrap(), committed);
    assert!(
        !event.exists(),
        "an uncertain event append must not be retried blindly"
    );
}
#[test]
fn ending_history_reports_grid_only_when_metadata_commit_fails() {
    use ending_completion::WriteResult as W;
    let f = fixture();
    let staging = f.sessions.join(format!("{ID}.json.tmp"));
    std::fs::create_dir_all(&staging).unwrap();
    let app = SessionApplication::new(f.state.clone());
    let before = std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap();
    let result = app.complete_session_ending_result(ID, f.generation, snapshot(), "ended");
    assert!(matches!(result.replay, W::Written));
    assert!(matches!(result.metadata, W::Failed(_)));
    assert!(matches!(result.event, W::NotAttempted));
    assert!(!result.published);
    assert_eq!(
        std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap(),
        before
    );
    std::fs::remove_dir(staging).unwrap();
    let fixed = app.complete_session_ending_result(ID, f.generation, snapshot(), "ended");
    assert!(fixed.published);
    assert!(matches!(fixed.metadata, W::Written));
    assert!(matches!(fixed.event, W::Written));
    let events = std::fs::read_to_string(f.events.join(format!("{ID}.ndjson"))).unwrap();
    assert_eq!(events.lines().count(), 1);
}
#[test]
fn ending_history_missing_record_and_workspace_are_runtime_only() {
    use ending_completion::WriteResult as W;
    for no_workspace in [false, true] {
        let f = fixture();
        std::fs::remove_file(f.sessions.join(format!("{ID}.json"))).unwrap();
        if no_workspace {
            f.state.workspace.lock().unwrap().take();
        }
        let result = SessionApplication::new(f.state.clone()).complete_session_ending_result(
            ID,
            f.generation,
            snapshot(),
            "ended",
        );
        assert!(result.published);
        assert!(!matches!(result.metadata, W::Written));
        assert!(matches!(result.event, W::NotAttempted));
        assert_eq!(f.state.sessions.lock().unwrap()[ID].info.status, "ended");
        assert!(!f.events.join(format!("{ID}.ndjson")).exists());
    }
}
#[test]
fn ending_history_corrupt_record_is_preserved_before_grid_effects() {
    use ending_completion::WriteResult as W;
    let f = fixture();
    let record = f.sessions.join(format!("{ID}.json"));
    std::fs::write(&record, b"broken-json").unwrap();
    let result = SessionApplication::new(f.state.clone()).complete_session_ending_result(
        ID,
        f.generation,
        snapshot(),
        "ended",
    );
    assert!(matches!(result.metadata, W::Failed(_)));
    assert!(!result.published);
    assert!(matches!(result.replay, W::NotAttempted));
    assert!(matches!(result.event, W::NotAttempted));
    assert_eq!(std::fs::read(record).unwrap(), b"broken-json");
    assert!(!f.events.join(format!("{ID}.terminal-size")).exists());
}
#[tokio::test]
async fn ending_history_cancelled_waiter_retains_definitive_completion() {
    use ending_completion::testing::{hold, Stage};
    let f = fixture();
    let mut pause = hold(&f.state, Stage::BeforeHistory);
    let state = f.state.clone();
    let generation = f.generation;
    let waiter = tokio::spawn(async move {
        SessionApplication::new(state)
            .finish_session_ending(ID.into(), generation, snapshot(), "ended".into())
            .await
            .unwrap()
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
        .await
        .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert!(f.state.workspace.try_lock().is_ok());
    assert!(f.state.sessions.try_lock().is_ok());
    drop(pause);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase == "ended" {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(f.events.join(format!("{ID}.ndjson")).exists());
}
#[tokio::test(flavor = "current_thread")]
async fn ending_history_runtime_finalizer_leaves_one_tokio_worker_available() {
    use ending_completion::testing::{hold, Stage};
    let f = fixture();
    let mut pause = hold(&f.state, Stage::BeforeHistory);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (progress_tx, progress_rx) = tokio::sync::oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let progressed = runtime.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
                .await
                .unwrap();
            let _ = entered_tx.send(());
            tokio::time::timeout(std::time::Duration::from_secs(2), progress_rx)
                .await
                .is_ok_and(|p| p.is_ok())
        });
        drop(pause);
        progressed
    });
    let state = f.state.clone();
    let generation = f.generation;
    let finalizer = tokio::spawn(async move {
        crate::runtime::terminal_runtime::finalize_session_ending(
            state,
            ID.into(),
            generation,
            "ended".into(),
        )
        .await;
    });
    entered_rx.await.unwrap();
    let _ = progress_tx.send(());
    assert!(
        watchdog.join().unwrap(),
        "final persistence blocked the only Tokio worker"
    );
    finalizer.await.unwrap();
    assert_eq!(
        f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
        "ended"
    );
}

#[tokio::test]
async fn ending_history_stage_order_is_grid_then_record_then_event_then_live() {
    use ending_completion::{
        testing::{hold, Stage},
        WriteResult as W,
    };
    for stage in [
        Stage::BeforeMetadata,
        Stage::BeforeEvent,
        Stage::BeforePublish,
    ] {
        let f = fixture();
        let mut pause = hold(&f.state, stage);
        let state = f.state.clone();
        let generation = f.generation;
        let writer = tokio::task::spawn_blocking(move || {
            SessionApplication::new(state).complete_session_ending_result(
                ID,
                generation,
                snapshot(),
                "ended",
            )
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), pause.entered())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(f.events.join(format!("{ID}.terminal-size"))).unwrap(),
            "150x50"
        );
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap())
                .unwrap();
        assert_eq!(
            record["lifecyclePhase"],
            if stage == Stage::BeforeMetadata {
                "ending"
            } else {
                "ended"
            }
        );
        let event = f.events.join(format!("{ID}.ndjson"));
        assert_eq!(event.exists(), stage == Stage::BeforePublish);
        assert_eq!(
            f.state.sessions.lock().unwrap()[ID].info.lifecycle_phase,
            "ending"
        );
        drop(pause);
        let result = writer.await.unwrap();
        assert!(result.published);
        assert!(matches!(result.replay, W::Written));
        assert!(matches!(result.metadata, W::Written));
        assert!(matches!(result.event, W::Written));
    }
}
#[test]
fn ending_history_grid_failure_with_absent_record_remains_runtime_only() {
    use ending_completion::WriteResult as W;
    let f = fixture();
    std::fs::remove_file(f.sessions.join(format!("{ID}.json"))).unwrap();
    std::fs::create_dir_all(f.events.join(format!("{ID}.terminal-size"))).unwrap();
    let result = SessionApplication::new(f.state.clone()).complete_session_ending_result(
        ID,
        f.generation,
        snapshot(),
        "ended",
    );
    assert!(matches!(result.metadata, W::Absent));
    assert!(matches!(result.replay, W::Failed(_)));
    assert!(matches!(result.event, W::NotAttempted));
    assert!(result.published);
    assert_eq!(f.state.sessions.lock().unwrap()[ID].info.status, "ended");
}

#[test]
fn ending_history_queued_blocking_job_retains_original_workspace() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let f = fixture();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            let _ = started_tx.send(());
            let _ = release_rx.recv();
        });
        started_rx.await.unwrap();
        let mut dispatched = ending_completion::testing::notice_dispatch(&f.state);
        let state = f.state.clone();
        let generation = f.generation;
        let waiter = tokio::spawn(async move {
            SessionApplication::new(state)
                .finish_session_ending(ID.into(), generation, snapshot(), "ended".into())
                .await
                .unwrap()
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), dispatched.entered())
            .await
            .unwrap();
        change_owner(&f, "workspace");
        let before = std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap();
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        let result = waiter.await.unwrap();
        assert!(
            !result.published,
            "queued finalizer adopted the replacement workspace"
        );
        assert_eq!(
            std::fs::read(f.sessions.join(format!("{ID}.json"))).unwrap(),
            before
        );
        assert!(!f.events.join(format!("{ID}.terminal-size")).exists());
    });
}

#[tokio::test]
async fn ending_history_zero_resize_cannot_replace_the_confirmable_runtime_grid() {
    let f = fixture();
    let (runtime, mut commands) = crate::runtime::session_runtime::SessionRuntime::live(50, 150);
    {
        let mut sessions = f.state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.runtime = runtime;
        h.info.lifecycle_phase = "active".into();
        h.info.lifecycle = "alive".into();
    }
    for (rows, cols) in [(0, 150), (50, 0)] {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            crate::runtime::session_runtime::apply_runtime_size_without_waiting_for_persistence(
                &f.state, ID, rows, cols,
            ),
        )
        .await;
        assert!(
            matches!(result, Ok(Err(()))),
            "zero grid was admitted instead of rejected"
        );
        assert!(
            commands.try_recv().is_err(),
            "invalid resize reached the native queue"
        );
        let sessions = f.state.sessions.lock().unwrap();
        let r = &sessions[ID].runtime;
        assert_eq!((r.last_cols, r.last_rows), (150, 50));
    }
}
