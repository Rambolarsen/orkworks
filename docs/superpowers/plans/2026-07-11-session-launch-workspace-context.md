# Session Launch Workspace Context Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create each new sidecar session against one immutable active-workspace snapshot so its launch, response, and persisted Git context always agree.

**Architecture:** At the beginning of `create_session`, copy the selected workspace path and a cloneable metadata-store handle while holding `state.workspace`, then release the mutex. Derive the launch CWD and one `GitContext` from that snapshot; after PTY startup, persist through that same metadata handle rather than reacquiring `state.workspace`. The no-workspace path retains the existing process-CWD fallback and omits persistence.

**Tech Stack:** Rust, Axum handlers, Tokio async tests, tempfile Git repositories, existing sidecar metadata store.

## Global Constraints

- Do not hold `std::sync::Mutex` workspace state across an `.await`.
- Keep the current no-workspace launch fallback: process CWD, then `/`.
- Do not change HTTP, IPC, or metadata schemas.
- Keep renderer and harness behavior unchanged.

---

## File structure

- `crates/orkworksd/src/metadata.rs` — make `MetadataStore` safely cloneable as a root-path handle so the handler can retain the selected persistence target across awaits.
- `crates/orkworksd/src/http/session_handlers.rs` — snapshot workspace and Git context, persist through that snapshot, and add the handler regression test.

### Task 1: Pin new-session context to the active workspace snapshot

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs:807-810`
- Modify: `crates/orkworksd/src/http/session_handlers.rs:815-1010`
- Test: `crates/orkworksd/src/http/session_handlers.rs` in `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `AppState.workspace: Mutex<Option<WorkspaceState>>`, `WorkspaceState { path: PathBuf, metadata: MetadataStore }`, `git::detect(&Path) -> GitContext`, and `MetadataStore::write_session` / `append_event`.
- Produces: `create_session` returns and persists identical `cwd`, `repo_root`, `branch`, `dirty`, `changed_files`, and `is_worktree` values for a selected workspace.

- [ ] **Step 1: Write the failing async regression test**

Add `create_session_uses_active_workspace_for_response_and_persisted_metadata` to the session-handler test module. Create a temporary Git repository with branch `main`, create `state` through `test_app_state_with_workspace(dir.path())`, call `create_session`, parse the JSON `SessionInfo`, and read the metadata through `state.workspace.lock().unwrap().as_ref().unwrap().metadata.read_session(&id)`.

Assert the response and metadata both have the canonical temporary-repository CWD and root, and `branch == Some("main")`. End the live runtime with `delete_session(State(state.clone()), Path(id.clone())).await`, then wait until `state.sessions.lock().unwrap().get(&id)` reports an ending or ended lifecycle before dropping the temporary directory.

```rust
assert_eq!(std::fs::canonicalize(&session.cwd).unwrap(), std::fs::canonicalize(dir.path()).unwrap());
assert_eq!(session.branch.as_deref(), Some("main"));
assert_eq!(metadata.cwd, session.cwd);
assert_eq!(metadata.repo_root, session.repo_root);
assert_eq!(metadata.branch, session.branch);
assert_eq!(delete_session(State(state.clone()), Path(id.clone())).await, axum::http::StatusCode::OK);
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml create_session_uses_active_workspace_for_response_and_persisted_metadata -- --exact
```

Expected: FAIL because `create_session` derives its CWD from `std::env::current_dir()` and only derives persisted Git data after reacquiring `state.workspace`.

- [ ] **Step 3: Implement the smallest snapshot-based fix**

Derive `Clone` for `MetadataStore`, whose sole field is the metadata-root `PathBuf`:

```rust
#[derive(Clone)]
pub struct MetadataStore {
    root: PathBuf,
}
```

At the start of `create_session`, acquire `state.workspace`, clone `ws.path` and `ws.metadata` into `Option<(PathBuf, MetadataStore)>`, then drop the guard before awaiting `state.harnesses.read()`. Choose `cwd` from the cloned path or retain the existing process-CWD fallback. Run `git::detect` once against that CWD.

After `start_session_runtime(...).await` succeeds, write the session and event only through the cloned `MetadataStore` and cloned workspace path. Populate all persisted Git fields from the original `git_ctx`; do not call `git::detect` again and do not reacquire `state.workspace`.

- [ ] **Step 4: Run the focused test to verify it passes**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml create_session_uses_active_workspace_for_response_and_persisted_metadata -- --exact
```

Expected: PASS, with the runtime cleanup completed before test teardown.

- [ ] **Step 5: Run sidecar regression tests**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: PASS with no handler, metadata, or runtime regressions.

- [ ] **Step 6: Commit the implementation**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/http/session_handlers.rs
git commit -m "fix(sidecar): bind new sessions to active workspace"
```

## Plan self-review

- Spec coverage: Task 1 snapshots workspace state before awaits, preserves the no-workspace fallback, uses one Git detection for response and persistence, asserts persisted metadata, and cleans up the test runtime.
- Placeholder scan: no deferred work or unspecified implementation steps remain.
- Type consistency: `MetadataStore` is made `Clone`; `PathBuf`, `GitContext`, `SessionInfo`, and `SessionMetadata` use existing fields and APIs.
