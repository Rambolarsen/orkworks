//! Owns every write to `observed_status`/`attention` across the live session
//! handle and persisted metadata. See ADR 0027.

use crate::metadata::{self, canonical_attention, PlanPathUpdate};
use crate::session_types::SessionInfo;
use crate::AppState;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Applies an externally-reported (or debug-injected) status observation to
/// the live session handle -- the in-memory mirror of what
/// `merge_agent_attention_signal_with_plan` just persisted. `attention` is
/// only derived while `info.lifecycle == "alive"`, matching the persisted
/// side's own gating; `summary` is only touched when a message is given.
pub(crate) fn apply_live_attention_fields(
    info: &mut SessionInfo,
    observed_status: &str,
    message: Option<&str>,
    source: &str,
    confidence: f64,
) {
    info.observed_status = Some(observed_status.to_string());
    if info.lifecycle == "alive" {
        info.attention = canonical_attention(Some(observed_status));
    }
    if let Some(message) = message {
        info.summary = Some(message.to_string());
    }
    info.metadata_source = Some(source.to_string());
    info.metadata_confidence = Some(confidence);
}

/// A transition the sidecar observes about itself, rather than a hook/debug
/// report of one.
#[derive(Clone, Copy)]
pub(crate) enum ProcessTransition {
    /// Committed terminal input implies the session is now working.
    CommittedWorking,
    /// The peon idle-timer sweep detected silence past the configured
    /// timeout.
    IdleTimeout,
}

pub(crate) struct ProcessTransitionFields {
    pub(crate) observed_status: &'static str,
    pub(crate) clear_question_fields: bool,
}

pub(crate) fn process_transition_fields(kind: ProcessTransition) -> ProcessTransitionFields {
    match kind {
        ProcessTransition::CommittedWorking => ProcessTransitionFields {
            observed_status: "working",
            clear_question_fields: true,
        },
        ProcessTransition::IdleTimeout => ProcessTransitionFields {
            observed_status: "idle",
            clear_question_fields: false,
        },
    }
}

pub(crate) fn apply_process_transition_to_handle(
    info: &mut SessionInfo,
    fields: &ProcessTransitionFields,
) {
    info.observed_status = Some(fields.observed_status.to_string());
    info.attention = Some(fields.observed_status.to_string());
    info.metadata_source = Some("process".to_string());
    info.metadata_confidence = Some(1.0);
    if fields.clear_question_fields {
        info.needs_user_input = None;
        info.detected_question = None;
        info.suggested_options = None;
    }
}

pub(crate) fn apply_process_transition_to_meta(
    meta: &mut metadata::SessionMetadata,
    fields: &ProcessTransitionFields,
) {
    meta.observed_status = Some(fields.observed_status.to_string());
    meta.attention = Some(fields.observed_status.to_string());
    meta.metadata_source = "process".to_string();
    meta.metadata_confidence = 1.0;
    if fields.clear_question_fields {
        meta.needs_user_input = None;
        meta.detected_question = None;
        meta.suggested_options = None;
    }
}

/// Self-locking: persists via `merge_agent_attention_signal_with_plan`, then
/// mirrors the result onto the live handle. Requires a workspace, matching
/// both current callers (`report_attention`, `apply_debug_attention`), which
/// already reject without one. Returns `None` when there is no workspace.
pub(crate) fn apply_attention_signal(
    state: &Arc<AppState>,
    id: &str,
    status: &str,
    message: Option<&str>,
    plan_path: &PlanPathUpdate,
    timestamp: &str,
    source: &str,
    confidence: f64,
    observed_at: Option<DateTime<Utc>>,
) -> Option<metadata::AttentionMergeResult> {
    let ws_guard = state.workspace.lock().unwrap();
    let ws = ws_guard.as_ref()?;
    let mut sessions = state.sessions.lock().unwrap();
    if observed_at.is_some_and(|timestamp| {
        sessions
            .get(id)
            .is_some_and(|handle| handle.runtime.last_hook_attention_at.is_some_and(|previous| timestamp <= previous))
    }) {
        return Some(metadata::AttentionMergeResult::Ignored);
    }
    let result = ws.metadata.merge_agent_attention_signal_with_plan(
        id, status, message, plan_path, timestamp, source, confidence,
    );
    if result == metadata::AttentionMergeResult::Accepted {
        if let Some(handle) = sessions.get_mut(id) {
            apply_live_attention_fields(&mut handle.info, status, message, source, confidence);
            if let Some(observed_at) = observed_at {
                handle.runtime.last_hook_attention_at = Some(observed_at);
            }
            // A fresh accepted attention report supersedes any
            // `pending_work_signal` previously armed by single-key
            // acceptance of an earlier hook-sourced `needs_you` (#273). The
            // 2026-07-17 design called this case "benign" on the assumption
            // that visible PTY output intersperses between consecutive
            // prompts, consuming the armed signal before the next hook
            // fires. That assumption does not hold in pathological chains
            // (a hook retry or a rapid chained prompt with no intervening
            // visible output), where a stale armed signal could spuriously
            // promote the session to `working` off output that arrives in
            // its 10-second window — overriding the newer hook-sourced
            // `needs_you` and violating the agent-over-process priority.
            // Clearing the signal on every accepted hook report closes that
            // window: the next promotion requires fresh evidence (a new
            // single-key answer in the new prompt's state, or a fresh
            // initial-prompt arm at session start).
            handle.pending_work_signal = None;
        }
    }
    Some(result)
}

/// Self-locking; used by the peon idle-timer sweep. `mark_committed_input_working`
/// bypasses this and calls `apply_process_transition_to_meta`/`_to_handle`
/// directly, since it must keep both locks held to atomically bump its own
/// input-generation bookkeeping alongside the status write (see issue #193).
///
/// For `IdleTimeout` only: applies solely when the session's current persisted
/// `observed_status` is `None` or `"working"` -- a session already in a more
/// specific state (e.g. `capped`) must not be silently downgraded by a
/// background sweep. Applies to both stores together or neither: previously
/// the live handle could be overwritten even when the persisted gate rejected
/// the write, letting disk and memory disagree.
///
/// This gate is idle-sweep-specific reasoning, not a universal rule for every
/// `ProcessTransition` -- `CommittedWorking`'s real semantics are unconditional
/// (committed input must flip a `blocked`/`capped`/`waiting_for_input` session
/// back to `working`), so it is intentionally excluded here rather than
/// applied to every kind this function might ever be called with.
pub(crate) fn apply_process_transition(state: &Arc<AppState>, id: &str, kind: ProcessTransition) {
    let fields = process_transition_fields(kind);
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ws) = ws_guard.as_ref() else {
        return;
    };
    let Some(mut meta) = ws.metadata.read_session(id) else {
        return;
    };
    if matches!(kind, ProcessTransition::IdleTimeout)
        && !matches!(meta.observed_status.as_deref(), None | Some("working"))
    {
        return;
    }
    apply_process_transition_to_meta(&mut meta, &fields);
    if ws.metadata.try_write_session(&meta).is_err() {
        tracing::warn!(session_id = %id, "failed to persist process transition");
        return;
    }
    if let Some(handle) = state.sessions.lock().unwrap().get_mut(id) {
        apply_process_transition_to_handle(&mut handle.info, &fields);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::session_runtime::{
        SessionRuntime, DEFAULT_TERMINAL_COLS, DEFAULT_TERMINAL_ROWS,
    };
    use crate::SessionHandle;

    fn test_handle(id: &str) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        SessionHandle {
            info: crate::test_support::test_session_info(
                id,
                "Known",
                "/workspace/s1",
                "running",
                "now",
            ),
            active_work_hook: false,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: SessionRuntime::detached(DEFAULT_TERMINAL_ROWS, DEFAULT_TERMINAL_COLS),
            terminal_attached: false,
            resume_in_progress: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
        }
    }

    fn alive_meta(id: &str) -> metadata::SessionMetadata {
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Session",
            "/workspace/s1",
            "running",
            "now",
            "now",
        );
        meta.lifecycle = "alive".to_string();
        meta.lifecycle_phase = "active".to_string();
        meta
    }

    fn bare_info(lifecycle: &str) -> SessionInfo {
        let mut info = crate::test_support::test_session_info(
            "s1",
            "Session",
            "/workspace/s1",
            "running",
            "now",
        );
        info.lifecycle = lifecycle.to_string();
        info
    }

    #[test]
    fn apply_live_attention_fields_sets_status_and_derives_attention_when_alive() {
        let mut info = bare_info("alive");
        apply_live_attention_fields(&mut info, "waiting_for_input", Some("hi"), "agent", 1.0);
        assert_eq!(info.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(info.attention.as_deref(), Some("needs_you"));
        assert_eq!(info.summary.as_deref(), Some("hi"));
        assert_eq!(info.metadata_source.as_deref(), Some("agent"));
        assert_eq!(info.metadata_confidence, Some(1.0));
    }

    #[test]
    fn apply_live_attention_fields_leaves_attention_untouched_when_not_alive() {
        let mut info = bare_info("dead");
        info.attention = Some("idle".to_string());
        apply_live_attention_fields(&mut info, "working", None, "process", 1.0);
        assert_eq!(info.observed_status.as_deref(), Some("working"));
        assert_eq!(info.attention.as_deref(), Some("idle"));
    }

    #[test]
    fn apply_live_attention_fields_leaves_summary_untouched_when_no_message() {
        let mut info = bare_info("alive");
        info.summary = Some("previous".to_string());
        apply_live_attention_fields(&mut info, "working", None, "process", 1.0);
        assert_eq!(info.summary.as_deref(), Some("previous"));
    }

    #[test]
    fn process_transition_fields_for_committed_working() {
        let fields = process_transition_fields(ProcessTransition::CommittedWorking);
        assert_eq!(fields.observed_status, "working");
        assert!(fields.clear_question_fields);
    }

    #[test]
    fn process_transition_fields_for_idle_timeout() {
        let fields = process_transition_fields(ProcessTransition::IdleTimeout);
        assert_eq!(fields.observed_status, "idle");
        assert!(!fields.clear_question_fields);
    }

    #[test]
    fn apply_process_transition_to_handle_clears_question_fields_when_flagged() {
        let mut info = bare_info("alive");
        info.needs_user_input = Some(true);
        info.detected_question = Some("what next?".to_string());
        info.suggested_options = Some(vec!["a".to_string()]);
        let fields = process_transition_fields(ProcessTransition::CommittedWorking);
        apply_process_transition_to_handle(&mut info, &fields);
        assert_eq!(info.observed_status.as_deref(), Some("working"));
        assert_eq!(info.attention.as_deref(), Some("working"));
        assert_eq!(info.metadata_source.as_deref(), Some("process"));
        assert_eq!(info.metadata_confidence, Some(1.0));
        assert_eq!(info.needs_user_input, None);
        assert_eq!(info.detected_question, None);
        assert_eq!(info.suggested_options, None);
    }

    #[test]
    fn apply_process_transition_to_handle_preserves_question_fields_when_not_flagged() {
        let mut info = bare_info("alive");
        info.needs_user_input = Some(true);
        info.detected_question = Some("what next?".to_string());
        let fields = process_transition_fields(ProcessTransition::IdleTimeout);
        apply_process_transition_to_handle(&mut info, &fields);
        assert_eq!(info.observed_status.as_deref(), Some("idle"));
        assert_eq!(info.needs_user_input, Some(true));
        assert_eq!(info.detected_question.as_deref(), Some("what next?"));
    }

    #[test]
    fn apply_process_transition_to_meta_mirrors_handle_behavior() {
        let mut meta = crate::test_support::test_session_metadata(
            "s1",
            "Session",
            "/workspace/s1",
            "running",
            "now",
            "now",
        );
        meta.needs_user_input = Some(true);
        let fields = process_transition_fields(ProcessTransition::CommittedWorking);
        apply_process_transition_to_meta(&mut meta, &fields);
        assert_eq!(meta.observed_status.as_deref(), Some("working"));
        assert_eq!(meta.attention.as_deref(), Some("working"));
        assert_eq!(meta.metadata_source, "process");
        assert_eq!(meta.metadata_confidence, 1.0);
        assert_eq!(meta.needs_user_input, None);
    }

    #[test]
    fn apply_attention_signal_clears_pending_work_signal_on_accepted_report() {
        // Codex P2 #4 regression test: arming `pending_work_signal` on a
        // single-key answer is only safe if the armed slot is invalidated
        // when a fresh hook report arrives and wins the priority merge. The
        // 2026-07-17 design assumed this case was benign because visible
        // PTY output intersperses between consecutive hooks and consumes
        // the signal first. That assumption does not hold for hook retries
        // or rapid chained prompts with no intervening visible output, in
        // which case a stale armed signal could spuriously promote the
        // session to `working` off later output and override the newer
        // hook-sourced `needs_you`, violating agent-over-process priority.
        // The fix is to clear `pending_work_signal` on every Accepted
        // `apply_attention_signal` result.
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&alive_meta("s1"));
        }

        // Pre-condition: simulate a hook-sourced needs_you + a single-key
        // answer that armed the work signal (mirror of
        // `bare_keystroke_arms_work_signal_for_hookless_agent_sourced_needs_you`).
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut("s1").unwrap();
            handle.info.attention = Some("needs_you".into());
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.info.metadata_source = Some("agent".into());
            handle.pending_work_signal =
                Some(crate::runtime::session_runtime::arm_pending_work_signal(
                    "y",
                    tokio::time::Instant::now(),
                ));
        }
        {
            let sessions = state.sessions.lock().unwrap();
            assert!(
                sessions["s1"].pending_work_signal.is_some(),
                "pre-condition: the armed signal is in place before the second hook report"
            );
        }

        // Second hook POST arrives (retry, or a chained prompt with no
        // intervening visible output): waiting_for_input, agent source,
        // fresh observed_at. merge_agent_attention_signal_with_plan accepts
        // it (agent over process, same-or-newer timestamp).
        let result = apply_attention_signal(
            &state,
            "s1",
            "waiting_for_input",
            None,
            &PlanPathUpdate::Unchanged,
            "2026-01-01T00:00:00Z",
            "agent",
            1.0,
            None,
        );

        assert_eq!(result, Some(metadata::AttentionMergeResult::Accepted));
        let sessions = state.sessions.lock().unwrap();
        assert!(
            sessions["s1"].pending_work_signal.is_none(),
            "an accepted attention report must clear pending_work_signal so \
             later output cannot spuriously promote over the fresh hook state"
        );
        assert_eq!(
            sessions["s1"].info.attention.as_deref(),
            Some("needs_you"),
            "the fresh hook state remains needs_you and is not overridden by spurious working"
        );
    }

    #[test]
    fn apply_attention_signal_persists_and_mirrors_to_handle() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&alive_meta("s1"));
        }

        let result = apply_attention_signal(
            &state,
            "s1",
            "waiting_for_input",
            Some("hi"),
            &PlanPathUpdate::Unchanged,
            "2026-01-01T00:00:00Z",
            "agent",
            1.0,
            None,
        );

        assert_eq!(result, Some(metadata::AttentionMergeResult::Accepted));
        let persisted = {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.read_session("s1").unwrap()
        };
        assert_eq!(
            persisted.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(persisted.attention.as_deref(), Some("needs_you"));
        let sessions = state.sessions.lock().unwrap();
        let info = &sessions.get("s1").unwrap().info;
        assert_eq!(info.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(info.attention.as_deref(), Some("needs_you"));
        assert_eq!(info.summary.as_deref(), Some("hi"));
    }

    #[test]
    fn apply_attention_signal_returns_none_without_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        *state.workspace.lock().unwrap() = None;

        let result = apply_attention_signal(
            &state,
            "s1",
            "working",
            None,
            &PlanPathUpdate::Unchanged,
            "2026-01-01T00:00:00Z",
            "agent",
            1.0,
            None,
        );

        assert_eq!(result, None);
    }

    #[test]
    fn apply_process_transition_applies_to_both_stores_when_gate_passes() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&alive_meta("s1"));
        }

        apply_process_transition(&state, "s1", ProcessTransition::IdleTimeout);

        let persisted = {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.read_session("s1").unwrap()
        };
        assert_eq!(persisted.observed_status.as_deref(), Some("idle"));
        assert_eq!(persisted.metadata_confidence, 1.0);
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(
            sessions.get("s1").unwrap().info.observed_status.as_deref(),
            Some("idle")
        );
    }

    #[test]
    fn apply_process_transition_skips_both_stores_when_gate_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let mut meta = alive_meta("s1");
            meta.observed_status = Some("capped".into());
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }

        apply_process_transition(&state, "s1", ProcessTransition::IdleTimeout);

        let persisted = {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.read_session("s1").unwrap()
        };
        assert_eq!(persisted.observed_status.as_deref(), Some("capped"));
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.get("s1").unwrap().info.observed_status, None);
    }

    /// One `workspace`-then-`sessions` critical section covers the persist
    /// and the mirror together, so two concurrent calls for the same
    /// session can never leave the two stores disagreeing -- whichever call
    /// applies last, live and persisted must show the same thing. This
    /// covers both `report_attention` and `apply_debug_attention`, which
    /// previously each had their own copy of this same regression test.
    #[test]
    fn apply_attention_signal_keeps_stores_in_agreement_under_concurrent_calls() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref()
                .unwrap()
                .metadata
                .write_session(&alive_meta("s1"));
        }

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let a = {
            let state = state.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                apply_attention_signal(
                    &state,
                    "s1",
                    "waiting_for_input",
                    Some("A"),
                    &PlanPathUpdate::Unchanged,
                    "t",
                    "agent",
                    1.0,
                    None,
                )
            })
        };
        let b = {
            let state = state.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                apply_attention_signal(
                    &state,
                    "s1",
                    "blocked",
                    Some("B"),
                    &PlanPathUpdate::Unchanged,
                    "t",
                    "agent",
                    1.0,
                    None,
                )
            })
        };
        a.join().unwrap();
        b.join().unwrap();

        let persisted = {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.read_session("s1").unwrap()
        };
        let live = state
            .sessions
            .lock()
            .unwrap()
            .get("s1")
            .unwrap()
            .info
            .clone();
        assert_eq!(
            live.observed_status.as_deref(),
            persisted.observed_status.as_deref()
        );
        assert_eq!(
            live.metadata_source.as_deref(),
            Some(persisted.metadata_source.as_str())
        );
        assert_eq!(live.summary.as_deref(), persisted.summary.as_deref());
    }

    /// Regression test: apply_process_transition must not update the live
    /// handle when the persisted write fails, or disk and memory disagree --
    /// exactly the failure mode this module exists to prevent (caught in PR
    /// #208 review).
    #[test]
    fn apply_process_transition_does_not_update_handle_when_persist_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        state
            .sessions
            .lock()
            .unwrap()
            .insert("s1".into(), test_handle("s1"));
        {
            let ws = state.workspace.lock().unwrap();
            let metadata = &ws.as_ref().unwrap().metadata;
            metadata.write_session(&alive_meta("s1"));
            // A directory squatting on the atomic-write temp path makes the
            // write fail while the session file itself remains readable.
            std::fs::create_dir_all(metadata.sessions_dir().join("s1.json.tmp")).unwrap();
        }

        apply_process_transition(&state, "s1", ProcessTransition::IdleTimeout);

        let persisted = {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.read_session("s1").unwrap()
        };
        assert_ne!(persisted.observed_status.as_deref(), Some("idle"));
        let sessions = state.sessions.lock().unwrap();
        assert_eq!(sessions.get("s1").unwrap().info.observed_status, None);
    }
}
