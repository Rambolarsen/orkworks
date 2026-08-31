//! Durable, bounded workflow-observation recording.
//!
//! This module owns validation, idempotency, sequencing, bounded per-session
//! persistence, and diagnostics for `WorkflowObservation` records, as defined
//! by ADR 0042 (which supersedes ADR 0024 and ADR 0029) and
//! `docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md`.
//! It is deliberately independent of the current-summary snapshot work on
//! `SessionMetadata` (tracked separately): the two features share no state.
//!
//! The external surface is small on purpose: `open`, `record_observation`,
//! `workspace_observations`, `delete_session_observations`, and
//! `diagnostics`. Callers (the future explicit-report HTTP adapter and the
//! Peon inference adapter) never see file paths or on-disk formats; they only
//! see this contract.
//!
//! On disk, records live under the workspace metadata root:
//!
//! ```text
//! <root>/workflow-observations/<session-id>.ndjson
//! <root>/workflow-observations/sequence
//! ```
//!
//! Each `.ndjson` line is a tagged JSON object (`"record": "observation"` or
//! `"record": "tombstone"`) so raw observations and internal idempotency
//! tombstones can share one append-oriented file per session. Tombstones are
//! never returned by `workspace_observations()`.

// This module's public surface becomes live when the explicit-report HTTP
// adapter and the Peon inference adapter (later tasks) start calling it.
// Mirrors the same staged-module rationale as harness::integration.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_EVIDENCE_CHARS: usize = 2_000;
const MAX_SEGMENT_OBSERVATIONS: usize = 1_000;
const MAX_SEGMENT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_WORKSPACE_OBSERVATIONS: usize = 10_000;
const IDEMPOTENCY_WINDOW_SECS: i64 = 15 * 60;
const MAX_TOMBSTONES: usize = 1_024;
const MAX_ACCEPTED_PER_SESSION_MINUTE: usize = 60;
/// Rolling window over which `MAX_ACCEPTED_PER_SESSION_MINUTE` is enforced
/// live in `record_observation`. Distinct from `IDEMPOTENCY_WINDOW_SECS`,
/// which governs duplicate-key replay, not acceptance rate.
const RATE_LIMIT_WINDOW_SECS: i64 = 60;

/// Confidence assigned to every authenticated agent-origin report. The
/// caller cannot override this; see `ObservationSource::Agent` policy in the
/// design doc's "Observation model" section.
const AGENT_CONFIDENCE: f64 = 0.9;

/// Diagnostics keyed under this pseudo-session-id describe workspace-wide
/// (not per-session) degradation, such as a malformed sequence counter.
const WORKSPACE_DIAGNOSTIC_KEY: &str = "__workspace__";

// This constant only justifies the tombstone reservation math documented in
// the design doc ("reserving up to 1,024 tombstones ... guarantees the
// 15-minute retry window"); it is not independently enforced by this module.
const _: () = assert!(
    MAX_TOMBSTONES >= MAX_ACCEPTED_PER_SESSION_MINUTE * (IDEMPOTENCY_WINDOW_SECS as usize / 60)
);

// ---------------------------------------------------------------------------
// Public contract types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationKind {
    Repetition,
    Obstacle,
    MissingContext,
    Assumption,
    Correction,
    Workaround,
    VerificationGap,
}

impl ObservationKind {
    fn as_str(&self) -> &'static str {
        match self {
            ObservationKind::Repetition => "repetition",
            ObservationKind::Obstacle => "obstacle",
            ObservationKind::MissingContext => "missing_context",
            ObservationKind::Assumption => "assumption",
            ObservationKind::Correction => "correction",
            ObservationKind::Workaround => "workaround",
            ObservationKind::VerificationGap => "verification_gap",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Impact {
    Low,
    Medium,
    High,
}

impl Impact {
    fn as_str(&self) -> &'static str {
        match self {
            Impact::Low => "low",
            Impact::Medium => "medium",
            Impact::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservationSource {
    Agent,
    Peon,
}

/// Server-owned: selected by the adapter that calls `record_observation`,
/// never deserialized from a request. Determines the confidence policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationOrigin {
    Agent,
    Peon,
}

/// Caller-supplied input. `confidence` is only consulted for
/// `ObservationOrigin::Peon`; an `ObservationOrigin::Agent` call always gets
/// the fixed `AGENT_CONFIDENCE`, ignoring whatever is set here.
#[derive(Clone, Debug)]
pub(crate) struct ObservationCandidate {
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowObservation {
    pub id: String,
    pub sequence: u64,
    pub session_id: String,
    pub observed_at: String,
    pub kind: ObservationKind,
    pub description: String,
    pub evidence: String,
    pub reported_impact: Impact,
    pub source: ObservationSource,
    pub confidence: f64,
    pub fingerprint: String,
}

/// On-disk representation of one accepted observation: the public record
/// plus the internal idempotency fields. `idempotency_key_hash` and
/// `payload_hash` are never exposed through `workspace_observations()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredObservation {
    #[serde(flatten)]
    observation: WorkflowObservation,
    idempotency_key_hash: String,
    payload_hash: String,
}

/// Compact record kept only long enough to preserve the 15-minute retry
/// window for an observation that bounded trimming has already evicted.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Tombstone {
    idempotency_key_hash: String,
    payload_hash: String,
    observation_id: String,
    sequence: u64,
    accepted_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservationDiagnostic {
    pub code: String,
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RecordOutcome {
    Accepted(WorkflowObservation),
    Duplicate {
        observation_id: String,
        sequence: u64,
        accepted_at: String,
    },
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) enum StoreError {
    Io(std::io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "workflow observation store I/O error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RecordError {
    EmptySessionId,
    EmptyIdempotencyKey,
    EmptyDescription,
    DescriptionTooLong,
    EmptyEvidence,
    EvidenceTooLong,
    MissingConfidence,
    ConfidenceOutOfRange,
    IdempotencyConflict,
    /// The workspace sequence counter is malformed; the store refuses to
    /// guess or reuse an order value until it is repaired.
    Degraded,
    PersistFailed,
    /// The session already has `MAX_ACCEPTED_PER_SESSION_MINUTE` accepted
    /// observations within the trailing `RATE_LIMIT_WINDOW_SECS`; the
    /// tombstone reservation math depends on this cap holding live.
    RateLimited,
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            RecordError::EmptySessionId => "session id must not be empty",
            RecordError::EmptyIdempotencyKey => "idempotency key must not be empty",
            RecordError::EmptyDescription => "description must not be empty",
            RecordError::DescriptionTooLong => "description exceeds the maximum length",
            RecordError::EmptyEvidence => "evidence must not be empty",
            RecordError::EvidenceTooLong => "evidence exceeds the maximum length",
            RecordError::MissingConfidence => {
                "peon-origin observations require a candidate confidence value"
            }
            RecordError::ConfidenceOutOfRange => "confidence must be between 0.0 and 1.0",
            RecordError::IdempotencyConflict => {
                "idempotency key was reused with a different payload"
            }
            RecordError::Degraded => {
                "the workflow observation store is degraded and rejects new observations"
            }
            RecordError::PersistFailed => "failed to durably persist the workflow observation",
            RecordError::RateLimited => {
                "the session exceeded the per-session accepted-observation rate cap"
            }
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for RecordError {}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct IdempotencyEntry {
    observation_id: String,
    sequence: u64,
    accepted_at: String,
    payload_hash: String,
}

#[derive(Default, Clone)]
struct SessionCache {
    /// Ascending by `sequence`.
    observations: Vec<StoredObservation>,
    /// Ascending by `sequence`.
    tombstones: Vec<Tombstone>,
}

struct StoreInner {
    last_issued_sequence: u64,
    degraded: bool,
    idempotency: HashMap<(String, String), IdempotencyEntry>,
    diagnostics: HashMap<String, Vec<ObservationDiagnostic>>,
    session_cache: HashMap<String, SessionCache>,
    #[cfg(test)]
    clock_override: Option<DateTime<Utc>>,
}

#[cfg(test)]
fn current_time(inner: &StoreInner) -> DateTime<Utc> {
    inner.clock_override.unwrap_or_else(Utc::now)
}

#[cfg(not(test))]
fn current_time(_inner: &StoreInner) -> DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

pub(crate) struct WorkflowObservationStore {
    dir: PathBuf,
    inner: Mutex<StoreInner>,
    evaluation_generation: AtomicU64,
}

impl WorkflowObservationStore {
    /// Opens (creating if needed) the workflow-observations directory under
    /// the given workspace metadata root, rebuilding idempotency state and
    /// the sequence counter from whatever is durably retained on disk.
    pub(crate) fn open(root: PathBuf) -> Result<Self, StoreError> {
        let dir = root.join("workflow-observations");
        fs::create_dir_all(&dir)?;

        let mut session_cache: HashMap<String, SessionCache> = HashMap::new();
        let mut diagnostics: HashMap<String, Vec<ObservationDiagnostic>> = HashMap::new();
        let mut idempotency: HashMap<(String, String), IdempotencyEntry> = HashMap::new();
        let mut max_seen_sequence: u64 = 0;
        let now = Utc::now();

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ndjson") {
                    continue;
                }
                let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let session_id = session_id.to_string();

                let Ok(parsed) = tolerant_parse(&path, &session_id) else {
                    continue;
                };
                let mut observations = parsed.observations;
                let mut tombstones = parsed.tombstones;

                if let Some(valid_len) = parsed.crash_tail_valid_len {
                    // Best-effort: the in-memory view already excludes the
                    // bad tail regardless of whether the physical truncate
                    // below succeeds.
                    let _ = truncate_to(&path, valid_len);
                }
                if !parsed.diagnostics.is_empty() {
                    diagnostics.insert(session_id.clone(), parsed.diagnostics);
                }

                for stored in &observations {
                    max_seen_sequence = max_seen_sequence.max(stored.observation.sequence);
                    if within_window(now, &stored.observation.observed_at) {
                        idempotency.insert(
                            (session_id.clone(), stored.idempotency_key_hash.clone()),
                            IdempotencyEntry {
                                observation_id: stored.observation.id.clone(),
                                sequence: stored.observation.sequence,
                                accepted_at: stored.observation.observed_at.clone(),
                                payload_hash: stored.payload_hash.clone(),
                            },
                        );
                    }
                }
                for tomb in &tombstones {
                    max_seen_sequence = max_seen_sequence.max(tomb.sequence);
                    if within_window(now, &tomb.accepted_at) {
                        idempotency.insert(
                            (session_id.clone(), tomb.idempotency_key_hash.clone()),
                            IdempotencyEntry {
                                observation_id: tomb.observation_id.clone(),
                                sequence: tomb.sequence,
                                accepted_at: tomb.accepted_at.clone(),
                                payload_hash: tomb.payload_hash.clone(),
                            },
                        );
                    }
                }

                observations.sort_by_key(|o| o.observation.sequence);
                tombstones.sort_by_key(|t| t.sequence);
                session_cache.insert(
                    session_id,
                    SessionCache {
                        observations,
                        tombstones,
                    },
                );
            }
        }

        let counter_path = dir.join("sequence");
        let mut degraded = false;
        let last_issued_sequence = match fs::read_to_string(&counter_path) {
            Ok(text) => match text.trim().parse::<u64>() {
                Ok(value) => value.max(max_seen_sequence),
                Err(_) => {
                    degraded = true;
                    diagnostics.insert(
                        WORKSPACE_DIAGNOSTIC_KEY.to_string(),
                        vec![ObservationDiagnostic {
                            code: "sequence_counter_corrupt".to_string(),
                            message: "The workflow-observation sequence counter is malformed; \
                                      new observations are rejected until it is repaired."
                                .to_string(),
                            session_id: None,
                        }],
                    );
                    0
                }
            },
            Err(_) => max_seen_sequence,
        };

        Ok(Self {
            dir,
            inner: Mutex::new(StoreInner {
                last_issued_sequence,
                degraded,
                idempotency,
                diagnostics,
                session_cache,
                #[cfg(test)]
                clock_override: None,
            }),
            evaluation_generation: AtomicU64::new(0),
        })
    }

    pub(crate) fn record_observation(
        &self,
        session_id: &str,
        origin: ObservationOrigin,
        idempotency_key: &str,
        candidate: ObservationCandidate,
    ) -> Result<RecordOutcome, RecordError> {
        if session_id.trim().is_empty() {
            return Err(RecordError::EmptySessionId);
        }
        if idempotency_key.is_empty() {
            return Err(RecordError::EmptyIdempotencyKey);
        }
        if candidate.description.trim().is_empty() {
            return Err(RecordError::EmptyDescription);
        }
        if candidate.description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(RecordError::DescriptionTooLong);
        }
        if candidate.evidence.trim().is_empty() {
            return Err(RecordError::EmptyEvidence);
        }
        if candidate.evidence.chars().count() > MAX_EVIDENCE_CHARS {
            return Err(RecordError::EvidenceTooLong);
        }

        let (source, confidence) = match origin {
            ObservationOrigin::Agent => (ObservationSource::Agent, AGENT_CONFIDENCE),
            ObservationOrigin::Peon => {
                let c = candidate.confidence.ok_or(RecordError::MissingConfidence)?;
                if !(0.0..=1.0).contains(&c) {
                    return Err(RecordError::ConfidenceOutOfRange);
                }
                (ObservationSource::Peon, c)
            }
        };

        let fingerprint = format!(
            "v1:{}:{}",
            candidate.kind.as_str(),
            normalize_description(&candidate.description)
        );
        let key_hash = hash_key(session_id, idempotency_key);
        let payload_hash = hash_payload(
            candidate.kind,
            &candidate.description,
            &candidate.evidence,
            candidate.reported_impact,
        );

        let mut inner = self.inner.lock().unwrap();
        if inner.degraded {
            return Err(RecordError::Degraded);
        }
        let now = current_time(&inner);

        let cache_key = (session_id.to_string(), key_hash.clone());
        if let Some(entry) = inner.idempotency.get(&cache_key).cloned() {
            if within_window(now, &entry.accepted_at) {
                if entry.payload_hash == payload_hash {
                    return Ok(RecordOutcome::Duplicate {
                        observation_id: entry.observation_id,
                        sequence: entry.sequence,
                        accepted_at: entry.accepted_at,
                    });
                }
                return Err(RecordError::IdempotencyConflict);
            }
            inner.idempotency.remove(&cache_key);
        }

        if let Some(cache) = inner.session_cache.get(session_id) {
            let accepted_in_window = cache
                .observations
                .iter()
                .filter(|stored| within_rate_window(now, &stored.observation.observed_at))
                .count();
            if accepted_in_window >= MAX_ACCEPTED_PER_SESSION_MINUTE {
                return Err(RecordError::RateLimited);
            }
        }

        let new_sequence = inner.last_issued_sequence + 1;
        self.write_counter(new_sequence)
            .map_err(|_| RecordError::PersistFailed)?;
        inner.last_issued_sequence = new_sequence;

        let observation = WorkflowObservation {
            id: uuid::Uuid::new_v4().to_string(),
            sequence: new_sequence,
            session_id: session_id.to_string(),
            observed_at: now.to_rfc3339(),
            kind: candidate.kind,
            description: candidate.description.clone(),
            evidence: candidate.evidence.clone(),
            reported_impact: candidate.reported_impact,
            source,
            confidence,
            fingerprint,
        };
        let stored = StoredObservation {
            observation: observation.clone(),
            idempotency_key_hash: key_hash.clone(),
            payload_hash: payload_hash.clone(),
        };

        self.append_or_rewrite(&mut inner, session_id, stored, now)
            .map_err(|_| RecordError::PersistFailed)?;

        inner.idempotency.insert(
            cache_key,
            IdempotencyEntry {
                observation_id: observation.id.clone(),
                sequence: observation.sequence,
                accepted_at: observation.observed_at.clone(),
                payload_hash,
            },
        );

        Ok(RecordOutcome::Accepted(observation))
    }

    /// Aggregates retained observations across every session segment, newest
    /// `MAX_WORKSPACE_OBSERVATIONS` only, ordered by `sequence`. Never
    /// returns internal tombstones.
    pub(crate) fn workspace_observations(&self) -> Result<Vec<WorkflowObservation>, StoreError> {
        let inner = self.inner.lock().unwrap();
        let mut all: Vec<WorkflowObservation> = inner
            .session_cache
            .values()
            .flat_map(|cache| cache.observations.iter().map(|s| s.observation.clone()))
            .collect();
        all.sort_by_key(|o| o.sequence);
        if all.len() > MAX_WORKSPACE_OBSERVATIONS {
            let drop_count = all.len() - MAX_WORKSPACE_OBSERVATIONS;
            all.drain(0..drop_count);
        }
        Ok(all)
    }

    /// Returns the number of retained accepted observations for one session.
    /// Tombstones and duplicate reports are intentionally excluded.
    pub(crate) fn session_observation_count(&self, session_id: &str) -> Result<usize, StoreError> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .session_cache
            .get(session_id)
            .map_or(0, |cache| cache.observations.len()))
    }

    pub(crate) fn delete_session_observations(&self, session_id: &str) -> Result<(), StoreError> {
        let mut inner = self.inner.lock().unwrap();
        let path = self.segment_path(session_id);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(StoreError::Io(e)),
        }
        inner.session_cache.remove(session_id);
        inner.idempotency.retain(|(sid, _), _| sid != session_id);
        inner.diagnostics.remove(session_id);
        Ok(())
    }

    pub(crate) fn next_evaluation_generation(&self) -> u64 {
        self.evaluation_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub(crate) fn evaluation_generation(&self) -> u64 {
        self.evaluation_generation.load(Ordering::SeqCst)
    }

    pub(crate) fn diagnostics(&self) -> Vec<ObservationDiagnostic> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<ObservationDiagnostic> =
            inner.diagnostics.values().flatten().cloned().collect();
        out.sort_by(|a, b| a.code.cmp(&b.code).then(a.session_id.cmp(&b.session_id)));
        out
    }

    #[cfg(test)]
    pub(crate) fn test_set_clock(&self, at: DateTime<Utc>) {
        self.inner.lock().unwrap().clock_override = Some(at);
    }

    fn segment_path(&self, session_id: &str) -> PathBuf {
        self.dir.join(format!("{session_id}.ndjson"))
    }

    fn write_counter(&self, value: u64) -> std::io::Result<()> {
        durable_write(&self.dir.join("sequence"), value.to_string().as_bytes())
    }

    /// Appends the new record via the cheap single-line path when the
    /// session segment would stay under both bounds and has nothing to
    /// reclaim; otherwise performs a full bounded, atomic rewrite. Always
    /// updates the in-memory session cache to match what was durably
    /// written.
    fn append_or_rewrite(
        &self,
        inner: &mut StoreInner,
        session_id: &str,
        stored: StoredObservation,
        now: DateTime<Utc>,
    ) -> std::io::Result<()> {
        let path = self.segment_path(session_id);
        let cache = inner
            .session_cache
            .entry(session_id.to_string())
            .or_default();

        let unexpired_tombs: Vec<Tombstone> = cache
            .tombstones
            .iter()
            .cloned()
            .filter(|t| within_window(now, &t.accepted_at))
            .collect();
        let tombs_changed = unexpired_tombs.len() != cache.tombstones.len();

        let line = serialize_observation_line(&stored).map_err(std::io::Error::other)?;
        let line_len = line.len() as u64 + 1;
        let current_len = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let projected_count = cache.observations.len() + 1;
        let projected_len = current_len + line_len;

        if !tombs_changed
            && projected_count <= MAX_SEGMENT_OBSERVATIONS
            && projected_len <= MAX_SEGMENT_BYTES
        {
            append_line(&path, &line)?;
            cache.observations.push(stored);
            return Ok(());
        }

        let mut obs = cache.observations.clone();
        obs.push(stored);
        let mut tombs = unexpired_tombs;

        while obs.len() > MAX_SEGMENT_OBSERVATIONS {
            let evicted = obs.remove(0);
            maybe_tombstone(&mut tombs, evicted, now);
        }
        while !obs.is_empty() && segment_size(&tombs, &obs) > MAX_SEGMENT_BYTES {
            let evicted = obs.remove(0);
            maybe_tombstone(&mut tombs, evicted, now);
        }
        if tombs.len() > MAX_TOMBSTONES {
            tombs.sort_by_key(|t| t.sequence);
            let excess = tombs.len() - MAX_TOMBSTONES;
            tombs.drain(0..excess);
        }
        tombs.sort_by_key(|t| t.sequence);
        obs.sort_by_key(|o| o.observation.sequence);

        let mut buf: Vec<u8> = Vec::new();
        for t in &tombs {
            let json = serialize_tombstone_line(t).map_err(std::io::Error::other)?;
            buf.extend_from_slice(json.as_bytes());
            buf.push(b'\n');
        }
        for o in &obs {
            let json = serialize_observation_line(o).map_err(std::io::Error::other)?;
            buf.extend_from_slice(json.as_bytes());
            buf.push(b'\n');
        }
        durable_write(&path, &buf)?;

        cache.observations = obs;
        cache.tombstones = tombs;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

fn within_window(now: DateTime<Utc>, iso: &str) -> bool {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => {
            now.signed_duration_since(dt.with_timezone(&Utc))
                <= chrono::Duration::seconds(IDEMPOTENCY_WINDOW_SECS)
        }
        Err(_) => false,
    }
}

/// Like `within_window` but over the shorter `RATE_LIMIT_WINDOW_SECS` used to
/// enforce `MAX_ACCEPTED_PER_SESSION_MINUTE` live in `record_observation`.
fn within_rate_window(now: DateTime<Utc>, iso: &str) -> bool {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => {
            now.signed_duration_since(dt.with_timezone(&Utc))
                <= chrono::Duration::seconds(RATE_LIMIT_WINDOW_SECS)
        }
        Err(_) => false,
    }
}

/// Trims, lowercases, and collapses every run of Unicode whitespace to one
/// ASCII space, per fingerprint version 1's normalization rule.
fn normalize_description(description: &str) -> String {
    let trimmed = description.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut last_was_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_was_space = false;
        }
    }
    out
}

fn hash_key(session_id: &str, idempotency_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(idempotency_key.as_bytes());
    hex::encode(hasher.finalize())
}

fn hash_payload(
    kind: ObservationKind,
    description: &str,
    evidence: &str,
    impact: Impact,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_str().as_bytes());
    hasher.update([0u8]);
    hasher.update(description.as_bytes());
    hasher.update([0u8]);
    hasher.update(evidence.as_bytes());
    hasher.update([0u8]);
    hasher.update(impact.as_str().as_bytes());
    hex::encode(hasher.finalize())
}

fn maybe_tombstone(tombs: &mut Vec<Tombstone>, evicted: StoredObservation, now: DateTime<Utc>) {
    if within_window(now, &evicted.observation.observed_at) {
        tombs.push(Tombstone {
            idempotency_key_hash: evicted.idempotency_key_hash,
            payload_hash: evicted.payload_hash,
            observation_id: evicted.observation.id,
            sequence: evicted.observation.sequence,
            accepted_at: evicted.observation.observed_at,
        });
    }
}

fn segment_size(tombs: &[Tombstone], obs: &[StoredObservation]) -> u64 {
    let mut total = 0u64;
    for t in tombs {
        if let Ok(json) = serialize_tombstone_line(t) {
            total += json.len() as u64 + 1;
        }
    }
    for o in obs {
        if let Ok(json) = serialize_observation_line(o) {
            total += json.len() as u64 + 1;
        }
    }
    total
}

fn serialize_observation_line(stored: &StoredObservation) -> serde_json::Result<String> {
    let mut value = serde_json::to_value(stored)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "record".to_string(),
            serde_json::Value::String("observation".to_string()),
        );
    }
    serde_json::to_string(&value)
}

fn serialize_tombstone_line(tombstone: &Tombstone) -> serde_json::Result<String> {
    let mut value = serde_json::to_value(tombstone)?;
    if let serde_json::Value::Object(map) = &mut value {
        map.insert(
            "record".to_string(),
            serde_json::Value::String("tombstone".to_string()),
        );
    }
    serde_json::to_string(&value)
}

enum ParsedLine {
    Observation(StoredObservation),
    Tombstone(Tombstone),
}

fn parse_line(bytes: &[u8]) -> Result<ParsedLine, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let record_type = value
        .get("record")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'record' field".to_string())?;
    match record_type {
        "observation" => serde_json::from_value::<StoredObservation>(value)
            .map(ParsedLine::Observation)
            .map_err(|e| e.to_string()),
        "tombstone" => serde_json::from_value::<Tombstone>(value)
            .map(ParsedLine::Tombstone)
            .map_err(|e| e.to_string()),
        other => Err(format!("unknown record type '{other}'")),
    }
}

struct ParsedSegment {
    observations: Vec<StoredObservation>,
    tombstones: Vec<Tombstone>,
    diagnostics: Vec<ObservationDiagnostic>,
    /// `Some(valid_len)` when the final line in the file is an unreadable
    /// crash tail; the file should be truncated to `valid_len` bytes.
    crash_tail_valid_len: Option<u64>,
}

/// Reads a per-session segment file, tolerating corruption: an unreadable
/// final line is reported as a crash tail (with the byte length to truncate
/// to); an unreadable interior line is skipped and reported as interior
/// corruption, without disturbing later valid records.
fn tolerant_parse(path: &Path, session_id: &str) -> std::io::Result<ParsedSegment> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ParsedSegment {
                observations: vec![],
                tombstones: vec![],
                diagnostics: vec![],
                crash_tail_valid_len: None,
            });
        }
        Err(e) => return Err(e),
    };

    let mut observations = Vec::new();
    let mut tombstones = Vec::new();
    let mut diagnostics = Vec::new();
    let mut crash_tail_valid_len = None;

    let len = bytes.len();
    let mut offset = 0usize;
    let mut consumed: u64 = 0;
    while offset < len {
        let newline_rel = bytes[offset..].iter().position(|&b| b == b'\n');
        let (line_bytes, chunk_len) = match newline_rel {
            Some(rel) => (&bytes[offset..offset + rel], rel + 1),
            None => (&bytes[offset..], len - offset),
        };
        let is_last_chunk = offset + chunk_len >= len;

        if line_bytes.iter().all(|b| b.is_ascii_whitespace()) {
            consumed += chunk_len as u64;
            offset += chunk_len;
            continue;
        }

        match parse_line(line_bytes) {
            Ok(ParsedLine::Observation(stored)) => {
                observations.push(stored);
                consumed += chunk_len as u64;
            }
            Ok(ParsedLine::Tombstone(tombstone)) => {
                tombstones.push(tombstone);
                consumed += chunk_len as u64;
            }
            Err(reason) => {
                if is_last_chunk {
                    crash_tail_valid_len = Some(consumed);
                    diagnostics.push(ObservationDiagnostic {
                        code: "crash_tail_truncated".to_string(),
                        message: format!(
                            "Recovered a partial trailing workflow-observation record \
                             ({reason}); truncated to the last complete entry."
                        ),
                        session_id: Some(session_id.to_string()),
                    });
                } else {
                    diagnostics.push(ObservationDiagnostic {
                        code: "interior_corruption".to_string(),
                        message: format!(
                            "Skipped an unreadable workflow-observation record ({reason}); \
                             analysis for this session is degraded."
                        ),
                        session_id: Some(session_id.to_string()),
                    });
                    consumed += chunk_len as u64;
                }
            }
        }
        offset += chunk_len;
    }

    Ok(ParsedSegment {
        observations,
        tombstones,
        diagnostics,
        crash_tail_valid_len,
    })
}

fn tmp_sibling(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Writes `contents` to `target` via a temp file in the same directory,
/// `sync_data`, an atomic replace, and a best-effort parent-directory sync —
/// so a crash never publishes a partially written segment or counter file.
fn durable_write(target: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_sibling(target);
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_data()?;
    }
    let target_existed = target.exists();
    let result = crate::harness::integration::atomic_replace(&tmp, target, target_existed);
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Appends one complete line to `path`, flushing and `sync_data`-ing before
/// returning — the fast path used when a write stays under both segment
/// bounds and needs no tombstone cleanup.
fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    file.sync_data()?;
    Ok(())
}

/// Shrinks `path` to `valid_len` bytes and syncs — used to drop a crash-tail
/// fragment without touching any byte that was already valid.
fn truncate_to(path: &Path, valid_len: u64) -> std::io::Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_len(valid_len)?;
    file.sync_data()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn candidate(kind: ObservationKind, description: &str, evidence: &str) -> ObservationCandidate {
        ObservationCandidate {
            kind,
            description: description.to_string(),
            evidence: evidence.to_string(),
            reported_impact: Impact::Medium,
            confidence: None,
        }
    }

    fn open_store(dir: &Path) -> WorkflowObservationStore {
        WorkflowObservationStore::open(dir.to_path_buf()).expect("open store")
    }

    // -- Domain / validation -------------------------------------------------

    #[test]
    fn accepts_all_seven_observation_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let kinds = [
            ObservationKind::Repetition,
            ObservationKind::Obstacle,
            ObservationKind::MissingContext,
            ObservationKind::Assumption,
            ObservationKind::Correction,
            ObservationKind::Workaround,
            ObservationKind::VerificationGap,
        ];
        for (i, kind) in kinds.iter().enumerate() {
            let outcome = store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(
                        *kind,
                        "Something made this harder than necessary",
                        "concrete evidence",
                    ),
                )
                .unwrap();
            match outcome {
                RecordOutcome::Accepted(obs) => {
                    assert_eq!(obs.kind, *kind);
                    assert!(obs
                        .fingerprint
                        .starts_with(&format!("v1:{}:", kind.as_str())));
                }
                other => panic!("expected Accepted, got {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_empty_description() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "   ", "evidence"),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::EmptyDescription);
    }

    #[test]
    fn rejects_empty_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", "  "),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::EmptyEvidence);
    }

    #[test]
    fn rejects_oversized_description() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let too_long = "a".repeat(MAX_DESCRIPTION_CHARS + 1);
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, &too_long, "evidence"),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::DescriptionTooLong);
    }

    #[test]
    fn rejects_oversized_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let too_long = "a".repeat(MAX_EVIDENCE_CHARS + 1);
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", &too_long),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::EvidenceTooLong);
    }

    #[test]
    fn agent_origin_ignores_caller_confidence_and_uses_fixed_value() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let mut c = candidate(ObservationKind::Obstacle, "description", "evidence");
        c.confidence = Some(0.1);
        let outcome = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c)
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => {
                assert_eq!(obs.confidence, AGENT_CONFIDENCE);
                assert_eq!(obs.source, ObservationSource::Agent);
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn peon_origin_requires_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let c = candidate(ObservationKind::Obstacle, "description", "evidence");
        let err = store
            .record_observation("session-1", ObservationOrigin::Peon, "key-1", c)
            .unwrap_err();
        assert_eq!(err, RecordError::MissingConfidence);
    }

    #[test]
    fn peon_origin_rejects_out_of_range_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let mut c = candidate(ObservationKind::Obstacle, "description", "evidence");
        c.confidence = Some(1.5);
        let err = store
            .record_observation("session-1", ObservationOrigin::Peon, "key-1", c)
            .unwrap_err();
        assert_eq!(err, RecordError::ConfidenceOutOfRange);
    }

    #[test]
    fn peon_origin_preserves_caller_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let mut c = candidate(ObservationKind::Obstacle, "description", "evidence");
        c.confidence = Some(0.73);
        let outcome = store
            .record_observation("session-1", ObservationOrigin::Peon, "key-1", c)
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => {
                assert_eq!(obs.confidence, 0.73);
                assert_eq!(obs.source, ObservationSource::Peon);
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn fingerprint_normalizes_case_and_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let a = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-a",
                candidate(ObservationKind::Obstacle, "  Fix   the Bug  ", "evidence a"),
            )
            .unwrap();
        let b = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-b",
                candidate(ObservationKind::Obstacle, "fix the bug", "evidence b"),
            )
            .unwrap();
        let fp = |o: RecordOutcome| match o {
            RecordOutcome::Accepted(obs) => obs.fingerprint,
            other => panic!("expected Accepted, got {other:?}"),
        };
        assert_eq!(fp(a), fp(b));
    }

    #[test]
    fn fingerprint_includes_kind() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let a = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-a",
                candidate(ObservationKind::Obstacle, "same text", "evidence a"),
            )
            .unwrap();
        let b = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-b",
                candidate(ObservationKind::Workaround, "same text", "evidence b"),
            )
            .unwrap();
        let fp = |o: RecordOutcome| match o {
            RecordOutcome::Accepted(obs) => obs.fingerprint,
            other => panic!("expected Accepted, got {other:?}"),
        };
        assert_ne!(fp(a), fp(b));
    }

    // -- Idempotency -----------------------------------------------------

    #[test]
    fn duplicate_key_same_payload_returns_same_identity() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let c = candidate(ObservationKind::Obstacle, "description", "evidence");
        let first = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c.clone())
            .unwrap();
        let (first_id, first_seq) = match first {
            RecordOutcome::Accepted(obs) => (obs.id, obs.sequence),
            other => panic!("expected Accepted, got {other:?}"),
        };

        let second = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c)
            .unwrap();
        match second {
            RecordOutcome::Duplicate {
                observation_id,
                sequence,
                ..
            } => {
                assert_eq!(observation_id, first_id);
                assert_eq!(sequence, first_seq);
            }
            other => panic!("expected Duplicate, got {other:?}"),
        }
    }

    #[test]
    fn session_observation_count_does_not_count_duplicate_replays() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let c = candidate(ObservationKind::Obstacle, "description", "evidence");

        let first = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c.clone())
            .unwrap();
        assert!(matches!(first, RecordOutcome::Accepted(_)));

        let duplicate = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c)
            .unwrap();
        assert!(matches!(duplicate, RecordOutcome::Duplicate { .. }));

        assert_eq!(store.session_observation_count("session-1").unwrap(), 1);
        assert_eq!(store.session_observation_count("session-2").unwrap(), 0);
    }

    #[test]
    fn session_observation_count_ignores_unrelated_workspace_retention_bound() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        let mut sequence = 1u64;
        let target = WorkflowObservation {
            id: uuid::Uuid::new_v4().to_string(),
            sequence,
            session_id: "target-session".to_string(),
            observed_at: Utc::now().to_rfc3339(),
            kind: ObservationKind::Obstacle,
            description: "target description".to_string(),
            evidence: "target evidence".to_string(),
            reported_impact: Impact::Medium,
            source: ObservationSource::Agent,
            confidence: AGENT_CONFIDENCE,
            fingerprint: "v1:obstacle:target description".to_string(),
        };
        let target_stored = StoredObservation {
            observation: target,
            idempotency_key_hash: "target-key-hash".to_string(),
            payload_hash: "target-payload-hash".to_string(),
        };
        fs::write(
            ndjson_dir.join("target-session.ndjson"),
            format!("{}\n", serialize_observation_line(&target_stored).unwrap()),
        )
        .unwrap();

        for session_index in 0..MAX_WORKSPACE_OBSERVATIONS {
            sequence += 1;
            let session_id = format!("unrelated-session-{session_index}");
            let observation = WorkflowObservation {
                id: uuid::Uuid::new_v4().to_string(),
                sequence,
                session_id: session_id.clone(),
                observed_at: Utc::now().to_rfc3339(),
                kind: ObservationKind::Obstacle,
                description: "unrelated description".to_string(),
                evidence: "unrelated evidence".to_string(),
                reported_impact: Impact::Medium,
                source: ObservationSource::Agent,
                confidence: AGENT_CONFIDENCE,
                fingerprint: "v1:obstacle:unrelated description".to_string(),
            };
            let stored = StoredObservation {
                observation,
                idempotency_key_hash: format!("key-hash-{sequence}"),
                payload_hash: format!("payload-hash-{sequence}"),
            };
            fs::write(
                ndjson_dir.join(format!("{session_id}.ndjson")),
                format!("{}\n", serialize_observation_line(&stored).unwrap()),
            )
            .unwrap();
        }
        fs::write(ndjson_dir.join("sequence"), sequence.to_string()).unwrap();

        let store = open_store(&root);

        assert_eq!(
            store.workspace_observations().unwrap().len(),
            MAX_WORKSPACE_OBSERVATIONS
        );
        assert_eq!(
            store.session_observation_count("target-session").unwrap(),
            1
        );
    }

    #[test]
    fn duplicate_key_different_payload_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description one", "evidence"),
            )
            .unwrap();

        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description two", "evidence"),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::IdempotencyConflict);
    }

    #[test]
    fn concurrent_same_key_calls_serialize_to_one_accept() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(open_store(dir.path()));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    store
                        .record_observation(
                            "session-1",
                            ObservationOrigin::Agent,
                            "shared-key",
                            candidate(ObservationKind::Obstacle, "description", "evidence"),
                        )
                        .unwrap()
                })
            })
            .collect();

        let results: Vec<RecordOutcome> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let accepted: Vec<_> = results
            .iter()
            .filter(|r| matches!(r, RecordOutcome::Accepted(_)))
            .collect();
        assert_eq!(
            accepted.len(),
            1,
            "expected exactly one Accepted, got {results:?}"
        );

        let (accepted_id, accepted_seq) = match accepted[0] {
            RecordOutcome::Accepted(obs) => (obs.id.clone(), obs.sequence),
            _ => unreachable!(),
        };
        for result in &results {
            if let RecordOutcome::Duplicate {
                observation_id,
                sequence,
                ..
            } = result
            {
                assert_eq!(observation_id, &accepted_id);
                assert_eq!(*sequence, accepted_seq);
            }
        }
    }

    #[test]
    fn idempotency_key_expires_after_fifteen_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let t0 = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.test_set_clock(t0);

        let c = candidate(ObservationKind::Obstacle, "description", "evidence");
        let first = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c.clone())
            .unwrap();
        let first_id = match first {
            RecordOutcome::Accepted(obs) => obs.id,
            other => panic!("expected Accepted, got {other:?}"),
        };

        store.test_set_clock(t0 + chrono::Duration::minutes(16));
        let second = store
            .record_observation("session-1", ObservationOrigin::Agent, "key-1", c)
            .unwrap();
        match second {
            RecordOutcome::Accepted(obs) => {
                assert_ne!(obs.id, first_id, "expired key must create a new occurrence");
            }
            other => panic!("expected a new Accepted occurrence, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_retry_within_window_survives_trim_via_tombstone_after_restart() {
        // Reaching >MAX_SEGMENT_OBSERVATIONS (1,000) accepted observations
        // for one session is now impossible through record_observation alone
        // within the 15-minute idempotency window, since the live rate cap
        // limits acceptance to MAX_ACCEPTED_PER_SESSION_MINUTE (60) per
        // rolling minute. So this test primes the original observation plus
        // enough filler directly to disk -- exactly mirroring what a real
        // restart rebuilds from already-durably-persisted history -- and
        // only exercises the SUT's live trim/tombstone path for the single
        // record that actually pushes the segment over the bound.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        // Older than the rate-limit window (60s) but well inside the
        // 15-minute idempotency/tombstone window, so the primed history
        // doesn't itself trip the live per-session rate cap on the one SUT
        // call below.
        let primed_at = Utc::now() - chrono::Duration::seconds(90);

        let original_id = "obs-original".to_string();
        let original_seq: u64 = 1;
        let original_stored = StoredObservation {
            observation: WorkflowObservation {
                id: original_id.clone(),
                sequence: original_seq,
                session_id: "session-1".to_string(),
                observed_at: primed_at.to_rfc3339(),
                kind: ObservationKind::Obstacle,
                description: "retried description".to_string(),
                evidence: "evidence".to_string(),
                reported_impact: Impact::Medium,
                source: ObservationSource::Agent,
                confidence: AGENT_CONFIDENCE,
                fingerprint: "v1:obstacle:retried description".to_string(),
            },
            idempotency_key_hash: hash_key("session-1", "retry-key"),
            payload_hash: hash_payload(
                ObservationKind::Obstacle,
                "retried description",
                "evidence",
                Impact::Medium,
            ),
        };

        let mut buf = String::new();
        buf.push_str(&serialize_observation_line(&original_stored).unwrap());
        buf.push('\n');

        let mut sequence = original_seq;
        for _ in 0..(MAX_SEGMENT_OBSERVATIONS - 1) {
            sequence += 1;
            let stored = StoredObservation {
                observation: WorkflowObservation {
                    id: format!("obs-filler-{sequence}"),
                    sequence,
                    session_id: "session-1".to_string(),
                    observed_at: primed_at.to_rfc3339(),
                    kind: ObservationKind::Obstacle,
                    description: "filler description".to_string(),
                    evidence: "evidence".to_string(),
                    reported_impact: Impact::Medium,
                    source: ObservationSource::Agent,
                    confidence: AGENT_CONFIDENCE,
                    fingerprint: "v1:obstacle:filler description".to_string(),
                },
                idempotency_key_hash: format!("hash-{sequence}"),
                payload_hash: format!("payload-{sequence}"),
            };
            buf.push_str(&serialize_observation_line(&stored).unwrap());
            buf.push('\n');
        }
        fs::write(ndjson_dir.join("session-1.ndjson"), &buf).unwrap();
        fs::write(ndjson_dir.join("sequence"), sequence.to_string()).unwrap();

        {
            let store = open_store(&root);
            // The segment is primed at exactly MAX_SEGMENT_OBSERVATIONS; one
            // more accepted call pushes it over the bound, forcing the SUT
            // to evict the oldest (the original observation) via the real
            // trim/rewrite path. That eviction happens well within the
            // 15-minute idempotency window, so it must retain a tombstone
            // rather than dropping the key outright.
            store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    "final-key",
                    candidate(ObservationKind::Obstacle, "final description", "evidence"),
                )
                .unwrap();
        }

        // Fresh store: idempotency state must be rebuilt from the retained
        // tombstone, not from the (now-trimmed) original observation line.
        let store = open_store(&root);
        let retry = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "retry-key",
                candidate(ObservationKind::Obstacle, "retried description", "evidence"),
            )
            .unwrap();
        match retry {
            RecordOutcome::Duplicate {
                observation_id,
                sequence,
                ..
            } => {
                assert_eq!(observation_id, original_id);
                assert_eq!(sequence, original_seq);
            }
            other => panic!("expected Duplicate sourced from tombstone, got {other:?}"),
        }
    }

    // -- Persistence / trim -------------------------------------------------

    #[test]
    fn segment_trims_to_newest_one_thousand_observations() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let base = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        // This test only cares about count/sequence trimming, not real
        // elapsed time. Advance the fake clock well clear of the live
        // per-session rate cap (60/rolling-minute) between every call so it
        // never trips while pushing far past MAX_SEGMENT_OBSERVATIONS.
        for i in 0..(MAX_SEGMENT_OBSERVATIONS + 5) {
            store.test_set_clock(base + chrono::Duration::seconds(61 * i as i64));
            store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(ObservationKind::Obstacle, "description", "evidence"),
                )
                .unwrap();
        }
        let observations = store.workspace_observations().unwrap();
        assert_eq!(observations.len(), MAX_SEGMENT_OBSERVATIONS);
        // The five oldest (sequence 1..=5) must be gone; the newest must remain.
        assert!(observations.iter().all(|o| o.sequence > 5));
    }

    #[test]
    fn segment_trims_to_two_mebibyte_bound() {
        // Each near-max-size record is a few KB, so hundreds of them would
        // cross MAX_SEGMENT_BYTES well before MAX_SEGMENT_OBSERVATIONS. To
        // exercise the real trim/rewrite code path in record_observation
        // without paying for hundreds of real fsyncs in this test, prime the
        // segment close to the byte bound via a direct fixture write (a
        // single unsynced write, not exercising the SUT), then make the SUT
        // itself durably append enough further records to push the segment
        // over the bound and observe it trim back down.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        let description = "d".repeat(MAX_DESCRIPTION_CHARS);
        let evidence = "e".repeat(MAX_EVIDENCE_CHARS);
        let mut buf = String::new();
        let priming_count: u64 = 700;
        for seq in 1..=priming_count {
            let observation = WorkflowObservation {
                id: uuid::Uuid::new_v4().to_string(),
                sequence: seq,
                session_id: "session-1".to_string(),
                observed_at: Utc::now().to_rfc3339(),
                kind: ObservationKind::Obstacle,
                description: description.clone(),
                evidence: evidence.clone(),
                reported_impact: Impact::Medium,
                source: ObservationSource::Agent,
                confidence: AGENT_CONFIDENCE,
                fingerprint: "v1:obstacle:primed".to_string(),
            };
            let stored = StoredObservation {
                observation,
                idempotency_key_hash: format!("hash-{seq}"),
                payload_hash: format!("payload-{seq}"),
            };
            buf.push_str(&serialize_observation_line(&stored).unwrap());
            buf.push('\n');
        }
        fs::write(ndjson_dir.join("session-1.ndjson"), &buf).unwrap();
        fs::write(ndjson_dir.join("sequence"), priming_count.to_string()).unwrap();

        let store = open_store(&root);
        // The 700 primed observations above share ~real "now" as their
        // observed_at. Move the fake clock well clear of the live
        // per-session rate window (60/rolling-minute) so that already-primed
        // history doesn't itself trip the cap on the SUT calls below (50
        // calls is comfortably under the cap on its own).
        store.test_set_clock(Utc::now() + chrono::Duration::seconds(120));
        // Each further record is a few KB; a handful of real, durable
        // appends is enough to cross MAX_SEGMENT_BYTES and trigger a real
        // trim/rewrite through the SUT.
        for i in 0..50 {
            store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(ObservationKind::Obstacle, &description, &evidence),
                )
                .unwrap();
        }
        let path = ndjson_dir.join("session-1.ndjson");
        let size = fs::metadata(&path).unwrap().len();
        assert!(size <= MAX_SEGMENT_BYTES, "segment grew to {size} bytes");

        let observations = store.workspace_observations().unwrap();
        assert!(
            observations.len() < (priming_count as usize + 50),
            "expected byte-bound trimming to have occurred"
        );
    }

    #[test]
    fn workspace_observations_bounds_to_ten_thousand_newest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        // Write 11 sessions x 1000 observations directly to disk (bypassing
        // record_observation's per-call fsync) with globally increasing
        // sequence numbers, then verify the aggregation bound via a fresh
        // open() + workspace_observations() — the real code path under test.
        let mut sequence = 0u64;
        for session_index in 0..11 {
            let session_id = format!("session-{session_index}");
            let mut buf = String::new();
            for _ in 0..1000 {
                sequence += 1;
                let observation = WorkflowObservation {
                    id: uuid::Uuid::new_v4().to_string(),
                    sequence,
                    session_id: session_id.clone(),
                    observed_at: Utc::now().to_rfc3339(),
                    kind: ObservationKind::Obstacle,
                    description: "description".to_string(),
                    evidence: "evidence".to_string(),
                    reported_impact: Impact::Medium,
                    source: ObservationSource::Agent,
                    confidence: AGENT_CONFIDENCE,
                    fingerprint: "v1:obstacle:description".to_string(),
                };
                let stored = StoredObservation {
                    observation,
                    idempotency_key_hash: format!("hash-{sequence}"),
                    payload_hash: format!("payload-{sequence}"),
                };
                buf.push_str(&serialize_observation_line(&stored).unwrap());
                buf.push('\n');
            }
            fs::write(ndjson_dir.join(format!("{session_id}.ndjson")), buf).unwrap();
        }
        // Also durably record the highest sequence so a fresh open() does not
        // treat this as a workspace with no counter file.
        fs::write(ndjson_dir.join("sequence"), sequence.to_string()).unwrap();

        let store = open_store(&root);
        let observations = store.workspace_observations().unwrap();
        assert_eq!(observations.len(), MAX_WORKSPACE_OBSERVATIONS);
        let min_sequence = observations.iter().map(|o| o.sequence).min().unwrap();
        assert_eq!(
            min_sequence,
            sequence - MAX_WORKSPACE_OBSERVATIONS as u64 + 1
        );
    }

    // -- Corruption / crash recovery ----------------------------------------

    #[test]
    fn crash_tail_is_truncated_with_diagnostic_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        let valid_observation = WorkflowObservation {
            id: "obs-1".to_string(),
            sequence: 1,
            session_id: "session-1".to_string(),
            observed_at: Utc::now().to_rfc3339(),
            kind: ObservationKind::Obstacle,
            description: "description".to_string(),
            evidence: "evidence".to_string(),
            reported_impact: Impact::Medium,
            source: ObservationSource::Agent,
            confidence: AGENT_CONFIDENCE,
            fingerprint: "v1:obstacle:description".to_string(),
        };
        let stored = StoredObservation {
            observation: valid_observation,
            idempotency_key_hash: "hash-1".to_string(),
            payload_hash: "payload-1".to_string(),
        };
        let valid_line = serialize_observation_line(&stored).unwrap();

        let path = ndjson_dir.join("session-1.ndjson");
        let mut contents = valid_line.clone();
        contents.push('\n');
        contents.push_str("{\"record\":\"observation\",\"id\":\"obs-2\",\"seque"); // truncated, no trailing newline
        fs::write(&path, &contents).unwrap();

        let store = open_store(&root);

        let diags = store.diagnostics();
        assert!(
            diags.iter().any(|d| d.code == "crash_tail_truncated"
                && d.session_id.as_deref() == Some("session-1")),
            "expected a crash_tail_truncated diagnostic, got {diags:?}"
        );

        let observations = store.workspace_observations().unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].id, "obs-1");

        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(
            on_disk,
            format!("{valid_line}\n"),
            "crash tail must be truncated from disk"
        );
    }

    #[test]
    fn interior_corruption_is_skipped_and_marks_diagnostics_degraded_but_keeps_later_records() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();

        let make_stored = |id: &str, sequence: u64| StoredObservation {
            observation: WorkflowObservation {
                id: id.to_string(),
                sequence,
                session_id: "session-1".to_string(),
                observed_at: Utc::now().to_rfc3339(),
                kind: ObservationKind::Obstacle,
                description: "description".to_string(),
                evidence: "evidence".to_string(),
                reported_impact: Impact::Medium,
                source: ObservationSource::Agent,
                confidence: AGENT_CONFIDENCE,
                fingerprint: "v1:obstacle:description".to_string(),
            },
            idempotency_key_hash: format!("hash-{id}"),
            payload_hash: format!("payload-{id}"),
        };

        let obs1 = serialize_observation_line(&make_stored("obs-1", 1)).unwrap();
        let obs2 = serialize_observation_line(&make_stored("obs-2", 2)).unwrap();
        let contents = format!("{obs1}\nnot json at all\n{obs2}\n");
        fs::write(ndjson_dir.join("session-1.ndjson"), contents).unwrap();

        let store = open_store(&root);

        let diags = store.diagnostics();
        assert!(
            diags
                .iter()
                .any(|d| d.code == "interior_corruption"
                    && d.session_id.as_deref() == Some("session-1")),
            "expected an interior_corruption diagnostic, got {diags:?}"
        );

        let observations = store.workspace_observations().unwrap();
        let ids: Vec<&str> = observations.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, vec!["obs-1", "obs-2"]);
    }

    // -- Sequencing -----------------------------------------------------

    #[test]
    fn sequence_numbers_are_monotonic_across_equal_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let frozen = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.test_set_clock(frozen);

        let mut sequences = Vec::new();
        for i in 0..5 {
            let outcome = store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(ObservationKind::Obstacle, "description", "evidence"),
                )
                .unwrap();
            match outcome {
                RecordOutcome::Accepted(obs) => {
                    assert_eq!(obs.observed_at, frozen.to_rfc3339());
                    sequences.push(obs.sequence);
                }
                other => panic!("expected Accepted, got {other:?}"),
            }
        }
        let mut sorted = sequences.clone();
        sorted.sort_unstable();
        assert_eq!(sequences, sorted, "sequences must already be increasing");
        assert_eq!(
            sequences.len(),
            sequences
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    #[test]
    fn sequence_numbers_survive_restart_without_reuse() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let max_seq;
        {
            let store = open_store(&root);
            let mut last = 0;
            for i in 0..3 {
                let outcome = store
                    .record_observation(
                        "session-1",
                        ObservationOrigin::Agent,
                        &format!("key-{i}"),
                        candidate(ObservationKind::Obstacle, "description", "evidence"),
                    )
                    .unwrap();
                last = match outcome {
                    RecordOutcome::Accepted(obs) => obs.sequence,
                    other => panic!("expected Accepted, got {other:?}"),
                };
            }
            max_seq = last;
        }

        let store = open_store(&root);
        let outcome = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-after-restart",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => assert_eq!(obs.sequence, max_seq + 1),
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn sequence_numbers_continue_after_session_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        let outcome = store
            .record_observation(
                "session-a",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        let seq_a = match outcome {
            RecordOutcome::Accepted(obs) => obs.sequence,
            other => panic!("expected Accepted, got {other:?}"),
        };

        store.delete_session_observations("session-a").unwrap();

        let outcome = store
            .record_observation(
                "session-b",
                ObservationOrigin::Agent,
                "key-2",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => assert_eq!(obs.sequence, seq_a + 1),
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn sequence_numbers_skip_over_a_deliberate_counter_gap() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();
        // Simulate a crash after the counter was durably advanced to 500 but
        // before any observation using it was ever written.
        fs::write(ndjson_dir.join("sequence"), "500").unwrap();

        let store = open_store(&root);
        let outcome = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => assert_eq!(obs.sequence, 501),
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn malformed_counter_file_degrades_store_and_rejects_writes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let ndjson_dir = root.join("workflow-observations");
        fs::create_dir_all(&ndjson_dir).unwrap();
        fs::write(ndjson_dir.join("sequence"), "not-a-number").unwrap();

        let store = open_store(&root);
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::Degraded);

        let diags = store.diagnostics();
        assert!(diags.iter().any(|d| d.code == "sequence_counter_corrupt"));
    }

    // -- Rate limiting -----------------------------------------------------

    #[test]
    fn caps_accepted_observations_at_sixty_per_session_per_rolling_minute() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());

        for i in 0..MAX_ACCEPTED_PER_SESSION_MINUTE {
            let outcome = store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(
                        ObservationKind::Obstacle,
                        &format!("description {i}"),
                        "evidence",
                    ),
                )
                .unwrap();
            assert!(
                matches!(outcome, RecordOutcome::Accepted(_)),
                "expected call {i} to be accepted, got {outcome:?}"
            );
        }

        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-overflow",
                candidate(
                    ObservationKind::Obstacle,
                    "description overflow",
                    "evidence",
                ),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::RateLimited);

        // A different session must not be affected by session-1's cap.
        let outcome = store
            .record_observation(
                "session-2",
                ObservationOrigin::Agent,
                "key-other-session",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        assert!(matches!(outcome, RecordOutcome::Accepted(_)));
    }

    #[test]
    fn rate_limit_does_not_consume_a_sequence_number_or_write_anything() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let store = open_store(&root);

        let mut last_sequence = 0;
        for i in 0..MAX_ACCEPTED_PER_SESSION_MINUTE {
            let outcome = store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(
                        ObservationKind::Obstacle,
                        &format!("description {i}"),
                        "evidence",
                    ),
                )
                .unwrap();
            last_sequence = match outcome {
                RecordOutcome::Accepted(obs) => obs.sequence,
                other => panic!("expected Accepted, got {other:?}"),
            };
        }

        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-overflow",
                candidate(
                    ObservationKind::Obstacle,
                    "description overflow",
                    "evidence",
                ),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::RateLimited);

        // The rejected call must not have advanced the durable sequence
        // counter or written anything: the next accepted call (in a
        // different, uncapped session) picks up right after the last
        // accepted call above, with no gap for the rate-limited attempt.
        let outcome = store
            .record_observation(
                "session-2",
                ObservationOrigin::Agent,
                "key-next",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();
        match outcome {
            RecordOutcome::Accepted(obs) => assert_eq!(obs.sequence, last_sequence + 1),
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_idempotency_replay_does_not_count_against_the_rate_cap() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());

        let first = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "retry-key",
                candidate(ObservationKind::Obstacle, "retried description", "evidence"),
            )
            .unwrap();
        let (first_id, first_seq) = match first {
            RecordOutcome::Accepted(obs) => (obs.id, obs.sequence),
            other => panic!("expected Accepted, got {other:?}"),
        };

        // Fill the session up to the cap with distinct observations.
        for i in 1..MAX_ACCEPTED_PER_SESSION_MINUTE {
            store
                .record_observation(
                    "session-1",
                    ObservationOrigin::Agent,
                    &format!("key-{i}"),
                    candidate(
                        ObservationKind::Obstacle,
                        &format!("description {i}"),
                        "evidence",
                    ),
                )
                .unwrap();
        }

        // The session is now at the cap: a genuinely new observation must be
        // rate-limited...
        let err = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-overflow",
                candidate(
                    ObservationKind::Obstacle,
                    "description overflow",
                    "evidence",
                ),
            )
            .unwrap_err();
        assert_eq!(err, RecordError::RateLimited);

        // ...but replaying the original idempotency key with the same
        // payload must still succeed as a Duplicate, since it does not
        // durably write anything new and therefore does not consume budget.
        let retry = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "retry-key",
                candidate(ObservationKind::Obstacle, "retried description", "evidence"),
            )
            .unwrap();
        match retry {
            RecordOutcome::Duplicate {
                observation_id,
                sequence,
                ..
            } => {
                assert_eq!(observation_id, first_id);
                assert_eq!(sequence, first_seq);
            }
            other => panic!("expected Duplicate, got {other:?}"),
        }
    }

    // -- Deletion ---------------------------------------------------------

    #[test]
    fn delete_session_observations_removes_segment_and_forgets_idempotency() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store(dir.path());
        store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(ObservationKind::Obstacle, "description", "evidence"),
            )
            .unwrap();

        store.delete_session_observations("session-1").unwrap();

        let observations = store.workspace_observations().unwrap();
        assert!(observations.is_empty());

        let path = dir
            .path()
            .join("workflow-observations")
            .join("session-1.ndjson");
        assert!(!path.exists());

        // The idempotency key must no longer be recognized as a duplicate:
        // recording it again with a different payload must succeed, not conflict.
        let outcome = store
            .record_observation(
                "session-1",
                ObservationOrigin::Agent,
                "key-1",
                candidate(
                    ObservationKind::Obstacle,
                    "a completely different description",
                    "evidence",
                ),
            )
            .unwrap();
        assert!(matches!(outcome, RecordOutcome::Accepted(_)));
    }
}
