# Resume Stale-Handle Conflict Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow an ended session whose obsolete, unattached in-memory handle has no live PTY to resume instead of returning HTTP 409.

**Architecture:** The resume endpoint retains its existing metadata-derived command selection. Its in-memory conflict guard will distinguish an attached or PID-backed live runtime from a stale handle using three inputs: handle attachment, handle lifecycle phase, and the session PID registry. No UI or protocol changes are required.

**Tech Stack:** Rust, Axum, Tokio, existing sidecar unit tests.

## Global Constraints

- Preserve HTTP 409 for a terminal-attached handle.
- Preserve HTTP 409 for a detached live handle with a tracked PTY PID.
- Permit replacement only when persisted lifecycle is `ended`, the handle is unattached, and no PID is tracked.
- Keep resume request/response shapes, metadata schema, and harness definitions unchanged.
- Add a focused regression test before implementation and run the Rust sidecar test suite.

---

### Task 1: Make the conflict guard lifecycle-consistent

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs:371-503`
- Test: `crates/orkworksd/src/http/session_handlers.rs:2279-2505`

**Interfaces:**
- Consumes: `SessionHandle.terminal_attached`, `SessionHandle.info.lifecycle_phase`, `AppState.session_pids`, and the persisted `SessionMetadata.lifecycle_phase` already loaded by `resume_session`.
- Produces: `resume_session` returns 409 only when the named runtime is attached or remains positively live; it replaces an ended, PID-free stale handle.

- [ ] **Step 1: Add failing guard and endpoint regression tests**

Add a unit test for the conflict predicate beside the existing
`resume_session_rejects_*_live_handle` tests. Construct an unattached
`SessionHandle` with `lifecycle_phase: "active"`; assert that ended metadata
with no tracked PID is not a conflict, then insert PID `42` and assert that it
is a conflict.

Also add an async endpoint regression using the existing `opencode` harness:
create an executable file named `opencode` that exits successfully, prepend
its directory with `FakePath::prepend`, persist `lifecycle_phase: "ended"`,
and install an unattached stale handle. Keep `state.session_pids` empty, call
`resume_session`, and assert `StatusCode::OK`.

```rust
assert!(!resume_handle_conflicts(&handle, true, false));
state.session_pids.lock().unwrap().insert(session_id.clone(), 42);
assert!(resume_handle_conflicts(&handle, true, true));

let response = resume_session(State(state), Path(session_id)).await.into_response();
assert_eq!(response.status(), axum::http::StatusCode::OK);
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
```

Expected: compilation fails because `resume_handle_conflicts` does not exist.

- [ ] **Step 3: Add the smallest pure conflict predicate**

Place this private helper directly above `resume_session`:

```rust
fn resume_handle_conflicts(
    handle: &SessionHandle,
    metadata_ended: bool,
    has_tracked_pid: bool,
) -> bool {
    handle.terminal_attached || !metadata_ended || has_tracked_pid
}
```

In `resume_session`, derive `metadata_ended` from the already-read `meta`, read `has_tracked_pid` from `state.session_pids`, and replace the current `terminal_attached || still_live` branch with this helper. Do not alter command construction, metadata resets, or runtime startup.

- [ ] **Step 4: Run focused regression tests**

Run:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_rejects
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_replaces_unattached_ended_stale_handle
```

Expected: the stale-handle predicate permits only an ended, unattached, PID-free replacement; the endpoint returns 200 for that state; both existing attached-live and detached-live handler tests remain 409.

- [ ] **Step 5: Commit the implementation and tests**

```bash
git add crates/orkworksd/src/http/session_handlers.rs
git commit -m "fix: resume ended sessions with stale handles"
```

### Task 2: Verify behavior and documentation currency

**Files:**
- Modify: `docs/superpowers/specs/2026-08-14-resume-stale-handle-conflict-design.md`
- Create: `docs/superpowers/plans/2026-08-14-resume-stale-handle-conflict.md`

**Interfaces:**
- Consumes: the one-task sidecar change from Task 1.
- Produces: evidence that the sidecar remains buildable and the repository has no required documentation drift.

- [ ] **Step 1: Run the sidecar build and full test suite**

Run:

```bash
rtk cargo build --manifest-path crates/orkworksd/Cargo.toml
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: both commands exit successfully.

- [ ] **Step 2: Run repository documentation and worktree checks**

Run:

```bash
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: no unresolved documentation trigger for the scoped sidecar guard; report unrelated worktree findings without modifying branches not owned by this task.

- [ ] **Step 3: Commit the reviewed design and implementation plan**

```bash
git add docs/superpowers/specs/2026-08-14-resume-stale-handle-conflict-design.md docs/superpowers/plans/2026-08-14-resume-stale-handle-conflict.md
git commit -m "docs: harden resume stale-handle design"
```
