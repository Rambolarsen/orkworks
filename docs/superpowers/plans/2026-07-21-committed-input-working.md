# Committed Terminal Input Implies Working Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every accepted, non-empty terminal input immediately transition its live session to the canonical `working` attention state.

**Architecture:** Keep the transition at `record_terminal_input`, which is invoked only after terminal input has been accepted for delivery. Add a small helper that updates the in-memory `SessionHandle` and persisted `SessionMetadata` together, clearing the prompt-specific fields from the answered turn. Retire the pending-output signal as a prerequisite for this input-driven transition while retaining unrelated output lifecycle handling.

**Tech Stack:** Rust, Tokio, Axum sidecar tests, Cargo test runner.

## Global Constraints

- Committed, non-empty input means accepted terminal delivery to a live session and must always produce `working`.
- The transition applies to every harness and input shape, including a single key and newline-terminated input.
- The transition sets metadata source to `process` and confidence to `1.0`, overriding a prior source including `user`.
- Clear `needs_user_input`, `detected_question`, and `suggested_options` together with the answered `Needs you` state.
- Rejected or empty input must not change attention.
- No new dependencies, IPC APIs, or cross-boundary imports.

---

## File Structure

- `crates/orkworksd/src/runtime/terminal_runtime.rs` owns accepted-input bookkeeping and will own the single immediate input-to-working transition.
- `crates/orkworksd/src/runtime/session_runtime.rs` owns the old output-gated work-signal tests; its tests and obsolete signal behavior must be updated so terminal output can no longer be required to clear committed input.
- `docs/superpowers/specs/2026-07-21-committed-input-working-design.md` is the already-committed behavior constraint; no product API or schema field changes are needed.

### Task 1: Pin immediate in-memory and persisted transitions

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:record_terminal_input` and its test module
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `record_terminal_input(state: &Arc<AppState>, id: &str, data: &str) -> Option<()>` after input delivery has been accepted.
- Produces: a live `SessionHandle.info` and matching `SessionMetadata` with `observed_status = Some("working")`, `attention = Some("working")`, `metadata_source = "process"`, `metadata_confidence = 1.0`, and cleared prompt fields.

- [ ] **Step 1: Write failing tests for accepted single-key and newline input**

Add a test-only state setup with a live session and metadata record initialized to a hook-sourced prompt:

```rust
handle.info.lifecycle = "alive".into();
handle.info.attention = Some("needs_you".into());
handle.info.observed_status = Some("waiting_for_input".into());
handle.info.metadata_source = Some("agent".into());
handle.info.needs_user_input = Some(true);
handle.info.detected_question = Some("Proceed?".into());
handle.info.suggested_options = Some(vec!["yes".into(), "no".into()]);
```

Call `record_terminal_input(&state, session_id, "y")` in one test and `record_terminal_input(&state, session_id, "yes\\r")` in another. Immediately assert, without driving a PTY output event:

```rust
assert_eq!(handle.info.attention.as_deref(), Some("working"));
assert_eq!(handle.info.observed_status.as_deref(), Some("working"));
assert_eq!(handle.info.metadata_source.as_deref(), Some("process"));
assert_eq!(handle.info.metadata_confidence, Some(1.0));
assert_eq!(handle.info.needs_user_input, None);
assert_eq!(handle.info.detected_question, None);
assert_eq!(handle.info.suggested_options, None);
```

Read the metadata record and assert the same persisted values.

- [ ] **Step 2: Run the focused tests and verify they fail for the missing immediate transition**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml committed_input -- --nocapture
```

Expected: FAIL because the current code leaves `attention` as `needs_you` until qualifying PTY output arrives.

- [ ] **Step 3: Implement one immediate committed-input transition helper**

Add a private helper in `terminal_runtime.rs`, called from `record_terminal_input` only when `data` is non-empty and the session is live. Its essential update is:

```rust
handle.info.observed_status = Some("working".into());
handle.info.attention = Some("working".into());
handle.info.metadata_source = Some("process".into());
handle.info.metadata_confidence = Some(1.0);
handle.info.needs_user_input = None;
handle.info.detected_question = None;
handle.info.suggested_options = None;
handle.pending_work_signal = None;
```

After releasing the session lock, read the matching metadata record and apply the analogous fields before writing it back. Do not call `merge_peon_inference` or change lifecycle/status fields. Keep empty-input behavior as a no-op.

- [ ] **Step 4: Run the focused tests and verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml committed_input -- --nocapture
```

Expected: PASS; both input shapes update memory and metadata before terminal output.

- [ ] **Step 5: Commit the focused transition**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "fix(sidecar): mark committed input as working"
```

### Task 2: Guard the universal rule and remove the obsolete output gate

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: a live session with any previous metadata source or `active_work_hook` setting.
- Produces: committed input that is immediately `working`; subsequent `report_attention(..., "waiting_for_input", ...)` remains able to return it to `needs_you`.

- [ ] **Step 1: Write failing regression tests for source, hook, empty input, and a new prompt**

Add focused tests that prove:

```rust
// Prior user metadata and active-work-hook configuration do not suppress input -> working.
handle.info.metadata_source = Some("user".into());
handle.active_work_hook = true;
record_terminal_input(&state, session_id, "1");
assert_eq!(handle.info.attention.as_deref(), Some("working"));

// Empty accepted data makes no transition.
record_terminal_input(&state, session_id, "");
assert_eq!(handle.info.attention.as_deref(), Some("needs_you"));
```

In the session-handler test module, begin from the now-working record, call `report_attention` with `AttentionReportRequest { status: "waiting_for_input".into(), message: None }`, and assert its attention returns to `needs_you`.

Replace or remove tests whose expected behavior is “single-key input waits for a visible output chunk before promotion”; that expectation contradicts the new constraint. Retain tests that only verify output does not independently invent `working` without committed input.

- [ ] **Step 2: Run the focused regression suite and verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime:: -- --nocapture
```

Expected: FAIL until the old fallback gate and its tests no longer control the committed-input state.

- [ ] **Step 3: Simplify the obsolete pending-work path**

Remove the single-key and newline-specific arming branches from `record_terminal_input`, or make them unreachable after the immediate transition, so `pending_work_signal` cannot delay or later overwrite the accepted-input result. Remove `session_runtime` helper/tests only if they have no remaining initial-prompt or lifecycle consumer; otherwise keep their remaining use narrowly documented and ensure the immediate transition clears any pre-existing signal.

Do not change `report_attention`: its accepted harness notification must continue to set `waiting_for_input` / `needs_you` for a new prompt.

- [ ] **Step 4: Run focused Rust tests and verify they pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml runtime:: -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml report_attention -- --nocapture
```

Expected: PASS; input is immediate and universal, empty input is inert, and a later hook report restores `Needs you`.

- [ ] **Step 5: Commit the regression coverage and cleanup**

```bash
git add crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/session_runtime.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "test(sidecar): cover committed input attention transitions"
```

### Task 3: Verify integration and documentation currency

**Files:**
- Verify: `docs/superpowers/specs/2026-07-21-committed-input-working-design.md`
- Verify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: completed committed-input state transition and regression suite.
- Produces: evidence that the sidecar builds, tests pass, and required documentation checks have no unaddressed flags.

- [ ] **Step 1: Run sidecar formatting and the complete Rust suite**

Run:

```bash
cargo fmt --check --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: both commands exit 0.

- [ ] **Step 2: Run repository documentation and worktree checks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: no unaddressed documentation trigger; report only other owners’ worktree warnings without modifying their worktrees.

- [ ] **Step 3: Inspect the final diff**

Run:

```bash
git diff --check HEAD~2..HEAD
git status --short
```

Expected: no whitespace errors and only intended changes.

## Plan Self-Review

- Spec coverage: Task 1 covers immediate memory/persistence, prompt-field clearing, input shapes, and source/confidence. Task 2 covers all-harness universality, empty-input safety, new prompts, and removal of the old output prerequisite. Task 3 covers build, full tests, doc currency, and worktree currency.
- Placeholder scan: no deferred work, unnamed test behavior, or unspecified code paths remain.
- Type consistency: `SessionInfo` and `SessionMetadata` use `observed_status`, `attention`, `metadata_source`, `metadata_confidence`, `needs_user_input`, `detected_question`, and `suggested_options`; the plan uses those exact Rust names.
