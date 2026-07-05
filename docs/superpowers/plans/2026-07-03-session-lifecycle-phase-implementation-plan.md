# Session Lifecycle Phase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the domain-owned session lifecycle state machine from the approved spec, including `workPhase`, `lifecyclePhase`, structured observed-status snapshots, ending-phase finalization, and frontend/API consumption.

**Architecture:** Introduce the new lifecycle and snapshot types in the Rust domain first, then project them through metadata persistence and runtime orchestration before updating API and renderer consumers. Finalization remains runtime-triggered, but all lifecycle transitions flow through domain/application operations and persist structured snapshots as the single source of truth.

**Tech Stack:** Rust, serde, Tokio runtime, Node built-in test runner with TypeScript, Electron/React TypeScript

## Global Constraints

- Use a worktree or owned branch before code changes because implementation touches `apps/desktop/` and `crates/orkworksd/`.
- Invoke `skills/starting-work` before code changes.
- Rename existing work-classification `phase` to `workPhase`.
- Add `lifecyclePhase` with `creating | active | ending | ended`.
- Keep `status` values `creating | running | ended | killed | error`; `running` remains the non-terminal lifecycle-visible state during `ending`.
- Persist `endingObservedStatusSnapshot` and `finalObservedStatusSnapshot` as canonical `ObservedStatusSnapshot` objects.
- Expose `finalObservedStatus` as API/frontend projection only; do not persist it as source of truth.
- `finalObservedStatus` must always serialize in frontend DTOs as `ObservedStatus | null`.
- `begin_ending(...)` must capture `endingObservedStatusSnapshot` atomically.
- `complete_ending(...)` must be idempotent; timeout completion and final-scan completion race through the same operation and first winner wins.
- Final scan timeout must be configurable and default to `2` seconds.
- All process-start, process-exit, kill, and error transitions must flow through domain/application lifecycle operations instead of direct runtime metadata writes.
- `endingObservedStatusSnapshot` and `pendingTerminalStatus` are backend-internal only; do not expose them in frontend DTOs.
- Normalized or finalized `ended` sessions must have a populated `finalObservedStatusSnapshot`, using the canonical synthesized null snapshot when no observed value exists.
- Keep the session list/detail layout unchanged.
- Update [docs/agents/domain-entities.md](/Users/froomiebot/workspace/orkworks/docs/agents/domain-entities.md:1), [AGENTS.md](/Users/froomiebot/workspace/orkworks/AGENTS.md:1) if needed, and run `bash .claude/hooks/doc-check.sh` before claiming completion.
- Do not edit the approved spec during implementation unless a new design correction is explicitly approved.

---

### Task 1: Add Domain Lifecycle And Snapshot Types

**Files:**
- Modify: `crates/orkworksd/src/domain/session/value_objects.rs`
- Modify: `crates/orkworksd/src/domain/session/entity.rs`
- Modify: `crates/orkworksd/src/domain/session/services.rs`
- Modify: `crates/orkworksd/src/application/session/handlers.rs`
- Test: `crates/orkworksd/src/domain/session/value_objects.rs`
- Test: `crates/orkworksd/src/domain/session/entity.rs`
- Test: `crates/orkworksd/src/domain/session/services.rs`

**Interfaces:**
- Consumes: existing `SessionStatus`, `Phase`, `Session`, `SessionLifecycle`
- Produces:
  - `pub enum WorkPhase { Ideation, Implementation, Review, Debugging, Unknown }`
  - `pub enum LifecyclePhase { Creating, Active, Ending, Ended }`
  - `pub enum TerminalOutcome { Ended, Killed, Error }`
  - `pub struct ObservedStatusSnapshot { pub value: Option<AttentionState>, pub source: String, pub confidence: Option<f64>, pub observed_at: Option<String> }`
  - `impl Session { pub fn mark_active(&mut self) -> Result<(), SessionTransitionError>; pub fn begin_ending(&mut self, pending_terminal_status: TerminalOutcome, ending_observed_status_snapshot: ObservedStatusSnapshot) -> Result<(), SessionTransitionError>; pub fn complete_ending(&mut self, final_observed_status_snapshot: ObservedStatusSnapshot) -> Result<(), SessionTransitionError>; }`

- [ ] **Step 1: Write the failing Rust tests for lifecycle and snapshot behavior**

```rust
#[test]
fn begin_ending_sets_lifecycle_and_captures_snapshot() {
    let mut s = make_test_session();
    s.mark_active().unwrap();
    s.begin_ending(
        TerminalOutcome::Ended,
        ObservedStatusSnapshot {
            value: Some(AttentionState::Blocked),
            source: "peon".into(),
            confidence: Some(0.82),
            observed_at: Some("2026-07-03T12:34:56Z".into()),
        },
    ).unwrap();
    assert_eq!(s.lifecycle_phase, LifecyclePhase::Ending);
    assert_eq!(s.status, SessionStatus::Running);
    assert_eq!(s.pending_terminal_status, Some(TerminalOutcome::Ended));
    assert_eq!(s.ending_observed_status_snapshot.as_ref().and_then(|x| x.value.as_ref()), Some(&AttentionState::Blocked));
}

#[test]
fn complete_ending_is_first_winner_and_sets_final_snapshot() {
    let mut s = make_test_session();
    s.mark_active().unwrap();
    s.begin_ending(
        TerminalOutcome::Killed,
        ObservedStatusSnapshot {
            value: None,
            source: "recovery".into(),
            confidence: None,
            observed_at: None,
        },
    ).unwrap();
    s.complete_ending(ObservedStatusSnapshot {
        value: Some(AttentionState::Done),
        source: "peon".into(),
        confidence: Some(0.91),
        observed_at: Some("2026-07-03T12:40:00Z".into()),
    }).unwrap();
    s.complete_ending(ObservedStatusSnapshot {
        value: Some(AttentionState::Failed),
        source: "peon".into(),
        confidence: Some(0.12),
        observed_at: Some("2026-07-03T12:41:00Z".into()),
    }).unwrap();
    assert_eq!(s.lifecycle_phase, LifecyclePhase::Ended);
    assert_eq!(s.status, SessionStatus::Killed);
    assert_eq!(s.final_observed_status_snapshot.as_ref().and_then(|x| x.value.as_ref()), Some(&AttentionState::Done));
}

#[test]
fn invalid_transition_shortcuts_are_rejected_and_completion_clears_pending_state() {
    let mut s = make_test_session();
    let snap = ObservedStatusSnapshot { value: None, source: "recovery".into(), confidence: None, observed_at: None };
    assert!(s.begin_ending(TerminalOutcome::Ended, snap.clone()).is_err());
    assert!(s.complete_ending(snap.clone()).is_err());
    s.mark_active().unwrap();
    s.begin_ending(TerminalOutcome::Error, snap.clone()).unwrap();
    s.complete_ending(snap).unwrap();
    assert_eq!(s.pending_terminal_status, None);
    assert_eq!(s.ending_observed_status_snapshot, None);
    assert!(s.mark_active().is_err());
}
```

- [ ] **Step 2: Run the focused Rust tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml domain::session::`

Expected: FAIL with missing fields and methods such as `lifecycle_phase`, `pending_terminal_status`, `ObservedStatusSnapshot`, `mark_active`, `begin_ending`, or `complete_ending`.

- [ ] **Step 3: Implement the domain types and methods**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkPhase {
    Ideation,
    Implementation,
    Review,
    Debugging,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    Creating,
    Active,
    Ending,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedStatusSnapshot {
    pub value: Option<AttentionState>,
    pub source: String,
    pub confidence: Option<f64>,
    #[serde(rename = "observedAt")]
    pub observed_at: Option<String>,
}

impl Session {
    pub fn mark_active(&mut self) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase != LifecyclePhase::Creating {
            return Err(SessionTransitionError::InvalidPhase);
        }
        self.status = SessionStatus::Running;
        self.lifecycle_phase = LifecyclePhase::Active;
        Ok(())
    }

    pub fn begin_ending(
        &mut self,
        pending_terminal_status: TerminalOutcome,
        ending_observed_status_snapshot: ObservedStatusSnapshot,
    ) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase != LifecyclePhase::Active {
            return Err(SessionTransitionError::InvalidPhase);
        }
        self.lifecycle_phase = LifecyclePhase::Ending;
        self.status = SessionStatus::Running;
        self.pending_terminal_status = Some(pending_terminal_status);
        self.ending_observed_status_snapshot = Some(ending_observed_status_snapshot);
        Ok(())
    }

    pub fn complete_ending(&mut self, final_observed_status_snapshot: ObservedStatusSnapshot) -> Result<(), SessionTransitionError> {
        if self.lifecycle_phase == LifecyclePhase::Ended {
            return Ok(());
        }
        if self.lifecycle_phase != LifecyclePhase::Ending {
            return Err(SessionTransitionError::InvalidPhase);
        }
        let final_status = self.pending_terminal_status.clone().ok_or(SessionTransitionError::MissingPendingOutcome)?;
        self.lifecycle_phase = LifecyclePhase::Ended;
        self.status = final_status.into();
        self.final_observed_status_snapshot = Some(final_observed_status_snapshot);
        self.pending_terminal_status = None;
        self.ending_observed_status_snapshot = None;
        Ok(())
    }
}
```

- [ ] **Step 4: Run the focused Rust tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml domain::session::`

Expected: PASS for the new lifecycle and snapshot tests.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/domain/session/value_objects.rs crates/orkworksd/src/domain/session/entity.rs crates/orkworksd/src/domain/session/services.rs crates/orkworksd/src/application/session/handlers.rs
git commit -m "feat: add session lifecycle domain state"
```

### Task 2: Persist Work Phase, Lifecycle Phase, And Snapshot Metadata

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/infrastructure/session_repository.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Test: `crates/orkworksd/src/metadata.rs`
- Test: `crates/orkworksd/src/session_types.rs`

**Interfaces:**
- Consumes:
  - `ObservedStatusSnapshot`
  - `WorkPhase`
  - `LifecyclePhase`
- Produces:
  - `SessionMetadata { work_phase: String, lifecycle_phase: String, pending_terminal_status: Option<String>, ending_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata>, final_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata> }`
  - `SessionInfo { workPhase: string, lifecyclePhase: string, finalObservedStatus: string | null }`
  - backward-compatible read normalization from legacy `phase` and `observedStatus`

- [ ] **Step 1: Write failing persistence and normalization tests**

```rust
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
    let meta: SessionMetadata = serde_json::from_str(raw).unwrap();
    assert_eq!(meta.work_phase, "review");
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.final_observed_status_snapshot.as_ref().and_then(|x| x.value.as_deref()), Some("blocked"));
}

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

#[test]
fn normalize_invalid_ending_without_pending_status_becomes_error_ended() {
    let raw = r#"{"id":"s3","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"ending","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.status, "error");
}

#[test]
fn normalize_unknown_legacy_phase_to_unknown_work_phase() {
    let raw = r#"{"id":"s4","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","phase":"freeform","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.work_phase, "unknown");
}

#[test]
fn normalize_pending_terminal_status_outside_ending_to_null_and_clear_live_observed_status() {
    let raw = r#"{"id":"s5","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"ended","lifecyclePhase":"ended","pendingTerminalStatus":"killed","observedStatus":"blocked","createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.pending_terminal_status, None);
    assert_eq!(meta.observed_status, None);
}

#[test]
fn normalize_recovery_prefers_existing_final_snapshot() {
    let raw = r#"{"id":"s6","label":"T","workspace":"/tmp","task":"","harnessId":"","modelId":"","cwd":"/tmp","status":"running","lifecyclePhase":"ending","pendingTerminalStatus":"ended","finalObservedStatusSnapshot":{"value":"done","source":"peon","confidence":0.9,"observedAt":"now"},"createdAt":"now","lastActivity":"now","metadataSource":"process","metadataConfidence":1.0}"#;
    let meta = normalize_session_metadata(serde_json::from_str(raw).unwrap());
    assert_eq!(meta.final_observed_status_snapshot.as_ref().and_then(|x| x.value.as_deref()), Some("done"));
}

#[test]
fn session_info_json_always_includes_final_observed_status_and_excludes_internal_snapshot_fields() {
    let info = test_session_info("s7".into(), "Label", "/tmp".into(), "ended");
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"finalObservedStatus\":null"));
    assert!(!json.contains("finalObservedStatusSnapshot"));
    assert!(!json.contains("pendingTerminalStatus"));
    assert!(!json.contains("endingObservedStatusSnapshot"));
}
```

- [ ] **Step 2: Run the focused Rust tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_types::`

Expected: FAIL with unknown `work_phase`, `lifecycle_phase`, `final_observed_status_snapshot`, or projection fields.

- [ ] **Step 3: Implement metadata structs, serde aliases, and projection logic**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservedStatusSnapshotMetadata {
    pub value: Option<String>,
    pub source: String,
    pub confidence: Option<f64>,
    #[serde(rename = "observedAt")]
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    #[serde(rename = "workPhase", alias = "phase", default)]
    pub work_phase: String,
    #[serde(rename = "lifecyclePhase", default = "default_lifecycle_phase")]
    pub lifecycle_phase: String,
    #[serde(rename = "pendingTerminalStatus", skip_serializing_if = "Option::is_none")]
    pub pending_terminal_status: Option<String>,
    #[serde(rename = "endingObservedStatusSnapshot", skip_serializing_if = "Option::is_none")]
    pub ending_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata>,
    #[serde(rename = "finalObservedStatusSnapshot", skip_serializing_if = "Option::is_none")]
    pub final_observed_status_snapshot: Option<ObservedStatusSnapshotMetadata>,
}

fn projected_final_observed_status(meta: &SessionMetadata) -> Option<String> {
    meta.final_observed_status_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.value.clone())
}
```

- [ ] **Step 4: Run the focused Rust tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml session_types::`

Expected: PASS for legacy normalization and DTO projection tests.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/infrastructure/session_repository.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/session_view.rs
git commit -m "feat: persist lifecycle phases and observed snapshots"
```

### Task 3: Route Runtime Exit Paths Through Domain Finalization

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Consumes:
  - `Session::mark_active()`
  - `Session::begin_ending(pending_terminal_status, ending_observed_status_snapshot)`
  - `Session::complete_ending(final_observed_status_snapshot)`
- Produces:
  - application lifecycle handlers:
    - `MarkSessionActiveHandler`
    - `BeginSessionEndingHandler`
    - `CompleteSessionEndingHandler`
  - lifecycle transition event coverage for `creating -> active`, `active -> ending`, and `ending -> ended`
  - `set_session_status` replaced or reduced to lifecycle-aware helpers that delegate to those handlers
  - configurable final-scan timeout with default `2s`
  - crash/restart recovery that normalizes lingering `ending` sessions
  - explicit exit-path matrix proving every current direct terminal status write now routes through the lifecycle helper:
    - `terminal_runtime.rs`: initial kill signal branch, runtime kill branch, spawn failure, PTY read/write error, detached cleanup timeout, normal child exit, error child exit
    - `http/session_handlers.rs`: HTTP kill/delete path and any helper tests still calling `set_session_status`

- [ ] **Step 1: Write failing runtime tests for ending and recovery**

```rust
#[test]
fn process_exit_enters_ending_before_final_status() {
    let state = test_app_state();
    insert_live_session(&state, "s1");
    begin_session_shutdown(&state, "s1", "ended");
    let info = state.sessions.lock().unwrap().get("s1").unwrap().info.clone();
    assert_eq!(info.lifecycle_phase.as_deref(), Some("ending"));
    assert_eq!(info.status, "running");
    assert_eq!(info.pending_terminal_status.as_deref(), Some("ended"));
}

#[tokio::test]
async fn final_scan_timeout_completes_to_ended_once() {
    let state = test_app_state_with_hanging_peon();
    insert_live_session(&state, "s2");
    finalize_session_with_timeout(&state, "s2", "killed").await;
    let info = state.sessions.lock().unwrap().get("s2").unwrap().info.clone();
    assert_eq!(info.lifecycle_phase.as_deref(), Some("ended"));
    assert_eq!(info.status, "killed");
}

#[tokio::test]
async fn ending_ignores_normal_inflight_peon_results_and_runs_final_scan_once() {
    let state = test_app_state_with_controlled_peon();
    insert_live_session(&state, "s3");
    let normal = queue_pending_normal_peon_result(&state, "s3", snapshot("blocked"));
    begin_session_shutdown(&state, "s3", "ended");
    queue_final_peon_result(&state, "s3", snapshot_with_null_value());
    resolve_pending_normal_peon_result(normal);
    drive_finalization(&state, "s3").await;
    let info = state.sessions.lock().unwrap().get("s3").unwrap().info.clone();
    assert_eq!(info.lifecycle_phase.as_deref(), Some("ended"));
    assert_eq!(info.final_observed_status, None);
    assert_eq!(count_final_scan_attempts(&state, "s3"), 1);
}

#[tokio::test]
async fn all_terminal_exit_paths_enter_ending_before_completion() {
    for scenario in [
        ExitScenario::InitialKillSignal,
        ExitScenario::ExplicitKill,
        ExitScenario::SpawnFailure,
        ExitScenario::PtyReadError,
        ExitScenario::PtyWriteError,
        ExitScenario::DetachedCleanup,
        ExitScenario::NormalChildExit,
        ExitScenario::ErrorChildExit,
        ExitScenario::HttpKill,
        ExitScenario::HttpDelete,
    ] {
        let state = test_app_state_for_exit_scenario(scenario);
        let session_id = seed_live_session_for_exit_scenario(&state, scenario);
        trigger_exit_scenario(&state, &session_id, scenario).await;
        assert_transitioned_through_ending_via_event_or_transition_log(
            &state,
            &session_id,
            expected_terminal_outcome(scenario),
        );
    }
}

#[test]
fn lifecycle_handlers_emit_transition_events() {
    let mut s = make_test_session();
    assert_emits_event(&mut s, expected_active_event_name(), |session| session.mark_active());
    assert_emits_event(&mut s, expected_ending_event_name(), |session| {
        session.begin_ending(TerminalOutcome::Ended, snapshot("blocked"))
    });
    assert_emits_event(&mut s, expected_ended_event_name(), |session| {
        session.complete_ending(snapshot_with_null_value())
    });
}

#[tokio::test]
async fn ending_with_disabled_or_empty_peon_skips_to_fallback_snapshot() {
    let state = test_app_state_with_disabled_peon();
    insert_live_session(&state, "s4");
    begin_session_shutdown(&state, "s4", "error");
    drive_finalization(&state, "s4").await;
    let info = state.sessions.lock().unwrap().get("s4").unwrap().info.clone();
    assert_eq!(info.lifecycle_phase.as_deref(), Some("ended"));
    assert_eq!(info.status, "error");
}

#[tokio::test]
async fn no_peon_inference_runs_after_ended_and_attention_writes_are_rejected_outside_active() {
    let state = test_app_state_with_controlled_peon();
    insert_ended_session(&state, "s5");
    drive_peon_tick(&state).await;
    assert_eq!(count_normal_peon_attempts(&state, "s5"), 0);
    let res = post_attention_for_test(&state, "s5", "blocked").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn configured_timeout_override_is_used() {
    let state = test_app_state_with_timeout_override(std::time::Duration::from_millis(25));
    insert_live_session(&state, "s6");
    begin_session_shutdown(&state, "s6", "ended");
    let started = std::time::Instant::now();
    drive_finalization(&state, "s6").await;
    assert!(started.elapsed().as_millis() >= 25);
    assert!(started.elapsed().as_millis() < 2000);
}

#[test]
fn default_final_scan_timeout_is_two_seconds() {
    let cfg = PeonConfig::default();
    assert_eq!(cfg.final_scan_timeout_secs, 2);
}

#[test]
fn restart_recovery_finalizes_persisted_ending_session() {
    let store = test_metadata_store();
    write_session_json(&store, serde_json::json!({
        "id": "s7",
        "label": "Recover",
        "workspace": "/tmp",
        "task": "",
        "harnessId": "",
        "modelId": "",
        "cwd": "/tmp",
        "status": "running",
        "lifecyclePhase": "ending",
        "pendingTerminalStatus": "ended",
        "endingObservedStatusSnapshot": {"value":"blocked","source":"peon","confidence":0.8,"observedAt":"now"},
        "createdAt": "now",
        "lastActivity": "now",
        "metadataSource": "process",
        "metadataConfidence": 1.0
    }));
    recover_workspace_sessions_for_test(&store);
    let meta = store.read_session("s7").unwrap();
    assert_eq!(meta.lifecycle_phase, "ended");
    assert_eq!(meta.status, "ended");
    assert_eq!(meta.pending_terminal_status, None);
    assert_eq!(meta.observed_status, None);
    assert_eq!(meta.final_observed_status_snapshot.as_ref().and_then(|x| x.value.as_deref()), Some("blocked"));
}
```

- [ ] **Step 2: Run the focused Rust runtime tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers::`

Expected: FAIL because exit paths still call `set_session_status(..., "ended" | "killed" | "error")` directly.

- [ ] **Step 3: Implement lifecycle-aware runtime orchestration**

```rust
pub(crate) fn begin_session_ending(
    state: &Arc<AppState>,
    id: &str,
    pending_terminal_status: &str,
    ending_snapshot: ObservedStatusSnapshotMetadata,
) {
    // delegate to BeginSessionEndingHandler, persist aggregate-backed metadata,
    // update SessionHandle projection only after handler success
}

pub(crate) async fn complete_session_ending(
    state: Arc<AppState>,
    id: String,
    final_snapshot: Option<ObservedStatusSnapshotMetadata>,
) {
    // choose authoritative snapshot, call CompleteSessionEndingHandler,
    // clear pending state, suppress later completions, update SessionHandle projection
}
```

Implementation note: make the exit-path matrix the table-driven test input itself. Each row should name the old direct `set_session_status(..., "ended" | "killed" | "error")` caller, the replacement helper, the expected `TerminalOutcome`, and the assertion site, so the test inventory cannot silently drift from the matrix.

- [ ] **Step 4: Run the focused Rust runtime tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::peon_runtime::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers::`

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::`

Expected: PASS for ending transition, timeout, idempotency, and recovery tests.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/peon_runtime.rs crates/orkworksd/src/peon.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/main.rs
git commit -m "feat: route runtime session exits through lifecycle finalization"
```

### Task 4: Update API And Frontend Consumers

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/sessionSort.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/labels.ts`
- Test: `apps/desktop/tests/api.test.ts`
- Test: `apps/desktop/tests/sessionSort.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/labels.test.ts`

**Interfaces:**
- Consumes:
  - backend DTO fields `workPhase`, `lifecyclePhase`, `finalObservedStatus`
- Produces:
  - `SessionInfo["lifecyclePhase"]`
  - `sessionAttentionStatus(session)` using `lifecyclePhase === "active"` in both `apps/desktop/src/sessionSort.ts` and `apps/desktop/src/domain/session.ts`
  - detail panel rendering final frozen state from `finalObservedStatus`

- [ ] **Step 1: Write failing frontend tests**

```ts
test("sessionAttentionStatus ignores final observed state when lifecycle is ended", () => {
  const session = {
    id: "s1",
    label: "Ended",
    status: "ended",
    lifecyclePhase: "ended",
    finalObservedStatus: "blocked",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "remembered",
    resumeStrategy: "none",
  };
  assert.equal(sessionAttentionStatus(session as any), "ended");
});

test("fromApiDto_preserves_finalObservedStatus_projection", () => {
  const session = fromApiDto({
    id: "s1",
    label: "Ended",
    status: "ended",
    lifecyclePhase: "ended",
    finalObservedStatus: "blocked",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "remembered",
    resumeStrategy: "none",
  } as any);
  assert.equal(session.finalObservedStatus, "blocked");
});

test("domain attention helper ignores stale observed state for ended sessions", () => {
  const session = fromApiDto({
    id: "s2",
    label: "Ended",
    status: "ended",
    lifecyclePhase: "ended",
    observedStatus: "blocked",
    finalObservedStatus: "blocked",
    cwd: "/tmp",
    created_at: "now",
    memoryState: "remembered",
    resumeStrategy: "none",
  } as any);
  assert.equal(sessionAttentionStatus(session as any), "ended");
});

test("SessionDetailPanel source contract gates final observed status behind ended lifecycle context", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /finalObservedStatus/);
  assert.match(source, /lifecyclePhase\s*===\s*["']ended["']/);
  assert.match(source, /Historical observed state|Final observed state/);
});
```

- [ ] **Step 2: Run the focused frontend tests to verify they fail**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/sessionSort.test.ts tests/dockview.test.ts tests/labels.test.ts`

Expected: FAIL with missing `lifecyclePhase`, `workPhase`, or incorrect attention projection from old `status`/`observedStatus` logic.

- [ ] **Step 3: Implement DTO and renderer updates**

```ts
export interface SessionInfo {
  workPhase: string;
  lifecyclePhase: "creating" | "active" | "ending" | "ended";
  finalObservedStatus: string | null;
}

export function sessionAttentionStatus(session: SessionInfo): string {
  if (session.lifecyclePhase === "active" && session.observedStatus) {
    return session.observedStatus;
  }
  if (session.lifecyclePhase === "active") {
    return session.status;
  }
  return session.status;
}
```

- [ ] **Step 4: Run the focused frontend tests to verify they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts tests/sessionSort.test.ts tests/dockview.test.ts tests/labels.test.ts`

Expected: PASS for DTO shape, lifecycle-aware attention, and detail-panel final-state rendering.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/api.ts apps/desktop/src/sessionSort.ts apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/src/domain/session.ts apps/desktop/src/components/SessionListPanel.tsx apps/desktop/src/labels.ts apps/desktop/tests/api.test.ts apps/desktop/tests/sessionSort.test.ts apps/desktop/tests/dockview.test.ts apps/desktop/tests/labels.test.ts
git commit -m "feat: consume lifecycle phase and final observed state in desktop UI"
```

### Task 5: Update Docs, Run Full Verification, And Close The Loop

**Files:**
- Modify: `docs/agents/domain-entities.md`
- Modify: `AGENTS.md`
- Test: `bash .claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: implemented lifecycle/work-phase/snapshot model from Tasks 1-4
- Produces:
  - updated domain terminology docs
  - final verification evidence for Rust and frontend

- [ ] **Step 1: Write failing doc or contract assertions as lightweight tests/checks**

```bash
rg -n '`phase`| phase:| Phase |current work phase' docs/agents/domain-entities.md AGENTS.md
```

Expected: legacy ambiguous references still exist and need replacement with `workPhase` or `lifecyclePhase`.

- [ ] **Step 2: Run full verification before doc edits complete**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Expected: PASS only after Tasks 1-4 are complete; if not, fix regressions before doc finalization.

- [ ] **Step 3: Update docs to match the shipped model**

```md
- `workPhase`: inferred task type (`ideation`, `implementation`, `review`, `debugging`, `unknown`)
- `lifecyclePhase`: runtime lifecycle (`creating`, `active`, `ending`, `ended`)
- `finalObservedStatusSnapshot`: persisted historical observer snapshot
- `finalObservedStatus`: API/frontend projection derived from the snapshot value
```

- [ ] **Step 4: Run final repository verification and doc currency check**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`

Run: `bash .claude/hooks/doc-check.sh`

Expected: all tests PASS; doc-check reports no unaddressed required doc updates.

- [ ] **Step 5: Commit**

```bash
git add docs/agents/domain-entities.md AGENTS.md
git commit -m "docs: align lifecycle phase and observed snapshot terminology"
```
