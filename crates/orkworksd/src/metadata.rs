use crate::harness::{ResumeMemory, ResumeState, ResumeStrategy};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::warn;

pub const TERMINAL_OUTPUT_MAX_LINES: usize = 10_000;

fn default_connectivity() -> String {
    "online".into()
}

fn default_work_phase() -> String {
    "unknown".into()
}

fn default_lifecycle_phase() -> String {
    String::new()
}

fn default_lifecycle() -> String {
    "alive".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservedStatusSnapshotMetadata {
    pub value: Option<String>,
    pub source: String,
    pub confidence: Option<f64>,
    #[serde(rename = "observedAt")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumeOption {
    pub strategy: ResumeStrategy,
    pub label: String,
    pub available: bool,
    pub preferred: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ResumeOption {
    fn new(
        strategy: ResumeStrategy,
        label: &'static str,
        available: bool,
        reason: Option<&'static str>,
    ) -> Self {
        Self {
            strategy,
            label: label.into(),
            available,
            preferred: false,
            reason: reason.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub label: String,
    pub workspace: String,
    pub task: String,
    #[serde(rename = "harnessId", alias = "harness", default)]
    pub harness: String,
    #[serde(rename = "modelId", alias = "model", default)]
    pub model: String,
    pub cwd: String,
    pub status: String,
    #[serde(rename = "workPhase", alias = "phase", default = "default_work_phase")]
    pub work_phase: String,
    #[serde(rename = "lifecyclePhase", default = "default_lifecycle_phase")]
    pub lifecycle_phase: String,
    #[serde(default = "default_lifecycle")]
    pub lifecycle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention: Option<String>,
    #[serde(default = "default_connectivity")]
    pub connectivity: String,
    #[serde(rename = "terminalOutcome", skip_serializing_if = "Option::is_none")]
    pub terminal_outcome: Option<String>,
    #[serde(rename = "pendingTerminalStatus", skip_serializing_if = "Option::is_none")]
    pub pending_terminal_status: Option<String>,
    #[serde(rename = "observedStatus")]
    pub observed_status: Option<String>,
    #[serde(rename = "endingObservedStatusSnapshot", skip_serializing_if = "Option::is_none")]
    pub ending_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata>,
    #[serde(rename = "finalObservedStatusSnapshot", skip_serializing_if = "Option::is_none")]
    pub final_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata>,
    pub summary: Option<String>,
    #[serde(rename = "nextAction")]
    pub next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    pub needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    pub detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    pub suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    pub blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    pub failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    pub failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    pub capacity_hints: Option<Vec<String>>,
    #[serde(rename = "peonLastInference")]
    pub peon_last_inference: Option<String>,
    #[serde(rename = "modelProviderId", alias = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(rename = "modelProviderLabel", alias = "providerLabel", skip_serializing_if = "Option::is_none")]
    pub provider_label: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    pub provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    #[serde(rename = "metadataSource")]
    pub metadata_source: String,
    #[serde(rename = "metadataConfidence")]
    pub metadata_confidence: f64,
    #[serde(rename = "repoRoot")]
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    pub changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    pub is_worktree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<ResumeMemory>,
    #[serde(rename = "resumeOptions", default)]
    pub resume_options: Vec<ResumeOption>,
    #[serde(rename = "harnessSessionIdSource", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_source: Option<String>,
    #[serde(rename = "harnessSessionIdConfidence", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_confidence: Option<f64>,
    #[serde(rename = "harnessSessionIdCapturedAt", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_captured_at: Option<String>,
    #[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
    pub resumed_from: Option<String>,
    #[serde(rename = "lastUserInput", skip_serializing_if = "Option::is_none")]
    pub last_user_input: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub status: String,
    #[serde(rename = "observedStatus")]
    pub observed_status: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMemory {
    #[serde(rename = "lastActiveSessionId", skip_serializing_if = "Option::is_none")]
    pub last_active_session_id: Option<String>,
    #[serde(rename = "lastActiveAt", skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
    #[serde(rename = "activeHarnessIds", default, skip_serializing_if = "Vec::is_empty")]
    pub active_harness_ids: Vec<String>,
}

fn normalize_session_metadata(mut meta: SessionMetadata) -> SessionMetadata {
    meta.work_phase = normalize_work_phase(&meta.work_phase);

    if meta.lifecycle_phase.is_empty() {
        meta.lifecycle_phase = default_lifecycle_phase_for_status(&meta.status);
    }

    if meta.lifecycle == "alive" {
        meta.lifecycle = match meta.lifecycle_phase.as_str() {
            "creating" => "creating",
            "ending" => "stopping",
            "ended" => "dead",
            _ => "alive",
        }
        .into();
    }
    if meta.attention.is_none() && meta.lifecycle == "alive" {
        meta.attention = match meta.observed_status.as_deref() {
            Some("stale" | "done") => Some("idle".into()),
            Some("waiting_for_input") => Some("needs_you".into()),
            Some("working" | "idle" | "blocked" | "failed" | "capped") => meta.observed_status.clone(),
            _ => None,
        };
    }

    if meta.lifecycle_phase == "ending" && meta.status != "running" {
        meta.status = "running".into();
    }

    let inferred_terminal_outcome = match meta.status.as_str() {
        "ended" => Some("ended"),
        "killed" => Some("killed"),
        "error" => Some("error"),
        _ => None,
    };

    if inferred_terminal_outcome.is_some() && meta.terminal_outcome.is_none() {
        meta.terminal_outcome = inferred_terminal_outcome.map(str::to_string);
    }

    if inferred_terminal_outcome.is_some() && meta.connectivity == "online" {
        meta.connectivity = "offline".into();
    }

    if meta.lifecycle_phase != "ending" {
        meta.pending_terminal_status = None;
        meta.ending_observed_status_snapshot = None;
    }

    if meta.lifecycle_phase == "ending" && meta.pending_terminal_status.is_none() {
        meta.lifecycle_phase = "ended".into();
        meta.status = "error".into();
        meta.terminal_outcome = Some("error".into());
        meta.connectivity = "offline".into();
    }

    if meta.lifecycle_phase == "ended" && meta.final_observed_status_snapshot.is_none() {
        meta.final_observed_status_snapshot = Some(
            meta.ending_observed_status_snapshot
                .clone()
                .or_else(|| snapshot_from_legacy_observed_status(&meta))
                .unwrap_or_else(|| canonical_null_snapshot("recovery", None)),
        );
    }

    if matches!(meta.lifecycle_phase.as_str(), "creating" | "ending" | "ended") {
        meta.observed_status = None;
    }

    if meta.lifecycle != "alive" {
        meta.attention = None;
    }

    if matches!(meta.status.as_str(), "ended" | "killed" | "error") {
        meta.lifecycle_phase = "ended".into();
        if meta.final_observed_status_snapshot.is_none() {
            meta.final_observed_status_snapshot = Some(
                snapshot_from_legacy_observed_status(&meta)
                    .unwrap_or_else(|| canonical_null_snapshot("recovery", None)),
            );
        }
        meta.pending_terminal_status = None;
        meta.ending_observed_status_snapshot = None;
        meta.observed_status = None;
    }

    meta
}

fn normalize_work_phase(raw: &str) -> String {
    match raw {
        "ideation" | "implementation" | "review" | "debugging" | "unknown" => raw.to_string(),
        "" => "unknown".into(),
        _ => "unknown".into(),
    }
}

fn canonical_attention(raw: Option<&str>) -> Option<String> {
    match raw {
        Some("waiting_for_input") => Some("needs_you".into()),
        Some("stale" | "done") => Some("idle".into()),
        Some("working" | "idle" | "blocked" | "failed" | "capped") => raw.map(str::to_string),
        _ => None,
    }
}

fn default_lifecycle_phase_for_status(status: &str) -> String {
    match status {
        "creating" => "creating".into(),
        "running" => "active".into(),
        "ended" | "killed" | "error" => "ended".into(),
        _ => "active".into(),
    }
}

fn snapshot_from_legacy_observed_status(
    meta: &SessionMetadata,
) -> Option<ObservedStatusSnapshotMetadata> {
    meta.observed_status.as_ref().map(|status| ObservedStatusSnapshotMetadata {
        value: Some(status.clone()),
        source: if meta.metadata_source.is_empty() {
            "recovery".into()
        } else {
            meta.metadata_source.clone()
        },
        confidence: Some(meta.metadata_confidence),
        observed_at: Some(meta.last_activity.clone()),
    })
}

pub(crate) fn canonical_null_snapshot(
    source: &str,
    observed_at: Option<String>,
) -> ObservedStatusSnapshotMetadata {
    ObservedStatusSnapshotMetadata {
        value: None,
        source: source.into(),
        confidence: None,
        observed_at,
    }
}

/// Completes the lifecycle of a session orphaned by a previous daemon run.
///
/// Sessions found "running"/"creating" on workspace open have no live process.
/// A session persisted mid-`ending` must consume its `pending_terminal_status`
/// as the final status here — writing a bare terminal `status` while
/// `lifecycle_phase` stays "ending" would be reverted by
/// `normalize_session_metadata`, which forces `status` back to "running" for
/// in-flight endings.
pub(crate) fn reconcile_orphaned_session(mut meta: SessionMetadata, now: &str) -> SessionMetadata {
    let final_status = if meta.lifecycle_phase == "ending" {
        meta.pending_terminal_status
            .take()
            .unwrap_or_else(|| "error".into())
    } else {
        "ended".into()
    };
    if meta.final_observed_status_snapshot.is_none() {
        meta.final_observed_status_snapshot = Some(
            meta.ending_observed_status_snapshot
                .clone()
                .or_else(|| snapshot_from_legacy_observed_status(&meta))
                .unwrap_or_else(|| canonical_null_snapshot("recovery", Some(now.to_string()))),
        );
    }
    meta.lifecycle_phase = "ended".into();
    meta.lifecycle = "dead".into();
    meta.attention = None;
    meta.terminal_outcome = Some(final_status.clone());
    meta.status = final_status;
    meta.connectivity = "offline".into();
    meta.pending_terminal_status = None;
    meta.ending_observed_status_snapshot = None;
    meta.observed_status = None;
    meta.last_activity = now.to_string();
    meta
}

pub fn derive_resume_options(
    preferred: &ResumeStrategy,
    resume: Option<&ResumeMemory>,
    supports_exact: bool,
    supports_latest_cwd: bool,
    supports_latest_repo: bool,
) -> Vec<ResumeOption> {
    let resume_available = resume
        .map(|memory| memory.state == ResumeState::Available)
        .unwrap_or(false);
    let exact_reason = if !supports_exact {
        Some("Harness does not support exact resume")
    } else if !resume_available {
        Some("No compatible remembered session exists")
    } else if resume.and_then(|memory| memory.harness_session_id.as_ref()).is_none() {
        Some("No harness session id was captured")
    } else {
        None
    };
    let latest_reason = |supported: bool| {
        if !supported {
            Some("Harness does not support folder-scoped resume")
        } else if !resume_available || !resume.map(|memory| memory.latest_fallback).unwrap_or(false)
        {
            Some("No compatible remembered session exists")
        } else {
            None
        }
    };
    let latest_repo_reason = if !supports_latest_repo {
        Some("Harness does not support repo-scoped resume")
    } else if !resume_available || !resume.map(|memory| memory.latest_fallback).unwrap_or(false) {
        Some("No compatible remembered session exists")
    } else {
        None
    };

    let mut options = vec![
        ResumeOption::new(
            ResumeStrategy::Exact,
            "Resume exact session",
            exact_reason.is_none(),
            exact_reason,
        ),
        ResumeOption::new(
            ResumeStrategy::LatestCwd,
            "Resume latest in folder",
            latest_reason(supports_latest_cwd).is_none(),
            latest_reason(supports_latest_cwd),
        ),
        ResumeOption::new(
            ResumeStrategy::LatestRepo,
            "Resume latest in repo",
            latest_repo_reason.is_none(),
            latest_repo_reason,
        ),
    ];

    for option in &mut options {
        option.preferred = option.strategy == *preferred;
    }

    options
}

#[cfg(test)]
pub(crate) fn assert_session_metadata_serializes_connectivity_terminal_outcome_and_last_activity() {
    let meta = SessionMetadata {
        id: "s1".into(),
        label: "Test".into(),
        workspace: "/tmp".into(),
        task: String::new(),
        harness: String::new(),
        model: String::new(),
        cwd: "/tmp".into(),
        status: "ended".into(),
        work_phase: "unknown".into(),
        lifecycle_phase: "ended".into(),
        lifecycle: "dead".into(),
        attention: None,
        connectivity: "offline".into(),
        terminal_outcome: Some("ended".into()),
        pending_terminal_status: None,
        observed_status: None,
        ending_observed_status_snapshot: None,
        final_observed_status_snapshot: Some(canonical_null_snapshot("recovery", None)),
        summary: None,
        next_action: None,
        needs_user_input: None,
        detected_question: None,
        suggested_options: None,
        blocker_description: None,
        failed_command: None,
        failed_test: None,
        capacity_hints: None,
        peon_last_inference: None,
        provider_id: None,
        provider_label: None,
        provider_model: None,
        provider_state: None,
        created_at: "2026-06-28T09:00:00Z".into(),
        last_activity: "2026-06-28T09:05:00Z".into(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
        repo_root: None,
        branch: None,
        dirty: None,
        changed_files: None,
        is_worktree: None,
        resume: None,
        resume_options: vec![],
        harness_session_id_source: None,
        harness_session_id_confidence: None,
        harness_session_id_captured_at: None,
        resumed_from: None,
        last_user_input: None,
    };

    let raw = serde_json::to_value(&meta).unwrap();
    assert_eq!(raw["connectivity"], "offline");
    assert_eq!(raw["terminalOutcome"], "ended");
    assert_eq!(raw["lastActivity"], "2026-06-28T09:05:00Z");
}

#[cfg(test)]
#[test]
fn session_metadata_serializes_connectivity_terminal_outcome_and_last_activity() {
    assert_session_metadata_serializes_connectivity_terminal_outcome_and_last_activity();
}

#[cfg(test)]
#[test]
fn session_metadata_reads_legacy_phase_and_projects_final_observed_status() {
    let raw = r#"{
      "id":"s1",
      "label":"Test",
      "workspace":"/tmp",
      "task":"",
      "harnessId":"",
      "modelId":"",
      "cwd":"/tmp",
      "status":"ended",
      "phase":"review",
      "createdAt":"now",
      "lastActivity":"now",
      "metadataSource":"process",
      "metadataConfidence":1.0,
      "observedStatus":"blocked"
    }"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.work_phase, "review");
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(
        meta.final_observed_status_snapshot
            .as_ref()
            .and_then(|x| x.value.as_deref()),
        Some("blocked")
    );
}

#[cfg(test)]
#[test]
fn normalizes_legacy_runtime_and_observer_values_to_canonical_state() {
    let raw = r#"{"id":"canonical","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"active","observedStatus":"stale","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());

    assert_eq!(meta.lifecycle, "alive");
    assert_eq!(meta.attention.as_deref(), Some("idle"));
}

#[cfg(test)]
#[test]
fn normalize_terminal_legacy_metadata_builds_canonical_null_snapshot() {
    let raw = r#"{"id":"s2","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"ended","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    let snap = meta.final_observed_status_snapshot.unwrap();
    assert_eq!(snap.value, None);
    assert_eq!(snap.source, "recovery");
    assert_eq!(snap.confidence, None);
    assert_eq!(snap.observed_at, None);
}

#[cfg(test)]
#[test]
fn normalize_invalid_ending_without_pending_status_becomes_error_ended() {
    let raw = r#"{"id":"s3","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"ending","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.status, "error");
}

#[cfg(test)]
#[test]
fn normalize_unknown_legacy_phase_to_unknown_work_phase() {
    let raw = r#"{"id":"s4","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","phase":"freeform","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.work_phase, "unknown");
}

#[cfg(test)]
#[test]
fn normalize_pending_terminal_status_outside_ending_to_null_and_clear_live_observed_status() {
    let raw = r#"{"id":"s5","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"ended","lifecyclePhase":"ended","pendingTerminalStatus":"killed","observedStatus":"blocked","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.pending_terminal_status, None);
    assert_eq!(meta.observed_status, None);
}

#[cfg(test)]
#[test]
fn normalize_recovery_prefers_existing_final_snapshot() {
    let raw = r#"{"id":"s6","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"ending","pendingTerminalStatus":"ended","finalObservedStatusSnapshot":{"value":"done","source":"peon","confidence":0.9,"observedAt":"now"},"createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(
        meta.final_observed_status_snapshot
            .as_ref()
            .and_then(|x| x.value.as_deref()),
        Some("done")
    );
}

#[cfg(test)]
#[test]
fn reconcile_orphaned_mid_ending_session_consumes_pending_status_and_survives_normalize() {
    let raw = r#"{"id":"s7","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"ending","pendingTerminalStatus":"killed","endingObservedStatusSnapshot":{"value":"blocked","source":"peon","confidence":0.8,"observedAt":"before"},"createdAt":"now","lastActivity":"now","metadataSource":"peon","metadataConfidence":0.8}"#;
    let meta = reconcile_orphaned_session(serde_json::from_str(raw).unwrap(), "later");
    assert_eq!(meta.status, "killed");
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.terminal_outcome.as_deref(), Some("killed"));
    assert_eq!(meta.pending_terminal_status, None);
    assert_eq!(meta.ending_observed_status_snapshot, None);
    assert_eq!(
        meta.final_observed_status_snapshot
            .as_ref()
            .and_then(|x| x.value.as_deref()),
        Some("blocked")
    );

    // A read of the reconciled file must not flip the session back to running.
    let normalized = normalize_session_metadata(
        serde_json::from_str(&serde_json::to_string(&meta).unwrap()).unwrap(),
    );
    assert_eq!(normalized.status, "killed");
    assert_eq!(normalized.lifecycle_phase, "ended");
}

#[cfg(test)]
#[test]
fn reconcile_orphaned_running_session_freezes_legacy_observed_status() {
    let raw = r#"{"id":"s8","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"active","observedStatus":"blocked","createdAt":"now","lastActivity":"now","metadataSource":"peon","metadataConfidence":0.8}"#;
    let meta = reconcile_orphaned_session(serde_json::from_str(raw).unwrap(), "later");
    assert_eq!(meta.status, "ended");
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.observed_status, None);
    assert_eq!(
        meta.final_observed_status_snapshot
            .as_ref()
            .and_then(|x| x.value.as_deref()),
        Some("blocked")
    );
}

#[cfg(test)]
#[test]
fn derive_resume_options_returns_disabled_entries_with_reasons() {
    let resume = ResumeMemory {
        state: ResumeState::Available,
        preferred_strategy: ResumeStrategy::Exact,
        harness_session_id: None,
        latest_fallback: false,
        last_seen_at: None,
    };
    let options = derive_resume_options(&ResumeStrategy::Exact, Some(&resume), true, false, false);

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].strategy, ResumeStrategy::Exact);
    assert!(!options[0].available);
    assert_eq!(
        options[0].reason.as_deref(),
        Some("No harness session id was captured"),
    );
    assert_eq!(options[1].strategy, ResumeStrategy::LatestCwd);
    assert!(!options[1].available);
    assert_eq!(
        options[1].reason.as_deref(),
        Some("Harness does not support folder-scoped resume"),
    );
    assert_eq!(options[2].strategy, ResumeStrategy::LatestRepo);
    assert!(!options[2].available);
    assert_eq!(
        options[2].reason.as_deref(),
        Some("Harness does not support repo-scoped resume"),
    );
}

pub const HARNESS_SESSION_ID_MIN_LEN: usize = 3;
pub const HARNESS_SESSION_ID_MAX_LEN: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessSessionReport {
    pub harness_session_id: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessSessionMergeResult {
    Accepted,
    IgnoredLowerConfidence,
    NotFound,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionMergeResult {
    Accepted,
    Ignored,
    NotFound,
    /// The signal was accepted but could not be persisted; callers must not
    /// acknowledge it as delivered (the hook needs a non-2xx so it can retry).
    PersistFailed,
}

/// Writes `contents` to `path` via a temp file in the same directory plus an
/// atomic rename, so readers never observe a partially written file and a
/// mid-write kill cannot corrupt the previous contents.
fn write_atomic(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    let tmp = tmp_write_path(path);
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

fn tmp_write_path(path: &std::path::Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

fn corrupt_session_path(path: &std::path::Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".corrupt");
    path.with_file_name(name)
}

pub fn valid_harness_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() >= HARNESS_SESSION_ID_MIN_LEN
        && id.len() <= HARNESS_SESSION_ID_MAX_LEN
        && !id.contains(char::is_whitespace)
}

pub fn valid_harness_session_report(report: &HarnessSessionReport) -> bool {
    valid_harness_session_id(&report.harness_session_id)
        && !report.source.trim().is_empty()
        && (0.0..=1.0).contains(&report.confidence)
}

pub struct MetadataStore {
    root: PathBuf,
}

impl MetadataStore {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    pub fn workspace_memory_path(&self) -> PathBuf {
        self.root.join("workspace.json")
    }

    pub fn read_workspace_memory(&self) -> Option<WorkspaceMemory> {
        let data = fs::read_to_string(self.workspace_memory_path()).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn write_workspace_memory(&self, memory: &WorkspaceMemory) {
        if let Err(e) = fs::create_dir_all(&self.root) {
            warn!("failed to create metadata root {:?}: {e}", self.root);
            return;
        }
        let path = self.workspace_memory_path();
        match serde_json::to_string_pretty(memory) {
            Ok(json) => {
                if let Err(e) = write_atomic(&path, &json) {
                    warn!("failed to write workspace memory {:?}: {e}", path);
                }
            }
            Err(e) => warn!("failed to serialize workspace memory: {e}"),
        }
    }

    pub fn read_all_sessions(&self) -> Vec<SessionMetadata> {
        let dir = self.sessions_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => return vec![],
        };
        let mut sessions: Vec<SessionMetadata> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
            .filter_map(|entry| self.load_session_file(&entry.path()))
            .collect();
        sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        sessions
    }

    /// Reads and parses one session file. A file that exists but does not
    /// parse is quarantined (renamed to `<id>.json.corrupt`) and logged, so a
    /// corrupt session disappears from the list observably instead of
    /// silently — and only once, not on every poll.
    fn load_session_file(&self, path: &std::path::Path) -> Option<SessionMetadata> {
        let data = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => {
                warn!("failed to read session file {:?}: {e}", path);
                return None;
            }
        };
        match serde_json::from_str::<SessionMetadata>(&data) {
            Ok(meta) => Some(normalize_session_metadata(meta)),
            Err(e) => {
                let quarantine = corrupt_session_path(path);
                match fs::rename(path, &quarantine) {
                    Ok(()) => warn!(
                        "session file {:?} is corrupt ({e}); quarantined to {:?}",
                        path, quarantine
                    ),
                    Err(rename_err) => warn!(
                        "session file {:?} is corrupt ({e}) and could not be quarantined: {rename_err}",
                        path
                    ),
                }
                None
            }
        }
    }

    /// Persists a session atomically: the JSON is written to a temp file in
    /// the same directory and renamed into place, so a process killed
    /// mid-write leaves the previous valid file, never a truncated one.
    pub fn try_write_session(&self, meta: &SessionMetadata) -> std::io::Result<()> {
        let dir = self.sessions_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", meta.id));
        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        write_atomic(&path, &json)
    }

    pub fn write_session(&self, meta: &SessionMetadata) {
        if let Err(e) = self.try_write_session(meta) {
            warn!("failed to write session {}: {e}", meta.id);
        }
    }

    /// True when a metadata file for this session is present on disk — even
    /// one that no longer parses. Cleanup paths must treat a corrupt-but-
    /// present session as existing, or it becomes undeletable.
    pub fn session_file_exists(&self, id: &str) -> bool {
        let path = self.sessions_dir().join(format!("{}.json", id));
        path.exists() || corrupt_session_path(&path).exists()
    }

    pub fn delete_session(&self, id: &str) -> std::io::Result<()> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        for target in [path.clone(), corrupt_session_path(&path)] {
            match fs::remove_file(&target) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn delete_events(&self, id: &str) -> std::io::Result<()> {
        let ndjson_path = self.events_dir().join(format!("{}.ndjson", id));
        let terminal_path = self.terminal_output_path(id);

        if let Err(e) = fs::remove_file(&ndjson_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
        if let Err(e) = fs::remove_file(&terminal_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn clear_last_active_session_if_matches(&self, id: &str) -> std::io::Result<()> {
        let Some(mut memory) = self.read_workspace_memory() else {
            return Ok(());
        };
        if memory.last_active_session_id.as_deref() == Some(id) {
            memory.last_active_session_id = None;
            memory.last_active_at = None;
            self.write_workspace_memory(&memory);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_events(&self, id: &str) -> Vec<Event> {
        let path = self.events_dir().join(format!("{}.ndjson", id));
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        data.lines()
            .filter_map(|line| serde_json::from_str::<Event>(line).ok())
            .collect()
    }

    pub fn read_session(&self, id: &str) -> Option<SessionMetadata> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        self.load_session_file(&path)
    }

    pub fn session_modified_secs_ago(&self, id: &str) -> Option<u64> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        let modified = fs::metadata(path).ok()?.modified().ok()?;
        modified.elapsed().ok().map(|elapsed| elapsed.as_secs())
    }

    pub fn append_event(&self, id: &str, event: &Event) {
        let dir = self.events_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("failed to create events dir {:?}: {e}", dir);
            return;
        }
        let path = dir.join(format!("{}.ndjson", id));
        match serde_json::to_string(event) {
            Ok(json) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "{json}") {
                        warn!("failed to write event to {id}: {e}");
                    }
                }
                Err(e) => warn!("failed to open event file for {id}: {e}"),
            },
            Err(e) => warn!("failed to serialize event for {id}: {e}"),
        }
    }

    pub fn persist_provider_context(
        &self,
        id: &str,
        provider: &crate::providers::ProviderObservation,
    ) {
        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return,
        };
        meta.provider_id = Some(provider.provider_id.clone());
        meta.provider_label = Some(provider.provider_label.clone());
        meta.provider_model = provider.provider_model.clone();
        meta.provider_state = Some(provider.provider_state.clone());
        if let Err(e) = self.try_write_session(&meta) {
            warn!("failed to persist provider context for {id}: {e}");
        }
    }

    pub fn merge_harness_session_report(
        &self,
        id: &str,
        report: &HarnessSessionReport,
        timestamp: &str,
    ) -> HarnessSessionMergeResult {
        if !valid_harness_session_report(report) {
            return HarnessSessionMergeResult::Invalid;
        }

        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return HarnessSessionMergeResult::NotFound,
        };

        let existing_confidence = meta.harness_session_id_confidence.unwrap_or(-1.0);
        let existing_id = meta
            .resume
            .as_ref()
            .and_then(|resume| resume.harness_session_id.as_deref());

        if existing_id.is_some() && report.confidence < existing_confidence {
            return HarnessSessionMergeResult::IgnoredLowerConfidence;
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
        self.write_session(&meta);

        self.append_event(id, &Event {
            event_type: "session.harness_session_captured".into(),
            timestamp: timestamp.to_string(),
            status: meta.status.clone(),
            observed_status: None,
            confidence: Some(report.confidence),
        });

        HarnessSessionMergeResult::Accepted
    }

    /// Writes a deterministic, harness-supplied attention signal (e.g. from a Claude Code
    /// `Notification` hook), gated by `should_overwrite`'s priority/staleness rule: it
    /// cannot clobber fresh `user` metadata, and cannot immediately clobber another fresh
    /// `agent` write either, but always outranks peon/backend_inference/process/unknown.
    /// Peon's own write path uses the shorter `peon_should_overwrite` window instead, so
    /// Peon can correct a stale `agent` status well before this 5-minute self-refresh
    /// window would let a second hook event do the same.
    pub fn merge_agent_attention_signal(
        &self,
        id: &str,
        status: &str,
        message: Option<&str>,
        timestamp: &str,
    ) -> AttentionMergeResult {
        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return AttentionMergeResult::NotFound,
        };

        let age = self.session_modified_secs_ago(id);
        if !crate::peon::should_overwrite(&meta.metadata_source, age) {
            return AttentionMergeResult::Ignored;
        }

        meta.observed_status = Some(status.to_string());
        if meta.lifecycle == "alive" {
            meta.attention = canonical_attention(Some(status));
        }
        if let Some(msg) = message {
            meta.summary = Some(msg.to_string());
        }
        meta.metadata_source = "agent".into();
        meta.metadata_confidence = 1.0;
        if let Err(e) = self.try_write_session(&meta) {
            warn!("failed to persist attention signal for {id}: {e}");
            return AttentionMergeResult::PersistFailed;
        }

        self.append_event(id, &Event {
            event_type: "session.attention_reported".into(),
            timestamp: timestamp.to_string(),
            status: meta.status.clone(),
            observed_status: Some(status.to_string()),
            confidence: Some(1.0),
        });

        AttentionMergeResult::Accepted
    }

    /// Returns `Err` when the merged metadata could not be persisted, so the
    /// caller does not treat the inference as landed (e.g. updating in-memory
    /// state to match a write that never happened).
    pub fn merge_peon_inference(
        &self,
        id: &str,
        inf: &crate::peon::PeonInference,
        timestamp: &str,
        provider: Option<&crate::providers::ProviderObservation>,
    ) -> std::io::Result<()> {
        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return Ok(()),
        };
        let peon_harness_session_report =
            inf.harness_session_id.as_ref().map(|sid| HarnessSessionReport {
                harness_session_id: sid.clone(),
                source: "peon".into(),
                confidence: inf.confidence.min(0.50),
            });

        // Observer-only inference cannot resume a finished/non-working session to
        // `working` on its own — that requires qualifying user input, which clears
        // `observed_status` via the terminal-input path first. See issue #170.
        let resumes_finished_state = inf.observed_status.as_deref() == Some("working")
            && crate::peon::is_terminal_observed_status(meta.observed_status.as_deref());
        if !resumes_finished_state {
            meta.observed_status = inf.observed_status.clone().or(meta.observed_status);
        }
        if meta.lifecycle == "alive" {
            meta.attention = canonical_attention(meta.observed_status.as_deref());
        }
        if let Some(ref phase) = inf.phase {
            meta.work_phase = normalize_work_phase(phase);
        }
        meta.summary = inf.summary.clone().or(meta.summary);
        if let Some(ref summary) = inf.summary {
            meta.label = summary.chars().take(100).collect();
        }
        meta.next_action = inf.next_action.clone().or(meta.next_action);
        meta.needs_user_input = inf.needs_user_input.or(meta.needs_user_input);
        // Normalize: treat empty-string question as absent (LLM may emit "" instead of null).
        let incoming_q = inf.detected_question.as_deref()
            .filter(|q| !q.is_empty())
            .map(str::to_string);
        // Options belong to their question; clear them when the question changes so
        // stale options never appear under a different question.
        if incoming_q.is_some() && incoming_q.as_deref() != meta.detected_question.as_deref() {
            meta.suggested_options = None;
        }
        meta.detected_question = incoming_q.or(meta.detected_question);
        meta.suggested_options = inf.suggested_options.clone().or(meta.suggested_options);
        meta.blocker_description = inf.blocker_description.clone().or(meta.blocker_description);
        meta.failed_command = inf.failed_command.clone().or(meta.failed_command);
        meta.failed_test = inf.failed_test.clone().or(meta.failed_test);
        meta.capacity_hints = inf.capacity_hints.clone().or(meta.capacity_hints);

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

        meta.peon_last_inference = Some(timestamp.to_string());
        meta.metadata_source = "peon".into();
        meta.metadata_confidence = inf.confidence;

        if let Some(p) = provider {
            meta.provider_id = Some(p.provider_id.clone());
            meta.provider_label = Some(p.provider_label.clone());
            meta.provider_model = p.provider_model.clone();
            meta.provider_state = Some(p.provider_state.clone());
        }

        self.try_write_session(&meta)?;

        self.append_event(id, &Event {
            event_type: "peon.inference".into(),
            timestamp: timestamp.to_string(),
            status: meta.status.clone(),
            observed_status: inf.observed_status.clone(),
            confidence: Some(inf.confidence),
        });

        if let Some(report) = peon_harness_session_report {
            let _ = self.merge_harness_session_report(id, &report, timestamp);
        }
        Ok(())
    }

    fn terminal_output_path(&self, id: &str) -> PathBuf {
        self.events_dir().join(format!("{}.terminal", id))
    }

    pub fn append_terminal_output_lines(&self, id: &str, lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        if let Err(e) = fs::create_dir_all(&self.events_dir()) {
            warn!("failed to create events dir for terminal output: {e}");
            return;
        }
        let path = self.terminal_output_path(id);
        let mut file = match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => f,
            Err(e) => {
                warn!("failed to open terminal output file for {id}: {e}");
                return;
            }
        };
        for line in lines {
            if let Err(e) = writeln!(file, "{line}") {
                warn!("failed to append terminal output for {id}: {e}");
                return;
            }
        }
        // Inline trim when the file exceeds 1.5x max_lines to prevent unbounded growth
        // during long-running sessions. Only check approximate size via metadata.
        let len_hint = file.metadata().map(|m| m.len()).unwrap_or(0);
        drop(file);
        // Rough estimate: 100 bytes per line, so 1.5x MAX_LINES ≈ 150 * MAX_LINES bytes
        if len_hint > (TERMINAL_OUTPUT_MAX_LINES as u64 * 150) {
            let _ = self.trim_terminal_output(id, TERMINAL_OUTPUT_MAX_LINES);
        }
    }

    pub fn read_terminal_output(&self, id: &str, max_lines: usize) -> Vec<String> {
        let path = self.terminal_output_path(id);
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        let all: Vec<&str> = data.lines().collect();
        let start = if all.len() > max_lines { all.len() - max_lines } else { 0 };
        all[start..].iter().map(|s| s.to_string()).collect()
    }

    pub fn delete_terminal_output(&self, id: &str) {
        let path = self.terminal_output_path(id);
        if let Err(e) = fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("failed to delete terminal output for {id}: {e}");
            }
        }
    }

    pub fn trim_terminal_output(&self, id: &str, max_lines: usize) {
        let path = self.terminal_output_path(id);
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let all: Vec<&str> = data.lines().collect();
        if all.len() <= max_lines {
            return;
        }
        let start = all.len() - max_lines;
        match fs::write(&path, all[start..].join("\n") + "\n") {
            Ok(_) => {}
            Err(e) => warn!("failed to trim terminal output for {id}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = SessionMetadata {
            id: "test-1".into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            work_phase: "implementation".into(),
            lifecycle_phase: "active".into(),
            lifecycle: "alive".into(),
            attention: None,
            connectivity: "online".into(),
            terminal_outcome: None,
            pending_terminal_status: None,
            observed_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
            resume: None,
            resume_options: vec![],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        };
        store.write_session(&meta);
        let read = store.read_session("test-1").unwrap();
        assert_eq!(read.status, "running");
    }

    #[test]
    fn append_and_read_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("test-2", &Event {
            event_type: "session.created".into(),
            timestamp: "now".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_event("test-2", &Event {
            event_type: "session.status".into(),
            timestamp: "later".into(),
            status: "running".into(),
            observed_status: None,
            confidence: None,
        });
        let path = store.events_dir().join("test-2.ndjson");
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().count(), 2);
    }

    #[test]
    fn read_events_returns_deserialized_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("test-3", &Event {
            event_type: "session.created".into(),
            timestamp: "t1".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_event("test-3", &Event {
            event_type: "session.status".into(),
            timestamp: "t2".into(),
            status: "running".into(),
            observed_status: None,
            confidence: None,
        });
        let events = store.read_events("test-3");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "session.created");
        assert_eq!(events[1].status, "running");
    }

    #[test]
    fn read_events_returns_empty_for_missing_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let events = store.read_events("nonexistent");
        assert!(events.is_empty());
    }

    #[test]
    fn merge_peon_inference_renames_session_when_harness_detected() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&SessionMetadata {
            id: "rename-test".into(),
            label: "Session abc12345".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
        work_phase: "unknown".into(),
        lifecycle_phase: "active".into(),
        lifecycle: "alive".into(),
        attention: None,
        connectivity: "online".into(),
            terminal_outcome: None,
            pending_terminal_status: None,
            observed_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resume_options: vec![],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        });

        // First inference: harness detected, no model
        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.8,
            detected_harness: Some("claude-code".into()),
            detected_model: None,
            harness_session_id: None,
        };
        store.merge_peon_inference("rename-test", &inf, "t1", None).unwrap();
        let meta = store.read_session("rename-test").unwrap();
        // Peon no longer updates the label — harness/model are recorded but label is unchanged
        assert_eq!(meta.label, "Session abc12345");
        assert_eq!(meta.harness, "claude-code");
        assert_eq!(meta.model, "");

        let inf2 = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: Some("claude-code".into()),
            detected_model: Some("claude-sonnet-4-5".into()),
            harness_session_id: None,
        };
        store.merge_peon_inference("rename-test", &inf2, "t2", None).unwrap();
        let meta2 = store.read_session("rename-test").unwrap();
        assert_eq!(meta2.label, "Session abc12345");
        assert_eq!(meta2.harness, "claude-code");
        assert_eq!(meta2.model, "claude-sonnet-4-5");
    }

    #[test]
    fn merge_peon_inference_preserves_lifecycle_status_and_writes_observer_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&SessionMetadata {
            id: "test-peon-observer".into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            work_phase: "unknown".into(),
            lifecycle_phase: "active".into(),
            lifecycle: "alive".into(),
            attention: None,
            connectivity: "online".into(),
            terminal_outcome: None,
            pending_terminal_status: None,
            observed_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resume_options: vec![],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        });

        let inf = crate::peon::PeonInference {
            observed_status: Some("waiting_for_input".into()),
            phase: Some("review".into()),
            summary: Some("Needs a decision".into()),
            next_action: Some("Pick an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.82,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };

        store.merge_peon_inference("test-peon-observer", &inf, "later", None).unwrap();

        let meta = store.read_session("test-peon-observer").unwrap();
        assert_eq!(meta.status, "running");

        let path = store.sessions_dir().join("test-peon-observer.json");
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(raw["observedStatus"], "waiting_for_input");
        assert_eq!(raw["summary"], "Needs a decision");
        assert_eq!(raw["needsUserInput"], true);
        assert_eq!(raw["peonLastInference"], "later");
    }

    #[test]
    fn peon_inference_cannot_resume_finished_state_to_working() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        for finished_status in ["idle", "waiting_for_input", "blocked", "failed", "stale", "done"] {
            let id = format!("finished-{finished_status}");
            let mut meta = test_metadata(&id);
            meta.observed_status = Some(finished_status.into());
            meta.attention = canonical_attention(Some(finished_status));
            store.write_session(&meta);

            let inf = crate::peon::PeonInference {
                observed_status: Some("working".into()),
                phase: None, summary: Some("still chattering".into()), next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None, confidence: 0.9,
                detected_harness: None, detected_model: None, harness_session_id: None,
            };
            store.merge_peon_inference(&id, &inf, "later", None).unwrap();

            let updated = store.read_session(&id).unwrap();
            assert_eq!(
                updated.observed_status.as_deref(),
                Some(finished_status),
                "observer-only inference should not resume {finished_status} to working"
            );
            // Non-status fields still update.
            assert_eq!(updated.summary.as_deref(), Some("still chattering"));
        }
    }

    #[test]
    fn peon_inference_resumes_to_working_once_user_input_cleared_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        // Mirrors the terminal-input path: qualifying user input clears
        // observed_status to None before the next Peon inference write.
        let mut meta = test_metadata("cleared-by-user");
        meta.observed_status = None;
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: None, detected_model: None, harness_session_id: None,
        };
        store.merge_peon_inference("cleared-by-user", &inf, "later", None).unwrap();

        let updated = store.read_session("cleared-by-user").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("working"));
    }

    #[test]
    fn suggested_options_cleared_when_question_changes_without_options() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("sess-q-change"));

        let make_inf = |question: &str, options: Option<Vec<String>>| crate::peon::PeonInference {
            observed_status: Some("waiting_for_input".into()),
            detected_question: Some(question.into()),
            suggested_options: options,
            phase: None, summary: None, next_action: None, needs_user_input: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.8,
            detected_harness: None, detected_model: None, harness_session_id: None,
        };

        // Poll 1: question with options
        store.merge_peon_inference("sess-q-change", &make_inf("Proceed?", Some(vec!["yes".into(), "no".into()])), "t1", None).unwrap();
        let meta = store.read_session("sess-q-change").unwrap();
        assert_eq!(meta.suggested_options.as_deref(), Some(["yes".to_string(), "no".to_string()].as_slice()));

        // Poll 2: different question, no options — stale options must not persist
        store.merge_peon_inference("sess-q-change", &make_inf("What filename?", None), "t2", None).unwrap();
        let meta = store.read_session("sess-q-change").unwrap();
        assert_eq!(meta.detected_question.as_deref(), Some("What filename?"));
        assert!(meta.suggested_options.is_none(), "stale options must be cleared when question changes");

        // Poll 3: different question WITH new options — new options must survive
        store.merge_peon_inference("sess-q-change", &make_inf("New question?", Some(vec!["a".into(), "b".into()])), "t3", None).unwrap();
        let meta = store.read_session("sess-q-change").unwrap();
        assert_eq!(meta.detected_question.as_deref(), Some("New question?"));
        assert_eq!(meta.suggested_options.as_deref(), Some(["a".to_string(), "b".to_string()].as_slice()),
            "new options must be kept when question changes with options provided");
    }

    #[test]
    fn empty_string_question_does_not_overwrite_real_question() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("sess-q-empty"));

        let inf_real = crate::peon::PeonInference {
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            observed_status: None, phase: None, summary: None, next_action: None,
            needs_user_input: None, blocker_description: None, failed_command: None,
            failed_test: None, capacity_hints: None, confidence: 0.8,
            detected_harness: None, detected_model: None, harness_session_id: None,
        };
        let inf_empty = crate::peon::PeonInference {
            detected_question: Some("".into()),
            suggested_options: None,
            ..inf_real.clone()
        };

        store.merge_peon_inference("sess-q-empty", &inf_real, "t1", None).unwrap();
        store.merge_peon_inference("sess-q-empty", &inf_empty, "t2", None).unwrap();

        let meta = store.read_session("sess-q-empty").unwrap();
        assert_eq!(meta.detected_question.as_deref(), Some("Proceed?"),
            "empty-string question must not overwrite a real question");
        assert_eq!(meta.suggested_options.as_deref(), Some(["yes".to_string(), "no".to_string()].as_slice()),
            "options must not be cleared by an empty-string question");
    }

    #[test]
    fn suggested_options_kept_when_same_question_repeated_without_options() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("sess-q-repeat"));

        let make_inf = |question: &str, options: Option<Vec<String>>| crate::peon::PeonInference {
            observed_status: Some("waiting_for_input".into()),
            detected_question: Some(question.into()),
            suggested_options: options,
            phase: None, summary: None, next_action: None, needs_user_input: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.8,
            detected_harness: None, detected_model: None, harness_session_id: None,
        };

        store.merge_peon_inference("sess-q-repeat", &make_inf("Proceed?", Some(vec!["yes".into(), "no".into()])), "t1", None).unwrap();
        // Same question, no options re-emitted — should retain existing options
        store.merge_peon_inference("sess-q-repeat", &make_inf("Proceed?", None), "t2", None).unwrap();
        let meta = store.read_session("sess-q-repeat").unwrap();
        assert_eq!(meta.suggested_options.as_deref(), Some(["yes".to_string(), "no".to_string()].as_slice()),
            "options for the same question must be retained when re-poll omits them");
    }

    fn test_metadata(id: &str) -> SessionMetadata {
        SessionMetadata {
            id: id.into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            work_phase: "unknown".into(),
            lifecycle_phase: "active".into(),
            lifecycle: "alive".into(),
            attention: None,
            connectivity: "online".into(),
            terminal_outcome: None,
            pending_terminal_status: None,
            observed_status: None,
            ending_observed_status_snapshot: None,
            final_observed_status_snapshot: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resume_options: vec![],
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        }
    }

    #[test]
    fn write_and_read_workspace_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        store.write_workspace_memory(&WorkspaceMemory {
            last_active_session_id: Some("session-1".into()),
            last_active_at: Some("2026-06-17T12:00:00Z".into()),
            active_harness_ids: vec![],
        });

        let memory = store.read_workspace_memory().unwrap();
        assert_eq!(memory.last_active_session_id.as_deref(), Some("session-1"));
        assert_eq!(memory.last_active_at.as_deref(), Some("2026-06-17T12:00:00Z"));
    }

    #[test]
    fn read_all_sessions_includes_resume_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("remembered");
        meta.resume = Some(crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-abc".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-17T12:00:00Z".into()),
        });
        store.write_session(&meta);

        let all = store.read_all_sessions();

        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
            Some("sess-abc"),
        );
    }

    #[test]
    fn harness_session_report_writes_resume_memory_and_capture_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("capture-test"));

        let result = store.merge_harness_session_report(
            "capture-test",
            &HarnessSessionReport {
                harness_session_id: "native-123".into(),
                source: "opencode_env".into(),
                confidence: 0.98,
            },
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, HarnessSessionMergeResult::Accepted);
        let updated = store.read_session("capture-test").unwrap();
        let resume = updated.resume.unwrap();
        assert_eq!(resume.state, ResumeState::Available);
        assert_eq!(resume.preferred_strategy, ResumeStrategy::Exact);
        assert_eq!(resume.harness_session_id.as_deref(), Some("native-123"));
        assert_eq!(resume.last_seen_at.as_deref(), Some("2026-06-26T12:00:00Z"));
        assert_eq!(updated.harness_session_id_source.as_deref(), Some("opencode_env"));
        assert_eq!(updated.harness_session_id_confidence, Some(0.98));
        assert_eq!(updated.harness_session_id_captured_at.as_deref(), Some("2026-06-26T12:00:00Z"));
    }

    #[test]
    fn lower_confidence_harness_session_report_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("confidence-test");
        meta.resume = Some(ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("native-high".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-26T11:00:00Z".into()),
        });
        meta.harness_session_id_source = Some("opencode_env".into());
        meta.harness_session_id_confidence = Some(0.98);
        meta.harness_session_id_captured_at = Some("2026-06-26T11:00:00Z".into());
        store.write_session(&meta);

        let result = store.merge_harness_session_report(
            "confidence-test",
            &HarnessSessionReport {
                harness_session_id: "native-low".into(),
                source: "peon".into(),
                confidence: 0.50,
            },
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, HarnessSessionMergeResult::IgnoredLowerConfidence);
        let updated = store.read_session("confidence-test").unwrap();
        assert_eq!(
            updated.resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
            Some("native-high"),
        );
        assert_eq!(updated.harness_session_id_source.as_deref(), Some("opencode_env"));
        assert_eq!(updated.harness_session_id_confidence, Some(0.98));
    }

    #[test]
    fn equal_confidence_harness_session_report_can_refresh_same_value() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("equal-confidence-test");
        meta.resume = Some(ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("native-123".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-26T11:00:00Z".into()),
        });
        meta.harness_session_id_source = Some("opencode_env".into());
        meta.harness_session_id_confidence = Some(0.98);
        meta.harness_session_id_captured_at = Some("2026-06-26T11:00:00Z".into());
        store.write_session(&meta);

        let result = store.merge_harness_session_report(
            "equal-confidence-test",
            &HarnessSessionReport {
                harness_session_id: "native-123".into(),
                source: "claude_hook".into(),
                confidence: 0.98,
            },
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, HarnessSessionMergeResult::Accepted);
        let updated = store.read_session("equal-confidence-test").unwrap();
        assert_eq!(updated.harness_session_id_source.as_deref(), Some("claude_hook"));
        assert_eq!(updated.harness_session_id_captured_at.as_deref(), Some("2026-06-26T12:00:00Z"));
    }

    #[test]
    fn agent_attention_signal_overwrites_lower_priority_source() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("attention-accept-test");
        meta.metadata_source = "process".into();
        store.write_session(&meta);

        let result = store.merge_agent_attention_signal(
            "attention-accept-test",
            "waiting_for_input",
            None,
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, AttentionMergeResult::Accepted);
        let updated = store.read_session("attention-accept-test").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("waiting_for_input"));
        assert_eq!(updated.metadata_source, "agent");
        assert_eq!(updated.metadata_confidence, 1.0);
    }

    #[test]
    fn agent_attention_signal_sets_summary_from_message() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("attention-message-test");
        store.write_session(&meta);

        store.merge_agent_attention_signal(
            "attention-message-test",
            "waiting_for_input",
            Some("Needs approval to proceed"),
            "2026-06-26T12:00:00Z",
        );

        let updated = store.read_session("attention-message-test").unwrap();
        assert_eq!(updated.summary.as_deref(), Some("Needs approval to proceed"));
    }

    #[test]
    fn agent_attention_signal_cannot_clobber_user_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("attention-user-test");
        meta.metadata_source = "user".into();
        meta.observed_status = Some("working".into());
        store.write_session(&meta);

        let result = store.merge_agent_attention_signal(
            "attention-user-test",
            "waiting_for_input",
            None,
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, AttentionMergeResult::Ignored);
        let updated = store.read_session("attention-user-test").unwrap();
        assert_eq!(updated.observed_status.as_deref(), Some("working"));
        assert_eq!(updated.metadata_source, "user");
    }

    #[test]
    fn agent_attention_signal_returns_not_found_for_unknown_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        let result = store.merge_agent_attention_signal(
            "missing-session",
            "waiting_for_input",
            None,
            "2026-06-26T12:00:00Z",
        );

        assert_eq!(result, AttentionMergeResult::NotFound);
    }

    #[test]
    fn peon_inference_writes_harness_session_id_to_resume_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("session-id-test");
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: Some("claude-code".into()),
            detected_model: Some("claude-sonnet-4-5".into()),
            harness_session_id: Some("sess-abc123".into()),
        };
        store.merge_peon_inference("session-id-test", &inf, "2026-06-20T12:00:00Z", None).unwrap();

        let updated = store.read_session("session-id-test").unwrap();
        let resume = updated.resume.unwrap();
        assert_eq!(resume.state, ResumeState::Available);
        assert_eq!(resume.preferred_strategy, ResumeStrategy::Exact);
        assert_eq!(resume.harness_session_id.as_deref(), Some("sess-abc123"));
        assert_eq!(resume.last_seen_at.as_deref(), Some("2026-06-20T12:00:00Z"));
        assert!(resume.latest_fallback);
    }

    #[test]
    fn peon_inference_does_not_overwrite_higher_confidence_harness_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("peon-confidence-test");
        meta.resume = Some(ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("native-high".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-26T11:00:00Z".into()),
        });
        meta.harness_session_id_source = Some("opencode_env".into());
        meta.harness_session_id_confidence = Some(0.98);
        meta.harness_session_id_captured_at = Some("2026-06-26T11:00:00Z".into());
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.7,
            detected_harness: None,
            detected_model: None,
            harness_session_id: Some("native-peon".into()),
        };
        store.merge_peon_inference("peon-confidence-test", &inf, "2026-06-26T12:00:00Z", None).unwrap();

        let updated = store.read_session("peon-confidence-test").unwrap();
        assert_eq!(
            updated.resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
            Some("native-high"),
        );
        assert_eq!(updated.harness_session_id_source.as_deref(), Some("opencode_env"));
    }

    #[test]
    fn peon_inference_ignores_empty_harness_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("empty-sid-test");
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: None,
            detected_model: None,
            harness_session_id: Some("".into()),
        };
        store.merge_peon_inference("empty-sid-test", &inf, "2026-06-20T12:00:00Z", None).unwrap();

        let updated = store.read_session("empty-sid-test").unwrap();
        assert!(updated.resume.is_none());
    }

    #[test]
    fn peon_inference_rejects_invalid_harness_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        // Too short
        {
            let meta = test_metadata("short-sid");
            store.write_session(&meta);
            let inf = crate::peon::PeonInference {
                observed_status: Some("working".into()),
                phase: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None, confidence: 0.9,
                detected_harness: None,
                detected_model: None,
                harness_session_id: Some("ab".into()),
            };
            store.merge_peon_inference("short-sid", &inf, "2026-06-20T12:00:00Z", None).unwrap();
            assert!(store.read_session("short-sid").unwrap().resume.is_none());
        }

        // Contains whitespace
        {
            let meta = test_metadata("whitespace-sid");
            store.write_session(&meta);
            let inf = crate::peon::PeonInference {
                observed_status: Some("working".into()),
                phase: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None, confidence: 0.9,
                detected_harness: None,
                detected_model: None,
                harness_session_id: Some("not an id".into()),
            };
            store.merge_peon_inference("whitespace-sid", &inf, "2026-06-20T12:00:00Z", None).unwrap();
            assert!(store.read_session("whitespace-sid").unwrap().resume.is_none());
        }
    }

    #[test]
    fn terminal_output_round_trip_and_trim() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        store.append_terminal_output_lines("test-session", &lines);

        let read = store.read_terminal_output("test-session", 50);
        assert_eq!(read.len(), 50);
        assert_eq!(read[0], "line 50");
        assert_eq!(read[49], "line 99");

        // Write more lines, trigger inline trim
        let more: Vec<String> = (100..200).map(|i| format!("line {}", i)).collect();
        store.append_terminal_output_lines("test-session", &more);

        // trim to 50
        store.trim_terminal_output("test-session", 50);
        let after_trim = store.read_terminal_output("test-session", 100);
        assert_eq!(after_trim.len(), 50);
        assert_eq!(after_trim[0], "line 150");
        assert_eq!(after_trim[49], "line 199");

        // Delete and verify
        store.delete_terminal_output("test-session");
        let after_delete = store.read_terminal_output("test-session", 100);
        assert!(after_delete.is_empty());
    }

    #[test]
    fn terminal_output_empty_session_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let lines = store.read_terminal_output("nonexistent", 50);
        assert!(lines.is_empty());
    }

    #[test]
    fn delete_session_removes_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("delete-me");
        store.write_session(&meta);
        assert!(store.read_session("delete-me").is_some());

        store.delete_session("delete-me").unwrap();
        assert!(store.read_session("delete-me").is_none());
    }

    #[test]
    fn delete_session_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        // Should not error if file doesn't exist
        assert!(store.delete_session("nonexistent").is_ok());
    }

    #[test]
    fn delete_events_removes_ndjson_and_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("del-test", &Event {
            event_type: "session.created".into(),
            timestamp: "t1".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_terminal_output_lines("del-test", &["line 1".into(), "line 2".into()]);

        let ndjson_path = store.events_dir().join("del-test.ndjson");
        let terminal_path = store.events_dir().join("del-test.terminal");
        assert!(ndjson_path.exists());
        assert!(terminal_path.exists());

        store.delete_events("del-test").unwrap();

        assert!(!ndjson_path.exists());
        assert!(!terminal_path.exists());
    }

    #[test]
    fn delete_events_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        assert!(store.delete_events("nonexistent").is_ok());
    }

    #[test]
    fn merge_peon_inference_persists_provider_context() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("provider-context"));

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None,
            summary: Some("still working".into()),
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.9,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };

        let provider = crate::providers::ProviderObservation {
            provider_id: "claude-code".into(),
            provider_label: "Claude Code".into(),
            provider_model: Some("sonnet".into()),
            provider_state: "healthy".into(),
        };

        store.merge_peon_inference("provider-context", &inf, "later", Some(&provider)).unwrap();

        let meta = store.read_session("provider-context").unwrap();
        assert_eq!(meta.provider_id.as_deref(), Some("claude-code"));
        assert_eq!(meta.provider_label.as_deref(), Some("Claude Code"));
        assert_eq!(meta.provider_model.as_deref(), Some("sonnet"));
        assert_eq!(meta.provider_state.as_deref(), Some("healthy"));
    }

    #[test]
    fn read_session_accepts_canonical_terminology_fields() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        std::fs::create_dir_all(store.sessions_dir()).unwrap();

        let raw = serde_json::json!({
            "id": "canonical-fields",
            "label": "Canonical Fields",
            "workspace": "/tmp",
            "task": "",
            "harnessId": "opencode",
            "modelId": "deepseek/deepseek-reasoner",
            "cwd": "/tmp",
            "status": "running",
            "phase": "",
            "modelProviderId": "openrouter",
            "createdAt": "now",
            "lastActivity": "now",
            "metadataSource": "process",
            "metadataConfidence": 1.0
        });

        std::fs::write(
            store.sessions_dir().join("canonical-fields.json"),
            serde_json::to_string_pretty(&raw).unwrap(),
        ).unwrap();

        let meta = store.read_session("canonical-fields").unwrap();
        assert_eq!(meta.harness, "opencode");
        assert_eq!(meta.model, "deepseek/deepseek-reasoner");
        assert_eq!(meta.provider_id.as_deref(), Some("openrouter"));
    }

    #[test]
    fn read_session_normalizes_legacy_terminal_status_without_new_fields() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        std::fs::create_dir_all(store.sessions_dir()).unwrap();

        let raw = serde_json::json!({
            "id": "legacy-ended",
            "label": "Legacy Ended",
            "workspace": "/tmp",
            "task": "",
            "harness": "",
            "model": "",
            "cwd": "/tmp",
            "status": "ended",
            "phase": "",
            "createdAt": "2026-06-28T09:00:00Z",
            "lastActivity": "2026-06-28T09:05:00Z",
            "metadataSource": "process",
            "metadataConfidence": 1.0
        });

        std::fs::write(
            store.sessions_dir().join("legacy-ended.json"),
            serde_json::to_string_pretty(&raw).unwrap(),
        ).unwrap();

        let meta = store.read_session("legacy-ended").unwrap();
        assert_eq!(meta.connectivity, "offline");
        assert_eq!(meta.terminal_outcome.as_deref(), Some("ended"));
    }

    #[test]
    fn read_session_quarantines_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("corrupt-read"));

        // Simulate a mid-write kill: truncated JSON left on disk.
        let path = store.sessions_dir().join("corrupt-read.json");
        std::fs::write(&path, "{\"id\": \"corrupt-read\", \"label\": \"Tru").unwrap();

        assert!(store.read_session("corrupt-read").is_none());
        assert!(
            !path.exists(),
            "corrupt session file must be quarantined, not left in place"
        );
        assert!(
            store.sessions_dir().join("corrupt-read.json.corrupt").exists(),
            "corrupt session file must be renamed to .corrupt so the loss is observable"
        );
    }

    #[test]
    fn read_all_sessions_skips_and_quarantines_corrupt_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("healthy"));
        std::fs::write(
            store.sessions_dir().join("mangled.json"),
            "{\"id\": \"mangled\",",
        )
        .unwrap();

        let sessions = store.read_all_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "healthy");
        assert!(!store.sessions_dir().join("mangled.json").exists());
        assert!(store.sessions_dir().join("mangled.json.corrupt").exists());
    }

    #[test]
    fn write_session_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("tmp-clean"));

        let leftovers: Vec<_> = std::fs::read_dir(store.sessions_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) != Some("json"))
            .collect();
        assert!(leftovers.is_empty(), "atomic write must not leave temp files: {leftovers:?}");
        assert_eq!(store.read_session("tmp-clean").unwrap().id, "tmp-clean");
    }

    #[test]
    fn try_write_session_reports_failure_when_sessions_dir_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        // A file squatting on the sessions dir path makes create_dir_all fail.
        std::fs::write(store.sessions_dir(), "not a directory").unwrap();

        assert!(store.try_write_session(&test_metadata("doomed")).is_err());
    }

    #[test]
    fn merge_agent_attention_signal_reports_persist_failure() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("att-fail"));
        // A directory squatting on the temp path makes the atomic write fail
        // while the session itself remains readable.
        std::fs::create_dir_all(store.sessions_dir().join("att-fail.json.tmp")).unwrap();

        let result =
            store.merge_agent_attention_signal("att-fail", "waiting_for_input", None, "now");
        assert_eq!(result, AttentionMergeResult::PersistFailed);
        // The stored metadata must not claim the signal landed.
        let meta = store.read_session("att-fail").unwrap();
        assert_eq!(meta.observed_status, None);
    }

    #[test]
    fn merge_peon_inference_reports_persist_failure() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("peon-fail"));
        std::fs::create_dir_all(store.sessions_dir().join("peon-fail.json.tmp")).unwrap();

        let inf: crate::peon::PeonInference =
            serde_json::from_str(r#"{"status":"working","confidence":0.9}"#).unwrap();
        assert!(store.merge_peon_inference("peon-fail", &inf, "now", None).is_err());
    }
}
