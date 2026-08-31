//! Owns every write to `observed_status`/`attention` across the live session
//! handle and persisted metadata. See ADR 0027.

use crate::metadata::{self, canonical_attention};
use crate::session_types::SessionInfo;

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
}
