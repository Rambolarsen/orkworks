# Session State Injection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a debug-only Details-panel control that injects one curated temporary session state, shows it immediately, and then lets normal OrkWorks runtime/metadata logic converge to the real state.

**Architecture:** Keep the mutation authority in the Rust sidecar. Add a small backend-owned injection catalog plus a narrow apply endpoint, persist canonical state directly where possible, and use a dedicated session-scoped metadata overlay only for the `running_capped` projection that cannot live in `SessionMetadata` today. Route renderer apply actions through Electron IPC so the existing `Show debug metadata` setting can gate the feature before the sidecar call happens.

**Tech Stack:** Rust + Axum + serde, Electron IPC/preload, React/TypeScript, Node built-in test runner, pnpm

## Global Constraints

- Invoke `skills/starting-work` before any code edits because implementation touches both `apps/desktop/` and `crates/orkworksd/`.
- Create a GitHub issue for this work before the first code commit because the approved spec does not yet have a matching implementation issue.
- Use `test-driven-development`: every production-code task starts with a failing test and a red-green cycle.
- Keep the feature gated by `settings.debug.showSessionIds`; do not add a second persistent debug toggle.
- Do not add a generic metadata-edit endpoint or free-form JSON editor.
- Injection writes must use `metadataSource = "debug"`. Persisted `SessionMetadata.metadata_confidence` stays `0.0` for this feature; projected renderer `SessionInfo.metadataConfidence` may be `null`.
- The `running_capped` scenario must not mutate provider-wide capacity state, shared capacity files, or any propagation input that marks sibling live sessions capped.
- The apply path must return an injected `SessionInfo` snapshot immediately so the renderer can show the perturbation before later correction wins.
- `active_fake_ending` must trigger the existing ending-finalization path; it must not leave sessions stuck in `ending`.
- Update [specs/orkworks-mvp.md](/Users/froomiebot/workspace/orkworks/specs/orkworks-mvp.md:157), [docs/adr/0005-metadata-source-priority.md](/Users/froomiebot/workspace/orkworks/docs/adr/0005-metadata-source-priority.md:1), and [AGENTS.md](/Users/froomiebot/workspace/orkworks/AGENTS.md:225) to include the new `debug` metadata source vocabulary.
- Run `bash .claude/hooks/doc-check.sh` before claiming completion.

---

### Task 1: Open The Tracking Issue And Add Backend Injection Primitives

**Files:**
- Create: `crates/orkworksd/src/debug_state_injection.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/debug_state_injection.rs`
- Test: `crates/orkworksd/src/metadata.rs`

**Interfaces:**
- Produces:
  - `pub(crate) struct SessionStateInjectionOption { pub id: &'static str, pub label: &'static str }`
  - `pub(crate) enum SessionStateInjectionId { ActiveFakeEnding, EndedStaleLiveAttention, EndedMissingFinalSnapshot, RunningBlocked, RunningIdleTooEarly, RunningCapped }`
  - `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] pub struct DebugInjectionMetadata { pub attention: String, #[serde(rename = "usageLimitResetHint", skip_serializing_if = "Option::is_none")] pub usage_limit_reset_hint: Option<String>, #[serde(rename = "appliedAt")] pub applied_at: String }`
  - `impl SessionStateInjectionId { pub(crate) fn parse(id: &str) -> Option<Self>; pub(crate) fn options() -> Vec<SessionStateInjectionOption>; }`
  - `struct SessionHandle { /* existing fields */, debug_injection: Option<metadata::DebugInjectionMetadata> }`
- Consumes:
  - `SessionMetadata`
  - current metadata normalization in `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Create the GitHub issue before code work**

```bash
rtk gh issue create \
  --repo Rambolarsen/orkworks \
  --title "Debug-only session state injection for convergence testing" \
  --body $'## Summary\nAdd a debug-only Details-panel control that applies a curated temporary session-state injection and lets the real runtime/metadata system converge afterward.\n\n## Acceptance criteria\n- [ ] Details panel exposes a debug-only State injection dropdown behind Show debug metadata\n- [ ] Sidecar owns the injection catalog and apply endpoint\n- [ ] Apply returns an injected SessionInfo snapshot immediately\n- [ ] running_capped stays session-scoped and does not affect provider/global capping\n- [ ] active_fake_ending triggers normal ending finalization\n- [ ] Metadata vocabulary/docs include metadataSource = \"debug\"'
```

- [ ] **Step 2: Write the failing Rust tests for the injection catalog and persisted overlay**

```rust
// crates/orkworksd/src/debug_state_injection.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_lists_all_curated_injections() {
        let ids: Vec<&'static str> = SessionStateInjectionId::options()
            .into_iter()
            .map(|option| option.id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "active_fake_ending",
                "ended_stale_live_attention",
                "ended_missing_final_snapshot",
                "running_blocked",
                "running_idle_too_early",
                "running_capped",
            ]
        );
    }

    #[test]
    fn parse_rejects_unknown_ids() {
        assert!(SessionStateInjectionId::parse("definitely_not_real").is_none());
    }
}

// crates/orkworksd/src/metadata.rs
#[test]
fn debug_injection_metadata_roundtrips() {
    let raw = r#"{
      "id":"s1",
      "label":"Test",
      "workspace":"/tmp",
      "task":"",
      "harnessId":"codex",
      "modelId":"",
      "cwd":"/tmp",
      "status":"running",
      "workPhase":"unknown",
      "lifecyclePhase":"active",
      "connectivity":"online",
      "createdAt":"now",
      "lastActivity":"now",
      "metadataSource":"debug",
      "metadataConfidence":0.0,
      "debugInjection":{"attention":"capped","usageLimitResetHint":"resets in 1h (debug)","appliedAt":"now"}
    }"#;
    let meta: SessionMetadata = serde_json::from_str(raw).unwrap();
    assert_eq!(meta.debug_injection.as_ref().map(|d| d.attention.as_str()), Some("capped"));
}

#[test]
fn normalize_session_metadata_preserves_debug_injection_overlay() {
    let meta = normalize_session_metadata(SessionMetadata {
        id: "s1".into(),
        label: "Test".into(),
        workspace: "/tmp".into(),
        task: "".into(),
        harness: "codex".into(),
        model: "".into(),
        cwd: "/tmp".into(),
        status: "running".into(),
        work_phase: "unknown".into(),
        lifecycle_phase: "active".into(),
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
        metadata_source: "debug".into(),
        metadata_confidence: 0.0,
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
        debug_injection: Some(DebugInjectionMetadata {
            attention: "capped".into(),
            usage_limit_reset_hint: Some("resets in 1h (debug)".into()),
            applied_at: "now".into(),
        }),
    });
    assert!(meta.debug_injection.is_some());
}
```

- [ ] **Step 3: Run the focused Rust tests and verify they fail**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml debug_state_injection
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml debug_injection_metadata_roundtrips
```

Expected:

- FAIL with missing `SessionStateInjectionId`, `DebugInjectionMetadata`, or `SessionMetadata.debug_injection`

- [ ] **Step 4: Implement the catalog module and persisted overlay**

```rust
// crates/orkworksd/src/debug_state_injection.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionStateInjectionId {
    ActiveFakeEnding,
    EndedStaleLiveAttention,
    EndedMissingFinalSnapshot,
    RunningBlocked,
    RunningIdleTooEarly,
    RunningCapped,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SessionStateInjectionOption {
    pub id: &'static str,
    pub label: &'static str,
}

impl SessionStateInjectionId {
    pub(crate) fn parse(id: &str) -> Option<Self> {
        match id {
            "active_fake_ending" => Some(Self::ActiveFakeEnding),
            "ended_stale_live_attention" => Some(Self::EndedStaleLiveAttention),
            "ended_missing_final_snapshot" => Some(Self::EndedMissingFinalSnapshot),
            "running_blocked" => Some(Self::RunningBlocked),
            "running_idle_too_early" => Some(Self::RunningIdleTooEarly),
            "running_capped" => Some(Self::RunningCapped),
            _ => None,
        }
    }

    pub(crate) fn options() -> Vec<SessionStateInjectionOption> {
        vec![
            SessionStateInjectionOption { id: "active_fake_ending", label: "Active -> fake ending" },
            SessionStateInjectionOption { id: "ended_stale_live_attention", label: "Ended -> stale live attention" },
            SessionStateInjectionOption { id: "ended_missing_final_snapshot", label: "Ended -> missing final snapshot" },
            SessionStateInjectionOption { id: "running_blocked", label: "Running -> blocked" },
            SessionStateInjectionOption { id: "running_idle_too_early", label: "Running -> idle too early" },
            SessionStateInjectionOption { id: "running_capped", label: "Running -> capped" },
        ]
    }
}
```

```rust
// crates/orkworksd/src/metadata.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebugInjectionMetadata {
    pub attention: String,
    #[serde(rename = "usageLimitResetHint", skip_serializing_if = "Option::is_none")]
    pub usage_limit_reset_hint: Option<String>,
    #[serde(rename = "appliedAt")]
    pub applied_at: String,
}

pub struct SessionMetadata {
    // ...
    #[serde(rename = "debugInjection", skip_serializing_if = "Option::is_none")]
    pub debug_injection: Option<DebugInjectionMetadata>,
}
```

```rust
// crates/orkworksd/src/main.rs
mod debug_state_injection;

struct SessionHandle {
    info: SessionInfo,
    kill_tx: tokio::sync::watch::Sender<bool>,
    output_buffer: peon::RingBuffer,
    scan_buf: String,
    command: harness::CommandSpec,
    initial_prompt: Option<String>,
    terminal_attached: bool,
    at_usage_limit_latched: bool,
    capacity_check_pending: bool,
    output_lines_seen: u64,
    scan_bytes_seen: u64,
    resume_scan_origin: Option<(u64, u64)>,
    pending_capacity_visible_once: bool,
    debug_injection: Option<metadata::DebugInjectionMetadata>,
}
```

- [ ] **Step 5: Run the focused Rust tests and verify they pass**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml debug_state_injection
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml debug_injection_metadata_roundtrips
```

Expected:

- PASS

- [ ] **Step 6: Commit**

```bash
rtk git add \
  crates/orkworksd/src/debug_state_injection.rs \
  crates/orkworksd/src/metadata.rs \
  crates/orkworksd/src/main.rs
rtk git commit -m "feat: add session state injection primitives"
```

### Task 2: Add Sidecar Injection Catalog, Apply Endpoint, And Convergence Hooks

**Files:**
- Modify: `crates/orkworksd/src/debug_state_injection.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Produces:
  - `#[derive(Deserialize)] pub(crate) struct ApplySessionStateInjectionRequest { #[serde(rename = "injectionId")] pub injection_id: String }`
  - `pub(crate) async fn list_session_state_injections() -> impl IntoResponse`
  - `pub(crate) async fn apply_session_state_injection(State(state): State<Arc<AppState>>, Path(id): Path<String>, Json(req): Json<ApplySessionStateInjectionRequest>) -> impl IntoResponse`
  - `pub(crate) fn inject_session_state(state: &Arc<AppState>, id: &str, injection: SessionStateInjectionId, now: &str) -> Result<SessionInfo, axum::http::StatusCode>`
  - `pub(crate) fn apply_debug_overlay_projection(info: &mut SessionInfo, live_debug: Option<&metadata::DebugInjectionMetadata>, meta: Option<&SessionMetadata>)`
  - `pub(crate) fn clear_superseded_debug_overlay(handle: &mut SessionHandle, meta: &mut SessionMetadata, effective_at_usage_limit: bool)`
- Consumes:
  - `SessionStateInjectionId`
  - `SessionMetadata.debug_injection`
  - existing `schedule_session_ending_finalization(...)`
  - existing `list_sessions(...)` capping propagation

- [ ] **Step 1: Write the failing sidecar tests**

```rust
#[tokio::test]
async fn apply_session_state_injection_rejects_unknown_injection_ids() {
    let state = test_app_state_with_workspace(tempfile::tempdir().unwrap().path());
    let response = apply_session_state_injection(
        State(state),
        Path("missing".to_string()),
        Json(ApplySessionStateInjectionRequest { injection_id: "not_real".into() }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn apply_session_state_injection_returns_not_found_for_unknown_session() {
    let state = test_app_state_with_workspace(tempfile::tempdir().unwrap().path());
    let response = apply_session_state_injection(
        State(state),
        Path("missing".to_string()),
        Json(ApplySessionStateInjectionRequest { injection_id: "running_blocked".into() }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn running_capped_overlay_does_not_cap_sibling_live_sessions_or_provider_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());

    {
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        for session_id in ["capped-target", "capped-sibling"] {
            ws.metadata.write_session(&test_session_metadata(
                session_id,
                session_id,
                dir.path(),
                "running",
                "active",
            ));
        }
    }

    for session_id in ["capped-target", "capped-sibling"] {
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        state.sessions.lock().unwrap().insert(
            session_id.to_string(),
            SessionHandle {
                info: SessionInfo {
                    id: session_id.to_string(),
                    harness_id: Some("codex".into()),
                    harness: Some("codex".into()),
                    metadata_source: Some("process".into()),
                    metadata_confidence: Some(1.0),
                    ..test_session_info(session_id.to_string(), session_id, dir.path().display().to_string(), "running", "now")
                },
                kill_tx,
                output_buffer: peon::RingBuffer::new(200),
                scan_buf: String::new(),
                command: default_shell_command(dir.path().display().to_string()),
                initial_prompt: None,
                terminal_attached: false,
                at_usage_limit_latched: false,
                capacity_check_pending: false,
                output_lines_seen: 0,
                scan_bytes_seen: 0,
                resume_scan_origin: None,
                pending_capacity_visible_once: false,
                debug_injection: None,
            },
        );
    }

    let injected = inject_session_state(
        &state,
        "capped-target",
        SessionStateInjectionId::RunningCapped,
        "now",
    )
    .unwrap();
    assert_eq!(injected.at_usage_limit, Some(true));
    assert_eq!(injected.metadata_source.as_deref(), Some("debug"));

    let response = list_sessions(State(state.clone())).await.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let target = sessions.iter().find(|s| s["id"] == "capped-target").unwrap();
    let sibling = sessions.iter().find(|s| s["id"] == "capped-sibling").unwrap();
    assert_eq!(target.get("atUsageLimit").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(sibling.get("atUsageLimit"), None);

    let providers = state.providers.get_providers_response();
    let codex = providers.providers.iter().find(|provider| provider.id == "codex").unwrap();
    assert_ne!(codex.effective_state, "capped");
}

#[tokio::test]
async fn active_fake_ending_schedules_real_finalization_and_leaves_session_ended() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let session_id = "ending-target".to_string();
    {
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            &session_id,
            "Ending Target",
            dir.path(),
            "running",
            "active",
        ));
    }
    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        SessionHandle {
            info: SessionInfo {
                lifecycle_phase: "active".into(),
                status: "running".into(),
                metadata_source: Some("process".into()),
                metadata_confidence: Some(1.0),
                ..test_session_info(session_id.clone(), "Ending Target", dir.path().display().to_string(), "running", "now")
            },
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            command: default_shell_command(dir.path().display().to_string()),
            initial_prompt: None,
            terminal_attached: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
            debug_injection: None,
        },
    );

    let injected = inject_session_state(
        &state,
        &session_id,
        SessionStateInjectionId::ActiveFakeEnding,
        "now",
    )
    .unwrap();
    assert_eq!(injected.lifecycle_phase, "ending");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let response = list_sessions(State(state.clone())).await.into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let ended = sessions.iter().find(|s| s["id"] == session_id).unwrap();
    assert_eq!(ended.get("lifecyclePhase").and_then(|v| v.as_str()), Some("ended"));
}

#[tokio::test]
async fn running_blocked_updates_live_and_persisted_session_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let session_id = "blocked-target";

    {
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            session_id,
            "Blocked Target",
            dir.path(),
            "running",
            "active",
        ));
    }

    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        session_id.to_string(),
        SessionHandle {
            info: test_session_info(session_id.to_string(), "Blocked Target", dir.path().display().to_string(), "running", "now"),
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            command: default_shell_command(dir.path().display().to_string()),
            initial_prompt: None,
            terminal_attached: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
            debug_injection: None,
        },
    );

    let injected = inject_session_state(&state, session_id, SessionStateInjectionId::RunningBlocked, "now").unwrap();
    assert_eq!(injected.observed_status.as_deref(), Some("blocked"));
    assert_eq!(injected.metadata_source.as_deref(), Some("debug"));

    let ws_guard = state.workspace.lock().unwrap();
    let ws = ws_guard.as_ref().unwrap();
    let persisted = ws.metadata.read_session(session_id).unwrap();
    assert_eq!(persisted.observed_status.as_deref(), Some("blocked"));
    assert_eq!(persisted.metadata_source, "debug");
}

#[tokio::test]
async fn running_capped_apply_response_is_projected_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let session_id = "capped-now";

    {
        let ws_guard = state.workspace.lock().unwrap();
        let ws = ws_guard.as_ref().unwrap();
        ws.metadata.write_session(&test_session_metadata(
            session_id,
            "Capped Now",
            dir.path(),
            "running",
            "active",
        ));
    }

    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        session_id.to_string(),
        SessionHandle {
            info: test_session_info(session_id.to_string(), "Capped Now", dir.path().display().to_string(), "running", "now"),
            kill_tx,
            output_buffer: peon::RingBuffer::new(200),
            scan_buf: String::new(),
            command: default_shell_command(dir.path().display().to_string()),
            initial_prompt: None,
            terminal_attached: false,
            at_usage_limit_latched: false,
            capacity_check_pending: false,
            output_lines_seen: 0,
            scan_bytes_seen: 0,
            resume_scan_origin: None,
            pending_capacity_visible_once: false,
            debug_injection: None,
        },
    );

    let injected = inject_session_state(&state, session_id, SessionStateInjectionId::RunningCapped, "now").unwrap();
    assert_eq!(injected.at_usage_limit, Some(true));
    assert_eq!(injected.usage_limit_reset_hint.as_deref(), Some("resets in 1h (debug)"));
}
```

- [ ] **Step 2: Run the focused Rust tests and verify they fail**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml apply_session_state_injection
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_capped_overlay_does_not_cap_sibling_live_sessions_or_provider_state
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml active_fake_ending_schedules_real_finalization_and_leaves_session_ended
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_blocked_updates_live_and_persisted_session_state
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_capped_apply_response_is_projected_immediately
```

Expected:

- FAIL with missing routes, missing request type, or missing overlay projection behavior

- [ ] **Step 3: Implement the catalog route, apply route, and mutation helpers**

```rust
pub(crate) async fn list_session_state_injections() -> impl IntoResponse {
    Json(SessionStateInjectionId::options())
}

pub(crate) async fn apply_session_state_injection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ApplySessionStateInjectionRequest>,
) -> impl IntoResponse {
    let Some(injection) = SessionStateInjectionId::parse(&req.injection_id) else {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    };

    let now = iso_now();
    match inject_session_state(&state, &id, injection, &now) {
        Ok(info) => Json(info).into_response(),
        Err(status) => status.into_response(),
    }
}
```

```rust
pub(crate) fn inject_session_state(
    state: &Arc<AppState>,
    id: &str,
    injection: SessionStateInjectionId,
    now: &str,
) -> Result<SessionInfo, axum::http::StatusCode> {
    let ws_guard = state.workspace.lock().unwrap();
    let ws = ws_guard.as_ref().ok_or(axum::http::StatusCode::CONFLICT)?;
    let mut meta = ws.metadata.read_session(id).ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let mut sessions = state.sessions.lock().unwrap();
    let handle = sessions.get_mut(id).ok_or(axum::http::StatusCode::NOT_FOUND)?;
    handle.info.metadata_source = Some("debug".into());
    handle.info.metadata_confidence = None;
    meta.metadata_source = "debug".into();
    meta.metadata_confidence = 0.0;

    match injection {
        SessionStateInjectionId::ActiveFakeEnding => {
            handle.info.status = "running".into();
            handle.info.lifecycle_phase = "ending".into();
            handle.info.observed_status = None;
            handle.info.terminal_outcome = None;
            handle.debug_injection = None;
            meta.status = "running".into();
            meta.lifecycle_phase = "ending".into();
            meta.observed_status = None;
            meta.pending_terminal_status = None;
            meta.final_observed_status_snapshot = None;
            meta.debug_injection = None;
        }
        SessionStateInjectionId::EndedStaleLiveAttention => {
            handle.info.status = "ended".into();
            handle.info.lifecycle_phase = "ended".into();
            handle.info.observed_status = Some("waiting_for_input".into());
            handle.debug_injection = None;
            meta.status = "ended".into();
            meta.lifecycle_phase = "ended".into();
            meta.observed_status = Some("waiting_for_input".into());
            meta.final_observed_status_snapshot = Some("idle".into());
            meta.debug_injection = None;
        }
        SessionStateInjectionId::EndedMissingFinalSnapshot => {
            handle.info.status = "ended".into();
            handle.info.lifecycle_phase = "ended".into();
            handle.info.final_observed_status = None;
            handle.debug_injection = None;
            meta.status = "ended".into();
            meta.lifecycle_phase = "ended".into();
            meta.final_observed_status_snapshot = None;
            meta.debug_injection = None;
        }
        SessionStateInjectionId::RunningBlocked => {
            handle.info.status = "running".into();
            handle.info.lifecycle_phase = "active".into();
            handle.info.observed_status = Some("blocked".into());
            handle.debug_injection = None;
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.observed_status = Some("blocked".into());
            meta.final_observed_status_snapshot = None;
            meta.debug_injection = None;
        }
        SessionStateInjectionId::RunningIdleTooEarly => {
            handle.info.status = "running".into();
            handle.info.lifecycle_phase = "active".into();
            handle.info.observed_status = Some("idle".into());
            handle.debug_injection = None;
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.observed_status = Some("idle".into());
            meta.final_observed_status_snapshot = None;
            meta.debug_injection = None;
        }
        SessionStateInjectionId::RunningCapped => {
            handle.info.status = "running".into();
            handle.info.lifecycle_phase = "active".into();
            handle.debug_injection = Some(metadata::DebugInjectionMetadata {
                attention: "capped".into(),
                usage_limit_reset_hint: Some("resets in 1h (debug)".into()),
                applied_at: now.to_string(),
            });
            meta.status = "running".into();
            meta.lifecycle_phase = "active".into();
            meta.debug_injection = Some(metadata::DebugInjectionMetadata {
                attention: "capped".into(),
                usage_limit_reset_hint: Some("resets in 1h (debug)".into()),
                applied_at: now.to_string(),
            });
        }
    }

    ws.metadata.write_session(&meta);
    let mut injected = handle.info.clone();
    apply_debug_overlay_projection(&mut injected, handle.debug_injection.as_ref(), Some(&meta));
    drop(sessions);
    drop(ws_guard);

    if matches!(injection, SessionStateInjectionId::ActiveFakeEnding) {
        schedule_session_ending_finalization(state.clone(), id.to_string(), "ended".into());
    }

    Ok(injected)
}
```

```rust
// crates/orkworksd/src/main.rs
.route("/sessions/debug-injections", get(list_session_state_injections))
.route("/sessions/:id/debug-injection", post(apply_session_state_injection))
```

- [ ] **Step 4: Apply the capped overlay only after provider propagation in `list_sessions`, and clear it once real runtime state supersedes it**

```rust
for info in &mut infos {
    let ws_guard = state.workspace.lock().unwrap();
    let ws = ws_guard.as_ref().unwrap();
    if let Some(meta) = metadata_map.get_mut(&info.id) {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(&info.id) {
            clear_superseded_debug_overlay(handle, meta, info.at_usage_limit == Some(true));
            apply_debug_overlay_projection(info, handle.debug_injection.as_ref(), Some(meta));
            ws.metadata.write_session(meta);
        }
    }
}
```

```rust
pub(crate) fn apply_debug_overlay_projection(
    info: &mut SessionInfo,
    live_debug: Option<&metadata::DebugInjectionMetadata>,
    meta: Option<&SessionMetadata>,
) {
    let debug = live_debug.or_else(|| meta.and_then(|record| record.debug_injection.as_ref()));
    let Some(debug) = debug else {
        return;
    };
    if debug.attention == "capped" {
        info.at_usage_limit = Some(true);
        info.usage_limit_reset_hint = debug.usage_limit_reset_hint.clone();
        info.metadata_source = Some("debug".into());
        info.metadata_confidence = None;
    }
}

pub(crate) fn clear_superseded_debug_overlay(
    handle: &mut SessionHandle,
    meta: &mut SessionMetadata,
    effective_at_usage_limit: bool,
) {
    let injected_capped = handle
        .debug_injection
        .as_ref()
        .is_some_and(|debug| debug.attention == "capped");
    if injected_capped && !effective_at_usage_limit {
        handle.debug_injection = None;
        meta.debug_injection = None;
    }
}
```

- [ ] **Step 5: Run the focused Rust tests and verify they pass**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml apply_session_state_injection
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_capped_overlay_does_not_cap_sibling_live_sessions_or_provider_state
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml active_fake_ending_schedules_real_finalization_and_leaves_session_ended
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_blocked_updates_live_and_persisted_session_state
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml running_capped_apply_response_is_projected_immediately
```

Expected:

- PASS

- [ ] **Step 6: Commit**

```bash
rtk git add \
  crates/orkworksd/src/debug_state_injection.rs \
  crates/orkworksd/src/http/session_handlers.rs \
  crates/orkworksd/src/main.rs
rtk git commit -m "feat: add session state injection sidecar endpoints"
```

### Task 3: Add The Electron-Gated Bridge And Renderer Contracts

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/orkworksWindow.d.ts`
- Modify: `apps/desktop/src/api.ts`
- Create: `apps/desktop/src/sessionStateInjection.ts`
- Test: `apps/desktop/tests/sessionStateInjection.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Produces:
  - `export interface SessionStateInjectionOption { id: string; label: string }`
  - `export function replaceSessionAfterInjection(sessions: SessionInfo[], injected: SessionInfo): SessionInfo[]`
  - `window.orkworks.listSessionStateInjections(): Promise<SessionStateInjectionOption[]>`
  - `window.orkworks.applySessionStateInjection(sessionId: string, injectionId: string): Promise<SessionInfo>`
- Consumes:
  - sidecar routes from Task 2
  - `currentSettings.debug.showSessionIds` in Electron main

- [ ] **Step 1: Write the failing desktop tests for the bridge and helper**

```ts
// apps/desktop/tests/sessionStateInjection.test.ts
import test from "node:test";
import assert from "node:assert/strict";
import { replaceSessionAfterInjection } from "../src/sessionStateInjection.ts";

test("replaceSessionAfterInjection swaps only the matching session and keeps list sortable", () => {
  const sessions = [
    { id: "a", status: "running", memoryState: "live" },
    { id: "b", status: "running", memoryState: "live" },
  ] as any;
  const injected = { id: "b", status: "ended", memoryState: "live" } as any;
  const next = replaceSessionAfterInjection(sessions, injected);
  assert.equal(next.find((session) => session.id === "a")?.status, "running");
  assert.equal(next.find((session) => session.id === "b")?.status, "ended");
});
```

```ts
// apps/desktop/tests/dockview.test.ts
test("Electron main gates state injection behind show debug metadata", () => {
  const source = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
  assert.match(source, /currentSettings\.debug\.showSessionIds/);
  assert.match(source, /list-session-state-injections/);
  assert.match(source, /apply-session-state-injection/);
});

test("preload exposes session state injection helpers", () => {
  const source = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  assert.match(source, /listSessionStateInjections:\s*\(\)\s*=>\s*ipcRenderer\.invoke\("list-session-state-injections"\)/);
  assert.match(source, /applySessionStateInjection:\s*\(sessionId: string, injectionId: string\)\s*=>/);
  assert.match(source, /ipcRenderer\.invoke\("apply-session-state-injection"/);
});
```

- [ ] **Step 2: Run the desktop tests and verify they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjection.test.ts tests/dockview.test.ts
```

Expected:

- FAIL with missing helper exports or missing IPC bridge names

- [ ] **Step 3: Implement the IPC bridge and shared helper**

```ts
// apps/desktop/src/sessionStateInjection.ts
import type { SessionInfo } from "./api";

export interface SessionStateInjectionOption {
  id: string;
  label: string;
}

export function replaceSessionAfterInjection(
  sessions: SessionInfo[],
  injected: SessionInfo,
): SessionInfo[] {
  return sessions.map((session) => session.id === injected.id ? injected : session);
}
```

```ts
// apps/desktop/electron/preload.ts
listSessionStateInjections: (): Promise<unknown> => ipcRenderer.invoke("list-session-state-injections"),
applySessionStateInjection: (sessionId: string, injectionId: string): Promise<unknown> =>
  ipcRenderer.invoke("apply-session-state-injection", { sessionId, injectionId }),
```

```ts
// apps/desktop/electron/main.ts
ipcMain.handle("list-session-state-injections", async () => {
  if (!(currentSettings ?? readSettings(app.getPath("userData"))).debug.showSessionIds) {
    throw new Error("debug metadata must be enabled before using state injection");
  }
  const port = await portPromise;
  const resp = await fetch(`http://127.0.0.1:${port}/sessions/debug-injections`);
  if (!resp.ok) throw new Error(`list state injections failed: ${resp.status}`);
  return resp.json();
});

ipcMain.handle("apply-session-state-injection", async (_event, payload: unknown) => {
  if (!(currentSettings ?? readSettings(app.getPath("userData"))).debug.showSessionIds) {
    throw new Error("debug metadata must be enabled before using state injection");
  }
  const { sessionId, injectionId } = payload as { sessionId: string; injectionId: string };
  const port = await portPromise;
  const resp = await fetch(`http://127.0.0.1:${port}/sessions/${sessionId}/debug-injection`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ injectionId }),
  });
  if (!resp.ok) throw new Error(`apply state injection failed: ${resp.status}`);
  return resp.json();
});
```

- [ ] **Step 4: Run the desktop tests and verify they pass**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjection.test.ts tests/dockview.test.ts
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
rtk git add \
  apps/desktop/electron/main.ts \
  apps/desktop/electron/preload.ts \
  apps/desktop/src/api.ts \
  apps/desktop/src/orkworksWindow.d.ts \
  apps/desktop/src/sessionStateInjection.ts \
  apps/desktop/tests/sessionStateInjection.test.ts \
  apps/desktop/tests/dockview.test.ts
rtk git commit -m "feat: add state injection desktop bridge"
```

### Task 4: Add The Detail-Panel UI And Immediate Snapshot Update

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/sessionStateInjection.test.ts`

**Interfaces:**
- Produces:
  - `onApplyStateInjection: (id: string, injectionId: string) => Promise<void>`
  - `stateInjectionOptions: SessionStateInjectionOption[]`
- Consumes:
  - `window.orkworks.listSessionStateInjections()`
  - `window.orkworks.applySessionStateInjection(...)`
  - `replaceSessionAfterInjection(...)`

- [ ] **Step 1: Write the failing UI tests**

```ts
test("SessionDetailPanel renders a State injection control only in debug mode", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  assert.match(source, /State injection/);
  assert.match(source, /showDebugMetadata/);
  assert.match(source, /selectedInjectionId/);
  assert.match(source, /Apply injection/);
});

test("App applies the returned injected snapshot immediately", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(source, /applySessionStateInjection/);
  assert.match(source, /replaceSessionAfterInjection/);
  assert.match(source, /Applied state injection/);
  assert.match(source, /Couldn\'t apply state injection/);
});
```

- [ ] **Step 2: Run the desktop tests and verify they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjection.test.ts tests/dockview.test.ts
```

Expected:

- FAIL with missing detail-panel controls or missing immediate session replacement logic

- [ ] **Step 3: Implement the App callback, prop plumbing, and panel controls**

```ts
// apps/desktop/src/App.tsx
const [stateInjectionOptions, setStateInjectionOptions] = useState<SessionStateInjectionOption[]>([]);

useEffect(() => {
  if (backendStatus !== "connected" || !settings?.debug.showSessionIds || stateInjectionOptions.length > 0) return;
  window.orkworks.listSessionStateInjections()
    .then(setStateInjectionOptions)
    .catch(() => pushToast("error", "Couldn't load state injection options."));
}, [backendStatus, settings?.debug.showSessionIds, stateInjectionOptions.length]);

const handleApplyStateInjection = useCallback(async (id: string, injectionId: string) => {
  try {
    const injected = await window.orkworks.applySessionStateInjection(id, injectionId);
    setSessions((prev) => sortSessions(replaceSessionAfterInjection(prev, injected)));
    pushToast("info", `Applied state injection: ${injectionId}`);
  } catch {
    pushToast("error", "Couldn't apply state injection.");
  }
}, []);
```

```tsx
// apps/desktop/src/components/DockviewApp.tsx
<SessionDetailPanel
  sessions={ctx.sessions}
  activeSessionId={ctx.activeSessionId}
  onResumeSession={ctx.onResumeSession}
  onApplyStateInjection={ctx.onApplyStateInjection}
  stateInjectionOptions={ctx.stateInjectionOptions}
  showDebugMetadata={ctx.debugSettings.showSessionIds}
/>
```

```tsx
// apps/desktop/src/components/SessionDetailPanel.tsx
const [selectedInjectionId, setSelectedInjectionId] = useState("");
const [applyingInjection, setApplyingInjection] = useState(false);

{showDebugMetadata && stateInjectionOptions.length > 0 && (
  <div className="detail-debug-injection">
    <div className="session-detail-label">State injection</div>
    <select value={selectedInjectionId} onChange={(event) => setSelectedInjectionId(event.target.value)}>
      <option value="">Choose a test scenario…</option>
      {stateInjectionOptions.map((option) => (
        <option key={option.id} value={option.id}>{option.label}</option>
      ))}
    </select>
    <button
      type="button"
      disabled={!selectedInjectionId || applyingInjection}
      onClick={async () => {
        setApplyingInjection(true);
        try {
          await onApplyStateInjection(active.id, selectedInjectionId);
          setSelectedInjectionId("");
        } finally {
          setApplyingInjection(false);
        }
      }}
    >
      Apply injection
    </button>
    <div className="detail-debug-note">
      Temporarily writes a debug state, then lets normal runtime and metadata logic overwrite it naturally.
    </div>
  </div>
)}
```

- [ ] **Step 4: Run the desktop tests and verify they pass**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/sessionStateInjection.test.ts tests/dockview.test.ts
cd apps/desktop && pnpm exec tsc --noEmit
```

Expected:

- PASS
- Type-check clean

- [ ] **Step 5: Commit**

```bash
rtk git add \
  apps/desktop/src/App.tsx \
  apps/desktop/src/App.css \
  apps/desktop/src/components/DockviewApp.tsx \
  apps/desktop/src/components/SessionDetailPanel.tsx \
  apps/desktop/tests/dockview.test.ts \
  apps/desktop/tests/sessionStateInjection.test.ts
rtk git commit -m "feat: add state injection detail panel controls"
```

### Task 5: Update Docs, Sync Board Metadata, And Verify End-To-End

**Files:**
- Modify: `specs/orkworks-mvp.md`
- Modify: `docs/adr/0005-metadata-source-priority.md`
- Modify: `AGENTS.md`
- Modify: `docs/superpowers/specs/2026-07-06-session-state-injection-design.md`

**Interfaces:**
- Consumes:
  - `metadataSource = "debug"`
  - persisted `debugInjection` overlay vocabulary
- Produces:
  - updated docs and spec references that match the implemented behavior

- [ ] **Step 1: Update the metadata-source vocabulary docs**

```md
<!-- specs/orkworks-mvp.md -->
Valid metadata sources:
- `user`
- `agent`
- `peon`
- `backend_inference`
- `process`
- `unknown`
- `debug` (debug-only temporary state injection; lower priority than normal runtime sources)
```

```md
<!-- docs/adr/0005-metadata-source-priority.md -->
Metadata priority is explicit and ordered:
user > agent > peon > backend_inference > process > unknown > debug

`debug` exists only for local, temporary state-injection testing and is always
eligible to be overwritten by later non-debug runtime writes.
```

- [ ] **Step 2: Run the full verification suite**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
cd apps/desktop && pnpm exec tsc --noEmit
bash .claude/hooks/doc-check.sh
```

Expected:

- all Rust tests PASS
- all desktop tests PASS
- type-check PASS
- doc-check prints no required follow-up files

- [ ] **Step 3: Run the required review gate and summarize any follow-up**

```bash
# use the repo's required /code-review process at medium effort or higher
```

- [ ] **Step 4: Commit**

```bash
rtk git add \
  specs/orkworks-mvp.md \
  docs/adr/0005-metadata-source-priority.md \
  AGENTS.md \
  docs/superpowers/specs/2026-07-06-session-state-injection-design.md
rtk git commit -m "docs: record debug state injection metadata source"
```
