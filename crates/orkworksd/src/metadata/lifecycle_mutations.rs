//! Metadata-only lifecycle decisions; live runtime authority stays at the caller.
use super::session_transactions::WriteScope;
use super::{ObservedStatusSnapshotMetadata, SessionMetadata};
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) enum LifecycleMutation {
    Transition {
        status: String,
        had_live_handle: bool,
        already_ending: bool,
        live_phase: Option<String>,
        resume: Option<crate::harness::ResumeMemory>,
        resumed_from: Option<String>,
        timestamp: String,
    },
    Finalize {
        snapshot: ObservedStatusSnapshotMetadata,
        fallback_status: String,
        timestamp: String,
    },
}
impl LifecycleMutation {
    pub(crate) fn valid(&self) -> bool {
        match self {
            Self::Transition { status, .. } => matches!(
                status.as_str(),
                "creating" | "running" | "killed" | "ended" | "error"
            ),
            Self::Finalize {
                fallback_status,
                snapshot,
                ..
            } => {
                matches!(fallback_status.as_str(), "killed" | "ended" | "error")
                    && snapshot.confidence.is_none_or(f64::is_finite)
            }
        }
    }
    pub(crate) fn apply(&self, meta: &mut SessionMetadata) -> Option<WriteScope> {
        use crate::session_view::{connectivity_for_status, terminal_outcome_for_status};
        match self {
            Self::Transition {
                status,
                had_live_handle,
                already_ending,
                live_phase,
                resume,
                resumed_from,
                timestamp,
            } => {
                let is_terminal = matches!(status.as_str(), "killed" | "ended" | "error");
                if (is_terminal
                    && *already_ending
                    && matches!(meta.lifecycle_phase.as_str(), "ending" | "ended"))
                    || (is_terminal
                        && meta.lifecycle_phase == "ended"
                        && (!had_live_handle
                            || live_phase.as_deref() == Some("ended")
                            || (live_phase.as_deref() == Some("ending")
                                && matches!(meta.status.as_str(), "killed" | "ended" | "error"))))
                    || (!is_terminal && matches!(live_phase.as_deref(), Some("ending" | "ended")))
                    || (!had_live_handle
                        && matches!(meta.lifecycle_phase.as_str(), "ending" | "ended"))
                {
                    return None;
                }
                if is_terminal {
                    meta.status = "running".into();
                    meta.lifecycle_phase = "ending".into();
                    meta.lifecycle = "stopping".into();
                    meta.attention = None;
                    meta.connectivity = connectivity_for_status("running").into();
                    meta.terminal_outcome = None;
                    meta.pending_terminal_status = Some(status.clone());
                    meta.ending_observed_status_snapshot = Some(ObservedStatusSnapshotMetadata {
                        attention_origin: Some(meta.attention_origin().to_string()),
                        value: meta.observed_status.clone(),
                        source: meta.attention_source().to_string(),
                        confidence: Some(meta.attention_confidence()),
                        observed_at: Some(timestamp.clone()),
                    });
                    meta.observed_status = None;
                } else {
                    meta.status = status.clone();
                    meta.lifecycle_phase = if status == "creating" {
                        "creating"
                    } else {
                        "active"
                    }
                    .into();
                    meta.lifecycle = if status == "creating" {
                        "creating"
                    } else {
                        "alive"
                    }
                    .into();
                    meta.connectivity = connectivity_for_status(status).into();
                    meta.terminal_outcome = terminal_outcome_for_status(status);
                }
                meta.last_activity = timestamp.clone();
                if resume.is_some() {
                    meta.resume = resume.clone();
                }
                if resumed_from.is_some() {
                    meta.resumed_from = resumed_from.clone();
                }
                Some(if is_terminal {
                    WriteScope::ATTENTION
                } else {
                    WriteScope::NONE
                })
            }
            Self::Finalize {
                snapshot,
                fallback_status,
                timestamp,
            } => {
                if meta.lifecycle_phase == "ended" {
                    return None;
                }
                let pending = meta
                    .pending_terminal_status
                    .clone()
                    .unwrap_or_else(|| fallback_status.clone());
                meta.status = pending.clone();
                meta.lifecycle_phase = "ended".into();
                meta.lifecycle = "dead".into();
                meta.attention = None;
                meta.connectivity = connectivity_for_status(&pending).into();
                meta.terminal_outcome = terminal_outcome_for_status(&pending);
                meta.pending_terminal_status = None;
                meta.ending_observed_status_snapshot = None;
                meta.final_observed_status_snapshot = Some(snapshot.clone());
                meta.observed_status = None;
                meta.last_activity = timestamp.clone();
                Some(WriteScope::ATTENTION)
            }
        }
    }
}
