//! Storage/application core only. No production relay or launch calls this
//! until exact version/configuration/complete-stream and #789 evidence qualify.
//! Caller must authenticate and accept hooks while holding workspace -> sessions.
use super::codex_completion::{Event, Hook, Tracker};
use super::observed_status::AttentionWriteToken;
use crate::metadata::{MetadataStore, SessionWriteToken};
use crate::SessionHandle;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Unchanged,
    Waiting,
    Working,
}

pub(super) struct Application {
    tracker: Tracker,
    generation: u64,
    qualified: bool,
    fence: Option<(String, String, u64, u64)>,
    ownership: Option<(AttentionWriteToken, SessionWriteToken)>,
}

impl Application {
    /// `qualified` means a source-owned installed launch row, API-only producers,
    /// actual child-context bootstrap and complete single-root stream all passed.
    /// There is deliberately no production caller while those gates are closed.
    pub fn new(generation: u64, qualified: bool) -> Self {
        Self {
            tracker: Tracker::new(generation),
            generation,
            qualified,
            fence: None,
            ownership: None,
        }
    }

    pub fn revoke(&mut self) {
        self.qualified = false;
        self.invalidate();
    }

    pub fn invalidate(&mut self) {
        self.tracker.invalidate();
        self.ownership = None;
        self.fence = None;
    }

    pub fn event(
        &mut self,
        store: &MetadataStore,
        handle: &mut SessionHandle,
        event: Event<'_>,
    ) -> io::Result<Outcome> {
        let result = self.apply(store, handle, event);
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    fn apply(
        &mut self,
        store: &MetadataStore,
        handle: &mut SessionHandle,
        event: Event<'_>,
    ) -> io::Result<Outcome> {
        if !self.qualified
            || !handle.active_work_hook
            || event.generation != self.generation
            || handle.runtime.run_generation() != self.generation
            || handle.info.memory_state != crate::session_types::MemoryState::Live
            || handle.info.lifecycle != "alive"
            || handle.info.lifecycle_phase != "active"
            || handle.info.status != "running"
            || handle
                .info
                .harness_id
                .as_deref()
                .or(handle.info.harness.as_deref())
                != Some("codex")
            || handle
                .info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref())
                != Some(event.root)
        {
            self.invalidate();
            return Ok(Outcome::Unchanged);
        }
        let fence = (
            handle.info.id.clone(),
            handle.runtime.runtime_instance_id.clone(),
            handle.runtime.input_generation,
            handle.runtime.committed_input_sequence,
        );
        if event.hook == Hook::Submitted {
            // The authenticated ordinary submission must already have written
            // working. Snapshot its durable and live ownership without rewriting.
            let Some((meta, token)) = store.read_owned_attention(&handle.info.id)? else {
                self.invalidate();
                return Ok(Outcome::Unchanged);
            };
            let clean = crate::metadata::internal_mutations::hook_tuple_matches(
                &meta, event.root, "working",
            ) && live_matches(handle, "working");
            self.tracker.event(event, clean);
            if self.tracker.has_baseline() {
                self.ownership = handle
                    .runtime
                    .attention_owner
                    .snapshot()
                    .map(|live| (live, token));
                self.fence = Some(fence);
            } else {
                self.ownership = None;
                self.fence = None;
            }
            return Ok(Outcome::Unchanged);
        }
        if self.fence.as_ref() != Some(&fence)
            || !self
                .ownership
                .as_ref()
                .is_some_and(|(live, _)| handle.runtime.attention_owner.owns(live))
        {
            self.invalidate();
            return Ok(Outcome::Unchanged);
        }
        let root = event.root.to_owned();
        let Some(effect) = self.tracker.event(event, false) else {
            return Ok(Outcome::Unchanged);
        };
        let (previous, next, outcome) = match effect {
            super::codex_completion::Effect::BindWait(_) => {
                ("working", "waiting_for_input", Outcome::Waiting)
            }
            super::codex_completion::Effect::ClearWait(_) => {
                ("waiting_for_input", "working", Outcome::Working)
            }
        };
        if !self.tracker.accepts(&effect) || !live_matches(handle, previous) {
            self.invalidate();
            return Ok(Outcome::Unchanged);
        }
        let Some((_, token)) = self.ownership.as_ref() else {
            return Ok(Outcome::Unchanged);
        };
        let updated = store.mutate_owned_attention(&handle.info.id, token, |latest| {
            Ok(crate::metadata::internal_mutations::apply_hook_completion(
                latest,
                &root,
                previous == "working",
                &crate::workspace_runtime::iso_now(),
            ))
        })?;
        let Some((meta, token)) = updated else {
            self.invalidate();
            return Ok(Outcome::Unchanged);
        };
        super::observed_status::apply_live_attention_fields(
            &mut handle.info,
            &mut handle.runtime.attention_owner,
            next,
            None,
            "codex_hook",
            1.0,
        );
        handle.info.needs_user_input = None;
        handle.info.detected_question = None;
        handle.info.suggested_options = None;
        handle.info.provenance.attention_updated_at = meta.provenance.attention_updated_at;
        handle.info.last_activity_at = Some(meta.last_activity);
        self.ownership = token.and_then(|token| {
            handle
                .runtime
                .attention_owner
                .snapshot()
                .map(|live| (live, token))
        });
        match effect {
            super::codex_completion::Effect::BindWait(_) => {
                self.tracker.acknowledge_wait(&effect);
            }
            super::codex_completion::Effect::ClearWait(_) => {
                self.tracker.acknowledge_clear(&effect);
            }
        }
        if self.ownership.is_none() {
            self.invalidate();
        }
        Ok(outcome)
    }
}

fn live_matches(handle: &SessionHandle, status: &str) -> bool {
    handle.info.observed_status.as_deref() == Some(status)
        && handle.info.attention == crate::metadata::canonical_attention(Some(status))
        && handle.info.provenance.attention_source.as_deref() == Some("agent")
        && handle.info.provenance.attention_origin.as_deref() == Some("harness_hook")
        && handle.info.provenance.attention_confidence == Some(1.0)
        && handle.info.needs_user_input.is_none()
        && handle.info.detected_question.is_none()
        && handle.info.suggested_options.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::ResumeMemory;
    use crate::test_support::{test_session_info, test_session_metadata};

    fn event(generation: u64, hook: Hook, seq: u64, tool: Option<&str>) -> Event<'_> {
        Event {
            generation,
            root: "root",
            turn: "turn",
            sequence: seq,
            observed_micros: seq as i64,
            hook,
            tool: Some("Bash"),
            invocation: tool,
        }
    }

    fn fixture() -> (tempfile::TempDir, MetadataStore, SessionHandle, Application) {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_session_metadata("completion", "test", "/tmp", "running", "now", "now");
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.harness = "codex".into();
        meta.observed_status = Some("working".into());
        meta.attention = Some("working".into());
        meta.provenance.attention_source = Some("agent".into());
        meta.provenance.attention_confidence = Some(1.0);
        meta.provenance.attention_origin = Some("harness_hook".into());
        meta.resume = Some(ResumeMemory {
            harness_session_id: Some("root".into()),
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::Exact,
            latest_fallback: false,
            last_seen_at: None,
        });
        store.write_session(&meta);
        let (runtime, _) = super::super::session_runtime::SessionRuntime::live(24, 80);
        let mut info = test_session_info("completion", "test", "/tmp", "running", "now");
        info.lifecycle = "alive".into();
        info.lifecycle_phase = "active".into();
        info.harness_id = Some("codex".into());
        info.observed_status = Some("working".into());
        info.attention = Some("working".into());
        info.provenance = meta.provenance.clone();
        info.resume = meta.resume.clone();
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let handle = SessionHandle {
            info,
            runtime,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            active_work_hook: true,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: crate::capacity_state::CapacityState::default(),
        };
        let app = Application::new(handle.runtime.run_generation(), true);
        (dir, store, handle, app)
    }

    fn begin_wait(store: &MetadataStore, handle: &mut SessionHandle, app: &mut Application) {
        let generation = handle.runtime.run_generation();
        app.event(store, handle, event(generation, Hook::Submitted, 1, None))
            .unwrap();
        app.event(
            store,
            handle,
            event(generation, Hook::Pre, 2, Some("tool-1")),
        )
        .unwrap();
        assert_eq!(
            app.event(store, handle, event(generation, Hook::Permission, 3, None))
                .unwrap(),
            Outcome::Waiting
        );
        assert_eq!(
            store
                .read_session("completion")
                .unwrap()
                .attention
                .as_deref(),
            Some("needs_you")
        );
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
    }

    #[test]
    fn exact_completion_persists_and_mirrors_working_without_losing_work() {
        let (_dir, store, mut handle, mut app) = fixture();
        begin_wait(&store, &mut handle, &mut app);
        store
            .mutate_session_optional(
                "completion",
                crate::metadata::session_transactions::WriteScope::WORK,
                |meta| {
                    meta.task = "new task".into();
                    Ok(true)
                },
            )
            .unwrap();
        let generation = handle.runtime.run_generation();
        assert_eq!(
            app.event(
                &store,
                &mut handle,
                event(generation, Hook::Post, 4, Some("tool-1"))
            )
            .unwrap(),
            Outcome::Working
        );
        let meta = store.read_session("completion").unwrap();
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert_eq!(meta.task, "new task");
        assert_eq!(meta.attention_origin(), "harness_hook");
        assert!(meta.needs_user_input.is_none());
        assert_eq!(
            app.event(
                &store,
                &mut handle,
                event(generation, Hook::Post, 5, Some("tool-1"))
            )
            .unwrap(),
            Outcome::Unchanged
        );
    }

    #[test]
    fn unrelated_completion_leaves_the_held_wait_persisted() {
        let (_dir, store, mut handle, mut app) = fixture();
        begin_wait(&store, &mut handle, &mut app);
        let path = store.sessions_dir().join("completion.json");
        let waiting = std::fs::read(&path).unwrap();
        let generation = handle.runtime.run_generation();
        // An ambiguous completion invalidates the stream; a later matching one
        // cannot revive the old permission wait's clearing authority.
        for (sequence, invocation) in [(4, "other-tool"), (5, "tool-1")] {
            assert_eq!(
                app.event(
                    &store,
                    &mut handle,
                    event(generation, Hook::Post, sequence, Some(invocation))
                )
                .unwrap(),
                Outcome::Unchanged
            );
            assert_eq!(std::fs::read(&path).unwrap(), waiting);
            assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        }
    }

    #[test]
    fn unqualified_context_never_mutates_or_clears() {
        let (_dir, store, mut handle, _) = fixture();
        let generation = handle.runtime.run_generation();
        let mut app = Application::new(generation, false);
        let before = std::fs::read(store.sessions_dir().join("completion.json")).unwrap();
        for (seq, hook, id) in [
            (1, Hook::Submitted, None),
            (2, Hook::Pre, Some("tool-1")),
            (3, Hook::Permission, None),
            (4, Hook::Post, Some("tool-1")),
        ] {
            assert_eq!(
                app.event(&store, &mut handle, event(generation, hook, seq, id))
                    .unwrap(),
                Outcome::Unchanged
            );
        }
        assert_eq!(
            std::fs::read(store.sessions_dir().join("completion.json")).unwrap(),
            before
        );
    }
    #[test]
    fn competing_attention_and_runtime_changes_preserve_the_current_record() {
        for competitor in 0..12 {
            let (_dir, store, mut handle, mut app) = fixture();
            begin_wait(&store, &mut handle, &mut app);
            let generation = handle.runtime.run_generation();
            match competitor {
                0 => {
                    handle.runtime.attention_owner.accepted_write();
                }
                1 => {
                    let meta = store.read_session("completion").unwrap();
                    store.try_write_session(&meta).unwrap();
                }
                2 => {
                    store
                        .mutate_session_optional(
                            "completion",
                            crate::metadata::session_transactions::WriteScope::ATTENTION,
                            |meta| {
                                meta.provenance.attention_source = Some("user".into());
                                Ok(true)
                            },
                        )
                        .unwrap();
                }
                3 => {
                    store
                        .mutate_session_optional(
                            "completion",
                            crate::metadata::session_transactions::WriteScope::ATTENTION,
                            |meta| {
                                meta.needs_user_input = Some(true);
                                meta.detected_question = Some("Which option?".into());
                                Ok(true)
                            },
                        )
                        .unwrap();
                }
                4 => handle.info.needs_user_input = Some(true),
                5 => handle.runtime.input_generation += 1,
                6 => handle.runtime.committed_input_sequence += 1,
                7 => {
                    handle.runtime = super::super::session_runtime::SessionRuntime::detached_test()
                }
                8 => handle.active_work_hook = false,
                9 => handle.info.lifecycle_phase = "ending".into(),
                10 => {
                    store
                        .mutate_session_optional(
                            "completion",
                            crate::metadata::session_transactions::WriteScope::NONE,
                            |meta| {
                                meta.lifecycle = "dead".into();
                                Ok(true)
                            },
                        )
                        .unwrap();
                }
                11 => {
                    store
                        .mutate_session_optional(
                            "completion",
                            crate::metadata::session_transactions::WriteScope::NONE,
                            |meta| {
                                meta.resume.as_mut().unwrap().harness_session_id =
                                    Some("other-root".into());
                                Ok(true)
                            },
                        )
                        .unwrap();
                }
                _ => unreachable!(),
            }
            let before = std::fs::read(store.sessions_dir().join("completion.json")).unwrap();
            assert_eq!(
                app.event(
                    &store,
                    &mut handle,
                    event(generation, Hook::Post, 4, Some("tool-1"))
                )
                .unwrap(),
                Outcome::Unchanged,
                "competitor {competitor}"
            );
            assert_eq!(
                std::fs::read(store.sessions_dir().join("completion.json")).unwrap(),
                before,
                "competitor {competitor}"
            );
        }
    }

    #[test]
    fn failed_clear_does_not_publish_working_or_retry_a_stale_proof() {
        let (_dir, store, mut handle, mut app) = fixture();
        begin_wait(&store, &mut handle, &mut app);
        let generation = handle.runtime.run_generation();
        let before = std::fs::read(store.sessions_dir().join("completion.json")).unwrap();
        let staged = store.sessions_dir().join("completion.json.tmp");
        std::fs::create_dir(&staged).unwrap();
        assert!(app
            .event(
                &store,
                &mut handle,
                event(generation, Hook::Post, 4, Some("tool-1"))
            )
            .is_err());
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(
            std::fs::read(store.sessions_dir().join("completion.json")).unwrap(),
            before
        );
        std::fs::remove_dir(staged).unwrap();
        assert_eq!(
            app.event(
                &store,
                &mut handle,
                event(generation, Hook::Post, 5, Some("tool-1"))
            )
            .unwrap(),
            Outcome::Unchanged
        );
    }

    #[test]
    fn corrupt_read_does_not_quarantine_or_clear_the_record() {
        let (_dir, store, mut handle, mut app) = fixture();
        begin_wait(&store, &mut handle, &mut app);
        let generation = handle.runtime.run_generation();
        let path = store.sessions_dir().join("completion.json");
        std::fs::write(&path, b"malformed").unwrap();
        assert!(app
            .event(
                &store,
                &mut handle,
                event(generation, Hook::Post, 4, Some("tool-1"))
            )
            .is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"malformed");
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
    }

    #[test]
    fn revisioned_agent_attention_wins_in_either_commit_order() {
        use crate::metadata::agent_patch::AgentPatchEnvelope;
        for agent_first in [true, false] {
            let (_dir, store, mut handle, mut app) = fixture();
            begin_wait(&store, &mut handle, &mut app);
            let generation = handle.runtime.run_generation();
            let agent_write = |status: &str| {
                let record = store
                    .try_read_revisioned_session("completion")
                    .unwrap()
                    .unwrap();
                let envelope: AgentPatchEnvelope = serde_json::from_value(serde_json::json!({
                    "metadataRevision": String::from(record.revision),
                    "patch": {"attentionState": {"observedStatus":status,
                        "needsUserInput":status=="waiting_for_input", "detectedQuestion":null, "suggestedOptions":null}}
                })).unwrap();
                store
                    .apply_agent_patch("completion", &envelope.metadata_revision, &envelope.patch)
                    .unwrap();
            };
            if agent_first {
                agent_write("waiting_for_input");
            }
            let outcome = app
                .event(
                    &store,
                    &mut handle,
                    event(generation, Hook::Post, 4, Some("tool-1")),
                )
                .unwrap();
            assert_eq!(
                outcome,
                if agent_first {
                    Outcome::Unchanged
                } else {
                    Outcome::Working
                }
            );
            if !agent_first {
                agent_write("waiting_for_input");
            }
            let meta = store.read_session("completion").unwrap();
            assert_eq!(meta.attention.as_deref(), Some("needs_you"));
            assert_eq!(meta.attention_origin(), "direct_agent");
        }
    }

    #[test]
    fn revoked_launch_cannot_reactivate_on_another_submission() {
        let (_dir, store, mut handle, mut app) = fixture();
        let generation = handle.runtime.run_generation();
        app.revoke();
        for (seq, hook, id) in [
            (1, Hook::Submitted, None),
            (2, Hook::Pre, Some("tool-1")),
            (3, Hook::Permission, None),
            (4, Hook::Post, Some("tool-1")),
        ] {
            assert_eq!(
                app.event(&store, &mut handle, event(generation, hook, seq, id))
                    .unwrap(),
                Outcome::Unchanged
            );
        }
        assert_eq!(
            store
                .read_session("completion")
                .unwrap()
                .attention
                .as_deref(),
            Some("working")
        );
    }

    #[test]
    fn conditional_clear_and_revisioned_patch_share_the_actual_storage_lane() {
        use crate::metadata::agent_patch::{AgentPatchEnvelope, AgentPatchError};
        use std::sync::{mpsc, Arc};
        let (_dir, store, mut handle, mut app) = fixture();
        begin_wait(&store, &mut handle, &mut app);
        let store = Arc::new(store);
        let (_, token) = store.read_owned_attention("completion").unwrap().unwrap();
        let record = store
            .try_read_revisioned_session("completion")
            .unwrap()
            .unwrap();
        let envelope: AgentPatchEnvelope = serde_json::from_value(serde_json::json!({
            "metadataRevision":String::from(record.revision),
            "patch":{"attentionState":{"observedStatus":"waiting_for_input","needsUserInput":true,"detectedQuestion":null,"suggestedOptions":null}}
        })).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let clear_store = store.clone();
        let clear = std::thread::spawn(move || {
            clear_store
                .mutate_owned_attention("completion", &token, |meta| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    meta.observed_status = Some("working".into());
                    meta.attention = Some("working".into());
                    Ok(true)
                })
                .unwrap()
        });
        entered_rx.recv().unwrap();
        let (attempt_tx, attempt_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let agent_store = store.clone();
        let agent = std::thread::spawn(move || {
            attempt_tx.send(()).unwrap();
            result_tx
                .send(agent_store.apply_agent_patch(
                    "completion",
                    &envelope.metadata_revision,
                    &envelope.patch,
                ))
                .unwrap();
        });
        attempt_rx.recv().unwrap();
        assert!(result_rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err());
        release_tx.send(()).unwrap();
        assert!(clear.join().unwrap().is_some());
        assert!(matches!(
            result_rx.recv().unwrap(),
            Err(AgentPatchError::Conflict)
        ));
        agent.join().unwrap();
        assert_eq!(
            store
                .read_session("completion")
                .unwrap()
                .attention
                .as_deref(),
            Some("working")
        );
    }
    #[cfg(unix)]
    #[test]
    fn ordered_publisher_queue_applies_only_its_persisted_permission_completion() {
        use super::super::codex_hook_report_relay::{
            completion::{tests::publish_native, CompletionQueue, Poll},
            CodexHookReportRelay,
        };
        let (_dir, store, mut handle, mut app) = fixture();
        let relay = CodexHookReportRelay::new().unwrap();
        let mut queue =
            CompletionQueue::create(&relay, handle.runtime.run_generation(), &"a".repeat(64))
                .unwrap();
        publish_native(queue.path(), "UserPromptSubmit", None);
        publish_native(queue.path(), "PreToolUse", Some("tool-1"));
        publish_native(queue.path(), "PermissionRequest", None);
        assert_eq!(
            queue
                .consume("root", |event| app
                    .event(&store, &mut handle, event)
                    .map(|_| ()))
                .unwrap(),
            Poll::Consumed(3)
        );
        assert_eq!(
            store
                .read_session("completion")
                .unwrap()
                .attention
                .as_deref(),
            Some("needs_you")
        );
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        publish_native(queue.path(), "PostToolUse", Some("tool-1"));
        assert_eq!(
            queue
                .consume("root", |event| app
                    .event(&store, &mut handle, event)
                    .map(|_| ()))
                .unwrap(),
            Poll::Consumed(1)
        );
        assert_eq!(
            store
                .read_session("completion")
                .unwrap()
                .attention
                .as_deref(),
            Some("working")
        );
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert_eq!(
            queue
                .consume("root", |_| panic!("acknowledged clear replayed"))
                .unwrap(),
            Poll::Consumed(0)
        );
    }
}
