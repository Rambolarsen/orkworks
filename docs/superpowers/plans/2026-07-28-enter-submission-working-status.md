# Enter Submission Working Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep session status unchanged while the user types, and transition to `working` only after accepted input contains an Enter line terminator outside bracketed paste.

**Architecture:** Retain `record_terminal_input_impl` as the single accepted-input boundary. Its existing paste-aware `line_completed` result becomes the sole input-driven transition gate. All accepted frames continue to advance runtime input bookkeeping and the Peon idle baseline; harness reports and Peon inference remain independent status sources.

**Tech Stack:** Rust, Tokio, Axum sidecar tests, Cargo test runner.

## Global Constraints

- Only accepted input with an outside-paste `\r` or `\n` may apply `ProcessTransition::CommittedWorking`.
- Empty Enter remains a submission and may transition to `working`.
- Bare keystrokes and bracketed-paste newlines must not change attention, observed status, source, confidence, or prompt fields.
- Accepted input must still advance `input_generation`, set `accepted_input_at`, update `min_peon_output_revision`, and refresh the Peon idle baseline.
- No dependency, API, metadata-schema, or Electron/renderer changes.

---

## File Structure

- `crates/orkworksd/src/runtime/terminal_runtime.rs` parses accepted input, performs the process transition, and contains persistence-focused regression coverage.
- `crates/orkworksd/src/runtime/session_runtime.rs` contains runtime-level tests that must submit a line rather than a bare key when they expect `working`.
- `docs/superpowers/specs/2026-07-28-terminal-input-working-status-design.md` records the approved behavior and this plan implements it.

### Task 1: Pin non-submitted input behavior

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:committed_single_key_immediately_clears_prompt_to_working`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs` test module
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `record_terminal_input(state: &Arc<AppState>, id: &str, data: &str) -> Option<()>` with a live, persisted session.
- Produces: an unchanged prompt/status for bare input, while accepted-input bookkeeping advances.

- [ ] **Step 1: Replace the obsolete single-key expectation with a failing regression**

Rename `committed_single_key_immediately_clears_prompt_to_working` to `bare_keystroke_preserves_idle_session_status_while_refreshing_bookkeeping`. Create an alive no-hook session with persisted `attention = Some("idle".into())` and `observed_status = Some("idle".into())`, capture its input generation and stale Peon baseline, then add these assertions:

```rust
assert_eq!(record_terminal_input(&state, session_id, "y"), None);
let handle = &state.sessions.lock().unwrap()[session_id];
assert_eq!(handle.info.attention.as_deref(), Some("idle"));
assert_eq!(handle.info.observed_status.as_deref(), Some("idle"));
assert_eq!(handle.runtime.input_generation, prior_generation + 1);
assert!(handle.runtime.accepted_input_at.is_some());
assert_eq!(handle.runtime.min_peon_output_revision, prior_revision);
let meta = state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(session_id).unwrap();
assert_eq!(meta.attention.as_deref(), Some("idle"));
assert_eq!(meta.observed_status.as_deref(), Some("idle"));
```

Also assert the refreshed `last_output` instant is newer than the stale baseline.

- [ ] **Step 2: Run the focused regression and verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml bare_keystroke_preserves_idle_session_status -- --nocapture
```

Expected: FAIL because the current hookless bare-keystroke path rewrites both stores to `working`.

- [ ] **Step 3: Update legacy test assumptions**

Keep `committed_newline_terminated_input_immediately_clears_prompt_to_working` and `committed_bare_enter_with_empty_buffer_still_clears_prompt_to_working`. Change the first input in `already_working_input_advances_the_invalidation_boundary_without_rewriting_metadata` from `"y"` to `"y\r"` so it still establishes the process-sourced working state. Keep the second `"z"` input to prove bookkeeping still advances without rewriting the disk canary.

- [ ] **Step 4: Run the terminal-runtime subset**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml committed_ -- --nocapture
```

Expected: FAIL only for tests that still encode the former bare-keystroke transition; preserve the empty-Enter and bracketed-paste behavior.

- [ ] **Step 5: Commit the red test coverage**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "test(sidecar): pin enter-only working transition"
```

### Task 2: Gate the process transition on line submission

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:mark_committed_input_working`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:record_terminal_input_impl`
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `line_completed: bool` from `collect_input_line`, whose parser treats only CR/LF outside bracketed paste as a submission.
- Produces: `ProcessTransition::CommittedWorking` only when `line_completed` is true.

- [ ] **Step 1: Implement the minimal gate**

Delete `bare_keystroke_is_trusted`, remove its persisted-metadata lookup, and replace the `commit_working` calculation with:

```rust
let commit_working = !already_working && line_completed;
```

Retain the existing `!commit_working || already_working` bookkeeping branch unchanged so accepted input still advances generation, timestamp, output boundary, and idle baseline. Update the function comments to state that only a completed submitted line commits the status transition.

- [ ] **Step 2: Run the focused terminal-runtime tests and verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml committed_ -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml bare_keystroke_preserves_idle_session_status -- --nocapture
```

Expected: PASS; ordinary typing preserves status, while newline-terminated and empty Enter submissions still become `working`.

- [ ] **Step 3: Commit the implementation**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "fix(sidecar): require enter before working"
```

### Task 3: Align runtime-level tests and verify the sidecar

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:terminal_input_immediately_marks_live_session_working_without_pending_signal`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:single_key_at_hook_sourced_needs_you_is_working_before_visible_output`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: terminal input routed through `record_terminal_input`.
- Produces: runtime tests whose `working` expectation explicitly uses submitted input.

- [ ] **Step 1: Change the direct runtime test to submit a line**

Rename `terminal_input_immediately_marks_live_session_working_without_pending_signal` to `submitted_terminal_input_immediately_marks_live_session_working_without_pending_signal`, and change its input from `"fix"` to `"fix\r"`:

```rust
assert!(crate::runtime::terminal_runtime::record_terminal_input(
    &state, session_id, "fix\r"
).is_some());
```

Keep the assertions that the attention is `working` and `pending_work_signal` is `None`.

- [ ] **Step 2: Change the hook-sourced end-to-end test to submit a line**

Rename `single_key_at_hook_sourced_needs_you_is_working_before_visible_output` to `submitted_input_at_hook_sourced_needs_you_is_working_before_visible_output`. Replace `"y"` with `"y\r"` and update the adjacent comment to say an accepted submitted line is sufficient evidence.

- [ ] **Step 3: Run the focused runtime tests and verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml submitted_terminal_input -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml submitted_input_at_hook_sourced -- --nocapture
```

Expected: PASS; both tests assert the retained explicit-submission behavior.

- [ ] **Step 4: Run formatting and the complete Rust suite**

Run:

```bash
cargo fmt --check --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: both commands exit 0.

- [ ] **Step 5: Commit aligned tests**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "test(sidecar): submit input before asserting working"
```

### Task 4: Verify repository currency

**Files:**
- Verify: `docs/superpowers/specs/2026-07-28-terminal-input-working-status-design.md`
- Verify: `docs/superpowers/plans/2026-07-28-enter-submission-working-status.md`

- [ ] **Step 1: Inspect the final diff**

Run:

```bash
git diff --check main...HEAD
git status --short
```

Expected: no whitespace errors and only the intended sidecar tests, implementation, design, and plan files.

- [ ] **Step 2: Run documentation and worktree currency checks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: address any flags for this branch; report other owners’ stale/merged worktrees without modifying them.

## Plan Self-Review

- Spec coverage: Tasks 1–2 pin and implement the exact Enter-only gate, including empty Enter, paste safety, persistence, and accepted-input bookkeeping. Task 3 updates every known runtime-level expectation that formerly relied on a bare key.
- Placeholder scan: no deferred behavior or unspecified test condition remains.
- Type consistency: all referenced fields and functions (`line_completed`, `record_terminal_input`, `input_generation`, `accepted_input_at`, `min_peon_output_revision`, `ProcessTransition::CommittedWorking`) exist in the current sidecar.
