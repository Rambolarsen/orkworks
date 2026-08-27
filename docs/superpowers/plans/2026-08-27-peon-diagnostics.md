# Debug-Only Peon Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose enough per-session Peon scheduler/provider state to explain whether a session was skipped, is running inference, failed, or completed, while showing it only behind the existing debug metadata setting.

**Architecture:** Keep authoritative diagnostics in the Rust sidecar alongside the existing `PeonState`; publish an optional serialized snapshot on `SessionInfo`; render it in `SessionDetailPanel` behind the existing `showDebugMetadata` prop. Track attempt generations per session so timeout cleanup and late provider completions cannot overwrite newer state. Count accepted workflow observations through the observation store's session-scoped interface.

**Tech Stack:** Rust/Axum sidecar, Tokio task runtime, serde camelCase JSON, React/TypeScript renderer, Node test runner, Cargo tests.

## Global Constraints

- Keep the feature debug-only in the renderer; do not add a new setting or panel.
- Diagnostics are best-effort and must never block session polling, session creation, Peon inference, or recommendation queries.
- Do not change recommendation eligibility, provider fallback policy, Peon frequency, or provider concurrency.
- Do not expose prompts, terminal transcripts, credentials, or raw model output.
- Preserve the Electron-main/renderer boundary; update renderer API types without importing Rust or Electron code.
- Use `pnpm` for Node package-management commands.

### Task 1: Define and test the diagnostic contract

**Files:**
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/workflow_observations.rs`
- Test: `crates/orkworksd/src/session_types.rs` and `crates/orkworksd/src/workflow_observations.rs` module tests

**Interfaces:**
- Produce `PeonDiagnostics` with serde camelCase fields: scheduler state, optional reason, attempt timestamps, provider ID/model, fallback step, attempt count, bounded error summary, and nullable observation count.
- Produce a per-session observation-store query that counts accepted observations and returns a store error without changing persisted data.

- [ ] **Step 1: Write failing serialization and observation-count tests.**

  Add a `SessionInfo` test that sets `peon_diagnostics` and asserts the JSON contains `peonDiagnostics`, `schedulerState`, `lastAttemptAt`, `lastSuccessfulInferenceAt`, `providerId`, `providerModel`, `fallbackStep`, `attemptCount`, `errorSummary`, and `observationCount`. Add a store test that records an accepted observation, replays it as a duplicate, and asserts the per-session count remains one.

- [ ] **Step 2: Run the focused tests and verify they fail for the missing contract.**

  Run:

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml session_types workflow_observations
  ```

  Expected: compilation/test failure because the diagnostics field and session-count method do not yet exist.

- [ ] **Step 3: Implement the minimal serializable types and count method.**

  Add a small enum for the five scheduler states and a `PeonDiagnostics` struct with optional fields where no attempt has occurred. Add `SessionInfo.peon_diagnostics: Option<PeonDiagnostics>` with `#[serde(rename = "peonDiagnostics", skip_serializing_if = "Option::is_none")]`. Implement `WorkflowObservationStore::session_observation_count(&self, session_id: &str) -> Result<usize, StoreError>` by using the existing retained-observation reader and filtering the requested session; do not count tombstones or duplicates.

- [ ] **Step 4: Run the focused tests and verify they pass.**

  Run the same Cargo command. Expected: PASS, with existing session serialization tests still green.

### Task 2: Track Peon lifecycle and provider outcomes

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Modify: `crates/orkworksd/src/providers.rs` only if an existing bounded error-summary helper must be reused
- Test: `crates/orkworksd/src/runtime/peon_runtime.rs`

**Interfaces:**
- Add per-session diagnostic bookkeeping to `PeonState`, protected by the same standard read/write lock pattern used by other Peon maps.
- Add helpers for candidate selection, attempt start, completion, failure, and timeout; each helper accepts a session ID and preserves an attempt generation.

- [ ] **Step 1: Write the failing two-session concurrency test.**

  Extend the Peon runtime test fixture with two eligible sessions and a fake provider that blocks both calls on a barrier. Start `peon_loop`, wait until both provider calls have entered, and assert both sessions report `in_flight` before releasing the barrier. Also assert each session has its own attempt count.

- [ ] **Step 2: Run the concurrency test and verify it fails because no diagnostics are recorded.**

  Run:

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop_runs_two_sessions_concurrently -- --nocapture
  ```

  Expected: FAIL because the diagnostic state and test observation hooks are absent.

- [ ] **Step 3: Implement state transitions at the existing lifecycle boundaries.**

  When the scheduler adds a session to `candidates`, write `candidate`. Immediately before spawning the provider task, increment the per-session attempt generation, write `in_flight`, and record `lastAttemptAt`. On a valid provider result, write `completed`, `lastSuccessfulInferenceAt`, provider ID/model, fallback step, and attempt count. On task failure, provider exhaustion, invalid output, or timeout, write `failed` and the bounded error summary. On timeout, release the scheduler lease immediately; retain the attempt generation and ignore any detached completion whose generation is stale. When no attempt is pending after completion, write `idle` with an explicit reason such as `no_new_silent_output`; use `disabled`, `not_active`, or `waiting_for_retry` where those conditions are known. Do not hold diagnostic locks while running provider code.

- [ ] **Step 4: Add observation-count refresh without changing scheduling.**

  After a successful output-mode persistence pass, refresh that session’s diagnostic observation count from `session_observation_count`. If the store read fails, set the count to null and retain the inference result. Duplicate reports must leave the count unchanged.

- [ ] **Step 5: Run the focused runtime tests and verify they pass.**

  Run:

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml peon_loop -- --nocapture
  ```

  Expected: PASS, including the new two-session concurrency test and existing no-duplicate/in-flight tests.

### Task 3: Publish diagnostics through session polling

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/session_application.rs` if the existing application seam is the correct place to compose `SessionInfo`
- Test: `crates/orkworksd/src/http/session_handlers.rs` or `crates/orkworksd/src/session_application.rs`

**Interfaces:**
- `GET /sessions` and existing session creation/resume responses include the optional `peonDiagnostics` snapshot for live sessions.
- Missing runtime diagnostics serialize as omitted/null and never turn a successful session response into an error.

- [ ] **Step 1: Write the failing endpoint test.**

  Create a test state with one session whose Peon diagnostic map contains a completed result and assert the list-session response includes the expected camelCase `peonDiagnostics` object. Add a second assertion for a session with no diagnostics that the endpoint remains successful and omits the optional field.

- [ ] **Step 2: Run the endpoint test and verify it fails.**

  Run:

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_includes_peon_diagnostics -- --nocapture
  ```

  Expected: FAIL because session projection does not copy the diagnostic snapshot.

- [ ] **Step 3: Project the snapshot into `SessionInfo`.**

  At the existing session-list projection point, copy the snapshot for each session ID and compute the accepted observation count through the store query. Treat a count read failure as `None`. Keep persisted `peonLastInference` separate from the current runtime diagnostic’s successful-attempt timestamp.

- [ ] **Step 4: Run endpoint and session serialization tests.**

  Run:

  ```bash
  cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions session_types
  ```

  Expected: PASS.

### Task 4: Render the debug-only diagnostics block

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/taskmaster.test.ts` or a new focused renderer source test

**Interfaces:**
- Add the TypeScript `PeonDiagnostics` shape matching the sidecar’s camelCase JSON.
- Render diagnostics only when `showDebugMetadata` is true, using the existing selected-session detail layout.

- [ ] **Step 1: Write failing renderer contract/gating tests.**

  Assert the API type declares `peonDiagnostics`, the detail panel references the diagnostic fields inside the `showDebugMetadata` branch, and the normal branch does not render the diagnostics block.

- [ ] **Step 2: Run the focused desktop tests and verify they fail.**

  Run:

  ```bash
  node --experimental-strip-types --test tests/taskmaster.test.ts
  ```

  Expected: FAIL because the API type and block are absent.

- [ ] **Step 3: Implement the gated presentation.**

  Add a compact `Peon diagnostics` block beside the existing debug fields. Display scheduler state/reason, attempt and success times, provider/model, fallback/attempt count, error summary, and accepted observation count. Show a clear unavailable marker for null values. Keep all labels plain and avoid exposing raw output.

- [ ] **Step 4: Add minimal styles and run focused tests.**

  Add only the styles needed for readable debug rows, then run:

  ```bash
  node --experimental-strip-types --test tests/taskmaster.test.ts tests/api.test.ts
  ```

  Expected: PASS.

### Task 5: Full verification and documentation currency

**Files:**
- Modify: documentation only if `scripts/doc-check.sh` flags a required update
- Test: repository-wide validation commands

- [ ] **Step 1: Run Rust formatting and tests.**

  ```bash
  cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
  cargo test --manifest-path crates/orkworksd/Cargo.toml
  ```

- [ ] **Step 2: Run desktop type-check and tests.**

  ```bash
  cd apps/desktop
  npx tsc --noEmit
  node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
  ```

- [ ] **Step 3: Run repository checks.**

  ```bash
  cd /Users/froomiebot/workspace/orkworks
  bash scripts/doc-check.sh
  bash .claude/hooks/worktree-check.sh
  git diff --check
  ```

- [ ] **Step 4: Review the diff against the approved design.**

  Confirm the change is debug-only in the UI, diagnostics are optional/best-effort, no raw provider output is exposed, two sessions can be in flight concurrently, and recommendation behavior is unchanged.

- [ ] **Step 5: Commit the implementation as one logical unit.**

  ```bash
  git add crates/orkworksd apps/desktop
  git commit -m "feat: expose debug-only Peon diagnostics"
  ```
