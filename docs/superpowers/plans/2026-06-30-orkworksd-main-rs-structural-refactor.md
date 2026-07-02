# orkworksd main.rs Structural Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `crates/orkworksd/src/main.rs` into focused modules while preserving behavior, public API shape, and current test outcomes.

**Architecture:** Add characterization coverage first, then extract neutral shared modules before moving HTTP handlers and runtime loops. Keep `AppState`, `WorkspaceState`, `SessionHandle`, router construction, startup, and task spawning bootstrap in `main.rs` as an intentional intermediate state; move leaf logic mechanically without introducing new behavior or new Peon/lifecycle abstractions, and do not add new behavior to `main.rs` during the extraction.

**Tech Stack:** Rust (`axum`, `tokio`, `serde`, `portable-pty`), existing Rust tests, repo doc-check hook

---

## File Structure

### Bootstrap and shared state

- Modify: `crates/orkworksd/src/main.rs`
  - Keep module declarations
  - Keep `AppState`, `WorkspaceState`, `SessionHandle`
  - Keep startup/router/task spawn wiring
  - Remove extracted helpers and handlers as follow-up tasks land

### Neutral shared modules

- Create: `crates/orkworksd/src/session_types.rs`
  - Move `SessionInfo`
  - Move `MemoryState`
- Create: `crates/orkworksd/src/harness_registry.rs`
  - Move `HarnessConfig`
  - Move `HarnessVoiceCapabilities`
  - Move builtin harness loading/saving/default capability logic
  - Move adapter/capability lookup helpers
- Create: `crates/orkworksd/src/session_view.rs`
  - Move session presentation helpers only
- Create: `crates/orkworksd/src/workspace_runtime.rs`
  - Move `iso_now`
  - Move `workspace_hash`
  - Move `orksworks_global_dir`

### HTTP modules

- Create: `crates/orkworksd/src/http/session_handlers.rs`
- Create: `crates/orkworksd/src/http/provider_handlers.rs`
- Create: `crates/orkworksd/src/http/harness_handlers.rs`
- Create: `crates/orkworksd/src/http/retention_handlers.rs`
- Create: `crates/orkworksd/src/http/mod.rs`

### Runtime modules

- Create: `crates/orkworksd/src/runtime/terminal_http.rs`
  - Move terminal HTTP/WebSocket endpoint glue
- Create: `crates/orkworksd/src/runtime/terminal_runtime.rs`
  - Move terminal PTY/env/status/runtime helpers
- Create: `crates/orkworksd/src/runtime/peon_runtime.rs`
  - Move existing Peon loop logic mechanically
- Create: `crates/orkworksd/src/runtime/retention.rs`
  - Move `retention_cleanup_task`
- Create: `crates/orkworksd/src/runtime/mod.rs`

### Documentation

- Modify: `docs/agents/architecture.md`
- Run: `.claude/hooks/doc-check.sh`

---

### Task 1: Add characterization coverage for route/status and retention behavior

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Add failing status-code characterization tests for existing handlers**

Add targeted tests in `crates/orkworksd/src/main.rs` covering:

```rust
#[tokio::test]
async fn get_provider_models_returns_not_found_for_unknown_provider() { /* ... */ }

#[tokio::test]
async fn forget_session_rejects_live_session_with_conflict() { /* ... */ }

#[tokio::test]
async fn delete_builtin_harness_returns_conflict() { /* ... */ }

#[tokio::test]
async fn session_routes_remain_registered_with_current_methods_and_paths() { /* ... */ }
```

Each test should assert the exact current HTTP status code and avoid changing implementation.
Use `tempfile::tempdir()` and explicit per-test setup for any filesystem-backed state.

- [ ] **Step 2: Run the targeted characterization tests to verify the current behavior**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml get_provider_models_returns_not_found_for_unknown_provider -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml forget_session_rejects_live_session_with_conflict -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_builtin_harness_returns_conflict -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_routes_remain_registered_with_current_methods_and_paths -- --exact
```

Expected: PASS if the behavior already exists; if any test fails, update the assertion to match the real current behavior before proceeding with refactors.

- [ ] **Step 3: Add failing retention cleanup characterization tests**

Add tests in `crates/orkworksd/src/main.rs` for:

```rust
#[tokio::test]
async fn retention_cleanup_keeps_live_sessions() { /* ... */ }

#[tokio::test]
async fn retention_cleanup_clears_last_active_when_session_is_deleted() { /* ... */ }
```

Each test should set up metadata and verify only current cleanup semantics.
Use temp directories and deterministic timestamps so the tests do not depend on developer-machine state.
Drive cleanup through a deterministic single-pass seam or existing non-loop invocation path; do not rely on sleeping against the background loop in these tests.

- [ ] **Step 4: Run the targeted retention tests and confirm the baseline**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml retention_cleanup_keeps_live_sessions -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml retention_cleanup_clears_last_active_when_session_is_deleted -- --exact
```

Expected: PASS after assertions reflect the true current behavior.

- [ ] **Step 5: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "test(orkworksd): add main.rs refactor characterization coverage"
```

---

### Task 2: Add characterization coverage for harness registry and terminal output behavior

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Add failing harness registry tests**

Add tests covering builtins/custom merge and persistence behavior:

```rust
#[test]
fn load_harnesses_merges_disk_overrides_with_builtins() { /* ... */ }

#[test]
fn load_harnesses_appends_custom_harnesses_after_builtins() { /* ... */ }
```

Scope the global harnesses path or home-equivalent explicitly per test so the cases cannot read or mutate developer-machine state.

- [ ] **Step 2: Add failing terminal output endpoint coverage**

Add tests covering persisted terminal output lookup independent of live session state:

```rust
#[tokio::test]
async fn get_terminal_output_reads_persisted_terminal_history_for_dead_session() { /* ... */ }
```

Use temp directories and explicit environment scoping for harness/global path behavior.

- [ ] **Step 3: Run the targeted tests to lock the current behavior**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml load_harnesses_merges_disk_overrides_with_builtins -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml load_harnesses_appends_custom_harnesses_after_builtins -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml get_terminal_output_reads_persisted_terminal_history_for_dead_session -- --exact
```

Expected: PASS after the tests describe the actual baseline.

- [ ] **Step 4: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "test(orkworksd): cover harness registry and terminal output baseline"
```

---

### Task 3: Extract neutral session and workspace helper modules

**Files:**
- Create: `crates/orkworksd/src/session_types.rs`
- Create: `crates/orkworksd/src/session_view.rs`
- Create: `crates/orkworksd/src/workspace_runtime.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Move `SessionInfo` and `MemoryState` into `session_types.rs`**

Create `crates/orkworksd/src/session_types.rs` and move:

```rust
#[derive(Clone, Debug, Serialize)]
pub(crate) struct SessionInfo { /* existing fields unchanged */ }

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MemoryState {
    Live,
    Remembered,
    Resumable,
    Unsupported,
}
```

Update `main.rs` imports to use `crate::session_types::{MemoryState, SessionInfo};` without changing field names or serde attributes.

- [ ] **Step 2: Move pure presentation helpers into `session_view.rs`**

Move only these functions into `crates/orkworksd/src/session_view.rs`:

```rust
pub(crate) fn detect_conflicts(/* unchanged signature */) -> Vec<(String, String)> { /* ... */ }
pub(crate) fn session_recommendation(/* unchanged signature */) -> Option<String> { /* ... */ }
pub(crate) fn connectivity_for_status(status: &str) -> &'static str { /* ... */ }
pub(crate) fn terminal_outcome_for_status(status: &str) -> Option<String> { /* ... */ }
pub(crate) fn merge_live_session_info(/* unchanged signature */) -> SessionInfo { /* ... */ }
pub(crate) fn derive_memory_state(/* unchanged signature */) -> (MemoryState, harness::ResumeStrategy) { /* ... */ }
```

Do not move adapter lookup helpers into this file.

- [ ] **Step 3: Move workspace/time helpers into `workspace_runtime.rs`**

Move:

```rust
pub(crate) fn iso_now() -> String { /* unchanged */ }
pub(crate) fn workspace_hash(path: &std::path::Path) -> String { /* unchanged */ }
pub(crate) fn orksworks_global_dir(workspace_path: &std::path::Path) -> Option<PathBuf> { /* unchanged */ }
```

- [ ] **Step 4: Run the affected targeted tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_uses_live_session_contract_fields_without_metadata -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml merge_live_session_info_uses_live_contract_fields_without_metadata -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml connectivity_for_status_marks_running_sessions_online -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_outcome_for_status_marks_ended_sessions_offline_with_terminal_outcome -- --exact
```

Expected: PASS with no assertion changes.

- [ ] **Step 5: Move the tests with the extracted modules, then commit at a stable checkpoint**

Relocate the tests that cover `SessionInfo`, `MemoryState`, and session-view helpers into `session_types.rs` and `session_view.rs` as module-local `#[cfg(test)]` blocks before removing their old copies from `main.rs`.

Re-run the same targeted tests after relocation and before committing to catch import, visibility, or module-local assumption breakage.

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/session_types.rs crates/orkworksd/src/session_view.rs crates/orkworksd/src/workspace_runtime.rs
git commit -m "refactor(orkworksd): extract shared session and workspace helpers"
```

---

### Task 4: Extract harness registry module

**Files:**
- Create: `crates/orkworksd/src/harness_registry.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Move harness config and registry helpers into `harness_registry.rs`**

Move:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct HarnessVoiceCapabilities { /* unchanged */ }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessConfig { /* unchanged */ }

pub(crate) fn global_harnesses_path() -> Option<std::path::PathBuf> { /* unchanged */ }
pub(crate) fn builtin_harness_configs() -> Vec<HarnessConfig> { /* unchanged */ }
pub(crate) fn load_harnesses() -> Vec<HarnessConfig> { /* unchanged */ }
pub(crate) fn save_harnesses(harnesses: &[HarnessConfig]) { /* unchanged */ }
pub(crate) fn default_shell_command(cwd: String) -> harness::CommandSpec { /* unchanged */ }
pub(crate) fn default_capabilities() -> harness::HarnessCapabilities { /* unchanged */ }
pub(crate) fn builtin_adapters() -> HashMap<String, harness::HarnessAdapter> { /* unchanged */ }
pub(crate) fn capabilities_for_harness(/* unchanged signature */) -> harness::HarnessCapabilities { /* unchanged */ }
pub(crate) fn adapter_for_harness(/* unchanged signature */) -> Option<&harness::HarnessAdapter> { /* unchanged */ }
pub(crate) fn resolve_adapter_harness_id(/* unchanged signature */) -> String { /* unchanged */ }
```

- [ ] **Step 2: Update imports and keep call sites unchanged**

Replace direct in-file references in `main.rs` with `crate::harness_registry::*` imports. Do not rename functions or restructure their logic.

- [ ] **Step 3: Run the harness-focused tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml load_harnesses_merges_disk_overrides_with_builtins -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml load_harnesses_appends_custom_harnesses_after_builtins -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resolve_session_launch_preserves_selected_harness_id_for_generic_shell_configs -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml generic_shell_memory_state_is_not_resumable -- --exact
```

Expected: PASS.

- [ ] **Step 4: Move harness-registry tests with the module, then commit at a stable checkpoint**

Relocate the harness registry tests into `harness_registry.rs` as module-local tests before removing their old copies from `main.rs`.

Re-run the same targeted harness tests after relocation and before committing.

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/harness_registry.rs
git commit -m "refactor(orkworksd): extract harness registry helpers"
```

---

### Task 5: Extract provider, retention, and harness HTTP modules

**Files:**
- Create: `crates/orkworksd/src/http/mod.rs`
- Create: `crates/orkworksd/src/http/provider_handlers.rs`
- Create: `crates/orkworksd/src/http/retention_handlers.rs`
- Create: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Move provider handlers into `http/provider_handlers.rs`**

Move unchanged:

```rust
pub(crate) async fn get_providers(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn set_provider_settings(/* unchanged signature */) -> impl IntoResponse { /* ... */ }

#[derive(Serialize)]
pub(crate) struct ProviderModelsResponse { /* unchanged */ }

#[derive(Serialize)]
pub(crate) struct ErrorResponse { /* unchanged */ }

pub(crate) async fn get_provider_models(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
```

Place `ErrorResponse` in `crates/orkworksd/src/http/mod.rs` so the mechanical refactor has one fixed shared location for it.

- [ ] **Step 2: Move retention and harness handlers into their HTTP modules**

Move unchanged:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct RetentionRequest { /* unchanged */ }

pub(crate) async fn set_retention(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn list_harnesses(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn create_harness(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn update_harness(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn delete_harness(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
```

- [ ] **Step 3: Update router wiring in `main.rs` to import from `http::*`**

Keep route paths and HTTP methods identical. Only change module-qualified handler references.

- [ ] **Step 4: Run targeted handler tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml get_provider_models_returns_not_found_for_unknown_provider -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_builtin_harness_returns_conflict -- --exact
```

Expected: PASS.

- [ ] **Step 5: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/http
git commit -m "refactor(orkworksd): extract provider retention and harness handlers"
```

---

### Task 6: Extract session HTTP handlers

**Files:**
- Create: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Move the session/workspace handler set into `http/session_handlers.rs`**

Move unchanged:

```rust
pub(crate) async fn set_workspace(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn set_active_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn set_active_harnesses(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn resume_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn report_harness_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn create_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn list_sessions(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn delete_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn forget_session(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
```

Keep `resolve_session_launch` in this module for now instead of creating a new utility bucket.

- [ ] **Step 2: Move only session-specific request/response DTOs with the handlers**

Move the DTOs that are only used by the session/workspace endpoint family. Leave shared types in `session_types.rs`.

- [ ] **Step 3: Run the session-handler regression tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness_session_report_writes_metadata_for_known_session -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness_session_report_keeps_resume_memory_in_sync_for_later_status_updates -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_does_not_duplicate_killed_sessions_with_metadata -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_derives_resume_options_for_remembered_sessions -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml forget_session_rejects_live_session_with_conflict -- --exact
```

Expected: PASS.

- [ ] **Step 4: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/http/mod.rs
git commit -m "refactor(orkworksd): extract session handlers"
```

---

### Task 7: Extract terminal HTTP glue, terminal runtime helpers, and retention runtime

**Files:**
- Create: `crates/orkworksd/src/runtime/mod.rs`
- Create: `crates/orkworksd/src/runtime/terminal_http.rs`
- Create: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Create: `crates/orkworksd/src/runtime/retention.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: `crates/orkworksd/src/main.rs`

- [ ] **Step 1: Move terminal endpoint glue into `runtime/terminal_http.rs`**

Move unchanged:

```rust
pub(crate) async fn get_terminal_output(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
pub(crate) async fn session_terminal_handler(/* unchanged signature */) -> impl IntoResponse { /* ... */ }
```

- [ ] **Step 2: Move terminal PTY/env/status helpers into `runtime/terminal_runtime.rs`**

Move unchanged:

```rust
pub(crate) async fn handle_session_terminal(/* unchanged signature */) { /* ... */ }
pub(crate) enum TerminalAction { /* unchanged */ }
pub(crate) fn dispatch_terminal_message(/* unchanged signature */) -> TerminalAction { /* ... */ }
pub(crate) fn collect_input_line(/* unchanged signature */) -> Option<String> { /* ... */ }
pub(crate) fn shell_cmd() -> (String, Vec<String>) { /* unchanged */ }
pub(crate) fn terminal_env_overrides() -> Vec<(String, String)> { /* unchanged */ }
pub(crate) fn session_env_overrides(/* unchanged signature */) -> Vec<(String, String)> { /* unchanged */ }
pub(crate) fn codex_thread_id_from_jsonl_line(line: &str) -> Option<String> { /* unchanged */ }
pub(crate) fn should_forward_terminal_env(key: &str) -> bool { /* unchanged */ }
pub(crate) fn set_session_status(/* unchanged signature */) { /* ... */ }
```

- [ ] **Step 3: Move retention background cleanup into `runtime/retention.rs`**

Move unchanged:

```rust
pub(crate) async fn retention_cleanup_task(state: Arc<AppState>) { /* ... */ }
```

Keep `RetentionConfig` near `AppState` unless a no-churn move becomes obvious.

- [ ] **Step 4: Move terminal tests with the extracted modules**

Relocate terminal endpoint/runtime tests into `terminal_http.rs` and `terminal_runtime.rs` as module-local `#[cfg(test)]` blocks before removing their old copies from `main.rs`.

- [ ] **Step 5: Run the terminal and retention regression tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_message_dispatches_kill -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_message_dispatches_input -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_message_dispatches_resize -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_message_dispatches_unknown_as_noop -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml get_terminal_output_reads_persisted_terminal_history_for_dead_session -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml retention_cleanup_keeps_live_sessions -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml retention_cleanup_clears_last_active_when_session_is_deleted -- --exact
```

Expected: PASS.

- [ ] **Step 6: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/runtime
git commit -m "refactor(orkworksd): extract terminal and retention runtime"
```

---

### Task 8: Extract Peon runtime mechanically

**Files:**
- Create: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`

- [ ] **Step 1: Move the existing Peon state and loop into `runtime/peon_runtime.rs` without redesign**

Move unchanged:

```rust
pub(crate) struct PeonState { /* unchanged fields */ }
pub(crate) async fn peon_loop(state: Arc<AppState>) { /* existing logic moved verbatim first */ }
```

Do not introduce new cleanup helpers or change source-priority / idle behavior in this step.

- [ ] **Step 2: Move Peon tests with the extracted module**

Relocate Peon tests into `runtime/peon_runtime.rs` as module-local tests before removing their old copies from `main.rs`.

- [ ] **Step 3: Run the Peon regression tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml test_peon_inference_writes_metadata -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop_does_not_start_duplicate_inference_while_in_flight -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop_records_failed_inference_attempt -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop_marks_idle_when_silent -- --exact
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop_does_not_overwrite_existing_observed_status_with_idle -- --exact
```

Expected: PASS.

- [ ] **Step 4: Commit at a stable checkpoint**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/src/runtime/mod.rs crates/orkworksd/src/runtime/peon_runtime.rs
git commit -m "refactor(orkworksd): extract peon runtime"
```

---

### Task 9: Update architecture docs and run final verification

**Files:**
- Modify: `docs/agents/architecture.md`
- Review if stale: `AGENTS.md`
- Review if stale: `README.md`

- [ ] **Step 1: Update the architecture doc**

Update `docs/agents/architecture.md` so the Rust sidecar module list no longer claims that `main.rs` owns all HTTP/WS handlers, PTY lifecycle, hashing, metadata directory resolution, and DTO shaping.

- [ ] **Step 2: Check whether `AGENTS.md` or `README.md` became stale because of the module-boundary change**

Only edit those files if the refactor changed documented architecture or workflow claims there.

- [ ] **Step 3: Run full verification**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
bash .claude/hooks/doc-check.sh
```

Expected:
- Rust test suite passes with no new failures
- doc check reports no unaddressed stale-doc triggers

- [ ] **Step 4: Commit the final doc updates**

```bash
git add docs/agents/architecture.md AGENTS.md README.md
git commit -m "docs(orkworksd): update architecture after main.rs refactor"
```

---

## Self-Review

- Spec coverage: the plan covers the reviewed split, neutral shared module extraction, missing `get_terminal_output`, characterization-first sequencing, and the doc update requirement.
- Placeholder scan: no `TODO`/`TBD` placeholders remain; every task names concrete files and verification commands.
- Type consistency: shared types stay in `session_types.rs` / `harness_registry.rs`, session view avoids adapter lookup concerns, `ErrorResponse` stays shared unless proven provider-only, and `resolve_session_launch` stays with session handlers to minimize churn.
