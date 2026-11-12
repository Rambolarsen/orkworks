use super::*;
use crate::session_application::sql_writes::{NativeLabelRefresh, SqlLabelOutcome};

async fn automatic(store: &Arc<crate::session_store::SessionStore>) {
    crate::codex_session_store::clear_label_refresh_generation(ID);
    store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::UserTitle { title: None },
        )
        .await
        .unwrap();
}
async fn begin(
    writes: &Arc<SessionWrites>,
    state: &Arc<AppState>,
) -> Option<Arc<NativeLabelRefresh>> {
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    writes
        .begin_native_label_refresh(
            ID.into(),
            TOKEN.into(),
            generation,
            "root-one".into(),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn sql_label_persists_and_publishes_only_display_fields_with_unchanged_noop() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let refresh = begin(&writes, &state).await.unwrap();
    assert_eq!(
        writes
            .persist_native_label(refresh.clone(), "Native topic".into())
            .await
            .unwrap(),
        SqlLabelOutcome::Applied
    );
    let after = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(after["label"], "Native topic");
    assert_eq!(after["labelSource"], "codex");
    for key in [
        "attention",
        "observedStatus",
        "attentionUpdatedAt",
        "lastActivity",
        "metadataUpdatedAt",
        "resume",
        "futureExtension",
    ] {
        assert_eq!(after[key], before[key], "{key}");
    }
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.label,
        "Native topic"
    );
    assert_eq!(
        writes
            .persist_native_label(refresh, "Native topic".into())
            .await
            .unwrap(),
        SqlLabelOutcome::Noop
    );
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), after);
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_label_late_lookup_cannot_replace_a_manual_title() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    store
        .mutate_internal(
            ID.into(),
            CREATED.into(),
            None,
            InternalMutation::UserTitle {
                title: Some("My title".into()),
            },
        )
        .await
        .unwrap();
    let before = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        writes
            .persist_native_label(refresh, "Late native topic".into())
            .await
            .unwrap(),
        SqlLabelOutcome::Rejected
    );
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    assert!(begin(&writes, &state).await.is_none());
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_label_original_lookup_rejects_reset_control_identity_and_token_changes() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for change in [
        "generation",
        "clear_generation",
        "reset",
        "root",
        "control",
        "token",
        "workspace",
    ] {
        let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
        automatic(&store).await;
        let refresh = begin(&writes, &state).await.unwrap();
        match change {
            "generation" => crate::codex_session_store::invalidate_label_refresh_generation(ID),
            "clear_generation" => crate::codex_session_store::clear_label_refresh_generation(ID),
            "reset" => crate::codex_session_store::block_native_label_refresh(ID, "root-one"),
            "root" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .info
                    .resume
                    .as_mut()
                    .unwrap()
                    .harness_session_id = Some("root-two".into())
            }
            "control" => {
                state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(ID)
                    .unwrap()
                    .runtime
                    .control_tx = tokio::sync::mpsc::channel(4).0
            }
            "token" => crate::runtime::terminal_runtime::set_workflow_report_token(
                ID,
                "replacement-token".into(),
            ),
            "workspace" => {
                state.workspace.lock().unwrap().as_mut().unwrap().path =
                    std::path::PathBuf::from("/replacement")
            }
            _ => unreachable!(),
        }
        let before = store.get(ID.into()).await.unwrap().unwrap();
        assert_eq!(
            writes
                .persist_native_label(refresh, "Stale topic".into())
                .await
                .unwrap(),
            SqlLabelOutcome::Rejected,
            "{change}"
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap(),
            before,
            "{change}"
        );
        clear_token();
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_label_stalled_admission_preserves_typing_and_attention() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let mut held = crate::session_store::test_worker::hold_nth_get(&store, ID, 1);
    let pending = tokio::spawn(async move {
        writes
            .persist_native_label(refresh, "Current topic".into())
            .await
    });
    held.entered().await;
    let attention = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.runtime.input_generation += 1;
        h.runtime.committed_input_sequence += 1;
        apply_live_attention_fields(
            &mut h.info,
            &mut h.runtime.attention_owner,
            "idle",
            None,
            "user",
            1.0,
        );
        h.runtime.attention_owner.snapshot()
    };
    drop(held);
    assert_eq!(pending.await.unwrap().unwrap(), SqlLabelOutcome::Applied);
    let sessions = state.sessions.lock().unwrap();
    let h = &sessions[ID];
    assert!(h.runtime.attention_owner.owns(attention.as_ref().unwrap()));
    assert_eq!(h.info.provenance.attention_source.as_deref(), Some("user"));
    drop(sessions);
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_label_after_claim_is_definitive_but_cannot_publish_a_revoked_lookup() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let old_label = state.sessions.lock().unwrap()[ID].info.label.clone();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .persist_native_label(refresh, "Committed topic".into())
            .await
    });
    held.entered().await;
    crate::codex_session_store::invalidate_label_refresh_generation(ID);
    drop(held);
    assert_eq!(
        pending.await.unwrap().unwrap(),
        SqlLabelOutcome::CommittedDetached
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["label"],
        "Committed topic"
    );
    assert_eq!(state.sessions.lock().unwrap()[ID].info.label, old_label);
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_label_caller_abort_before_claim_discards_and_after_claim_finishes() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for claimed in [false, true] {
        let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
        automatic(&store).await;
        let before = store.get(ID.into()).await.unwrap().unwrap();
        let refresh = begin(&writes, &state).await.unwrap();
        if claimed {
            let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
            let owner = writes.clone();
            let pending = tokio::spawn(async move {
                owner
                    .persist_native_label(refresh, "Owned topic".into())
                    .await
            });
            held.entered().await;
            pending.abort();
            let _ = pending.await;
            let published = writes.test_before_callback();
            drop(held);
            tokio::time::timeout(std::time::Duration::from_secs(2), published)
                .await
                .unwrap()
                .unwrap();
            // Callback notification is sent while holding sessions; the lock waits for publication.
            assert_eq!(state.sessions.lock().unwrap()[ID].info.label, "Owned topic");
            assert_eq!(
                store.get(ID.into()).await.unwrap().unwrap()["label"],
                "Owned topic"
            );
        } else {
            let reservation = store
                .prepare_internal(
                    ID.into(),
                    CREATED.into(),
                    InternalMutation::UserTitle { title: None },
                )
                .unwrap();
            let lane = reservation.lane().unwrap().lock_owned().await;
            let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
            let pending = tokio::spawn(async move {
                writes
                    .persist_native_label(refresh, "Cancelled topic".into())
                    .await
            });
            ready.await.unwrap();
            pending.abort();
            let _ = pending.await;
            drop(lane);
            drop(reservation);
            assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
        }
        clear_token();
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn sql_label_reset_is_rechecked_after_waiting_for_the_shared_lane() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let before = store.get(ID.into()).await.unwrap().unwrap();
    let reservation = store
        .prepare_internal(
            ID.into(),
            CREATED.into(),
            InternalMutation::UserTitle { title: None },
        )
        .unwrap();
    let lane = reservation.lane().unwrap().lock_owned().await;
    let ready = crate::session_application::sql_writes::testing::before_hook_lane(&writes);
    let pending = tokio::spawn(async move {
        writes
            .persist_native_label(refresh, "Invalidated topic".into())
            .await
    });
    ready.await.unwrap();
    crate::codex_session_store::invalidate_label_refresh_generation(ID);
    drop(lane);
    drop(reservation);
    assert!(matches!(pending.await.unwrap(), Err(StoreError::Conflict)));
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), before);
    clear_token();
    store.close().await.unwrap();
}
#[tokio::test]
async fn sql_label_storage_failure_is_typed_and_never_falls_back_to_json() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let old_label = state.sessions.lock().unwrap()[ID].info.label.clone();
    store.close().await.unwrap();
    assert!(matches!(
        writes
            .persist_native_label(refresh, "Unavailable topic".into())
            .await,
        Err(StoreError::Unavailable)
    ));
    assert_eq!(state.sessions.lock().unwrap()[ID].info.label, old_label);
    assert!(!dir
        .path()
        .join("sessions")
        .join(format!("{ID}.json"))
        .exists());
    clear_token();
}

#[tokio::test]
async fn sql_label_scheduler_reads_real_exact_native_name_and_persists_through_sql() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let path = dir.path().join("native.sqlite");
    let native = rusqlite::Connection::open(&path).unwrap();
    native
        .execute_batch(
            "CREATE TABLE threads(id TEXT PRIMARY KEY, name TEXT, title TEXT);
        INSERT INTO threads VALUES('root-other', 'Wrong topic', 'Wrong title');
        INSERT INTO threads VALUES('root-one', 'Saved native topic', 'Generated topic');",
        )
        .unwrap();
    drop(native);
    assert_eq!(
        writes
            .test_refresh_native_label_at(refresh, path)
            .await
            .unwrap(),
        SqlLabelOutcome::Applied
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["label"],
        "Saved native topic"
    );
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.label,
        "Saved native topic"
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_label_accepts_native_display_bound_at_a_word_boundary() {
    // Label admission must be uncontended before checking the display boundary.
    let module = module_path!().split_once("::").unwrap().1;
    if super::super::sql_tests::isolated_test(format!(
        "{module}::sql_label_accepts_native_display_bound_at_a_word_boundary"
    ))
    .await
    {
        return;
    }
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let path = dir.path().join("native.sqlite");
    let native = rusqlite::Connection::open(&path).unwrap();
    native
        .execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, name TEXT, title TEXT);")
        .unwrap();
    let name = format!("{} next", "a".repeat(99));
    native
        .execute(
            "INSERT INTO threads VALUES(?1, ?2, NULL)",
            ("root-one", name),
        )
        .unwrap();
    drop(native);
    assert_eq!(
        writes
            .test_refresh_native_label_at(refresh, path)
            .await
            .unwrap(),
        SqlLabelOutcome::Applied
    );
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["label"],
        format!("{} ", "a".repeat(99))
    );
    clear_token();
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sql_label_stalled_execution_keeps_real_pty_input_and_title_publication_available() {
    use crate::runtime::session_runtime::RuntimeCommand;
    use std::io::{Read, Write};
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, mut commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let pending = tokio::spawn(async move {
        writes
            .persist_native_label(refresh, "Responsive topic".into())
            .await
    });
    held.entered().await;
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut child = pair
        .slave
        .spawn_command(portable_pty::CommandBuilder::new("/bin/cat"))
        .unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut terminal = pair.master.take_writer().unwrap();
    let (seen, observed) = std::sync::mpsc::channel();
    let reading = std::thread::spawn(move || {
        let mut bytes = [0; 4096];
        let mut captured = Vec::new();
        let result = loop {
            match reader.read(&mut bytes) {
                Ok(0) => break Ok(captured),
                Ok(n) => {
                    captured.extend_from_slice(&bytes[..n]);
                    if captured.windows(16).any(|s| s == b"responsive-input")
                        || captured.len() > 16_384
                    {
                        break Ok(captured);
                    }
                }
                Err(error) => break Err(error),
            }
        };
        let _ = seen.send(result);
    });
    let delivery = tokio::task::spawn_blocking(move || match commands.blocking_recv().unwrap() {
        RuntimeCommand::Input {
            data,
            accepted: Some(accepted),
        } => {
            let result = terminal
                .write_all(data.as_bytes())
                .and_then(|()| terminal.flush())
                .map_err(|_| ());
            let _ = accepted.send(result);
        }
        _ => panic!("expected ordinary input"),
    });
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &state,
            ID,
            "responsive-input\r".into(),
        ),
    )
    .await;
    let echoed = observed.recv_timeout(std::time::Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);
    reading.join().unwrap();
    delivery.await.unwrap();
    drop(held);
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .unwrap()
        .windows(16)
        .any(|s| s == b"responsive-input"));
    assert_eq!(pending.await.unwrap().unwrap(), SqlLabelOutcome::Applied);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "3"
    );
    assert!(!state.sessions.lock().unwrap()[ID].active_work_hook);
    assert_eq!(
        state.sessions.lock().unwrap()[ID].info.label,
        "Responsive topic"
    );
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_label_busy_publication_retries_without_rewriting_an_identical_title() {
    // Only this case's deliberate publication lock may contend with its retry.
    let module = module_path!().split_once("::").unwrap().1;
    if super::super::sql_tests::isolated_test(format!(
        "{module}::sql_label_busy_publication_retries_without_rewriting_an_identical_title"
    ))
    .await
    {
        return;
    }
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _commands) = fixture(Some("root-one")).await;
    automatic(&store).await;
    let refresh = begin(&writes, &state).await.unwrap();
    let mut held = crate::session_store::test_worker::hold_next_internal(&store, ID);
    let owner = writes.clone();
    let retry = refresh.clone();
    let pending = tokio::spawn(async move {
        owner
            .persist_native_label(refresh, "Retry topic".into())
            .await
    });
    held.entered().await;
    let transition = crate::runtime::prompt_authority::transition_lock()
        .lock()
        .unwrap();
    drop(held);
    assert_eq!(pending.await.unwrap().unwrap(), SqlLabelOutcome::Retry);
    drop(transition);
    let committed = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(committed["label"], "Retry topic");
    assert_ne!(state.sessions.lock().unwrap()[ID].info.label, "Retry topic");
    assert_eq!(
        writes
            .persist_native_label(retry, "Retry topic".into())
            .await
            .unwrap(),
        SqlLabelOutcome::Noop
    );
    assert_eq!(store.get(ID.into()).await.unwrap().unwrap(), committed);
    assert_eq!(state.sessions.lock().unwrap()[ID].info.label, "Retry topic");
    clear_token();
    store.close().await.unwrap();
}

#[tokio::test]
async fn sql_label_normal_observer_sync_does_not_revoke_a_published_identity_lookup() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    for (previous, root) in [(None, "root-one"), (Some("root-one"), "root-two")] {
        let (_dir, state, store, writes, _commands) = fixture(previous).await;
        if let Some(previous) = previous {
            crate::codex_session_store::block_native_label_refresh(ID, previous);
        }
        assert_eq!(
            apply(
                &writes,
                &state,
                report(
                    root,
                    Some((
                        if previous.is_some() {
                            "clear"
                        } else {
                            "startup"
                        },
                        "SessionStart"
                    ))
                )
            )
            .await
            .unwrap(),
            SqlIdentityOutcome::Accepted(Merge::Accepted)
        );
        automatic(&store).await;
        let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
        let refresh = writes
            .begin_native_label_refresh(
                ID.into(),
                TOKEN.into(),
                generation,
                root.into(),
                Arc::new(AtomicBool::new(false)),
            )
            .await
            .unwrap()
            .unwrap();
        let observation = writes
            .begin_native_observation(ID.into(), generation, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observation.root(), root);
        assert_eq!(
            writes
                .persist_native_label(refresh, "Observed same identity".into())
                .await
                .unwrap(),
            SqlLabelOutcome::Applied
        );
        assert_eq!(
            store.get(ID.into()).await.unwrap().unwrap()["label"],
            "Observed same identity"
        );
        clear_token();
        store.close().await.unwrap();
    }
}
