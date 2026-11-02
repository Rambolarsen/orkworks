//! Shared validated hook state stages; persistence stays with each application owner.
use super::*;
#[derive(Clone, Copy)]
pub(crate) struct PreparedHook {
    pub(crate) observed: chrono::DateTime<chrono::Utc>,
    pub(crate) boundary: bool,
    pub(crate) activation: bool,
}
pub(crate) fn prepare(
    tracker: &mut NativeApprovalState,
    handle: &SessionHandle,
    meta: &crate::metadata::SessionMetadata,
    first_receipt: Instant,
    report: &NativeHookReport,
    fingerprint: Option<&str>,
) -> Result<PreparedHook, HookReportOutcome> {
    tracker.sync(handle, meta);
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
        return Err(HookReportOutcome::Rejected);
    }
    let observed = observed.expect("validated timestamp");
    let age = chrono::Utc::now().signed_duration_since(observed);
    let delayed_permission =
        age > chrono::Duration::seconds(30) && report.event == HookEvent::PermissionRequest;
    if age < chrono::Duration::seconds(-1)
        || (age > chrono::Duration::seconds(30) && !delayed_permission)
    {
        tracker.reducer.disconnected();
        tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
        return Err(HookReportOutcome::Rejected);
    }
    let Some(root) = tracker.root.as_deref() else {
        return Err(HookReportOutcome::Retry);
    };
    if root != report.root_id || meta.lifecycle != "alive" {
        tracker.invalidate();
        return Err(HookReportOutcome::Rejected);
    }
    if handle
        .runtime
        .accepted_input_at
        .is_some_and(|input| observed <= input)
    {
        tracker.reducer.disconnected();
        tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
        return Err(HookReportOutcome::Rejected);
    }
    let boundary = matches!(report.event, HookEvent::Stop | HookEvent::UserPromptSubmit);
    if let Some(external) = handle.runtime.last_hook_attention_at.filter(|at| {
        tracker
            .last_private_attention_at
            .is_none_or(|private| *at > private)
    }) {
        tracker.fenced_hook_at = Some(
            tracker
                .fenced_hook_at
                .map_or(external, |at| at.max(external)),
        );
    }
    // A valid boundary fences its older queued events even if newer
    // correlation or attention makes replaying its transition stale.
    // Input and all scalar/auth/root checks have already passed.
    if boundary {
        tracker.fenced_hook_at = Some(
            tracker
                .fenced_hook_at
                .map_or(observed, |at| at.max(observed)),
        );
    }
    if tracker
        .fenced_hook_at
        .is_some_and(|at| observed < at || (observed == at && !boundary))
    {
        tracker.reducer.disconnected();
        tracker.hook_revision = tracker.hook_revision.and_then(|r| r.checked_add(1));
        return Err(HookReportOutcome::Rejected);
    }
    // UUID filename order is not event order. Ties and reversed delivery
    // revoke correlation; an authenticated permission remains conservative.
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
            return Err(HookReportOutcome::Rejected);
        }
    }
    // A delayed permission is still validated evidence that a wait exists,
    // but its old timestamp cannot support native correlation or clearing.
    // Disconnect first so the reducer retains it only through its locked,
    // post-grace conservative path.
    if delayed_permission {
        tracker.reducer.disconnected();
    }
    let activation = report.event == HookEvent::PermissionRequest && !handle.active_work_hook;
    // A staged application owns its continuation even after checked counters
    // exhaust. Replacing this marker revokes older async hook/effect captures.
    tracker.hook_application = Arc::new(());
    Ok(PreparedHook {
        observed,
        boundary,
        activation,
    })
}
pub(crate) fn advance(
    tracker: &mut NativeApprovalState,
    handle: &mut SessionHandle,
    report: &NativeHookReport,
    prepared: &PreparedHook,
) {
    let PreparedHook {
        observed,
        boundary,
        activation,
    } = *prepared;
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
    if report.event != HookEvent::PermissionRequest {
        tracker.validated_permission = None;
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
    if boundary || report.event == HookEvent::PermissionRequest {
        let attention_at = handle
            .runtime
            .last_hook_attention_at
            .map_or(observed, |at| at.max(observed));
        handle.runtime.last_hook_attention_at = Some(attention_at);
        tracker.last_private_attention_at = Some(attention_at);
    }
    tracker.live_revision = handle.runtime.attention_owner.snapshot();
    if tracker.hook_revision.is_none() || !tracker.reducer.assistance_available() {
        tracker.reducer.disconnected();
    }
}

/// Reuse the file owner's attention publication rules only after SQL commits.
pub(crate) fn publish_transition(
    handle: &mut SessionHandle,
    meta: &crate::metadata::SessionMetadata,
    transition: crate::metadata::internal_mutations::HookTransition,
) {
    use crate::metadata::internal_mutations::HookTransition;
    if matches!(transition, HookTransition::FallbackPermission) {
        apply_live_attention_fields(
            &mut handle.info,
            &mut handle.runtime.attention_owner,
            "waiting_for_input",
            None,
            "codex_hook",
            1.0,
        );
    } else {
        handle.runtime.attention_owner.accepted_write();
        handle.info.observed_status = meta.observed_status.clone();
        handle.info.attention = meta.attention.clone();
        handle.info.needs_user_input = None;
        handle.info.detected_question = None;
        handle.info.suggested_options = None;
        handle.pending_work_signal = None;
    }
    handle.info.provenance = meta.provenance.clone();
    if matches!(transition, HookTransition::FallbackPermission) {
        if let Some(tracker) = handle.runtime.native_approval.as_mut() {
            tracker.record_validated_permission(&handle.runtime.attention_owner);
            tracker.ownership = None;
            tracker.sql_ownership = None;
            tracker.live_revision = handle.runtime.attention_owner.snapshot();
        }
    }
}
pub(crate) fn fallback_allowed(
    handle: &SessionHandle,
    meta: &crate::metadata::SessionMetadata,
) -> bool {
    meta.attention_source() != "user"
        && handle
            .info
            .provenance
            .attention_source
            .as_deref()
            .or(handle.info.metadata_source.as_deref())
            != Some("user")
        && meta.needs_user_input != Some(true)
        && meta.detected_question.is_none()
        && handle.info.needs_user_input != Some(true)
        && handle.info.detected_question.is_none()
}

pub(crate) fn advance_sql(
    h: &mut SessionHandle,
    meta: &crate::metadata::SessionMetadata,
    report: &NativeHookReport,
    prepared: &PreparedHook,
    first_receipt: Instant,
    generation: u64,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    authority: Option<super::super::prompt_authority::AuthoritySnapshot>,
) -> Option<(
    Arc<super::sql_context::ObservationContext>,
    Vec<super::sql_context::IssuedEffect>,
    bool,
)> {
    let mut tracker = h.runtime.native_approval.take()?;
    advance(&mut tracker, h, report, prepared);
    let mut effects = tracker.hook_sql(
        &h.runtime.attention_owner,
        report.event,
        report.tool_use_id.as_deref(),
        first_receipt,
    );
    let fallback = report.event == HookEvent::PermissionRequest
        && (tracker.hook_revision.is_none() || !tracker.reducer.assistance_available());
    if fallback {
        effects.clear();
    } else if effects.is_empty() {
        effects = tracker.tick_sql(&h.runtime.attention_owner, Instant::now());
    }
    tracker.live_revision = h.runtime.attention_owner.snapshot();
    if report.event == HookEvent::PermissionRequest
        && h.info.provenance.attention_origin.as_deref() == Some("harness_hook")
        && h.info.observed_status.as_deref() == Some("waiting_for_input")
    {
        tracker.record_validated_permission(&h.runtime.attention_owner);
    }
    h.runtime.native_approval = Some(tracker);
    let fallback = fallback && fallback_allowed(h, &meta);
    let context =
        super::sql_context::ObservationContext::capture_hook(h, generation, cancelled, authority)?;
    Some((Arc::new(context), effects, fallback))
}
