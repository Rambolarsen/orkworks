//! Owns every write to `observed_status`/`attention` across the live session
//! handle and persisted metadata. See ADR 0027.

use crate::metadata::session_transactions::WriteScope;
use crate::metadata::{self, canonical_attention};
use crate::runtime::session_runtime::RuntimePromptKind;
use crate::session_types::SessionInfo;
use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
use crate::{AppState, SessionHandle};
use std::sync::Arc;

/// Non-serialized, per-owner write proof. Identical accepted writes revoke it.
#[derive(Debug)]
pub(crate) struct AttentionOwner {
    identity: std::sync::Arc<()>,
    revision: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) struct AttentionWriteToken {
    identity: std::sync::Arc<()>,
    revision: u64,
}

impl Default for AttentionOwner {
    fn default() -> Self {
        Self {
            identity: std::sync::Arc::new(()),
            revision: Some(0),
        }
    }
}

impl AttentionOwner {
    pub(crate) fn accepted_write(&mut self) -> Option<AttentionWriteToken> {
        self.revision = self.revision.and_then(|revision| revision.checked_add(1));
        self.revision.map(|revision| AttentionWriteToken {
            identity: self.identity.clone(),
            revision,
        })
    }

    pub(crate) fn snapshot(&self) -> Option<AttentionWriteToken> {
        self.revision.map(|revision| AttentionWriteToken {
            identity: self.identity.clone(),
            revision,
        })
    }

    pub(crate) fn owns(&self, token: &AttentionWriteToken) -> bool {
        std::sync::Arc::ptr_eq(&self.identity, &token.identity)
            && self.revision == Some(token.revision)
    }
}

/// Applies an externally-reported (or debug-injected) status observation to
/// the live session handle -- the in-memory mirror of what
/// `merge_agent_attention_signal_with_plan` just persisted. `attention` is
/// only derived while `info.lifecycle == "alive"`, matching the persisted
/// side's own gating; `summary` is only touched when a message is given.
pub(crate) fn apply_live_attention_fields(
    info: &mut SessionInfo,
    owner: &mut AttentionOwner,
    observed_status: &str,
    message: Option<&str>,
    source: &str,
    confidence: f64,
) {
    owner.accepted_write();
    info.observed_status = Some(observed_status.to_string());
    if info.lifecycle == "alive" {
        info.attention = canonical_attention(Some(observed_status));
    }
    if let Some(message) = message {
        info.summary = Some(message.to_string());
    }
    info.provenance.attention_source = Some(
        if source == "codex_hook" {
            "agent"
        } else {
            source
        }
        .to_string(),
    );
    info.provenance.attention_confidence = Some(confidence);
    info.provenance.attention_origin = Some(
        match source {
            "codex_hook" => "harness_hook",
            "agent" => "direct_agent",
            "user" => "user",
            "peon" => "peon",
            "process" => "process",
            "debug" => "debug",
            _ => "backend_inference",
        }
        .to_string(),
    );
    info.provenance.attention_updated_at = Some(crate::workspace_runtime::iso_now());
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
    owner: &mut AttentionOwner,
    fields: &ProcessTransitionFields,
) {
    if info
        .provenance
        .attention_source
        .as_deref()
        .or(info.metadata_source.as_deref())
        == Some("user")
    {
        return;
    }
    owner.accepted_write();
    info.observed_status = Some(fields.observed_status.to_string());
    info.attention = Some(fields.observed_status.to_string());
    info.provenance.attention_source = Some("process".into());
    info.provenance.attention_confidence = Some(1.0);
    info.provenance.attention_origin = Some("process".into());
    info.provenance.attention_updated_at = Some(crate::workspace_runtime::iso_now());
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
    meta.provenance.attention_source = Some("process".into());
    meta.provenance.attention_confidence = Some(1.0);
    meta.provenance.attention_origin = Some("process".into());
    if fields.clear_question_fields {
        meta.needs_user_input = None;
        meta.detected_question = None;
        meta.suggested_options = None;
    }
}

/// How a committed-metadata mirror updates `handle.runtime.active_prompt_kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromptKindSync {
    Preserve,
    Set(Option<RuntimePromptKind>),
}

/// How a committed-metadata mirror updates `handle.info.provenance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProvenanceSync {
    /// Replace the entire provenance struct from committed metadata.
    Full,
    /// Replace only the attention provenance fields, preserving work/summary provenance.
    AttentionOnly,
}

/// Canonical policy describing how one committed `SessionMetadata` record is
/// mirrored onto the live `SessionHandle` and its runtime invalidation state.
/// Construct via the named policy methods below so invalidation invariants
/// cannot be bypassed ad hoc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttentionMirrorOptions {
    record_accepted_write: bool,
    clear_question_fields: bool,
    provenance: ProvenanceSync,
    sync_metadata_source: bool,
    invalidate_native_approval: bool,
    clear_pending_work_signal: bool,
    prompt_kind: PromptKindSync,
}

impl AttentionMirrorOptions {
    /// Mirrors a committed Peon inference write (JSON or SQL path).
    pub(crate) const fn peon(attention_written: bool) -> Self {
        Self {
            record_accepted_write: attention_written,
            clear_question_fields: false,
            provenance: ProvenanceSync::Full,
            sync_metadata_source: true,
            invalidate_native_approval: false,
            clear_pending_work_signal: false,
            prompt_kind: PromptKindSync::Preserve,
        }
    }

    /// Mirrors an authenticated agent metadata patch carrying `WriteScope::ATTENTION`.
    pub(crate) const fn agent_patch() -> Self {
        Self {
            record_accepted_write: true,
            clear_question_fields: false,
            provenance: ProvenanceSync::AttentionOnly,
            sync_metadata_source: false,
            invalidate_native_approval: true,
            clear_pending_work_signal: true,
            prompt_kind: PromptKindSync::Set(None),
        }
    }

    /// Mirrors a committed prompt-authority record (Claude/Copilot prompt wait or clear).
    pub(crate) const fn prompt_commit(next_kind: Option<RuntimePromptKind>) -> Self {
        Self {
            record_accepted_write: true,
            clear_question_fields: true,
            provenance: ProvenanceSync::AttentionOnly,
            sync_metadata_source: false,
            invalidate_native_approval: false,
            clear_pending_work_signal: false,
            prompt_kind: PromptKindSync::Set(next_kind),
        }
    }

    /// Mirrors a committed Codex hook attention transition (JSON or SQL path).
    /// The Codex approval application owns the native-approval tracker directly.
    pub(crate) const fn codex_hook_transition() -> Self {
        Self {
            record_accepted_write: true,
            clear_question_fields: true,
            provenance: ProvenanceSync::Full,
            sync_metadata_source: false,
            invalidate_native_approval: false,
            clear_pending_work_signal: true,
            prompt_kind: PromptKindSync::Preserve,
        }
    }

    /// Mirrors a committed native terminal-status (OSC) transition.
    pub(crate) const fn native_terminal_status() -> Self {
        Self {
            record_accepted_write: true,
            clear_question_fields: false,
            provenance: ProvenanceSync::Full,
            sync_metadata_source: false,
            invalidate_native_approval: false,
            clear_pending_work_signal: true,
            prompt_kind: PromptKindSync::Preserve,
        }
    }
}

fn sync_interactive_activity_from_metadata(
    handle: &mut SessionHandle,
    stored: &metadata::SessionMetadata,
) {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&stored.last_activity) {
        let at = parsed.with_timezone(&chrono::Utc);
        if at > handle.runtime.last_interactive_at {
            handle.runtime.record_interactive_activity(at);
        }
    }
    if stored.observed_status.as_deref() == Some("working") {
        handle.runtime.clear_auto_archive_warning();
    }
}

/// Canonical committed-metadata-to-`SessionHandle` attention mirror.
/// Enforces `attention_owner.accepted_write()`, `native_approval.invalidate()`,
/// `pending_work_signal`, `active_prompt_kind`, question fields, and provenance
/// synchronization according to `options`.
pub(crate) fn mirror_committed_attention(
    handle: &mut SessionHandle,
    stored: &metadata::SessionMetadata,
    options: AttentionMirrorOptions,
) {
    if options.record_accepted_write {
        handle.runtime.attention_owner.accepted_write();
    }
    handle.info.observed_status = stored.observed_status.clone();
    handle.info.attention = stored.attention.clone();
    if options.clear_question_fields {
        handle.info.needs_user_input = None;
        handle.info.detected_question = None;
        handle.info.suggested_options = None;
    } else {
        handle.info.needs_user_input = stored.needs_user_input;
        handle.info.detected_question = stored.detected_question.clone();
        handle.info.suggested_options = stored.suggested_options.clone();
    }
    match options.provenance {
        ProvenanceSync::Full => {
            handle.info.provenance = stored.provenance.clone();
        }
        ProvenanceSync::AttentionOnly => {
            handle.info.provenance.attention_source = stored.provenance.attention_source.clone();
            handle.info.provenance.attention_confidence = stored.provenance.attention_confidence;
            handle.info.provenance.attention_origin = stored.provenance.attention_origin.clone();
            handle.info.provenance.attention_updated_at =
                stored.provenance.attention_updated_at.clone();
        }
    }
    if options.sync_metadata_source {
        handle.info.metadata_source = Some(stored.metadata_source.clone());
        handle.info.metadata_confidence = Some(stored.metadata_confidence);
    }
    if options.invalidate_native_approval {
        if let Some(tracker) = handle.runtime.native_approval.as_mut() {
            tracker.invalidate();
        }
    }
    if options.clear_pending_work_signal {
        handle.pending_work_signal = None;
    }
    match options.prompt_kind {
        PromptKindSync::Preserve => {}
        PromptKindSync::Set(next_kind) => {
            handle.runtime.active_prompt_kind = next_kind;
        }
    }
    sync_interactive_activity_from_metadata(handle, stored);
}

/// Mirrors a committed authenticated agent metadata patch onto the live
/// `SessionHandle`, routing attention-scope updates through
/// [`mirror_committed_attention`].
pub(crate) fn mirror_agent_metadata_patch(
    handle: &mut SessionHandle,
    stored: &metadata::SessionMetadata,
    scope: WriteScope,
    updates_summary: bool,
) {
    handle.info.last_activity_at = Some(stored.last_activity.clone());
    handle.info.worktree_binding = stored.worktree_binding.clone();
    if scope.attention {
        mirror_committed_attention(handle, stored, AttentionMirrorOptions::agent_patch());
    } else {
        sync_interactive_activity_from_metadata(handle, stored);
    }
    if scope.work {
        handle.info.work_phase = stored.work_phase.clone();
        handle.info.next_action = stored.next_action.clone();
        handle.info.blocker_description = stored.blocker_description.clone();
        handle.info.failed_command = stored.failed_command.clone();
        handle.info.failed_test = stored.failed_test.clone();
        handle.info.metadata_source = Some(stored.metadata_source.clone());
        handle.info.metadata_confidence = Some(stored.metadata_confidence);
        handle.info.provenance.work_metadata_updated_at =
            stored.provenance.work_metadata_updated_at.clone();
    }
    if updates_summary {
        handle.info.summary = stored.summary.clone();
        handle.info.provenance.summary_source = stored.provenance.summary_source.clone();
        handle.info.provenance.summary_confidence = stored.provenance.summary_confidence;
        handle.info.provenance.summary_observed_at = stored.provenance.summary_observed_at.clone();
    }
}

/// Clears the live attention tuple and active prompt kind after prompt-authority
/// reset or revocation, optionally mirroring updated provenance from disk.
pub(crate) fn clear_live_attention_tuple(
    handle: &mut SessionHandle,
    provenance: Option<metadata::MetadataProvenance>,
) {
    handle.runtime.attention_owner.accepted_write();
    handle.info.observed_status = None;
    handle.info.attention = None;
    handle.info.needs_user_input = None;
    handle.info.detected_question = None;
    handle.info.suggested_options = None;
    if let Some(provenance) = provenance {
        handle.info.provenance = provenance;
    }
    handle.runtime.active_prompt_kind = None;
}

/// Mirrors a committed lifecycle record (e.g. SQL lifecycle publication) onto
/// the live `SessionHandle`.
pub(crate) fn mirror_committed_lifecycle(
    handle: &mut SessionHandle,
    stored: &metadata::SessionMetadata,
    attention_written: bool,
) {
    if attention_written {
        handle.runtime.attention_owner.accepted_write();
    }
    handle.info.status = stored.status.clone();
    handle.info.lifecycle_phase = stored.lifecycle_phase.clone();
    handle.info.lifecycle = stored.lifecycle.clone();
    handle.info.attention = stored.attention.clone();
    handle.info.connectivity = Some(stored.connectivity.clone());
    handle.info.terminal_outcome = stored.terminal_outcome.clone();
    handle.info.observed_status = stored.observed_status.clone();
    handle.info.final_observed_status = stored
        .final_observed_status_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.value.clone());
    handle.info.last_activity_at = Some(stored.last_activity.clone());
    handle.info.provenance = stored.provenance.clone();
    if handle.info.lifecycle_phase == "ended" {
        handle.resume_in_progress = false;
    }
    if handle.info.lifecycle != "alive" || handle.info.lifecycle_phase != "active" {
        handle.runtime.clear_auto_archive_warning();
    } else {
        sync_interactive_activity_from_metadata(handle, stored);
    }
}

/// Mirrors a finalized ended lifecycle transition onto the live `SessionHandle`.
pub(crate) fn mirror_ended_lifecycle(
    handle: &mut SessionHandle,
    final_status: &str,
    final_observed_status: Option<String>,
    now: String,
) {
    handle.runtime.attention_owner.accepted_write();
    handle.info.status = final_status.to_string();
    handle.info.lifecycle_phase = "ended".into();
    handle.info.lifecycle = "dead".into();
    handle.info.attention = None;
    handle.info.connectivity = Some(connectivity_for_status(final_status).into());
    handle.info.terminal_outcome = terminal_outcome_for_status(final_status);
    handle.info.observed_status = None;
    handle.info.final_observed_status = final_observed_status;
    handle.info.last_activity_at = Some(now);
    handle.resume_in_progress = false;
    handle.runtime.clear_auto_archive_warning();
}

/// Marks a live `SessionHandle` as entering the `ending` lifecycle phase.
pub(crate) fn mark_handle_ending(handle: &mut SessionHandle, now: String) {
    handle.runtime.attention_owner.accepted_write();
    handle.info.status = "running".into();
    handle.info.lifecycle_phase = "ending".into();
    handle.info.lifecycle = "stopping".into();
    handle.info.attention = None;
    handle.info.connectivity = Some(connectivity_for_status("running").into());
    handle.info.terminal_outcome = None;
    handle.info.observed_status = None;
    handle.info.last_activity_at = Some(now);
    handle.runtime.clear_auto_archive_warning();
}

/// Refreshes an already-ending `SessionHandle` on a subsequent terminal
/// status signal without re-entering ending or restoring live attention.
pub(crate) fn refresh_ending_handle(handle: &mut SessionHandle, now: String) {
    handle.runtime.attention_owner.accepted_write();
    handle.info.last_activity_at = Some(now);
    handle.info.observed_status = None;
    handle.runtime.clear_auto_archive_warning();
}

/// Terminal input characteristics used by [`mark_committed_input_working`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct CommittedInput {
    pub(crate) output_boundary: Option<u64>,
    pub(crate) line_completed: bool,
    pub(crate) printable_keystroke: bool,
    pub(crate) mouse_button_event: bool,
}

struct CommittedInputDecision {
    codex_permission_owned: bool,
    harness_permission_owned: bool,
    is_elicitation: bool,
    committed_input: bool,
    commit_working: bool,
}

fn evaluate_committed_input(
    handle: &SessionHandle,
    id: &str,
    input: CommittedInput,
) -> CommittedInputDecision {
    let already_working = handle.info.observed_status.as_deref() == Some("working")
        && handle.info.attention.as_deref() == Some("working")
        && handle.info.provenance.attention_source.as_deref() == Some("process")
        && handle.info.provenance.attention_confidence == Some(1.0)
        && handle.info.needs_user_input.is_none()
        && handle.info.detected_question.is_none()
        && handle.info.suggested_options.is_none()
        && handle.pending_work_signal.is_none();
    // A hook-reported prompt normally defers to the hook for the working
    // transition. A validated permission prompt can be answered with one key.
    let codex_permission_owned = handle
        .runtime
        .native_approval
        .as_ref()
        .is_some_and(|tracker| {
            tracker.protects_permission_wait(&handle.runtime.attention_owner, &handle.info)
        });
    let validated_harness_prompt = matches!(
        handle
            .info
            .harness_id
            .as_deref()
            .or(handle.info.harness.as_deref()),
        Some("claude-code" | "copilot")
    ) && crate::runtime::prompt_authority::registry().is_active(id);
    let harness_permission_owned = matches!(
        handle.runtime.active_prompt_kind,
        Some(RuntimePromptKind::Permission)
    ) && validated_harness_prompt;
    let is_elicitation = handle
        .runtime
        .active_prompt_kind
        .is_some_and(RuntimePromptKind::is_elicitation);
    let permission_owned = codex_permission_owned || harness_permission_owned;
    let attention_source = handle
        .info
        .provenance
        .attention_source
        .as_deref()
        .or(handle.info.metadata_source.as_deref());
    let single_key_policy = match handle.runtime.active_prompt_kind {
        Some(RuntimePromptKind::Permission) => permission_owned,
        Some(_) => false,
        None => {
            permission_owned
                || matches!(
                    (handle.active_work_hook, attention_source),
                    (false, Some("agent" | "process"))
                )
        }
    };
    let single_key_qualifies = input.printable_keystroke
        && handle.info.attention.as_deref() == Some("needs_you")
        && single_key_policy;
    let codex_mouse_click_qualifies = input.mouse_button_event
        && handle.active_work_hook
        && handle.info.attention.as_deref() == Some("needs_you")
        && permission_owned;
    let committed_input =
        input.line_completed || single_key_qualifies || codex_mouse_click_qualifies;
    let commit_working = !handle.runtime.terminal_status_active
        && !already_working
        && committed_input
        && attention_source != Some("user")
        && (!is_elicitation || (input.line_completed && validated_harness_prompt))
        && (!handle.active_work_hook
            || permission_owned
            || (input.line_completed
                && (matches!(
                    attention_source,
                    None | Some("unknown" | "debug" | "backend_inference" | "process" | "peon")
                ) || (is_elicitation && validated_harness_prompt))));
    CommittedInputDecision {
        codex_permission_owned,
        harness_permission_owned,
        is_elicitation,
        committed_input,
        commit_working,
    }
}

fn finish_input_fences(
    handle: &mut SessionHandle,
    next_generation: u64,
    accepted_at: chrono::DateTime<chrono::Utc>,
    output_boundary: Option<u64>,
    mark_committed: bool,
) {
    handle.runtime.input_generation = next_generation;
    handle.runtime.accepted_input_at = Some(accepted_at);
    if mark_committed {
        handle.runtime.committed_input_at = Some(accepted_at);
        handle.runtime.active_prompt_kind = None;
    }
    handle.runtime.min_peon_output_revision =
        output_boundary.unwrap_or(handle.runtime.peon_output_revision);
}

fn record_peon_output_touch(state: &Arc<AppState>, id: &str) {
    state
        .peon
        .last_output
        .write()
        .unwrap()
        .insert(id.to_string(), tokio::time::Instant::now());
}

fn apply_committed_working_to_handle(handle: &mut SessionHandle, fields: &ProcessTransitionFields) {
    apply_process_transition_to_handle(
        &mut handle.info,
        &mut handle.runtime.attention_owner,
        fields,
    );
    if let Some(tracker) = handle.runtime.native_approval.as_mut() {
        tracker.invalidate();
    }
    handle.pending_work_signal = None;
}

/// Atomically advances terminal input fences and, when the input qualifies as
/// committed working activity, persists and mirrors `ProcessTransition::CommittedWorking`
/// under the documented `workspace -> sessions` lock order (ADR 0027).
///
/// Caller contract (not enforced here): call only for a non-empty frame whose
/// delivery to the PTY was accepted. `line_completed` is whether this frame
/// finished a submitted line (`\r`/`\n` outside bracketed paste);
/// `printable_keystroke` is whether the frame appended a printable character
/// (`buf_grew`), gating the narrow single-key direct-commit path for
/// hook-sourced `needs_you` prompts without waiting on PTY output redraws.
/// Every accepted frame advances the input generation and idle baseline even
/// when it does not commit a working attention transition.
pub(crate) fn mark_committed_input_working(state: &Arc<AppState>, id: &str, input: CommittedInput) {
    let ws_guard = state.workspace.lock().unwrap();
    let mut sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get_mut(id) else {
        return;
    };
    if handle.info.lifecycle != "alive" {
        return;
    }
    let decision = evaluate_committed_input(handle, id, input);
    let Some(next_generation) = handle.runtime.input_generation.checked_add(1) else {
        tracing::warn!(session_id = %id, "input generation overflow");
        return;
    };
    let accepted_at = chrono::Utc::now();
    let cleared_auto_archive_warning = handle.runtime.record_interactive_activity(accepted_at);
    if decision.committed_input {
        // The PTY accepted this input before this function was called. Advance
        // both in-memory ordering fences before metadata I/O, so a queued or
        // timestamped pre-input prompt cannot be replayed if persistence fails.
        handle.runtime.committed_input_sequence =
            handle.runtime.committed_input_sequence.saturating_add(1);
        handle.runtime.committed_input_at = Some(accepted_at);
    }
    if !decision.commit_working {
        finish_input_fences(
            handle,
            next_generation,
            accepted_at,
            input.output_boundary,
            decision.committed_input,
        );
        drop(sessions);
        drop(ws_guard);
        if cleared_auto_archive_warning {
            state.updates.sessions_changed();
        }
        record_peon_output_touch(state, id);
        return;
    }
    let fields = process_transition_fields(ProcessTransition::CommittedWorking);
    if ws_guard.is_none() {
        // A detached test/runtime has no metadata store, so there is no
        // durable state to diverge from. Apply the exact same live-handle
        // transition and invalidation invariants as the workspace-backed path
        // (including `native_approval.invalidate()` per ADR 0027).
        apply_committed_working_to_handle(handle, &fields);
        finish_input_fences(
            handle,
            next_generation,
            accepted_at,
            input.output_boundary,
            true,
        );
        drop(sessions);
        drop(ws_guard);
        if cleared_auto_archive_warning {
            state.updates.sessions_changed();
        }
        record_peon_output_touch(state, id);
        return;
    }
    let ws = ws_guard.as_ref().expect("workspace checked above");
    let live_observed_status = handle.info.observed_status.clone();
    let live_attention = handle.info.attention.clone();
    let active_work_hook = handle.active_work_hook;
    let persisted = ws
        .metadata
        .mutate_session_optional(id, WriteScope::ATTENTION, |meta| {
            if meta.lifecycle != "alive" || meta.attention_source() == "user" {
                return Ok(false);
            }
            let matching_codex_permission = decision.codex_permission_owned
                && meta.observed_status.as_deref() == Some("waiting_for_input")
                && meta.attention.as_deref() == Some("needs_you")
                && meta.attention_source() == "agent"
                && matches!(meta.attention_origin(), "native_codex" | "harness_hook");
            let matching_harness_permission = decision.harness_permission_owned
                && meta.observed_status.as_deref() == Some("waiting_for_input")
                && meta.attention.as_deref() == Some("needs_you")
                && meta.attention_source() == "process"
                && meta.attention_origin() == "process";
            let matching_input_wait = !decision.is_elicitation
                && (!active_work_hook
                    || matches!(
                        meta.attention_source(),
                        "unknown" | "debug" | "backend_inference" | "process" | "peon"
                    ))
                && meta.observed_status == live_observed_status
                && meta.attention == live_attention
                && matches!(meta.observed_status.as_deref(), Some("waiting_for_input"));
            if !matching_codex_permission
                && !matching_harness_permission
                && !matching_input_wait
                && !metadata::source_priority::can_overwrite(
                    "process",
                    meta.attention_source(),
                    meta.attention_age_secs(),
                )
            {
                return Ok(false);
            }
            apply_process_transition_to_meta(meta, &fields);
            Ok(true)
        });
    let attention_committed = match persisted {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(error) => {
            tracing::warn!(session_id = %id, %error, "failed to persist input attention transition");
            false
        }
    };
    if !attention_committed {
        finish_input_fences(
            handle,
            next_generation,
            accepted_at,
            input.output_boundary,
            false,
        );
        drop(sessions);
        drop(ws_guard);
        if cleared_auto_archive_warning {
            state.updates.sessions_changed();
        }
        record_peon_output_touch(state, id);
        return;
    }
    apply_committed_working_to_handle(handle, &fields);
    finish_input_fences(
        handle,
        next_generation,
        accepted_at,
        input.output_boundary,
        true,
    );
    drop(sessions);
    drop(ws_guard);
    if cleared_auto_archive_warning {
        state.updates.sessions_changed();
    }
    record_peon_output_touch(state, id);
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn identical_accepted_write_revokes_opaque_ownership() {
        let mut owner = AttentionOwner::default();
        let first = owner.accepted_write().unwrap();
        owner.accepted_write();
        assert!(!owner.owns(&first));
        let replacement = AttentionOwner::default();
        assert!(!replacement.owns(&first));
    }

    #[test]
    fn attention_revision_exhaustion_never_reuses_a_token() {
        let mut owner = AttentionOwner::default();
        owner.revision = Some(u64::MAX - 1);
        let last = owner.accepted_write().unwrap();
        assert!(owner.owns(&last));
        assert!(owner.accepted_write().is_none());
        assert!(!owner.owns(&last));
        assert!(owner.accepted_write().is_none());
    }

    #[test]
    fn apply_live_attention_fields_sets_status_and_derives_attention_when_alive() {
        let mut info = bare_info("alive");
        apply_live_attention_fields(
            &mut info,
            &mut AttentionOwner::default(),
            "waiting_for_input",
            Some("hi"),
            "agent",
            1.0,
        );
        assert_eq!(info.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(info.attention.as_deref(), Some("needs_you"));
        assert_eq!(info.summary.as_deref(), Some("hi"));
        assert_eq!(info.provenance.attention_source.as_deref(), Some("agent"));
        assert_eq!(info.provenance.attention_confidence, Some(1.0));
    }

    #[test]
    fn apply_live_attention_fields_leaves_attention_untouched_when_not_alive() {
        let mut info = bare_info("dead");
        info.attention = Some("idle".to_string());
        apply_live_attention_fields(
            &mut info,
            &mut AttentionOwner::default(),
            "working",
            None,
            "process",
            1.0,
        );
        assert_eq!(info.observed_status.as_deref(), Some("working"));
        assert_eq!(info.attention.as_deref(), Some("idle"));
    }

    #[test]
    fn apply_live_attention_fields_leaves_summary_untouched_when_no_message() {
        let mut info = bare_info("alive");
        info.summary = Some("previous".to_string());
        apply_live_attention_fields(
            &mut info,
            &mut AttentionOwner::default(),
            "working",
            None,
            "process",
            1.0,
        );
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
        apply_process_transition_to_handle(&mut info, &mut AttentionOwner::default(), &fields);
        assert_eq!(info.observed_status.as_deref(), Some("working"));
        assert_eq!(info.attention.as_deref(), Some("working"));
        assert_eq!(info.provenance.attention_source.as_deref(), Some("process"));
        assert_eq!(info.provenance.attention_confidence, Some(1.0));
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
        apply_process_transition_to_handle(&mut info, &mut AttentionOwner::default(), &fields);
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
        assert_eq!(meta.provenance.attention_source.as_deref(), Some("process"));
        assert_eq!(meta.provenance.attention_confidence, Some(1.0));
        assert_eq!(meta.needs_user_input, None);
    }

    #[test]
    fn user_attention_is_not_replaced_by_process_mirror() {
        let mut info = bare_info("alive");
        info.provenance.attention_source = Some("user".into());
        info.observed_status = Some("blocked".into());
        let mut owner = AttentionOwner::default();
        let token = owner.snapshot().unwrap();
        let fields = process_transition_fields(ProcessTransition::CommittedWorking);
        apply_process_transition_to_handle(&mut info, &mut owner, &fields);
        assert_eq!(info.observed_status.as_deref(), Some("blocked"));
        assert!(owner.owns(&token));
    }

    fn live_handle(id: &str) -> SessionHandle {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = bare_info("alive");
        info.id = id.to_string();
        info.lifecycle_phase = "active".into();
        info.provenance.summary_source = Some("peon".into());
        info.provenance.work_metadata_updated_at = Some("before-work".into());
        let mut runtime = crate::runtime::session_runtime::SessionRuntime::detached_test();
        runtime.native_approval = Some(
            crate::runtime::codex_approval_application::NativeApprovalState::new(
                runtime.run_generation(),
            ),
        );
        runtime.active_prompt_kind = Some(RuntimePromptKind::Permission);
        SessionHandle {
            info,
            active_work_hook: false,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(32),
            scan_buf: String::new(),
            pending_work_signal: Some(crate::runtime::session_runtime::arm_pending_work_signal(
                "run tests",
                tokio::time::Instant::now(),
                std::time::Duration::from_secs(1),
            )),
            runtime,
            terminal_attached: false,
            resume_in_progress: true,
            capacity: crate::capacity_state::CapacityState::default(),
        }
    }

    fn sample_meta(id: &str) -> metadata::SessionMetadata {
        let mut meta = crate::test_support::test_session_metadata(
            id,
            "Session",
            "/workspace/s1",
            "running",
            "before",
            "after-activity",
        );
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.observed_status = Some("waiting_for_input".into());
        meta.attention = Some("needs_you".into());
        meta.needs_user_input = Some(true);
        meta.detected_question = Some("Approve command?".into());
        meta.suggested_options = Some(vec!["Yes".into(), "No".into()]);
        meta.work_phase = "verifying".into();
        meta.next_action = Some("Run tests".into());
        meta.summary = Some("Updated summary".into());
        meta.metadata_source = "agent".into();
        meta.metadata_confidence = 0.9;
        meta.provenance.attention_source = Some("agent".into());
        meta.provenance.attention_confidence = Some(1.0);
        meta.provenance.attention_origin = Some("direct_agent".into());
        meta.provenance.attention_updated_at = Some("after-attention".into());
        meta.provenance.summary_source = Some("agent".into());
        meta.provenance.summary_confidence = Some(0.95);
        meta.provenance.summary_observed_at = Some("after-summary".into());
        meta.provenance.work_metadata_updated_at = Some("after-work".into());
        meta
    }

    #[test]
    fn mirror_committed_attention_enforces_agent_patch_invariants() {
        let mut handle = live_handle("s-agent");
        let meta = sample_meta("s-agent");
        let before_owner = handle.runtime.attention_owner.snapshot().unwrap();
        let before_hook_rev = handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();

        mirror_committed_attention(&mut handle, &meta, AttentionMirrorOptions::agent_patch());

        assert!(!handle.runtime.attention_owner.owns(&before_owner));
        assert_eq!(
            handle.info.observed_status.as_deref(),
            Some("waiting_for_input")
        );
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(handle.info.needs_user_input, Some(true));
        assert_eq!(
            handle.info.detected_question.as_deref(),
            Some("Approve command?")
        );
        assert_eq!(
            handle.info.suggested_options.as_deref(),
            Some(&["Yes".to_string(), "No".to_string()][..])
        );
        assert_eq!(
            handle.info.provenance.attention_source.as_deref(),
            Some("agent")
        );
        assert_eq!(
            handle.info.provenance.attention_updated_at.as_deref(),
            Some("after-attention")
        );
        // AttentionOnly provenance sync preserves existing summary/work provenance.
        assert_eq!(
            handle.info.provenance.summary_source.as_deref(),
            Some("peon")
        );
        assert_eq!(
            handle.info.provenance.work_metadata_updated_at.as_deref(),
            Some("before-work")
        );
        assert_ne!(
            handle
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            before_hook_rev
        );
        assert!(handle.pending_work_signal.is_none());
        assert_eq!(handle.runtime.active_prompt_kind, None);
    }

    #[test]
    fn mirror_committed_attention_distinguishes_peon_prompt_and_codex_policies() {
        // Peon with attention_written=false must NOT revoke attention_owner or native_approval.
        let mut peon_noop = live_handle("s-peon-noop");
        let meta = sample_meta("s-peon-noop");
        let owner_before = peon_noop.runtime.attention_owner.snapshot().unwrap();
        let rev_before = peon_noop
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();
        mirror_committed_attention(&mut peon_noop, &meta, AttentionMirrorOptions::peon(false));
        assert!(peon_noop.runtime.attention_owner.owns(&owner_before));
        assert_eq!(
            peon_noop
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert!(peon_noop.pending_work_signal.is_some());
        assert_eq!(
            peon_noop.runtime.active_prompt_kind,
            Some(RuntimePromptKind::Permission)
        );
        assert_eq!(peon_noop.info.metadata_source.as_deref(), Some("agent"));
        assert_eq!(peon_noop.info.metadata_confidence, Some(0.9));

        // Peon with attention_written=true revokes attention_owner while preserving prompt kind.
        let mut peon_write = live_handle("s-peon-write");
        let owner_before = peon_write.runtime.attention_owner.snapshot().unwrap();
        mirror_committed_attention(&mut peon_write, &meta, AttentionMirrorOptions::peon(true));
        assert!(!peon_write.runtime.attention_owner.owns(&owner_before));
        assert_eq!(
            peon_write.info.provenance.summary_source.as_deref(),
            Some("agent")
        );

        // Prompt commit clears question fields, updates only attention provenance, sets prompt kind.
        let mut prompt = live_handle("s-prompt");
        prompt.info.needs_user_input = Some(true);
        prompt.info.detected_question = Some("stale".into());
        let owner_before = prompt.runtime.attention_owner.snapshot().unwrap();
        mirror_committed_attention(
            &mut prompt,
            &meta,
            AttentionMirrorOptions::prompt_commit(Some(RuntimePromptKind::Permission)),
        );
        assert!(!prompt.runtime.attention_owner.owns(&owner_before));
        assert_eq!(prompt.info.needs_user_input, None);
        assert_eq!(prompt.info.detected_question, None);
        assert_eq!(prompt.info.suggested_options, None);
        assert_eq!(
            prompt.info.provenance.summary_source.as_deref(),
            Some("peon")
        );
        assert_eq!(
            prompt.runtime.active_prompt_kind,
            Some(RuntimePromptKind::Permission)
        );

        // Codex hook transition clears question fields and pending_work_signal without invalidating
        // its caller-owned native_approval tracker.
        let mut codex = live_handle("s-codex");
        let rev_before = codex
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();
        mirror_committed_attention(
            &mut codex,
            &meta,
            AttentionMirrorOptions::codex_hook_transition(),
        );
        assert_eq!(codex.info.needs_user_input, None);
        assert!(codex.pending_work_signal.is_none());
        assert_eq!(
            codex
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );

        // Native terminal status copies question fields and full provenance, clears
        // pending_work_signal, and preserves native_approval and active_prompt_kind.
        let mut osc = live_handle("s-osc");
        let owner_before = osc.runtime.attention_owner.snapshot().unwrap();
        let rev_before = osc
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();
        mirror_committed_attention(
            &mut osc,
            &meta,
            AttentionMirrorOptions::native_terminal_status(),
        );
        assert!(!osc.runtime.attention_owner.owns(&owner_before));
        assert_eq!(osc.info.needs_user_input, Some(true));
        assert_eq!(osc.info.provenance.summary_source.as_deref(), Some("agent"));
        assert!(osc.pending_work_signal.is_none());
        assert_eq!(
            osc.runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert_eq!(
            osc.runtime.active_prompt_kind,
            Some(RuntimePromptKind::Permission)
        );
    }

    #[test]
    fn mirror_agent_metadata_patch_gates_attention_work_and_summary_scopes() {
        let mut handle = live_handle("s-patch");
        let meta = sample_meta("s-patch");
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        let rev_before = handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();

        // Work + summary without attention scope must leave attention_owner and native_approval intact.
        mirror_agent_metadata_patch(&mut handle, &meta, WriteScope::WORK, true);
        assert!(handle.runtime.attention_owner.owns(&owner_before));
        assert_eq!(
            handle
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert!(handle.pending_work_signal.is_some());
        assert_eq!(handle.info.work_phase, "verifying");
        assert_eq!(handle.info.next_action.as_deref(), Some("Run tests"));
        assert_eq!(handle.info.summary.as_deref(), Some("Updated summary"));
        assert_eq!(
            handle.info.last_activity_at.as_deref(),
            Some("after-activity")
        );

        // Attention scope revokes owner, invalidates native_approval, and clears signals.
        mirror_agent_metadata_patch(&mut handle, &meta, WriteScope::ATTENTION, false);
        assert!(!handle.runtime.attention_owner.owns(&owner_before));
        assert_ne!(
            handle
                .runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert!(handle.pending_work_signal.is_none());
        assert_eq!(handle.runtime.active_prompt_kind, None);
    }

    #[test]
    fn clear_live_attention_tuple_and_lifecycle_mirrors_enforce_invariants() {
        let mut handle = live_handle("s-life");
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.attention = Some("needs_you".into());
        handle.info.needs_user_input = Some(true);
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();

        let mut replacement_prov = metadata::MetadataProvenance::default();
        replacement_prov.attention_source = Some("process".into());
        clear_live_attention_tuple(&mut handle, Some(replacement_prov));
        assert!(!handle.runtime.attention_owner.owns(&owner_before));
        assert_eq!(handle.info.observed_status, None);
        assert_eq!(handle.info.attention, None);
        assert_eq!(handle.info.needs_user_input, None);
        assert_eq!(handle.runtime.active_prompt_kind, None);
        assert_eq!(
            handle.info.provenance.attention_source.as_deref(),
            Some("process")
        );

        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        mark_handle_ending(&mut handle, "ending-at".into());
        assert!(!handle.runtime.attention_owner.owns(&owner_before));
        assert_eq!(handle.info.lifecycle_phase, "ending");
        assert_eq!(handle.info.lifecycle, "stopping");
        assert_eq!(handle.info.last_activity_at.as_deref(), Some("ending-at"));

        handle.info.observed_status = Some("working".into());
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        refresh_ending_handle(&mut handle, "still-ending".into());
        assert!(!handle.runtime.attention_owner.owns(&owner_before));
        assert_eq!(handle.info.observed_status, None);
        assert_eq!(
            handle.info.last_activity_at.as_deref(),
            Some("still-ending")
        );

        let mut committed_meta = sample_meta("s-life");
        committed_meta.status = "ended".into();
        committed_meta.lifecycle_phase = "ended".into();
        committed_meta.lifecycle = "dead".into();
        committed_meta.attention = None;
        committed_meta.observed_status = None;
        committed_meta.final_observed_status_snapshot =
            Some(metadata::ObservedStatusSnapshotMetadata {
                value: Some("working".into()),
                source: "process".into(),
                confidence: Some(1.0),
                attention_origin: Some("process".into()),
                observed_at: Some("ended-snap".into()),
            });
        handle.resume_in_progress = true;
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        mirror_committed_lifecycle(&mut handle, &committed_meta, false);
        assert!(handle.runtime.attention_owner.owns(&owner_before));
        assert!(!handle.resume_in_progress);
        assert_eq!(
            handle.info.final_observed_status.as_deref(),
            Some("working")
        );

        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        mirror_committed_lifecycle(&mut handle, &committed_meta, true);
        assert!(!handle.runtime.attention_owner.owns(&owner_before));

        handle.resume_in_progress = true;
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        mirror_ended_lifecycle(&mut handle, "ended", Some("idle".into()), "ended-at".into());
        assert!(!handle.runtime.attention_owner.owns(&owner_before));
        assert_eq!(handle.info.lifecycle_phase, "ended");
        assert_eq!(handle.info.lifecycle, "dead");
        assert_eq!(handle.info.final_observed_status.as_deref(), Some("idle"));
        assert!(!handle.resume_in_progress);
    }

    #[test]
    fn mark_committed_input_working_persists_mirrors_and_invalidates_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let id = "s-committed-input";
        let mut meta = sample_meta(id);
        meta.provenance.attention_source = Some("process".into());
        meta.provenance.attention_origin = Some("process".into());
        {
            let ws = state.workspace.lock().unwrap();
            ws.as_ref().unwrap().metadata.write_session(&meta);
        }
        let mut handle = live_handle(id);
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.attention = Some("needs_you".into());
        handle.info.needs_user_input = Some(true);
        handle.info.provenance.attention_source = Some("process".into());
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        let rev_before = handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();
        state.sessions.lock().unwrap().insert(id.into(), handle);

        mark_committed_input_working(
            &state,
            id,
            CommittedInput {
                output_boundary: Some(7),
                line_completed: true,
                printable_keystroke: false,
                mouse_button_event: false,
            },
        );

        let sessions = state.sessions.lock().unwrap();
        let live = sessions.get(id).unwrap();
        assert!(!live.runtime.attention_owner.owns(&owner_before));
        assert_ne!(
            live.runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert_eq!(live.info.observed_status.as_deref(), Some("working"));
        assert_eq!(live.info.attention.as_deref(), Some("working"));
        assert_eq!(live.info.needs_user_input, None);
        assert!(live.pending_work_signal.is_none());
        assert_eq!(live.runtime.active_prompt_kind, None);
        assert_eq!(live.runtime.input_generation, 1);
        assert_eq!(live.runtime.committed_input_sequence, 1);
        assert_eq!(live.runtime.min_peon_output_revision, 7);
        drop(sessions);

        let stored = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .read_session(id)
            .unwrap();
        assert_eq!(stored.observed_status.as_deref(), Some("working"));
        assert_eq!(stored.attention.as_deref(), Some("working"));
        assert_eq!(stored.needs_user_input, None);
    }

    #[test]
    fn mark_committed_input_working_detached_workspace_invalidates_native_approval() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        *state.workspace.lock().unwrap() = None;
        let id = "s-detached-committed-input";
        let mut handle = live_handle(id);
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.attention = Some("needs_you".into());
        handle.info.needs_user_input = Some(true);
        handle.info.provenance.attention_source = Some("process".into());
        let owner_before = handle.runtime.attention_owner.snapshot().unwrap();
        let rev_before = handle
            .runtime
            .native_approval
            .as_ref()
            .unwrap()
            .hook_revision();
        state.sessions.lock().unwrap().insert(id.into(), handle);

        mark_committed_input_working(
            &state,
            id,
            CommittedInput {
                output_boundary: Some(11),
                line_completed: true,
                printable_keystroke: false,
                mouse_button_event: false,
            },
        );

        let sessions = state.sessions.lock().unwrap();
        let live = sessions.get(id).unwrap();
        assert!(!live.runtime.attention_owner.owns(&owner_before));
        assert_ne!(
            live.runtime
                .native_approval
                .as_ref()
                .unwrap()
                .hook_revision(),
            rev_before
        );
        assert_eq!(live.info.observed_status.as_deref(), Some("working"));
        assert_eq!(live.info.attention.as_deref(), Some("working"));
        assert_eq!(live.info.needs_user_input, None);
        assert!(live.pending_work_signal.is_none());
        assert_eq!(live.runtime.active_prompt_kind, None);
        assert_eq!(live.runtime.input_generation, 1);
        assert_eq!(live.runtime.committed_input_sequence, 1);
        assert_eq!(live.runtime.min_peon_output_revision, 11);
    }
}
