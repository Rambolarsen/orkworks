# Harness Session ID Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic harness-native session ID capture contract, wire reliable first sources for OpenCode, Claude Code, and Codex, and add a repo skill that guides future harness additions.

**Architecture:** The sidecar owns the normalized metadata write through `POST /sessions/:id/harness-session`. Harness-specific capture remains thin and source-specific: OpenCode reports `OPENCODE_SESSION_ID` through a shell reporter path, Claude extends its explicit hook command, and Codex exec captures `thread.started.thread_id` from JSONL. Peon remains a fallback writer and cannot overwrite higher-confidence capture.

**Tech Stack:** Rust/Axum/serde for the sidecar endpoint and metadata, shell snippets for harness reporters, Node built-in test runner only if frontend/API types are touched, repo skills under `skills/`, Markdown docs.

---

## Execution Setup

This plan touches code under `crates/orkworksd/`, so execute it on a branch or worktree per `skills/starting-work/SKILL.md`.

Recommended branch: `harness-session-id-capture`

Recommended setup:

```bash
git worktree add ../orkworks-harness-session-id-capture -b harness-session-id-capture
cd ../orkworks-harness-session-id-capture
cd apps/desktop && pnpm install
```

Use the worktree as the project root for all commands below. If executing in an already-isolated checkout on your own branch, use that checkout and skip worktree creation.

## File Structure

- Modify `crates/orkworksd/src/metadata.rs`
  - Add capture metadata fields to `SessionMetadata`.
  - Add validation and confidence-aware merge helpers.
  - Update Peon inference to use the merge helper with low-confidence source.
  - Add unit tests for merge and overwrite rules.
- Modify `crates/orkworksd/src/main.rs`
  - Add request DTO and route handler for `POST /sessions/:id/harness-session`.
  - Add the route to the Axum router.
  - Add dynamic `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` env injection for spawned PTYs.
  - Add Codex JSONL parser helper tests.
- Create `crates/orkworksd/scripts/report-opencode-session.sh`
  - A small guarded reporter that posts `$OPENCODE_SESSION_ID` to the generic endpoint.
- Create `crates/orkworksd/scripts/report-claude-session-from-hook.sh`
  - A guarded hook helper that reads Claude hook JSON from stdin and reports `session_id`.
- Create `skills/adding-harness/SKILL.md`
  - Repo skill checklist for adding a new harness.
- Modify `AGENTS.md`
  - Mention the new `adding-harness` skill under repo-level skills.
- Modify `docs/agents/architecture.md`
  - Add the new endpoint and capture flow to the API/sidecar docs.

## Task 1: Metadata Capture Fields And Merge Helper

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Write failing metadata tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `crates/orkworksd/src/metadata.rs`:

```rust
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
```

- [ ] **Step 2: Run metadata tests to verify they fail**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests::harness_session_report_writes_resume_memory_and_capture_metadata metadata::tests::lower_confidence_harness_session_report_does_not_overwrite metadata::tests::equal_confidence_harness_session_report_can_refresh_same_value
```

Expected: FAIL because `HarnessSessionReport`, `HarnessSessionMergeResult`, and metadata capture fields do not exist.

- [ ] **Step 3: Add metadata fields and helper types**

In `crates/orkworksd/src/metadata.rs`, add these fields to `SessionMetadata` after `resume`:

```rust
    #[serde(rename = "harnessSessionIdSource", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_source: Option<String>,
    #[serde(rename = "harnessSessionIdConfidence", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_confidence: Option<f64>,
    #[serde(rename = "harnessSessionIdCapturedAt", skip_serializing_if = "Option::is_none")]
    pub harness_session_id_captured_at: Option<String>,
```

Add these public helper types near `WorkspaceMemory`:

```rust
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
```

Update every `SessionMetadata { ... }` initializer in `crates/orkworksd/src/metadata.rs` and `crates/orkworksd/src/main.rs` to include:

```rust
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
```

- [ ] **Step 4: Implement the merge helper**

Add this method inside `impl MetadataStore`:

```rust
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
```

- [ ] **Step 5: Run metadata tests to verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests::harness_session_report_writes_resume_memory_and_capture_metadata metadata::tests::lower_confidence_harness_session_report_does_not_overwrite metadata::tests::equal_confidence_harness_session_report_can_refresh_same_value
```

Expected: PASS.

- [ ] **Step 6: Commit metadata helper**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/main.rs
git commit -m "feat: add harness session id metadata merge"
```

## Task 2: Generic Harness Session Endpoint

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write focused handler tests**

Add these tests in the existing `#[cfg(test)] mod tests` in `crates/orkworksd/src/main.rs`:

```rust
#[tokio::test]
async fn harness_session_report_rejects_invalid_native_id() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let response = report_harness_session(
        State(state),
        Path("missing".into()),
        Json(HarnessSessionReportRequest {
            harness_session_id: "bad id".into(),
            source: "test".into(),
            confidence: 0.9,
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn harness_session_report_returns_not_found_for_unknown_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    let response = report_harness_session(
        State(state),
        Path("missing".into()),
        Json(HarnessSessionReportRequest {
            harness_session_id: "native-123".into(),
            source: "test".into(),
            confidence: 0.9,
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn harness_session_report_writes_metadata_for_known_session() {
    let dir = tempfile::tempdir().unwrap();
    let state = test_app_state_with_workspace(dir.path());
    {
        let ws = state.workspace.lock().unwrap();
        ws.as_ref().unwrap().metadata.write_session(&metadata::SessionMetadata {
            id: "known".into(),
            label: "Known".into(),
            workspace: dir.path().display().to_string(),
            task: "".into(),
            harness: "opencode".into(),
            model: "".into(),
            cwd: dir.path().display().to_string(),
            status: "running".into(),
            phase: "".into(),
            observed_status: None,
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
            harness_session_id_source: None,
            harness_session_id_confidence: None,
            harness_session_id_captured_at: None,
            resumed_from: None,
            last_user_input: None,
        });
    }

    let response = report_harness_session(
        State(state.clone()),
        Path("known".into()),
        Json(HarnessSessionReportRequest {
            harness_session_id: "native-123".into(),
            source: "opencode_env".into(),
            confidence: 0.98,
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let ws = state.workspace.lock().unwrap();
    let updated = ws.as_ref().unwrap().metadata.read_session("known").unwrap();
    assert_eq!(
        updated.resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
        Some("native-123"),
    );
}
```

Add this helper in the test module:

```rust
fn test_app_state_with_workspace(path: &std::path::Path) -> Arc<AppState> {
    let metadata_root = path.join(".orkworks-test");
    Arc::new(AppState {
        session_module: SessionModule::new(),
        sessions: Mutex::new(HashMap::new()),
        workspace: Mutex::new(Some(WorkspaceState {
            path: path.to_path_buf(),
            metadata: metadata::MetadataStore::new(&metadata_root),
            watcher: watcher::MetadataWatcher::start(&metadata_root.join("sessions")),
        })),
        peon: PeonState {
            last_output: StdRwLock::new(HashMap::new()),
            last_inference: StdRwLock::new(HashMap::new()),
            in_flight: StdRwLock::new(HashSet::new()),
            label_hint: StdRwLock::new(HashMap::new()),
            label_pending: StdRwLock::new(HashSet::new()),
            config: peon::PeonConfig::from_env(),
        },
        providers: providers::ProviderManager::new(),
        adapters: builtin_adapters(),
        retention_config: tokio::sync::RwLock::new(RetentionConfig::default()),
        harnesses: tokio::sync::RwLock::new(vec![]),
    })
}
```

- [ ] **Step 2: Run handler tests to verify they fail**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness_session_report_
```

Expected: FAIL because `report_harness_session`, `HarnessSessionReportRequest`, and the route do not exist.

- [ ] **Step 3: Add request DTO and handler**

Add near the other request DTOs in `crates/orkworksd/src/main.rs`:

```rust
#[derive(Deserialize)]
struct HarnessSessionReportRequest {
    #[serde(rename = "harnessSessionId")]
    harness_session_id: String,
    source: String,
    confidence: f64,
}
```

Add the route:

```rust
        .route("/sessions/:id/harness-session", post(report_harness_session))
```

Place it next to the other `/sessions/:id/...` routes.

Add the handler near `resume_session`:

```rust
async fn report_harness_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<HarnessSessionReportRequest>,
) -> impl IntoResponse {
    let report = metadata::HarnessSessionReport {
        harness_session_id: req.harness_session_id,
        source: req.source,
        confidence: req.confidence,
    };
    if !metadata::valid_harness_session_report(&report) {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    let Some(ref ws) = *ws_guard else {
        return axum::http::StatusCode::CONFLICT.into_response();
    };

    match ws.metadata.merge_harness_session_report(&id, &report, &now) {
        metadata::HarnessSessionMergeResult::Accepted
        | metadata::HarnessSessionMergeResult::IgnoredLowerConfidence => {
            axum::http::StatusCode::OK.into_response()
        }
        metadata::HarnessSessionMergeResult::NotFound => {
            axum::http::StatusCode::NOT_FOUND.into_response()
        }
        metadata::HarnessSessionMergeResult::Invalid => {
            axum::http::StatusCode::BAD_REQUEST.into_response()
        }
    }
}
```

- [ ] **Step 4: Run handler tests to verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml harness_session_report_
```

Expected: PASS.

- [ ] **Step 5: Commit endpoint**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: add harness session id report endpoint"
```

## Task 3: Peon Fallback Must Respect Capture Confidence

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Write failing Peon overwrite test**

Add this test in `crates/orkworksd/src/metadata.rs`:

```rust
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
    store.merge_peon_inference("peon-confidence-test", &inf, "2026-06-26T12:00:00Z", None);

    let updated = store.read_session("peon-confidence-test").unwrap();
    assert_eq!(
        updated.resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
        Some("native-high"),
    );
    assert_eq!(updated.harness_session_id_source.as_deref(), Some("opencode_env"));
}
```

- [ ] **Step 2: Run the new test to verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests::peon_inference_does_not_overwrite_higher_confidence_harness_session_id
```

Expected: FAIL because `merge_peon_inference` still writes `resume.harnessSessionId` directly.

- [ ] **Step 3: Change Peon merge to call the shared helper**

Replace the direct `if let Some(ref sid) = inf.harness_session_id { ... }` block in `merge_peon_inference` with:

```rust
        let peon_harness_session_report = inf.harness_session_id.as_ref().map(|sid| HarnessSessionReport {
            harness_session_id: sid.clone(),
            source: "peon".into(),
            confidence: inf.confidence.min(0.50),
        });
```

After the normal metadata write and event append, add:

```rust
        if let Some(report) = peon_harness_session_report {
            let _ = self.merge_harness_session_report(id, &report, timestamp);
        }
```

Keep Peon's existing observer metadata write intact. The helper will reject invalid IDs and ignore lower-confidence conflicts.

- [ ] **Step 4: Run Peon metadata tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml metadata::tests::peon_inference_writes_harness_session_id_to_resume_memory metadata::tests::peon_inference_does_not_overwrite_higher_confidence_harness_session_id metadata::tests::peon_inference_rejects_invalid_harness_session_id
```

Expected: PASS.

- [ ] **Step 5: Commit Peon fallback change**

```bash
git add crates/orkworksd/src/metadata.rs
git commit -m "fix: keep peon below deterministic session id capture"
```

## Task 4: PTY Environment Injection

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write env helper tests**

Add these tests in `crates/orkworksd/src/main.rs`:

```rust
#[test]
fn session_env_overrides_include_orkworks_session_and_port() {
    let overrides = session_env_overrides("session-123", Some(5173));
    assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
    assert!(overrides.contains(&("ORKWORKS_PORT".into(), "5173".into())));
}

#[test]
fn session_env_overrides_omit_port_when_unknown() {
    let overrides = session_env_overrides("session-123", None);
    assert!(overrides.contains(&("ORKWORKS_SESSION_ID".into(), "session-123".into())));
    assert!(!overrides.iter().any(|(key, _)| key == "ORKWORKS_PORT"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml session_env_overrides_
```

Expected: FAIL because `session_env_overrides` does not exist.

- [ ] **Step 3: Add port storage to AppState**

Add this import:

```rust
use std::sync::atomic::{AtomicU16, Ordering};
```

Add a field to `AppState`:

```rust
    bound_port: AtomicU16,
```

Initialize it in every `AppState { ... }`:

```rust
        bound_port: AtomicU16::new(0),
```

After `let bound_addr = listener.local_addr().unwrap();` in `main()`, set:

```rust
    state.bound_port.store(bound_addr.port(), Ordering::Relaxed);
```

- [ ] **Step 4: Add session env override helper and use it**

Change `terminal_env_overrides()` to return owned pairs:

```rust
fn terminal_env_overrides() -> Vec<(String, String)> {
    vec![
        ("TERM".into(), "xterm-256color".into()),
        ("COLORTERM".into(), "truecolor".into()),
        ("FORCE_COLOR".into(), "1".into()),
        ("CLICOLOR".into(), "1".into()),
        ("TERM_PROGRAM".into(), "OrkWorks".into()),
    ]
}
```

Add:

```rust
fn session_env_overrides(session_id: &str, port: Option<u16>) -> Vec<(String, String)> {
    let mut env = vec![("ORKWORKS_SESSION_ID".into(), session_id.to_string())];
    if let Some(port) = port {
        env.push(("ORKWORKS_PORT".into(), port.to_string()));
    }
    env
}
```

In `handle_session_terminal`, after applying `terminal_env_overrides()`, add:

```rust
    let port = match state.bound_port.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    for (key, value) in session_env_overrides(&id, port) {
        cmd.env(&key, &value);
    }
```

- [ ] **Step 5: Run env tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml session_env_overrides_ terminal_env_overrides_
```

Expected: PASS. Update existing `terminal_env_overrides_` tests to use `.iter().any(...)` instead of positional indexing after `terminal_env_overrides()` changes to return a `Vec`.

- [ ] **Step 6: Commit env injection**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: inject OrkWorks session env into PTYs"
```

## Task 5: OpenCode Reporter Script

**Files:**
- Create: `crates/orkworksd/scripts/report-opencode-session.sh`
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Add script existence test**

Add this Rust test:

```rust
#[test]
fn opencode_reporter_script_posts_native_session_env() {
    let script = include_str!("../scripts/report-opencode-session.sh");
    assert!(script.contains("OPENCODE_SESSION_ID"));
    assert!(script.contains("ORKWORKS_SESSION_ID"));
    assert!(script.contains("ORKWORKS_PORT"));
    assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
    assert!(script.contains("\"source\":\"opencode_env\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml opencode_reporter_script_posts_native_session_env
```

Expected: FAIL because the script file does not exist.

- [ ] **Step 3: Create the reporter script**

Create `crates/orkworksd/scripts/report-opencode-session.sh`:

```bash
#!/usr/bin/env bash
set -u

if [ -z "${ORKWORKS_SESSION_ID:-}" ] || [ -z "${ORKWORKS_PORT:-}" ] || [ -z "${OPENCODE_SESSION_ID:-}" ]; then
  exit 0
fi

payload=$(printf '{"harnessSessionId":"%s","source":"opencode_env","confidence":0.98}' "$OPENCODE_SESSION_ID")

curl -sS -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
  -H "Content-Type: application/json" \
  -d "$payload" >/dev/null || exit 0
```

Make it executable:

```bash
chmod +x crates/orkworksd/scripts/report-opencode-session.sh
```

- [ ] **Step 4: Run the script test**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml opencode_reporter_script_posts_native_session_env
```

Expected: PASS.

- [ ] **Step 5: Commit OpenCode reporter**

```bash
git add crates/orkworksd/scripts/report-opencode-session.sh crates/orkworksd/src/main.rs
git commit -m "feat: add opencode session id reporter"
```

## Task 6: Codex JSONL Session ID Parser

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Write parser tests**

Add:

```rust
fn codex_thread_id_from_jsonl_line(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(|v| v.as_str()) != Some("thread.started") {
        return None;
    }
    value.get("thread_id").and_then(|v| v.as_str()).map(str::to_string)
}

#[test]
fn codex_jsonl_parser_extracts_thread_started_id() {
    let line = r#"{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}"#;
    assert_eq!(
        codex_thread_id_from_jsonl_line(line).as_deref(),
        Some("0199a213-81c0-7800-8aa1-bbab2a035a53"),
    );
}

#[test]
fn codex_jsonl_parser_ignores_other_events() {
    let line = r#"{"type":"turn.started"}"#;
    assert_eq!(codex_thread_id_from_jsonl_line(line), None);
}
```

Place `codex_thread_id_from_jsonl_line` outside the test module so an exec-mode launcher can call the same parser.

- [ ] **Step 2: Run parser tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml codex_jsonl_parser_
```

Expected: PASS. The purpose of this task is to pin the documented Codex JSONL shape before wiring an exec-mode launcher.

- [ ] **Step 3: Commit Codex parser**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "test: pin codex exec session id jsonl parser"
```

## Task 7: Claude Hook Reporter Script

**Files:**
- Create: `crates/orkworksd/scripts/report-claude-session-from-hook.sh`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `docs/superpowers/specs/2026-06-25-attention-signal-claude-code-hook-design.md`

- [ ] **Step 1: Add script source test**

Add this Rust test:

```rust
#[test]
fn claude_hook_reporter_extracts_session_id_and_posts() {
    let script = include_str!("../scripts/report-claude-session-from-hook.sh");
    assert!(script.contains("session_id"));
    assert!(script.contains("ORKWORKS_SESSION_ID"));
    assert!(script.contains("ORKWORKS_PORT"));
    assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
    assert!(script.contains("\"source\":\"claude_hook\""));
    assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/attention"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml claude_hook_reporter_extracts_session_id_and_posts
```

Expected: FAIL because the script file does not exist.

- [ ] **Step 3: Create Claude hook reporter script**

Create `crates/orkworksd/scripts/report-claude-session-from-hook.sh`:

```bash
#!/usr/bin/env bash
set -u

payload="$(cat || true)"
claude_session_id="$(
  printf '%s' "$payload" |
    python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("session_id",""))' 2>/dev/null ||
    true
)"

if [ -n "$ORKWORKS_SESSION_ID" ] && [ -n "$ORKWORKS_PORT" ] && [ -n "$claude_session_id" ]; then
  curl -sS -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
    -H "Content-Type: application/json" \
    -d "{\"harnessSessionId\":\"$claude_session_id\",\"source\":\"claude_hook\",\"confidence\":0.98}" >/dev/null || true
fi

if [ -n "$ORKWORKS_SESSION_ID" ] && [ -n "$ORKWORKS_PORT" ]; then
  curl -sS -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d '{"status":"waiting_for_input"}' >/dev/null || true
fi
```

Make it executable:

```bash
chmod +x crates/orkworksd/scripts/report-claude-session-from-hook.sh
```

- [ ] **Step 4: Run script source test**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml claude_hook_reporter_extracts_session_id_and_posts
```

Expected: PASS.

- [ ] **Step 5: Update prior Claude hook design spec**

Patch `docs/superpowers/specs/2026-06-25-attention-signal-claude-code-hook-design.md` so its installed command section mentions both:

- `POST /sessions/:id/harness-session` with `session_id`
- `POST /sessions/:id/attention` with `waiting_for_input`

Run:

```bash
rg -n "harness-session|claude_hook|session_id" docs/superpowers/specs/2026-06-25-attention-signal-claude-code-hook-design.md
```

Expected: all three terms are present.

- [ ] **Step 6: Commit Claude hook reporter**

```bash
git add crates/orkworksd/scripts/report-claude-session-from-hook.sh crates/orkworksd/src/main.rs docs/superpowers/specs/2026-06-25-attention-signal-claude-code-hook-design.md
git commit -m "feat: add claude hook session id reporter"
```

## Task 8: New Harness Checklist Skill

**Files:**
- Create: `skills/adding-harness/SKILL.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Create the repo skill**

Create `skills/adding-harness/SKILL.md`:

```markdown
---
name: adding-harness
description: Use before adding or changing an OrkWorks harness adapter so launch, resume, native session ID capture, hooks, status probes, voice, capacity, tests, and docs are reviewed consistently.
---

# Adding Harnesses

Use this skill before adding a new harness or changing an existing harness adapter.

## Required Checks

1. Confirm the harness is covered by an authoritative OrkWorks spec or create/update the spec first.
2. Record the launch command, required working directory behavior, model argument syntax, and whether OrkWorks must preserve the selected model string exactly.
3. Verify exact resume support from primary documentation or a local CLI help command. Record the command shape.
4. Verify latest-session fallback semantics. If undocumented, do not invent fallback behavior.
5. Identify native session ID capture sources in reliability order:
   - environment variable
   - hook JSON payload
   - structured JSONL event
   - documented status command
   - deterministic output parser
   - manual entry
   - Peon inference
6. Mark any capture path that types into the harness session or writes harness config as user-approved only.
7. Record provider/model detection behavior and whether Peon is allowed to infer missing fields.
8. Record native voice support. Voice must remain pass-through unless a spec explicitly says otherwise.
9. Record capacity/context/status signals the harness exposes and whether they are documented enough to parse.
10. Add or update tests for launch command rendering, resume strategy selection, session ID capture, and remembered-session UI state.
11. Update `docs/agents/architecture.md`, relevant specs, and ADRs if the adapter adds routes, metadata fields, protocol changes, or new boundaries.

## Output

Before implementation, write a short harness adapter note in the relevant spec or plan with:

- harness ID
- adapter ID
- launch command
- exact resume command
- latest fallback behavior
- native session ID capture source
- confidence/source string for capture
- user-approval requirements
- test files to update
```

- [ ] **Step 2: Mention the skill in AGENTS.md**

Add a sentence in the “Repo-level skills” section:

```markdown
Use `skills/adding-harness/` before adding or changing a harness adapter; it forces the launch/resume/session-ID/voice/capacity checklist for the harness.
```

- [ ] **Step 3: Verify skill docs**

Run:

```bash
test -f skills/adding-harness/SKILL.md
rg -n "adding-harness|native session ID|exact resume" AGENTS.md skills/adding-harness/SKILL.md
```

Expected: both commands exit 0.

- [ ] **Step 4: Commit skill**

```bash
git add skills/adding-harness/SKILL.md AGENTS.md
git commit -m "docs: add harness adapter checklist skill"
```

## Task 9: Architecture Docs And API Types

**Files:**
- Modify: `docs/agents/architecture.md`

- [ ] **Step 1: Update architecture docs**

In `docs/agents/architecture.md`, update the endpoint list to include:

```markdown
`POST /sessions/:id/harness-session`
```

Add a short paragraph under the sidecar/API section:

```markdown
Harness-native session IDs are reported through `POST /sessions/:id/harness-session`, which writes `resume.harnessSessionId` plus source/confidence/captured-at metadata. Deterministic harness sources such as OpenCode env, Claude hook JSON, and Codex exec JSONL outrank Peon inference; interactive status probes remain user-triggered.
```

- [ ] **Step 2: Leave frontend API types unchanged**

Do not modify `apps/desktop/src/api.ts` in this slice. The UI already receives `resume.harnessSessionId` through `ResumeMemory`; source/confidence rendering is outside this implementation plan.

- [ ] **Step 3: Run docs/type tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/api.test.ts
```

Expected: PASS.

- [ ] **Step 4: Commit architecture docs update**

```bash
git add docs/agents/architecture.md
git commit -m "docs: document harness session id endpoint"
```

## Task 10: Full Verification And Doc Check

**Files:**
- No planned edits unless verification surfaces failures.

- [ ] **Step 1: Run Rust tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS.

- [ ] **Step 2: Run frontend tests**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: PASS.

- [ ] **Step 3: Run TypeScript check**

Run:

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: PASS.

- [ ] **Step 4: Run doc currency hook**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: no required documentation updates remain. If it flags docs, update the named docs and rerun this command.

- [ ] **Step 5: Inspect final diff**

Run:

```bash
git status --short
git log --oneline --max-count=8
```

Expected: only intentional committed changes on the feature branch, no unrelated dirty files.
