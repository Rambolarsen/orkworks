//! Runtime-local native approval application. All mutations hold workspace → sessions.
use super::codex_approval::{ApprovalReducer, Effect, Fence, HookEvent};
use super::observed_status::{apply_live_attention_fields, AttentionWriteToken};
use crate::metadata::{MetadataStore, SessionWriteToken};
use crate::{AppState, SessionHandle};
use std::sync::Arc;
use std::time::Instant;

// MVP explicitly supports uncoordinated agent JSON writes. A process-local
// revision cannot make a file replacement atomic with that producer.
const COORDINATED_METADATA_WRITERS: bool = false;

pub(crate) struct NativeApprovalState {
    reducer: ApprovalReducer,
    fence: Fence,
    root: Option<String>,
    turn: Option<String>,
    hook_revision: Option<u64>,
    last_hook_at: Option<chrono::DateTime<chrono::Utc>>,
    ownership: Option<(AttentionWriteToken, SessionWriteToken)>,
    live_revision: Option<AttentionWriteToken>,
    committed_input_sequence: u64,
}
impl std::fmt::Debug for NativeApprovalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeApprovalState { .. }")
    }
}
impl NativeApprovalState {
    pub(crate) fn new(generation: u64) -> Self {
        let fence = Fence {
            runtime_generation: generation,
            ..Fence::default()
        };
        Self {
            reducer: ApprovalReducer::new(fence),
            fence,
            root: None,
            turn: None,
            hook_revision: Some(0),
            last_hook_at: None,
            ownership: None,
            live_revision: None,
            committed_input_sequence: 0,
        }
    }
}

/// Private mailbox scalars only. Never include the reporting capability.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeHookReport {
    pub(crate) root_id: String,
    pub(crate) turn_id: Option<String>,
    pub(crate) tool_use_id: Option<String>,
    pub(crate) event: HookEvent,
    pub(crate) observed_at: String,
    pub(crate) hook_fingerprint: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HookReportOutcome {
    Accepted,
    Retry,
    Rejected,
}

fn expected_fingerprint() -> Option<String> {
    let reporter = dirs::home_dir()?
        .join(".orkworks/hook-scripts")
        .join(crate::harness::integrations::ReporterPlatform::current().asset_name());
    crate::harness::integrations::current_codex_hook_fingerprint(&reporter).ok()
}

fn bounded_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn accepted_root(meta: &crate::metadata::SessionMetadata) -> Option<&str> {
    (meta.harness == "codex" && meta.harness_session_id_source.as_deref() == Some("codex_hook"))
        .then_some(meta.resume.as_ref()?.harness_session_id.as_deref()?)
        .filter(|root| bounded_id(root))
}

impl NativeApprovalState {
    pub(crate) fn invalidate(&mut self) {
        self.hook_revision = self.hook_revision.and_then(|r| r.checked_add(1));
        self.reducer.invalidate(self.fence);
        self.ownership = None;
        self.live_revision = None;
    }

    fn sync(&mut self, handle: &SessionHandle, meta: &crate::metadata::SessionMetadata) {
        let root = accepted_root(meta).map(str::to_owned);
        if self.root != root {
            self.root = root;
            match self.fence.identity_revision.checked_add(1) {
                Some(revision) => self.fence.identity_revision = revision,
                None => self.hook_revision = None,
            }
            self.turn = None;
            self.last_hook_at = None;
            self.invalidate();
        }
        if self.fence.input_revision != handle.runtime.input_generation
            || self.committed_input_sequence != handle.runtime.committed_input_sequence
            || self
                .live_revision
                .as_ref()
                .is_some_and(|token| !handle.runtime.attention_owner.owns(token))
        {
            self.fence.input_revision = handle.runtime.input_generation;
            self.committed_input_sequence = handle.runtime.committed_input_sequence;
            self.invalidate();
        }
        self.live_revision = handle.runtime.attention_owner.snapshot();
        if meta.metadata_source == "user"
            || handle.info.metadata_source.as_deref() == Some("user")
            || meta.needs_user_input == Some(true)
            || meta.detected_question.is_some()
            || handle.info.needs_user_input == Some(true)
            || handle.info.detected_question.is_some()
        {
            self.reducer.disconnected();
        }
    }
}

/// The relay must retain `first_receipt` unchanged across Retry. Reports are
/// bounded scalar evidence, never authority asserted by their caller.
pub(crate) fn report_hook(
    state: &Arc<AppState>,
    id: &str,
    token: &str,
    generation: u64,
    first_receipt: Instant,
    report: &NativeHookReport,
) -> HookReportOutcome {
    if !super::terminal_runtime::verify_workflow_report_token(id, token) {
        return HookReportOutcome::Rejected;
    }
    let fingerprint = expected_fingerprint();
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return HookReportOutcome::Rejected;
    };
    let mut sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get_mut(id).filter(|h| {
        h.runtime.run_generation() == generation
            && h.info.lifecycle == "alive"
            && h.info.lifecycle_phase == "active"
            && h.info.harness_id.as_deref().or(h.info.harness.as_deref()) == Some("codex")
    }) else {
        return HookReportOutcome::Rejected;
    };
    if !super::terminal_runtime::verify_workflow_report_token(id, token) {
        return HookReportOutcome::Rejected;
    }
    let Some(mut tracker) = handle.runtime.native_approval.take() else {
        return HookReportOutcome::Rejected;
    };
    let outcome = (|| {
        let Some(mut meta) = workspace.metadata.read_session(id) else {
            return HookReportOutcome::Retry;
        };
        tracker.sync(handle, &meta);
        let now = Instant::now();
        let observed = chrono::DateTime::parse_from_rfc3339(&report.observed_at)
            .ok()
            .map(|t| t.with_timezone(&chrono::Utc));
        if fingerprint.as_deref() != Some(report.hook_fingerprint.as_str())
            || !crate::metadata::valid_hook_fingerprint(&report.hook_fingerprint)
            || !bounded_id(&report.root_id)
            || report.turn_id.as_deref().is_some_and(|id| !bounded_id(id))
            || report
                .tool_use_id
                .as_deref()
                .is_some_and(|id| !bounded_id(id))
            || report.observed_at.len() > 64
            || first_receipt > now
            || observed.is_none()
        {
            tracker.reducer.disconnected();
            tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
            return HookReportOutcome::Rejected;
        }
        let observed = observed.expect("validated timestamp");
        let age = chrono::Utc::now().signed_duration_since(observed);
        if age < chrono::Duration::seconds(-1) || age > chrono::Duration::seconds(30) {
            tracker.reducer.disconnected();
            tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
            return HookReportOutcome::Rejected;
        }
        let Some(root) = tracker.root.as_deref() else {
            return HookReportOutcome::Retry;
        };
        if root != report.root_id || meta.lifecycle != "alive" {
            tracker.invalidate();
            return HookReportOutcome::Rejected;
        }
        // UUID filename order is not event order. Ties and reversed delivery
        // revoke correlation; an authenticated permission remains conservative.
        if handle
            .runtime
            .accepted_input_at
            .is_some_and(|input| observed <= input)
        {
            tracker.reducer.disconnected();
            tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
            return HookReportOutcome::Rejected;
        }
        let reordered = tracker
            .last_hook_at
            .is_some_and(|previous| observed <= previous)
            || handle
                .runtime
                .last_hook_attention_at
                .is_some_and(|previous| observed < previous);
        if reordered {
            tracker.reducer.disconnected();
            tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
            if report.event != HookEvent::PermissionRequest {
                return HookReportOutcome::Rejected;
            }
        }
        let boundary = matches!(report.event, HookEvent::Stop | HookEvent::UserPromptSubmit);
        let activation = report.event == HookEvent::PermissionRequest && !handle.active_work_hook;
        if (activation || boundary)
            && meta.metadata_source != "user"
            && handle.info.metadata_source.as_deref() != Some("user")
        {
            meta.observed_status = match report.event {
                HookEvent::Stop => Some("idle".into()),
                HookEvent::UserPromptSubmit => Some("working".into()),
                _ => None,
            };
            meta.attention = meta.observed_status.clone();
            meta.needs_user_input = None;
            meta.detected_question = None;
            meta.suggested_options = None;
            meta.metadata_source = "codex_hook".into();
            meta.metadata_confidence = 1.0;
            meta.last_activity = crate::workspace_runtime::iso_now();
            if workspace.metadata.try_write_session(&meta).is_err() {
                return HookReportOutcome::Retry;
            }
            handle.runtime.attention_owner.accepted_write();
            handle.info.observed_status = meta.observed_status.clone();
            handle.info.attention = meta.attention.clone();
            handle.info.needs_user_input = None;
            handle.info.detected_question = None;
            handle.info.suggested_options = None;
            handle.info.metadata_source = Some("codex_hook".into());
            handle.info.metadata_confidence = Some(1.0);
            handle.pending_work_signal = None;
        }
        if activation || boundary {
            handle.active_work_hook = true;
        }
        if report.turn_id.is_none() {
            tracker.reducer.disconnected();
        }
        if !boundary && tracker.turn.is_some() && tracker.turn != report.turn_id {
            // Missing or reordered turn evidence is ambiguity, not a trusted
            // turn boundary that may discard an outstanding permission.
            tracker.reducer.disconnected();
        }
        if boundary && tracker.turn != report.turn_id {
            match tracker.fence.turn_revision.checked_add(1) {
                Some(revision) => tracker.fence.turn_revision = revision,
                None => tracker.hook_revision = None,
            }
            tracker.invalidate();
        }
        if tracker.turn.is_none() || boundary {
            tracker.turn.clone_from(&report.turn_id);
        }
        tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
        tracker.last_hook_at = Some(
            tracker
                .last_hook_at
                .map_or(observed, |previous| previous.max(observed)),
        );
        handle.runtime.last_hook_attention_at = tracker.last_hook_at;
        tracker.live_revision = handle.runtime.attention_owner.snapshot();
        if tracker.hook_revision.is_none() || !tracker.reducer.assistance_available() {
            tracker.reducer.disconnected();
        }
        let effects =
            tracker
                .reducer
                .hook(report.event, report.tool_use_id.as_deref(), first_receipt);
        if report.event == HookEvent::PermissionRequest
            && (tracker.hook_revision.is_none() || !tracker.reducer.assistance_available())
        {
            // Checked-counter exhaustion permanently removes correlation
            // authority; ordinary conservative permission reporting still works.
            if meta.metadata_source != "user"
                && handle.info.metadata_source.as_deref() != Some("user")
                && meta.needs_user_input != Some(true)
                && meta.detected_question.is_none()
                && handle.info.needs_user_input != Some(true)
                && handle.info.detected_question.is_none()
            {
                meta.observed_status = Some("waiting_for_input".into());
                meta.attention = Some("needs_you".into());
                meta.metadata_source = "codex_hook".into();
                meta.metadata_confidence = 1.0;
                if workspace.metadata.try_write_session(&meta).is_err() {
                    return HookReportOutcome::Retry;
                }
                apply_live_attention_fields(
                    &mut handle.info,
                    &mut handle.runtime.attention_owner,
                    "waiting_for_input",
                    None,
                    "codex_hook",
                    1.0,
                );
            }
            tracker.ownership = None;
            tracker.live_revision = handle.runtime.attention_owner.snapshot();
            return HookReportOutcome::Accepted;
        }
        for effect in effects {
            apply_effect(
                &workspace.metadata,
                handle,
                &mut tracker,
                &effect,
                Instant::now,
                COORDINATED_METADATA_WRITERS,
            );
        }
        let effects = tracker.reducer.tick(Instant::now());
        for effect in effects {
            apply_effect(
                &workspace.metadata,
                handle,
                &mut tracker,
                &effect,
                Instant::now,
                COORDINATED_METADATA_WRITERS,
            );
        }
        tracker.live_revision = handle.runtime.attention_owner.snapshot();
        HookReportOutcome::Accepted
    })();
    handle.runtime.native_approval = Some(tracker);
    outcome
}

/// A malformed/ambiguous mailbox batch cannot preserve correlation authority.
/// The relay calls this only with its non-persisted runtime reporting capability.
pub(crate) fn revoke_correlation(state: &Arc<AppState>, id: &str, token: &str, generation: u64) {
    let _workspace = state.workspace.lock().unwrap();
    let mut sessions = state.sessions.lock().unwrap();
    if !super::terminal_runtime::verify_workflow_report_token(id, token) {
        return;
    }
    if let Some(tracker) = sessions
        .get_mut(id)
        .filter(|h| h.runtime.run_generation() == generation)
        .and_then(|h| h.runtime.native_approval.as_mut())
    {
        tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
        tracker.reducer.disconnected();
    }
}

pub(crate) struct ObservationSnapshot {
    pub(crate) root: String,
    fence: Fence,
    hook_revision: u64,
    live_write: AttentionWriteToken,
}

pub(crate) fn begin_observation(
    state: &Arc<AppState>,
    id: &str,
    generation: u64,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Option<ObservationSnapshot> {
    let workspace = state.workspace.lock().unwrap();
    let workspace = workspace.as_ref()?;
    let mut sessions = state.sessions.lock().unwrap();
    let handle = sessions.get_mut(id).filter(|h| {
        h.runtime.run_generation() == generation
            && h.info.lifecycle == "alive"
            && h.info.lifecycle_phase == "active"
    })?;
    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
        return None;
    }
    let mut tracker = handle.runtime.native_approval.take()?;
    let result = (|| {
        let meta = workspace.metadata.read_session(id)?;
        tracker.sync(handle, &meta);
        for effect in tracker.reducer.tick(Instant::now()) {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return None;
            }
            apply_effect(
                &workspace.metadata,
                handle,
                &mut tracker,
                &effect,
                Instant::now,
                COORDINATED_METADATA_WRITERS,
            );
        }
        tracker.live_revision = handle.runtime.attention_owner.snapshot();
        Some(ObservationSnapshot {
            root: tracker.root.clone()?,
            fence: tracker.fence,
            hook_revision: tracker.hook_revision?,
            live_write: handle.runtime.attention_owner.snapshot()?,
        })
    })();
    handle.runtime.native_approval = Some(tracker);
    result
}

pub(crate) fn finish_observation(
    state: &Arc<AppState>,
    id: &str,
    snapshot: ObservationSnapshot,
    result: Result<super::codex_native::NativeObservation, super::codex_native::NativeError>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> bool {
    let workspace = state.workspace.lock().unwrap();
    let Some(workspace) = workspace.as_ref() else {
        return false;
    };
    let mut sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get_mut(id).filter(|h| {
        h.runtime.run_generation() == snapshot.fence.runtime_generation
            && h.info.lifecycle == "alive"
            && h.info.lifecycle_phase == "active"
    }) else {
        return false;
    };
    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
        return false;
    }
    let Some(mut tracker) = handle.runtime.native_approval.take() else {
        return false;
    };
    let applied = (|| {
        let Some(meta) = workspace.metadata.read_session(id) else {
            return false;
        };
        tracker.sync(handle, &meta);
        if tracker.fence != snapshot.fence
            || tracker.hook_revision != Some(snapshot.hook_revision)
            || tracker.root.as_deref() != Some(&snapshot.root)
            || !handle.runtime.attention_owner.owns(&snapshot.live_write)
        {
            return false;
        }
        let now = Instant::now();
        let effects = match result {
            Ok(observation) => tracker.reducer.observe(
                observation.status,
                observation.complete_singleton_root,
                observation.started_at,
                now,
            ),
            Err(_) => {
                tracker.reducer.disconnected();
                tracker.reducer.tick(now)
            }
        };
        for effect in effects {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return false;
            }
            apply_effect(
                &workspace.metadata,
                handle,
                &mut tracker,
                &effect,
                Instant::now,
                COORDINATED_METADATA_WRITERS,
            );
        }
        tracker.live_revision = handle.runtime.attention_owner.snapshot();
        true
    })();
    handle.runtime.native_approval = Some(tracker);
    applied
}

#[derive(Debug, PartialEq, Eq)]
enum ApplyOutcome {
    Applied,
    PersistFailed,
    Rejected,
}

fn reject(tracker: &mut NativeApprovalState) -> ApplyOutcome {
    tracker.reducer.invalidate(tracker.fence);
    tracker.ownership = None;
    ApplyOutcome::Rejected
}

fn apply_effect(
    store: &MetadataStore,
    handle: &mut SessionHandle,
    tracker: &mut NativeApprovalState,
    effect: &Effect,
    clock: impl Fn() -> Instant,
    coordinated_writers: bool,
) -> ApplyOutcome {
    if !tracker.reducer.accepts(effect)
        || handle.runtime.run_generation() != tracker.fence.runtime_generation
        || handle.runtime.input_generation != tracker.fence.input_revision
        || handle.runtime.committed_input_sequence != tracker.committed_input_sequence
        || !handle.active_work_hook
        || handle.info.lifecycle != "alive"
        || handle.info.lifecycle_phase != "active"
    {
        return reject(tracker);
    }
    let Some(mut meta) = store.read_session(&handle.info.id) else {
        return reject(tracker);
    };
    if meta.lifecycle != "alive"
        || meta.metadata_source == "user"
        || handle.info.metadata_source.as_deref() == Some("user")
        || meta.needs_user_input == Some(true)
        || meta.detected_question.is_some()
        || meta.suggested_options.is_some()
        || handle.info.needs_user_input == Some(true)
        || handle.info.detected_question.is_some()
        || handle.info.suggested_options.is_some()
    {
        return reject(tracker);
    }
    match effect {
        Effect::ShowWait { .. } => {
            if meta.attention.as_deref() == Some("needs_you")
                || handle.info.attention.as_deref() == Some("needs_you")
            {
                return reject(tracker);
            }
            meta.observed_status = Some("waiting_for_input".into());
            meta.attention = Some("needs_you".into());
            meta.metadata_source = "codex_hook".into();
            meta.metadata_confidence = 1.0;
            meta.last_activity = crate::workspace_runtime::iso_now();
            let durable = match store.try_write_session_owned(&meta) {
                Ok(token) => token,
                Err(_) => return ApplyOutcome::PersistFailed,
            };
            apply_live_attention_fields(
                &mut handle.info,
                &mut handle.runtime.attention_owner,
                "waiting_for_input",
                None,
                "codex_hook",
                1.0,
            );
            let live = handle.runtime.attention_owner.snapshot();
            tracker.ownership = live.zip(durable);
            tracker.reducer.acknowledge_show(effect);
            ApplyOutcome::Applied
        }
        Effect::ClearWait { .. } => {
            // This gate is independent of executable/version compatibility.
            if !coordinated_writers {
                tracker.reducer.disconnected();
                return ApplyOutcome::Rejected;
            }
            if !tracker.reducer.clear_is_fresh(clock()) {
                return reject(tracker);
            }
            let Some((live, durable)) = tracker.ownership.as_ref() else {
                return reject(tracker);
            };
            if !handle.runtime.attention_owner.owns(live)
                || handle.info.needs_user_input.is_some()
                || meta.needs_user_input.is_some()
                || handle.info.observed_status.as_deref() != Some("waiting_for_input")
                || handle.info.attention.as_deref() != Some("needs_you")
                || handle.info.metadata_source.as_deref() != Some("codex_hook")
                || handle.info.metadata_confidence != Some(1.0)
                || meta.observed_status.as_deref() != Some("waiting_for_input")
                || meta.attention.as_deref() != Some("needs_you")
                || meta.metadata_source != "codex_hook"
                || meta.metadata_confidence != 1.0
            {
                return reject(tracker);
            }
            meta.observed_status = Some("working".into());
            meta.attention = Some("working".into());
            meta.last_activity = crate::workspace_runtime::iso_now();
            match store.try_write_session_if_owned_when(&meta, durable, || {
                tracker.reducer.clear_is_fresh(clock())
            }) {
                Err(_) => return ApplyOutcome::PersistFailed,
                Ok(false) => return reject(tracker),
                Ok(true) => {}
            }
            apply_live_attention_fields(
                &mut handle.info,
                &mut handle.runtime.attention_owner,
                "working",
                None,
                "codex_hook",
                1.0,
            );
            tracker.reducer.acknowledge_clear(effect);
            tracker.ownership = None;
            ApplyOutcome::Applied
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::codex_approval::NativeStatus;
    use super::*;
    use crate::test_support::{test_session_info, test_session_metadata};
    use std::time::Duration;

    fn fixture() -> (
        tempfile::TempDir,
        MetadataStore,
        SessionHandle,
        NativeApprovalState,
        Instant,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta =
            test_session_metadata("native-owner", "native", "/tmp", "running", "now", "now");
        meta.harness = "codex".into();
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        store.write_session(&meta);
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let handle = SessionHandle {
            info: test_session_info("native-owner", "native", "/tmp", "running", "now"),
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime: super::super::session_runtime::SessionRuntime::detached_test(),
            terminal_attached: false,
            resume_in_progress: false,
            capacity: crate::capacity_state::CapacityState::default(),
            active_work_hook: true,
        };
        let t = Instant::now();
        let mut tracker = NativeApprovalState::new(handle.runtime.run_generation());
        tracker.reducer.observe(NativeStatus::Active, true, t, t);
        tracker.reducer.hook(HookEvent::PreToolUse, Some("tool"), t);
        tracker.reducer.hook(HookEvent::PermissionRequest, None, t);
        (dir, store, handle, tracker, t)
    }

    #[test]
    fn stale_or_competing_writes_cannot_clear_the_owned_tuple() {
        for competitor in 0..7 {
            let (_dir, store, mut handle, mut tracker, t) = fixture();
            let at = t + Duration::from_secs(2);
            let show = tracker
                .reducer
                .observe(NativeStatus::ApprovalPending, true, at, at)[0];
            assert_eq!(
                apply_effect(&store, &mut handle, &mut tracker, &show, || at, true),
                ApplyOutcome::Applied
            );
            let next = at + Duration::from_millis(100);
            let clear = tracker
                .reducer
                .observe(NativeStatus::Active, true, next, next)[0];
            let mut meta = store.read_session(&handle.info.id).unwrap();
            match competitor {
                0 => {
                    handle.runtime.attention_owner.accepted_write();
                }
                1 => store.try_write_session(&meta).unwrap(),
                2 => {
                    meta.metadata_source = "user".into();
                    store.try_write_session(&meta).unwrap();
                }
                3 => {
                    handle.info.needs_user_input = Some(true);
                }
                4 => {
                    handle.runtime.input_generation += 1;
                }
                5 => {
                    handle.runtime = super::super::session_runtime::SessionRuntime::detached_test();
                }
                _ => {}
            }
            let commit = if competitor == 6 {
                next + Duration::from_millis(301)
            } else {
                next
            };
            assert_eq!(
                apply_effect(&store, &mut handle, &mut tracker, &clear, || commit, true),
                ApplyOutcome::Rejected,
                "competitor {competitor}"
            );
            assert_eq!(
                store
                    .read_session(&handle.info.id)
                    .unwrap()
                    .attention
                    .as_deref(),
                Some("needs_you")
            );
            assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
            assert!(!tracker.reducer.accepts(&clear));
        }
    }

    #[cfg(unix)]
    #[test]
    fn metadata_read_delay_counts_toward_clear_commit_freshness() {
        use std::io::Write;
        use std::os::unix::ffi::OsStrExt;

        let (_dir, store, mut handle, _, _) = fixture();
        let entry = Instant::now();
        let start = entry - Duration::from_secs(3);
        let mut tracker = NativeApprovalState::new(handle.runtime.run_generation());
        tracker
            .reducer
            .observe(NativeStatus::Active, true, start, start);
        tracker
            .reducer
            .hook(HookEvent::PreToolUse, Some("tool"), start);
        tracker
            .reducer
            .hook(HookEvent::PermissionRequest, None, start);
        let pending = entry - Duration::from_millis(250);
        let show = tracker
            .reducer
            .observe(NativeStatus::ApprovalPending, true, pending, pending)[0];
        assert_eq!(
            apply_effect(&store, &mut handle, &mut tracker, &show, || pending, true),
            ApplyOutcome::Applied
        );
        let rpc_started = entry - Duration::from_millis(200);
        let clear = tracker
            .reducer
            .observe(NativeStatus::Active, true, rpc_started, entry)[0];
        let path = store
            .sessions_dir()
            .join(format!("{}.json", handle.info.id));
        let original = std::fs::read(&path).unwrap();
        // A FIFO makes the actual read_session call wait before staging starts.
        // No metadata implementation hooks or sleeps in production are needed.
        std::fs::remove_file(&path).unwrap();
        let fifo = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let writer_path = path.clone();
        let writer_bytes = original.clone();
        let writer = std::thread::spawn(move || {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(writer_path)
                .unwrap();
            std::thread::sleep(Duration::from_millis(200));
            file.write_all(&writer_bytes).unwrap();
        });
        let result = apply_effect(
            &store,
            &mut handle,
            &mut tracker,
            &clear,
            Instant::now,
            true,
        );
        writer.join().unwrap();
        // Restore a regular fixture file before checking durable state; a
        // rejected clear must have left the FIFO untouched instead of renaming.
        use std::os::unix::fs::FileTypeExt;
        let rejected_before_rename = std::fs::metadata(&path).unwrap().file_type().is_fifo();
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, original).unwrap();
        assert_eq!(result, ApplyOutcome::Rejected);
        assert!(rejected_before_rename);
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(
            store
                .read_session(&handle.info.id)
                .unwrap()
                .attention
                .as_deref(),
            Some("needs_you")
        );
        assert!(!tracker.reducer.accepts(&clear));
    }

    #[test]
    fn production_clear_gate_preserves_wait_even_with_valid_owned_resolution() {
        let (_dir, store, mut handle, mut tracker, t) = fixture();
        let at = t + Duration::from_secs(2);
        let show = tracker
            .reducer
            .observe(NativeStatus::ApprovalPending, true, at, at)[0];
        assert_eq!(
            apply_effect(
                &store,
                &mut handle,
                &mut tracker,
                &show,
                || at,
                COORDINATED_METADATA_WRITERS
            ),
            ApplyOutcome::Applied
        );
        let next = at + Duration::from_millis(100);
        let clear = tracker
            .reducer
            .observe(NativeStatus::Active, true, next, next)[0];
        assert_eq!(
            apply_effect(
                &store,
                &mut handle,
                &mut tracker,
                &clear,
                || next,
                COORDINATED_METADATA_WRITERS
            ),
            ApplyOutcome::Rejected
        );
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        assert_eq!(
            store
                .read_session(&handle.info.id)
                .unwrap()
                .attention
                .as_deref(),
            Some("needs_you")
        );
    }

    fn callback_fixture() -> (
        tempfile::TempDir,
        Arc<AppState>,
        u64,
        NativeHookReport,
        Instant,
    ) {
        let (dir, _store, mut handle, _tracker, t) = fixture();
        let session_id = format!("callback-{}", uuid::Uuid::new_v4());
        handle.info.id = session_id.clone();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let generation = handle.runtime.run_generation();
        handle.info.harness = Some("codex".into());
        handle.info.observed_status = Some("waiting_for_input".into());
        handle.info.attention = Some("needs_you".into());
        handle.info.needs_user_input = Some(true);
        handle.info.detected_question = Some("old question".into());
        handle.info.suggested_options = Some(vec!["old option".into()]);
        handle.active_work_hook = false;
        handle.runtime.native_approval = Some(NativeApprovalState::new(generation));
        let mut meta =
            test_session_metadata(&session_id, "native", "/tmp", "running", "now", "now");
        meta.harness = "codex".into();
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.attention = handle.info.attention.clone();
        meta.observed_status = handle.info.observed_status.clone();
        meta.needs_user_input = handle.info.needs_user_input;
        meta.detected_question = handle.info.detected_question.clone();
        meta.suggested_options = handle.info.suggested_options.clone();
        meta.resume = Some(crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::Exact,
            harness_session_id: Some("root-id".into()),
            latest_fallback: false,
            last_seen_at: None,
        });
        meta.harness_session_id_source = Some("codex_hook".into());
        state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .write_session(&meta);
        state
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), handle);
        super::super::terminal_runtime::set_workflow_report_token(
            &session_id,
            "test-report-token".into(),
        );
        let report = NativeHookReport {
            root_id: "root-id".into(),
            turn_id: Some("turn".into()),
            tool_use_id: None,
            event: HookEvent::PermissionRequest,
            observed_at: chrono::Utc::now().to_rfc3339(),
            hook_fingerprint: expected_fingerprint().unwrap(),
        };
        (dir, state, generation, report, t)
    }

    #[test]
    fn authenticated_permission_activates_and_clears_prior_tuple_before_grace() {
        let (_dir, state, generation, report, t) = callback_fixture();
        let id = state
            .sessions
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        assert_eq!(
            report_hook(&state, &id, "test-report-token", generation, t, &report),
            HookReportOutcome::Accepted
        );
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions[&id];
        assert!(handle.active_work_hook);
        assert!(handle.info.attention.is_none());
        assert!(handle.info.observed_status.is_none());
        assert!(handle.info.needs_user_input.is_none());
        assert!(handle.info.detected_question.is_none());
        assert!(handle.info.suggested_options.is_none());
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn cancelled_observer_cannot_apply_a_response_already_waiting_for_locks() {
        let (_dir, state, generation, report, t) = callback_fixture();
        let id = state
            .sessions
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        assert_eq!(
            report_hook(&state, &id, "test-report-token", generation, t, &report),
            HookReportOutcome::Accepted
        );
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let snapshot = begin_observation(&state, &id, generation, &cancelled).unwrap();
        cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(!finish_observation(
            &state,
            &id,
            snapshot,
            Ok(super::super::codex_native::NativeObservation {
                status: NativeStatus::Active,
                complete_singleton_root: true,
                started_at: Instant::now(),
            }),
            &cancelled
        ));
        assert!(begin_observation(&state, &id, generation, &cancelled).is_none());
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn hook_input_reset_and_root_changes_discard_inflight_rpc_results() {
        for boundary in 0..6 {
            let (_dir, state, generation, report, t) = callback_fixture();
            let id = state
                .sessions
                .lock()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .clone();
            assert_eq!(
                report_hook(&state, &id, "test-report-token", generation, t, &report),
                HookReportOutcome::Accepted
            );
            let cancelled = std::sync::atomic::AtomicBool::new(false);
            let snapshot = begin_observation(&state, &id, generation, &cancelled).unwrap();
            match boundary {
                0 => {
                    let mut sessions = state.sessions.lock().unwrap();
                    let tracker = sessions
                        .get_mut(&id)
                        .unwrap()
                        .runtime
                        .native_approval
                        .as_mut()
                        .unwrap();
                    tracker.hook_revision = tracker.hook_revision.map(|r| r + 1);
                }
                1 => {
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get_mut(&id)
                        .unwrap()
                        .runtime
                        .input_generation += 1
                }
                2 => {
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get_mut(&id)
                        .unwrap()
                        .runtime
                        .committed_input_sequence += 1
                }
                3 => state
                    .sessions
                    .lock()
                    .unwrap()
                    .get_mut(&id)
                    .unwrap()
                    .runtime
                    .native_approval
                    .as_mut()
                    .unwrap()
                    .invalidate(),
                4 => {
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get_mut(&id)
                        .unwrap()
                        .runtime
                        .attention_owner
                        .accepted_write();
                }
                _ => {
                    let workspace = state.workspace.lock().unwrap();
                    let store = &workspace.as_ref().unwrap().metadata;
                    let mut meta = store.read_session(&id).unwrap();
                    meta.resume.as_mut().unwrap().harness_session_id =
                        Some("replacement-root".into());
                    store.write_session(&meta);
                }
            }
            assert!(
                !finish_observation(
                    &state,
                    &id,
                    snapshot,
                    Ok(super::super::codex_native::NativeObservation {
                        status: NativeStatus::Active,
                        complete_singleton_root: true,
                        started_at: Instant::now(),
                    }),
                    &cancelled
                ),
                "boundary {boundary}"
            );
            super::super::terminal_runtime::clear_workflow_report_token(&id);
        }
    }

    #[test]
    fn exhausted_correlation_uses_conservative_permission_without_suppression() {
        let (_dir, state, generation, report, t) = callback_fixture();
        let id = state
            .sessions
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        state
            .sessions
            .lock()
            .unwrap()
            .get_mut(&id)
            .unwrap()
            .runtime
            .native_approval
            .as_mut()
            .unwrap()
            .hook_revision = None;
        assert_eq!(
            report_hook(&state, &id, "test-report-token", generation, t, &report),
            HookReportOutcome::Accepted
        );
        assert_eq!(
            state.sessions.lock().unwrap()[&id]
                .info
                .attention
                .as_deref(),
            Some("needs_you")
        );
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn activation_retry_keeps_original_permission_deadline() {
        let (_dir, state, generation, mut report, _) = callback_fixture();
        let id = state
            .sessions
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        let first_receipt = Instant::now() - Duration::from_secs(3);
        report.observed_at = (chrono::Utc::now() - chrono::Duration::seconds(3)).to_rfc3339();
        let staging = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .sessions_dir()
            .join(format!("{id}.json.tmp"));
        std::fs::create_dir(&staging).unwrap();
        assert_eq!(
            report_hook(
                &state,
                &id,
                "test-report-token",
                generation,
                first_receipt,
                &report
            ),
            HookReportOutcome::Retry
        );
        assert!(!state.sessions.lock().unwrap()[&id].active_work_hook);
        std::fs::remove_dir(staging).unwrap();
        assert_eq!(
            report_hook(
                &state,
                &id,
                "test-report-token",
                generation,
                first_receipt,
                &report
            ),
            HookReportOutcome::Accepted
        );
        assert_eq!(
            state.sessions.lock().unwrap()[&id]
                .info
                .attention
                .as_deref(),
            Some("needs_you")
        );
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn callback_auth_root_bounds_and_generation_cannot_be_asserted_by_caller() {
        for invalid in 0..6 {
            let (_dir, state, generation, mut report, t) = callback_fixture();
            let id = state
                .sessions
                .lock()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .clone();
            match invalid {
                0 => report.root_id = "different-root".into(),
                1 => report.hook_fingerprint = "a".repeat(64),
                2 => report.tool_use_id = Some("x".repeat(129)),
                3 => report.turn_id = Some("bad\nturn".into()),
                _ => {}
            }
            let token = if invalid == 4 {
                "wrong"
            } else {
                "test-report-token"
            };
            let generation = if invalid == 5 {
                generation + 1
            } else {
                generation
            };
            assert_eq!(
                report_hook(&state, &id, token, generation, t, &report),
                HookReportOutcome::Rejected,
                "invalid {invalid}"
            );
            assert!(!state.sessions.lock().unwrap()[&id].active_work_hook);
            assert_eq!(
                state.sessions.lock().unwrap()[&id]
                    .info
                    .detected_question
                    .as_deref(),
                Some("old question")
            );
            super::super::terminal_runtime::clear_workflow_report_token(&id);
        }
    }

    #[test]
    fn unresolved_identity_retries_and_user_source_survives_activation() {
        let (_dir, state, generation, report, t) = callback_fixture();
        let id = state
            .sessions
            .lock()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut meta = store.read_session(&id).unwrap();
            meta.harness_session_id_source = None;
            store.write_session(&meta);
        }
        assert_eq!(
            report_hook(&state, &id, "test-report-token", generation, t, &report),
            HookReportOutcome::Retry
        );
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            let mut meta = store.read_session(&id).unwrap();
            meta.harness_session_id_source = Some("codex_hook".into());
            meta.metadata_source = "user".into();
            store.write_session(&meta);
        }
        assert_eq!(
            report_hook(&state, &id, "test-report-token", generation, t, &report),
            HookReportOutcome::Accepted
        );
        assert_eq!(
            state.sessions.lock().unwrap()[&id]
                .info
                .detected_question
                .as_deref(),
            Some("old question")
        );
        super::super::terminal_runtime::clear_workflow_report_token(&id);
    }

    #[test]
    fn atomic_show_and_clear_acknowledge_only_persisted_owner_effects() {
        let (_dir, store, mut handle, mut tracker, t) = fixture();
        let at = t + Duration::from_secs(2);
        let effect = tracker
            .reducer
            .observe(NativeStatus::ApprovalPending, true, at, at)[0];
        assert_eq!(
            apply_effect(&store, &mut handle, &mut tracker, &effect, || at, true),
            ApplyOutcome::Applied
        );
        assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
        let next = at + Duration::from_millis(100);
        let clear = tracker
            .reducer
            .observe(NativeStatus::Active, true, next, next)[0];
        assert_eq!(
            apply_effect(&store, &mut handle, &mut tracker, &clear, || next, true),
            ApplyOutcome::Applied
        );
        assert_eq!(handle.info.attention.as_deref(), Some("working"));
        assert!(tracker
            .reducer
            .observe(NativeStatus::Active, true, next, next)
            .is_empty());
    }
}
