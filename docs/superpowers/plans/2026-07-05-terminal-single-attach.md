# Terminal Single-Attach Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent a second concurrent terminal WebSocket from spawning a second PTY for the same session.

**Architecture:** Keep terminal ownership as in-memory runtime state on `SessionHandle`. Add an atomic claim/release helper in the Rust terminal runtime, reject attaches for already-owned or ending sessions, and block resume-time handle reuse while a live terminal owner still exists.

**Tech Stack:** Rust, Axum WebSocket handlers, Tokio, existing `orkworksd` unit tests

## Global Constraints

- Keep the fix scoped to backend terminal ownership; do not add PTY handoff or new frontend architecture.
- Use TDD: write the failing regression test first, run it red, then implement the minimal fix.
- Preserve existing session lifecycle semantics, including `ending` finalization.
- Do not persist terminal ownership state to metadata.

---

### Task 1: Add the ownership guard and regression tests

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `AppState.sessions`, `SessionHandle`, existing lifecycle/status fields
- Produces: `try_claim_terminal_attachment(state: &Arc<AppState>, id: &str) -> TerminalAttachClaim`

- [ ] **Step 1: Write the failing tests**

Add tests in `crates/orkworksd/src/runtime/terminal_runtime.rs` covering:

```rust
#[test]
fn terminal_attachment_claim_rejects_duplicate_owner() { /* first wins, second rejected */ }

#[test]
fn terminal_attachment_claim_rejects_ending_session() { /* running+ending must reject */ }

#[test]
fn terminal_attachment_release_is_owner_scoped() { /* rejected claim must not clear first */ }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_attachment_ -- --nocapture`
Expected: FAIL because the claim helper and `terminal_attached` state do not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Implement:

- `terminal_attached: bool` on `SessionHandle`
- a small claim result / guard helper in `terminal_runtime.rs`
- atomic claim under one `state.sessions` lock
- rejection for terminal statuses and `lifecycle_phase in {"ending", "ended"}`
- owner-scoped release

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_attachment_ -- --nocapture`
Expected: PASS

### Task 2: Wire the terminal runtime to use the guard

**Files:**
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

**Interfaces:**
- Consumes: `try_claim_terminal_attachment(...)`
- Produces: guarded `handle_session_terminal(...)` startup and teardown

- [ ] **Step 1: Update `handle_session_terminal(...)` to claim before PTY setup**

Use the ownership helper before the PTY is opened. Duplicate or ending-session attaches must close immediately without spawning a child.

- [ ] **Step 2: Ensure all exit paths release through the guard**

Keep release centralized so early returns, kill paths, spawn failures, and websocket shutdown all free the claim exactly once.

- [ ] **Step 3: Run targeted runtime tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime -- --nocapture`
Expected: PASS for the updated runtime tests.

### Task 3: Protect resume-time handle reuse

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Consumes: `SessionHandle.terminal_attached`
- Produces: resume conflict when a live terminal owner still exists

- [ ] **Step 1: Write the failing resume-path test**

Add a test that inserts an attached live handle for an existing session id, calls `resume_session(...)`, and expects `409 CONFLICT`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session -- --nocapture`
Expected: FAIL because `resume_session(...)` currently rewrites the handle in place.

- [ ] **Step 3: Implement the minimal guard**

Under the same `state.sessions` lock used to find/reuse the handle, return conflict if `terminal_attached` is still true. When reusing a safe handle, explicitly reset `terminal_attached` to `false`.

- [ ] **Step 4: Run the resume tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session -- --nocapture`
Expected: PASS

### Task 4: Full verification

**Files:**
- Verify only

**Interfaces:**
- Consumes: all changes above
- Produces: verification evidence for the bugfix

- [ ] **Step 1: Run the focused Rust test suites**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_attachment_ resume_session -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the full Rust test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: PASS

- [ ] **Step 3: Run the repo doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: exit 0, or actionable doc follow-ups listed and addressed
