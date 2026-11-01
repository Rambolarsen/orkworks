//! Pure internal metadata intents, shared by file and SQL persistence owners.
use super::session_transactions::WriteScope;
use super::{HarnessSessionMergeResult, HarnessSessionReport, SessionMetadata};
use serde::Serialize;
use std::io;
#[derive(Clone, Debug, Serialize)]
pub(crate) enum PeonPolicy {
    Infer,
    NonPrompt,
    PreserveAttention,
    PreserveHook {
        status: String,
        confidence: f64,
        source: String,
    },
}
#[derive(Clone, Debug, Serialize)]
pub(crate) enum InternalMutation {
    Peon {
        inference: crate::peon::PeonInference,
        timestamp: String,
        provider: Option<crate::providers::ProviderObservation>,
        history_summary: Option<String>,
        policy: PeonPolicy,
    },
    HarnessReport {
        report: HarnessSessionReport,
        timestamp: String,
        allow_codex_identity_replacement: bool,
    },
    ProviderContext(crate::providers::ProviderObservation),
    UserTitle {
        title: Option<String>,
    },
}
#[derive(Debug, Serialize)]
pub(crate) enum Disposition {
    Applied,
    PeonSkipped { permanent_hold: bool },
    HarnessReport(HarnessSessionMergeResult),
}
#[derive(Debug, Serialize)]
pub(crate) struct Decision {
    pub(crate) scope: Option<WriteScope>,
    pub(crate) disposition: Disposition,
}
impl InternalMutation {
    pub(crate) fn apply(&self, meta: &mut SessionMetadata) -> io::Result<Decision> {
        match self {
            Self::Peon {
                inference,
                timestamp,
                provider,
                history_summary,
                policy,
            } => {
                if !inference.confidence.is_finite()
                    || matches!(policy,PeonPolicy::PreserveHook{confidence,..} if !confidence.is_finite())
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "non-finite metadata confidence",
                    ));
                }
                apply_peon(
                    meta,
                    inference,
                    timestamp,
                    provider.as_ref(),
                    history_summary.as_deref(),
                    policy.borrowed(),
                )
            }
            Self::HarnessReport {
                report,
                timestamp,
                allow_codex_identity_replacement,
            } => apply_harness_report(meta, report, timestamp, *allow_codex_identity_replacement),
            Self::ProviderContext(provider) => {
                apply_provider(meta, provider);
                Ok(Decision {
                    scope: Some(WriteScope::NONE),
                    disposition: Disposition::Applied,
                })
            }
            Self::UserTitle { title } => apply_user_title(meta, title.as_deref()),
        }
    }
}
impl PeonPolicy {
    fn borrowed(&self) -> super::PeonAttentionPolicy<'_> {
        match self {
            Self::Infer => super::PeonAttentionPolicy::Infer,
            Self::NonPrompt => super::PeonAttentionPolicy::NonPrompt,
            Self::PreserveAttention => super::PeonAttentionPolicy::PreserveAttention,
            Self::PreserveHook {
                status,
                confidence,
                source,
            } => super::PeonAttentionPolicy::PreserveHook {
                status,
                confidence: *confidence,
                source,
            },
        }
    }
}
pub(crate) fn apply_provider(
    meta: &mut SessionMetadata,
    provider: &crate::providers::ProviderObservation,
) {
    meta.provider_id = Some(provider.provider_id.clone());
    meta.provider_label = Some(provider.provider_label.clone());
    meta.provider_model = provider.provider_model.clone();
    meta.provider_state = Some(provider.provider_state.clone());
}
pub(crate) fn normalize_user_title(title: &str) -> io::Result<String> {
    if title.chars().any(char::is_control)
        || title.trim().is_empty()
        || title.trim().chars().count() > 100
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid session title",
        ));
    }
    Ok(title.trim().to_owned())
}
pub(crate) fn apply_user_title(
    meta: &mut SessionMetadata,
    title: Option<&str>,
) -> io::Result<Decision> {
    let (label, source) = if let Some(title) = title {
        (normalize_user_title(title)?, super::LabelSource::User)
    } else {
        meta.last_user_input
            .as_deref()
            .filter(|input| crate::peon::is_descriptive_input(input))
            .map(|input| {
                (
                    input.chars().take(100).collect(),
                    super::LabelSource::TerminalInput,
                )
            })
            .unwrap_or_else(|| {
                (
                    crate::session_types::placeholder_label(&meta.id),
                    super::LabelSource::Placeholder,
                )
            })
    };
    meta.label = label;
    meta.label_source = source;
    meta.label_from_initial_prompt = false;
    meta.label_context_refined = false;
    Ok(Decision {
        scope: Some(WriteScope::NONE),
        disposition: Disposition::Applied,
    })
}
pub(crate) fn apply_harness_report(
    meta: &mut SessionMetadata,
    report: &HarnessSessionReport,
    timestamp: &str,
    allow_codex_identity_replacement: bool,
) -> io::Result<Decision> {
    use crate::harness::{ResumeMemory, ResumeState, ResumeStrategy};
    if !super::valid_harness_session_report(report) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid harness identity report",
        ));
    }
    let mut disposition = HarnessSessionMergeResult::Accepted;
    let changed = (|| -> io::Result<bool> {
        let existing_confidence = meta.harness_session_id_confidence.unwrap_or(-1.0);
        let existing_id = meta
            .resume
            .as_ref()
            .and_then(|resume| resume.harness_session_id.as_deref());
        if existing_id.is_some() && report.confidence < existing_confidence {
            disposition = HarnessSessionMergeResult::IgnoredLowerConfidence;
            return Ok(false);
        }
        if meta.harness == "codex"
            && existing_id.is_some_and(|existing| existing != report.harness_session_id)
            && !allow_codex_identity_replacement
        {
            disposition = HarnessSessionMergeResult::IgnoredIdentityChange;
            return Ok(false);
        }
        if existing_id == Some(report.harness_session_id.as_str())
            && meta.harness_session_id_source.as_deref() == Some(report.source.as_str())
            && meta.harness_session_id_confidence == Some(report.confidence)
        {
            disposition = HarnessSessionMergeResult::IgnoredUnchanged;
            if let Some(resume) = meta.resume.as_mut() {
                resume.last_seen_at = Some(timestamp.to_string());
                return Ok(true);
            }
            return Ok(false);
        }
        let mut resume = meta.resume.take().unwrap_or_else(|| ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::None,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: None,
        });
        resume.state = ResumeState::Available;
        resume.harness_session_id = Some(report.harness_session_id.clone());
        resume.last_seen_at = Some(timestamp.to_string());
        if resume.preferred_strategy == ResumeStrategy::None {
            resume.preferred_strategy = ResumeStrategy::Exact;
        }
        meta.resume = Some(resume);
        meta.harness_session_id_source = Some(report.source.clone());
        meta.harness_session_id_confidence = Some(report.confidence);
        meta.harness_session_id_captured_at = Some(timestamp.to_string());
        Ok(true)
    })()?;
    Ok(Decision {
        scope: changed.then_some(WriteScope::NONE),
        disposition: Disposition::HarnessReport(disposition),
    })
}
pub(crate) fn apply_peon(
    meta: &mut SessionMetadata,
    inf: &crate::peon::PeonInference,
    timestamp: &str,
    provider: Option<&crate::providers::ProviderObservation>,
    history_summary: Option<&str>,
    policy: super::PeonAttentionPolicy<'_>,
) -> io::Result<Decision> {
    use super::{
        canonical_attention, normalize_work_phase, session_transactions, source_priority,
        PeonAttentionPolicy,
    };
    let mut permanent_hold = false;
    let scope = (|| -> io::Result<Option<WriteScope>> {
        let work_allowed = source_priority::can_overwrite(
            "peon",
            &meta.metadata_source,
            meta.provenance.work_age_secs(),
        );
        let protected_hook_matches = match policy {
            PeonAttentionPolicy::PreserveHook {
                status,
                confidence,
                source,
            } => {
                let source = if source == "codex_hook" {
                    "agent"
                } else {
                    source
                };
                let origin = meta.attention_origin();
                let validated_origin = matches!(origin, "harness_hook" | "native_codex")
                    || (origin == "legacy_unknown"
                        && matches!(source, "claude_hook" | "copilot_hook"))
                    || (origin == "process" && source == "process");
                meta.observed_status.as_deref() == Some(status)
                    && meta.attention_source() == source
                    && meta.attention_confidence() == confidence
                    && validated_origin
            }
            _ => false,
        };
        let infer_attention = !matches!(
            policy,
            PeonAttentionPolicy::NonPrompt | PeonAttentionPolicy::PreserveAttention
        );
        let mut attention_allowed = !matches!(policy, PeonAttentionPolicy::PreserveAttention)
            && !protected_hook_matches
            && source_priority::can_overwrite(
                "peon",
                meta.attention_source(),
                meta.attention_age_secs(),
            );
        if infer_attention
            && inf.observed_status.as_deref() == Some("working")
            && crate::peon::is_terminal_observed_status(meta.observed_status.as_deref())
        {
            if inf.next_action.is_none()
                && inf.blocker_description.is_none()
                && inf.failed_command.is_none()
                && inf.failed_test.is_none()
                && inf.capacity_hints.is_none()
            {
                return Ok(None);
            }
            attention_allowed = false;
        }
        let summary_allowed = history_summary.is_some()
            && meta.metadata_source != "user"
            && !matches!(
                meta.provenance.summary_source.as_deref(),
                Some("agent" | "user")
            );
        permanent_hold = meta.metadata_source == "user" && meta.attention_source() == "user";
        if !work_allowed && !attention_allowed && !summary_allowed {
            return Ok(None);
        }

        let reconciled_peon_prompt = if attention_allowed
            && matches!(policy, PeonAttentionPolicy::NonPrompt)
            && !matches!(meta.attention_source(), "user" | "agent" | "codex_hook")
        {
            let before = (
                meta.observed_status.clone(),
                meta.attention.clone(),
                meta.needs_user_input,
                meta.detected_question.clone(),
                meta.suggested_options.clone(),
            );
            if meta.observed_status.as_deref() == Some("waiting_for_input") {
                meta.observed_status = None;
            }
            if meta.lifecycle == "alive" {
                meta.attention = canonical_attention(meta.observed_status.as_deref());
            }
            meta.needs_user_input = None;
            meta.detected_question = None;
            meta.suggested_options = None;
            let changed = before
                != (
                    meta.observed_status.clone(),
                    meta.attention.clone(),
                    meta.needs_user_input,
                    meta.detected_question.clone(),
                    meta.suggested_options.clone(),
                );
            if changed {
                meta.last_activity = timestamp.to_string();
            }
            changed
        } else {
            false
        };

        // Observer-only inference cannot resume a terminal observed status to
        // working; descriptive work and summary still have separate authority.
        let _ = reconciled_peon_prompt;
        // Peon reruns on any new PTY output, including non-substantive terminal
        // chatter (TUI redraws, spinner frames), so it can conclude the same
        // situation repeatedly. Only bump `last_activity` when the inference
        // actually changes the observed situation — otherwise an idle session
        // whose TUI keeps repainting would show "just now" forever. The fields
        // compared here are exactly the ones that drive the situation hero
        // (situationHeadline/situationTail in labels.ts).
        let situation_before = (
            meta.observed_status.clone(),
            meta.work_phase.clone(),
            meta.summary.clone(),
            meta.next_action.clone(),
            meta.detected_question.clone(),
            meta.blocker_description.clone(),
            meta.suggested_options.clone(),
            meta.failed_command.clone(),
            meta.failed_test.clone(),
        );

        if attention_allowed {
            match policy {
                PeonAttentionPolicy::PreserveAttention => {}
                PeonAttentionPolicy::Infer
                | PeonAttentionPolicy::NonPrompt
                | PeonAttentionPolicy::PreserveHook { .. } => {
                    if !matches!(policy, PeonAttentionPolicy::NonPrompt)
                        || (inf.observed_status.as_deref() != Some("waiting_for_input")
                            && !(inf.observed_status.as_deref() == Some("working")
                                && crate::peon::is_terminal_observed_status(
                                    meta.observed_status.as_deref(),
                                )))
                    {
                        meta.observed_status =
                            inf.observed_status.clone().or(meta.observed_status.take());
                    }
                    if meta.lifecycle == "alive" {
                        meta.attention = canonical_attention(meta.observed_status.as_deref());
                    }
                }
            }
        }
        if work_allowed {
            if let Some(ref phase) = inf.phase {
                meta.work_phase = normalize_work_phase(phase);
            }
            // `label` is a one-shot topic, not the turn-by-turn activity summary —
            // it must not be clobbered here (ADR 0029).
            meta.next_action = inf.next_action.clone().or(meta.next_action.take());
        }
        if attention_allowed && infer_attention {
            meta.needs_user_input = inf.needs_user_input.or(meta.needs_user_input);
        }
        // Normalize: treat empty-string question as absent (LLM may emit "" instead of null).
        if attention_allowed && infer_attention {
            let incoming_q = inf
                .detected_question
                .as_deref()
                .filter(|q| !q.is_empty())
                .map(str::to_string);
            // Options belong to their question; clear them when the question changes so
            // stale options never appear under a different question.
            if incoming_q.is_some() && incoming_q.as_deref() != meta.detected_question.as_deref() {
                meta.suggested_options = None;
            }
            meta.detected_question = incoming_q.or(meta.detected_question.take());
            meta.suggested_options = inf
                .suggested_options
                .clone()
                .or(meta.suggested_options.take());
        }
        // Hook authority covers attention and prompt fields only. Peon still
        // contributes useful diagnostics about blockers and failed work while
        // the coding tool is processing the turn.
        if work_allowed {
            meta.blocker_description = inf
                .blocker_description
                .clone()
                .or(meta.blocker_description.take());
            meta.failed_command = inf.failed_command.clone().or(meta.failed_command.take());
            meta.failed_test = inf.failed_test.clone().or(meta.failed_test.take());
            meta.capacity_hints = inf.capacity_hints.clone().or(meta.capacity_hints.take());

            if let Some(ref h) = inf.detected_harness {
                if meta.harness.is_empty() {
                    meta.harness = h.clone();
                }
            }
            if let Some(ref m) = inf.detected_model {
                let is_peon_own_model = provider
                    .and_then(|p| p.provider_model.as_ref())
                    .map(|pm| pm == m)
                    .unwrap_or(false);
                if meta.model.is_empty() && !is_peon_own_model {
                    meta.model = m.clone();
                }
            }
        }

        if let Some(summary) = history_summary.filter(|_| summary_allowed) {
            meta.summary = Some(summary.to_string());
        }

        let situation_after = (
            meta.observed_status.clone(),
            meta.work_phase.clone(),
            meta.summary.clone(),
            meta.next_action.clone(),
            meta.detected_question.clone(),
            meta.blocker_description.clone(),
            meta.suggested_options.clone(),
            meta.failed_command.clone(),
            meta.failed_test.clone(),
        );
        if situation_after != situation_before {
            meta.last_activity = timestamp.to_string();
        }
        meta.peon_last_inference = Some(timestamp.to_string());
        if work_allowed {
            meta.metadata_source = "peon".into();
            meta.metadata_confidence = inf.confidence;
        }

        if attention_allowed {
            meta.provenance.attention_source = Some("peon".into());
            meta.provenance.attention_confidence = Some(inf.confidence);
            meta.provenance.attention_origin = Some("peon".into());
        }

        if summary_allowed {
            meta.provenance.summary_source = Some("peon".into());
            meta.provenance.summary_confidence = Some(inf.confidence);
            meta.provenance.summary_observed_at = Some(timestamp.to_string());
        }

        if work_allowed {
            if let Some(p) = provider {
                meta.provider_id = Some(p.provider_id.clone());
                meta.provider_label = Some(p.provider_label.clone());
                meta.provider_model = p.provider_model.clone();
                meta.provider_state = Some(p.provider_state.clone());
            }
        }

        Ok(Some(session_transactions::WriteScope {
            attention: attention_allowed,
            work: work_allowed,
        }))
    })()?;
    Ok(Decision {
        scope,
        disposition: if scope.is_some() {
            Disposition::Applied
        } else {
            Disposition::PeonSkipped { permanent_hold }
        },
    })
}
