//! Application-layer harness session reporting used by HTTP routes and the local Codex relay.

use crate::metadata;
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::session_application::{self, SessionApplication};
use crate::AppState;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct HarnessSessionReportRequest {
    #[serde(rename = "harnessSessionId")]
    pub(crate) harness_session_id: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
    #[serde(rename = "hookFingerprint", default)]
    pub(crate) hook_fingerprint: Option<String>,
    #[serde(rename = "sessionStartSource", default)]
    pub(crate) session_start_source: Option<String>,
    #[serde(rename = "sessionStartEvent", default)]
    pub(crate) session_start_event: Option<String>,
    #[serde(rename = "promptHookGeneration", default)]
    pub(crate) prompt_hook_generation: Option<String>,
    #[serde(rename = "sessionStartObservedAt", default)]
    pub(crate) session_start_observed_at: Option<String>,
}

pub(crate) struct HarnessSessionReportCommand {
    pub(crate) id: String,
    pub(crate) auth_token: Option<String>,
    pub(crate) req: HarnessSessionReportRequest,
    pub(crate) reported_cwd: Option<String>,
    pub(crate) observation_stamp: Option<session_application::hook_observation::Receipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HarnessReportOutcome {
    Ok,
    Accepted,
    BadRequest,
    Unauthorized,
    NotFound,
    Conflict,
    ServiceUnavailable,
    InternalError,
}

pub(crate) async fn report_harness_session(
    state: Arc<AppState>,
    command: HarnessSessionReportCommand,
) -> HarnessReportOutcome {
    let HarnessSessionReportCommand {
        id,
        auth_token,
        req,
        reported_cwd,
        observation_stamp,
    } = command;
    let observation_state = state.clone();
    let is_codex_hook = req.source == "codex_hook";
    let observation_requested = is_codex_hook
        && req
            .hook_fingerprint
            .as_deref()
            .is_some_and(metadata::valid_hook_fingerprint);
    // Preserve the request's original authentication class across the legacy
    // metadata lookup. A trusted relay receipt is already capability-bound.
    let relay_receipt = observation_stamp.is_some();
    let original_observation_token = observation_requested
        .then(|| {
            auth_token
                .as_deref()
                .filter(|token| relay_receipt || verify_workflow_report_token(&id, token))
                .map(str::to_owned)
        })
        .flatten();
    let (target_harness, mut observation_capture) =
        match session_application::hook_observation::capture_target(
            &state,
            &id,
            observation_requested,
            observation_stamp,
        )
        .await
        {
            Ok(target) => target,
            Err(session_application::hook_observation::TargetReadError::Conflict) => {
                return HarnessReportOutcome::Conflict;
            }
            Err(session_application::hook_observation::TargetReadError::Busy) => {
                return HarnessReportOutcome::ServiceUnavailable;
            }
            Err(session_application::hook_observation::TargetReadError::Unavailable) => {
                return HarnessReportOutcome::InternalError;
            }
        };
    if let Some(capture) = observation_capture.as_mut() {
        if let (Ok(capture), Some(token)) =
            (capture.as_mut(), original_observation_token.as_deref())
        {
            capture.authenticate(token);
        } else if relay_receipt && original_observation_token.is_none() {
            *capture = Err(session_application::SessionError::Conflict);
        }
    }
    let prompt_harness = match target_harness.as_deref() {
        Some("claude-code") => Some("claude-code"),
        Some("copilot") => Some("copilot"),
        _ => None,
    };
    if matches!(req.source.as_str(), "claude_hook" | "copilot_hook") && prompt_harness.is_none() {
        return HarnessReportOutcome::BadRequest;
    }
    if let Some(expected_source) = prompt_harness.map(|harness| match harness {
        "claude-code" => "claude_hook",
        _ => "copilot_hook",
    }) {
        if req.source != expected_source {
            return HarnessReportOutcome::BadRequest;
        }
    }
    let native_session_id = req.harness_session_id.clone();
    let report = metadata::HarnessSessionReport {
        harness_session_id: req.harness_session_id.clone(),
        source: req.source.clone(),
        confidence: req.confidence,
    };
    if !metadata::valid_harness_session_report(&report) {
        return HarnessReportOutcome::BadRequest;
    }
    if let Some(harness_id) = prompt_harness {
        let lifecycle_valid = match (
            harness_id,
            req.session_start_event.as_deref(),
            req.session_start_source.as_deref(),
        ) {
            (_, None, None) => true,
            ("claude-code", Some("SessionStart"), Some("clear")) => true,
            ("copilot", Some("sessionStart"), Some("new")) => req
                .session_start_observed_at
                .as_deref()
                .and_then(|raw| crate::workspace_runtime::parse_hook_observed_at(raw).ok())
                .is_some(),
            _ => false,
        };
        if !lifecycle_valid {
            return HarnessReportOutcome::BadRequest;
        }
    }
    let lifecycle_observed_at = req
        .session_start_observed_at
        .as_deref()
        .and_then(|raw| crate::workspace_runtime::parse_hook_observed_at(raw).ok());
    let _prompt_transition = prompt_harness.map(|_| {
        crate::runtime::prompt_authority::transition_lock()
            .lock()
            .unwrap()
    });
    let private_lookup_authorized = is_codex_hook
        && auth_token
            .as_deref()
            .is_some_and(|token| verify_workflow_report_token(&id, token));
    if private_lookup_authorized {
        if let Some(Ok(capture)) = observation_capture.as_mut() {
            capture.authenticate(
                auth_token
                    .as_deref()
                    .expect("authorized report has a token"),
            );
        }
    }
    let prompt_registration_authorized = if let Some(expected_harness) = prompt_harness {
        let token = auth_token.as_deref();
        let generation = req.prompt_hook_generation.as_deref();
        let matching_harness = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|workspace| workspace.metadata.read_session(&id))
            .is_some_and(|session| session.harness == expected_harness);
        let capability_matches = matching_harness
            && token.is_some_and(|token| verify_workflow_report_token(&id, token))
            && generation.is_some_and(|generation| {
                crate::runtime::prompt_authority::registry().generation_matches(
                    &id,
                    expected_harness,
                    generation,
                )
            });
        if !capability_matches {
            return HarnessReportOutcome::BadRequest;
        }
        let live_phase = state
            .sessions
            .lock()
            .unwrap()
            .get(&id)
            .filter(|handle| {
                handle.info.harness_id.as_deref() == Some(expected_harness)
                    || handle.info.harness.as_deref() == Some(expected_harness)
            })
            .map(|handle| {
                (
                    handle.info.lifecycle.clone(),
                    handle.info.lifecycle_phase.clone(),
                )
            });
        match live_phase
            .as_ref()
            .map(|(lifecycle, phase)| (lifecycle.as_str(), phase.as_str()))
        {
            Some(("alive", "active")) => {}
            // No identity or attention mutation before runtime activation.
            // The same authenticated launch may retry once startup completes.
            Some(("creating", "creating")) if expected_harness == "claude-code" => {
                return HarnessReportOutcome::ServiceUnavailable;
            }
            _ => return HarnessReportOutcome::BadRequest,
        }
        match crate::runtime::prompt_authority::registry().register_native_id(
            &id,
            expected_harness,
            generation.expect("authorized generation exists"),
            &native_session_id,
            req.session_start_event.is_some(),
            lifecycle_observed_at,
        ) {
            crate::runtime::prompt_authority::BindResult::Bound
            | crate::runtime::prompt_authority::BindResult::Unchanged => {
                store_authenticated_reported_cwd(&state, &id, reported_cwd.as_deref());
                true
            }
            crate::runtime::prompt_authority::BindResult::Held => {
                store_authenticated_reported_cwd(&state, &id, reported_cwd.as_deref());
                if crate::runtime::prompt_authority::registry().acknowledged_candidate_matches(
                    &id,
                    expected_harness,
                    generation.expect("authorized generation exists"),
                    &native_session_id,
                ) {
                    SessionApplication::new(state.clone())
                        .commit_prompt_identity_reset_under_transition(&id);
                }
                return HarnessReportOutcome::Accepted;
            }
            crate::runtime::prompt_authority::BindResult::Rejected => {
                return HarnessReportOutcome::BadRequest;
            }
        }
    } else {
        false
    };
    let completion = match SessionApplication::new(state.clone())
        .report_harness_session_with_completion(
            &id,
            report,
            req.session_start_source.as_deref(),
            req.session_start_event.as_deref(),
            private_lookup_authorized || prompt_registration_authorized,
            auth_token.as_deref(),
        ) {
        Ok(result) => result,
        Err(session_application::SessionError::Conflict) => {
            return HarnessReportOutcome::Conflict;
        }
        Err(_) => return HarnessReportOutcome::InternalError,
    };

    let result = completion.result;

    // Prompt registration and identity merge are complete; non-Codex routes
    // have no asynchronous work below this cutoff.
    drop(_prompt_transition);

    // Schedule the independently owned label follow-up before the new file
    // await. An observation stall must not adopt a replacement runtime later.
    if private_lookup_authorized
        && matches!(
            result,
            metadata::HarnessSessionMergeResult::Accepted
                | metadata::HarnessSessionMergeResult::IgnoredUnchanged
        )
    {
        let reservation = completion.reserve_codex_label_refresh(&id, &native_session_id);
        if let Some((runtime_identity, refresh_epoch)) = reservation {
            schedule_codex_label_refresh(
                state,
                id.clone(),
                native_session_id,
                runtime_identity,
                refresh_epoch,
            );
        }
    }

    if let Some(fingerprint) = req.hook_fingerprint {
        if is_codex_hook
            && metadata::valid_hook_fingerprint(&fingerprint)
            && matches!(
                result,
                metadata::HarnessSessionMergeResult::Accepted
                    | metadata::HarnessSessionMergeResult::IgnoredLowerConfidence
                    | metadata::HarnessSessionMergeResult::IgnoredIdentityChange
                    | metadata::HarnessSessionMergeResult::IgnoredUnchanged
            )
        {
            if let Err(error) = SessionApplication::new(observation_state)
                .record_codex_hook_observation(
                    &id,
                    &fingerprint,
                    observation_capture
                        .take()
                        .unwrap_or(Err(session_application::SessionError::Conflict)),
                )
                .await
            {
                if !matches!(&error, session_application::hook_observation::Error::Io(_)) {
                    tracing::warn!(session_id = %id, error = %error, "failed to record Codex hook observation");
                }
            }
        }
    }

    match result {
        metadata::HarnessSessionMergeResult::Accepted
        | metadata::HarnessSessionMergeResult::IgnoredLowerConfidence
        | metadata::HarnessSessionMergeResult::IgnoredIdentityChange
        | metadata::HarnessSessionMergeResult::IgnoredUnchanged => HarnessReportOutcome::Ok,
        metadata::HarnessSessionMergeResult::NotFound => HarnessReportOutcome::NotFound,
        metadata::HarnessSessionMergeResult::Invalid => HarnessReportOutcome::BadRequest,
        metadata::HarnessSessionMergeResult::PersistFailed => HarnessReportOutcome::InternalError,
    }
}

pub(crate) async fn report_harness_session_from_local_relay(
    state: Arc<AppState>,
    id: String,
    mut report: HarnessSessionReportRequest,
    report_token: &str,
    observation: Option<session_application::hook_observation::Receipt>,
) -> HarnessReportOutcome {
    // The relay may have awaited filesystem I/O since its initial token check.
    // The shared handler also accepts unauthenticated reports, so reject a
    // revoked capability before dispatching this queued report.
    if report_token.is_empty()
        || report_token
            .bytes()
            .any(|byte| !byte.is_ascii() || byte.is_ascii_control())
        || !verify_workflow_report_token(&id, report_token)
    {
        return HarnessReportOutcome::Unauthorized;
    }
    // A relay without retained receipt authority may still deliver identity,
    // but must not adopt a fresh observation epoch after its mailbox read.
    if observation.is_none() {
        report.hook_fingerprint = None;
    }
    report_harness_session(
        state,
        HarnessSessionReportCommand {
            id,
            auth_token: Some(report_token.to_owned()),
            req: report,
            reported_cwd: None,
            observation_stamp: observation,
        },
    )
    .await
}

fn store_authenticated_reported_cwd(state: &AppState, id: &str, cwd: Option<&str>) {
    if let Some(cwd) = cwd.filter(|cwd| !cwd.trim().is_empty()) {
        state
            .peon
            .reported_cwd
            .write()
            .unwrap()
            .insert(id.to_string(), cwd.to_string());
    }
}

fn schedule_codex_label_refresh(
    state: Arc<AppState>,
    session_id: String,
    native_session_id: String,
    runtime_identity: crate::runtime::session_runtime::RuntimeIdentity,
    refresh_epoch: u64,
) {
    tokio::spawn(async move {
        for delay in [
            std::time::Duration::ZERO,
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(500),
        ] {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let lookup_id = native_session_id.clone();
            let candidate = tokio::task::spawn_blocking(move || {
                crate::codex_session_store::lookup_label_candidate(&lookup_id)
                    .ok()
                    .flatten()
            })
            .await
            .ok()
            .flatten();
            let Some(candidate) = candidate else {
                continue;
            };
            if crate::codex_session_store::native_label_refresh_is_blocked(
                &session_id,
                &native_session_id,
            ) {
                return;
            }
            if SessionApplication::new(state.clone()).persist_codex_label_for_runtime(
                &session_id,
                &native_session_id,
                candidate.text,
                &runtime_identity,
                refresh_epoch,
            ) {
                return;
            }
        }
    });
}
