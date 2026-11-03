use super::observation_tests::{add_runtime, fixture, request};
use super::*;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identity_merge_snapshot_does_not_publish_into_a_replacement_runtime() {
    for change in [
        "runtime",
        "control",
        "creation",
        "record_creation",
        "record_harness",
        "harness",
        "harness_id",
        "adoption",
        "removal",
    ] {
        let (_dir, state) = fixture();
        let _commands = add_runtime(&state).1;
        let mut adoption = if change == "adoption" {
            state.sessions.lock().unwrap().remove("observation")
        } else {
            None
        };
        if matches!(change, "record_creation" | "record_harness") {
            let ws = state.workspace.lock().unwrap();
            let metadata = &ws.as_ref().unwrap().metadata;
            let mut meta = metadata.read_session("observation").unwrap();
            if change == "record_creation" {
                meta.created_at = "replacement-created".into();
            } else {
                meta.harness = "claude-code".into();
            }
            metadata.try_write_session(&meta).unwrap();
        }
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let mut held = crate::session_application::identity_completion::testing::hold(root);
        let app = state.clone();
        let pending = tokio::task::spawn_blocking(move || {
            SessionApplication::new(app).report_harness_session_with_codex_context(
                "observation",
                metadata::HarnessSessionReport {
                    harness_session_id: "native-from-commit".into(),
                    source: "codex_hook".into(),
                    confidence: 0.98,
                },
                None,
                None,
                false,
            )
        });
        tokio::time::timeout(Duration::from_secs(5), held.entered())
            .await
            .unwrap();
        {
            let mut sessions = state.sessions.lock().unwrap();
            match change {
                "runtime" => {
                    sessions.get_mut("observation").unwrap().runtime =
                        crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
                }
                "control" => {
                    sessions.get_mut("observation").unwrap().runtime.control_tx =
                        tokio::sync::mpsc::channel(8).0
                }
                "creation" => {
                    sessions.get_mut("observation").unwrap().info.created_at =
                        "replacement-created".into()
                }
                "harness" => {
                    sessions.get_mut("observation").unwrap().info.harness =
                        Some("claude-code".into())
                }
                "harness_id" => {
                    sessions.get_mut("observation").unwrap().info.harness_id =
                        Some("replacement".into())
                }
                "adoption" => {
                    sessions.insert("observation".into(), adoption.take().unwrap());
                }
                "removal" => {
                    sessions.remove("observation");
                }
                "record_creation" | "record_harness" => {}
                _ => unreachable!(),
            }
        }
        drop(held);
        assert_eq!(
            pending.await.unwrap().unwrap(),
            metadata::HarnessSessionMergeResult::Accepted,
            "{change}"
        );
        if let Some(h) = state.sessions.lock().unwrap().get("observation") {
            assert!(
                h.info
                    .resume
                    .as_ref()
                    .and_then(|r| r.harness_session_id.as_ref())
                    .is_none(),
                "old transaction published after {change}"
            );
        }
        assert_eq!(
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .read_session("observation")
                .unwrap()
                .resume
                .unwrap()
                .harness_session_id
                .as_deref(),
            Some("native-from-commit")
        );
    }
}

#[tokio::test]
async fn identity_merge_snapshot_avoids_the_post_commit_json_reread() {
    let (_dir, state) = fixture();
    let _commands = add_runtime(&state).1;
    let path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .sessions_dir()
        .join("observation.json");
    let mut held = metadata::session_reader::testing::hold_nth_read(&path, 2);
    let (done, completed) = tokio::sync::oneshot::channel();
    let watchdog = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let reread=runtime.block_on(async { tokio::select! {
            _=held.entered()=>true,
            _=completed=>false,
            _=tokio::time::sleep(Duration::from_secs(10))=>panic!("identity handler stalled before completion")
        }});
        drop(held);
        reread
    });
    let mut req = request();
    req.hook_fingerprint = None;
    let response =
        report_harness_session(State(state.clone()), Path("observation".into()), Json(req))
            .await
            .into_response();
    let _ = done.send(());
    assert!(
        !watchdog.join().unwrap(),
        "identity follow-up reread JSON after its transaction"
    );
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        state.sessions.lock().unwrap()["observation"]
            .info
            .resume
            .as_ref()
            .and_then(|r| r.harness_session_id.as_deref()),
        Some("native-observation")
    );
}

#[tokio::test]
async fn identity_merge_snapshot_label_reservation_rechecks_revoked_capability() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state) = fixture();
    {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        let mut meta = metadata.read_session("observation").unwrap();
        meta.label_source = metadata::LabelSource::Placeholder;
        metadata.try_write_session(&meta).unwrap();
    }
    let _commands = add_runtime(&state).1;
    crate::codex_session_store::clear_label_refresh_generation("observation");
    crate::codex_session_store::clear_native_label_refresh_block("observation");
    let token = "identity-snapshot-original-token";
    crate::runtime::terminal_runtime::set_workflow_report_token("observation", token.into());
    let completion = SessionApplication::new(state.clone())
        .report_harness_session_with_completion(
            "observation",
            metadata::HarnessSessionReport {
                harness_session_id: "native-snapshot".into(),
                source: "codex_hook".into(),
                confidence: 0.98,
            },
            None,
            None,
            true,
            Some(token),
        )
        .unwrap();
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    assert!(
        completion
            .reserve_codex_label_refresh("observation", "native-snapshot")
            .is_none(),
        "revoked identity capability reserved a label effect"
    );
    crate::codex_session_store::clear_label_refresh_generation("observation");
}

fn label_fixture(
    source: metadata::LabelSource,
) -> (
    tempfile::TempDir,
    Arc<AppState>,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
) {
    let (dir, state) = fixture();
    {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        let mut meta = metadata.read_session("observation").unwrap();
        meta.label_source = source;
        metadata.try_write_session(&meta).unwrap();
    }
    let commands = add_runtime(&state).1;
    crate::codex_session_store::clear_label_refresh_generation("observation");
    crate::codex_session_store::clear_native_label_refresh_block("observation");
    crate::runtime::terminal_runtime::set_workflow_report_token(
        "observation",
        "snapshot-token".into(),
    );
    (dir, state, commands)
}
fn completion(
    state: &Arc<AppState>,
    authenticated: bool,
) -> crate::session_application::identity_completion::Completion {
    SessionApplication::new(state.clone())
        .report_harness_session_with_completion(
            "observation",
            metadata::HarnessSessionReport {
                harness_session_id: "native-snapshot".into(),
                source: "codex_hook".into(),
                confidence: 0.98,
            },
            None,
            None,
            authenticated,
            Some("snapshot-token"),
        )
        .unwrap()
}
fn clear_label_fixture() {
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    crate::codex_session_store::clear_label_refresh_generation("observation");
    crate::codex_session_store::clear_native_label_refresh_block("observation");
}

#[tokio::test]
async fn identity_merge_snapshot_label_reservation_rejects_stale_context() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "current",
        "workspace",
        "reader",
        "lease",
        "runtime",
        "control",
        "creation",
        "harness",
        "native",
        "wrong_id",
        "unauthenticated",
        "user",
        "token_replaced",
    ] {
        let (_dir, state, _commands) = label_fixture(if change == "user" {
            metadata::LabelSource::User
        } else {
            metadata::LabelSource::Placeholder
        });
        let completed = completion(&state, change != "unauthenticated");
        let replacement = tempfile::tempdir().unwrap();
        match change {
            "workspace" => crate::test_support::swap_workspace(&state, replacement.path()),
            "reader" => {
                let mut selected = state.workspace.lock().unwrap();
                let ws = selected.as_mut().unwrap();
                ws.metadata = metadata::MetadataStore::new(&ws.metadata.root_path());
            }
            "lease" => state.workspace.lock().unwrap().as_mut().unwrap().lease = None,
            "runtime" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime = crate::runtime::session_runtime::SessionRuntime::detached(24, 80)
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(8).0
            }
            "creation" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .info
                    .created_at = "replacement-created".into()
            }
            "harness" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .info
                    .harness = Some("claude-code".into())
            }
            "token_replaced" => crate::runtime::terminal_runtime::set_workflow_report_token(
                "observation",
                "replacement-token".into(),
            ),
            "native" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut("observation")
                    .unwrap()
                    .info
                    .resume
                    .as_mut()
                    .unwrap()
                    .harness_session_id = Some("another-root".into())
            }
            _ => {}
        }
        let reserved = completed.reserve_codex_label_refresh(
            if change == "wrong_id" {
                "another-id"
            } else {
                "observation"
            },
            "native-snapshot",
        );
        assert_eq!(reserved.is_some(), change == "current", "{change}");
        clear_label_fixture();
    }
}

#[tokio::test]
async fn identity_merge_snapshot_label_reservation_needs_no_record_reread() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, _commands) = label_fixture(metadata::LabelSource::Placeholder);
    let completed = completion(&state, true);
    let path = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .sessions_dir()
        .join("observation.json");
    let mut held = metadata::session_reader::testing::hold_nth_read(&path, 1);
    let (done, finished) = tokio::sync::oneshot::channel();
    let watchdog =
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let reread=runtime.block_on(async { tokio::select! {
            _=held.entered()=>true,_=finished=>false,
            _=tokio::time::sleep(Duration::from_secs(10))=>panic!("label reservation stalled")
        }});
            drop(held);
            reread
        });
    let reserved = completed.reserve_codex_label_refresh("observation", "native-snapshot");
    let _ = done.send(());
    assert!(reserved.is_some());
    assert!(
        !watchdog.join().unwrap(),
        "label reservation reread JSON under its memory gate"
    );
    clear_label_fixture();
}

#[tokio::test]
async fn identity_merge_snapshot_later_user_title_still_wins_label_persistence() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, _commands) = label_fixture(metadata::LabelSource::Placeholder);
    let completed = completion(&state, true);
    {
        let ws = state.workspace.lock().unwrap();
        let metadata = &ws.as_ref().unwrap().metadata;
        let mut meta = metadata.read_session("observation").unwrap();
        meta.label = "User owns title".into();
        meta.label_source = metadata::LabelSource::User;
        metadata.try_write_session(&meta).unwrap();
    }
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut("observation")
        .unwrap()
        .info
        .label = "User owns title".into();
    let (runtime, epoch) = completed
        .reserve_codex_label_refresh("observation", "native-snapshot")
        .unwrap();
    assert!(
        !SessionApplication::new(state.clone()).persist_codex_label_for_runtime(
            "observation",
            "native-snapshot",
            "Late native label".into(),
            &runtime,
            epoch
        )
    );
    assert_eq!(
        state.sessions.lock().unwrap()["observation"].info.label,
        "User owns title"
    );
    assert_eq!(
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session("observation")
            .unwrap()
            .label,
        "User owns title"
    );
    clear_label_fixture();
}

#[tokio::test]
async fn identity_merge_snapshot_failed_commit_has_no_publication_or_label_authority() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, _commands) = label_fixture(metadata::LabelSource::Placeholder);
    let (path, staging) = {
        let ws = state.workspace.lock().unwrap();
        let dir = ws.as_ref().unwrap().metadata.sessions_dir();
        (
            dir.join("observation.json"),
            dir.join("observation.json.tmp"),
        )
    };
    let before = std::fs::read(&path).unwrap();
    std::fs::create_dir(&staging).unwrap();
    let failed = completion(&state, true);
    assert_eq!(
        failed.result,
        metadata::HarnessSessionMergeResult::PersistFailed
    );
    assert!(state.sessions.lock().unwrap()["observation"]
        .info
        .resume
        .as_ref()
        .and_then(|r| r.harness_session_id.as_ref())
        .is_none());
    assert!(failed
        .reserve_codex_label_refresh("observation", "native-snapshot")
        .is_none());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_dir(staging).unwrap();
    let recovered = completion(&state, true);
    assert_eq!(
        recovered.result,
        metadata::HarnessSessionMergeResult::Accepted
    );
    assert!(recovered
        .reserve_codex_label_refresh("observation", "native-snapshot")
        .is_some());
    clear_label_fixture();
}

#[tokio::test]
async fn identity_merge_snapshot_preserves_unchanged_and_ignored_transaction_semantics() {
    let (_dir, state) = fixture();
    let ws = state.workspace.lock().unwrap();
    let store = &ws.as_ref().unwrap().metadata;
    let report = metadata::HarnessSessionReport {
        harness_session_id: "native-snapshot".into(),
        source: "codex_hook".into(),
        confidence: 0.98,
    };
    let accepted =
        store.merge_harness_session_identity("observation", &report, "2026-01-01T00:01:00Z", false);
    assert_eq!(
        accepted.result,
        metadata::HarnessSessionMergeResult::Accepted
    );
    let identity = accepted.identity.unwrap();
    assert_eq!(identity.id, "observation");
    assert_eq!(identity.created_at, "2026-01-01T00:00:00Z");
    assert_eq!(
        identity.resume.unwrap().last_seen_at.as_deref(),
        Some("2026-01-01T00:01:00Z")
    );
    assert_eq!(store.read_events("observation").len(), 1);
    let unchanged =
        store.merge_harness_session_identity("observation", &report, "2026-01-01T00:02:00Z", false);
    assert_eq!(
        unchanged.result,
        metadata::HarnessSessionMergeResult::IgnoredUnchanged
    );
    assert_eq!(
        unchanged
            .identity
            .unwrap()
            .resume
            .unwrap()
            .last_seen_at
            .as_deref(),
        Some("2026-01-01T00:02:00Z")
    );
    assert_eq!(store.read_events("observation").len(), 1);
    let path = store.sessions_dir().join("observation.json");
    let before = std::fs::read(&path).unwrap();
    let ignored = store.merge_harness_session_identity(
        "observation",
        &metadata::HarnessSessionReport {
            harness_session_id: "another-root".into(),
            confidence: 0.1,
            ..report.clone()
        },
        "2026-01-01T00:03:00Z",
        false,
    );
    assert_eq!(
        ignored.result,
        metadata::HarnessSessionMergeResult::IgnoredLowerConfidence
    );
    assert!(ignored.identity.is_none());
    let other = store.merge_harness_session_identity(
        "observation",
        &metadata::HarnessSessionReport {
            harness_session_id: "another-root".into(),
            ..report.clone()
        },
        "2026-01-01T00:03:00Z",
        false,
    );
    assert_eq!(
        other.result,
        metadata::HarnessSessionMergeResult::IgnoredIdentityChange
    );
    assert!(other.identity.is_none());
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(store.read_events("observation").len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn identity_merge_snapshot_revocation_during_label_wait_cannot_reserve() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, _commands) = label_fixture(metadata::LabelSource::Placeholder);
    let completed = completion(&state, true);
    let root = state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .root_path();
    let mut held = crate::session_application::identity_completion::testing::hold(
        root.join("label-reservation"),
    );
    let pending = tokio::task::spawn_blocking(move || {
        completed.reserve_codex_label_refresh("observation", "native-snapshot")
    });
    tokio::time::timeout(Duration::from_secs(5), held.entered())
        .await
        .unwrap();
    crate::runtime::terminal_runtime::clear_workflow_report_token("observation");
    drop(held);
    assert!(
        pending.await.unwrap().is_none(),
        "label reservation reused an authorization check from before its wait"
    );
    clear_label_fixture();
}
