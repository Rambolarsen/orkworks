# Taskmaster Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the actionable PR #333 review findings while preserving the accepted Taskmaster evidence and lifecycle contract.

**Architecture:** Keep workflow observations workspace-scoped and immutable, use the existing atomic file replacement primitive, and make workspace admission/cleanup boundaries explicit. Reevaluate persisted observations at workspace open and route all observation writes through active-workspace membership checks.

**Tech Stack:** Rust sidecar, Axum, Tokio, serde, filesystem metadata stores, Rust unit tests, GitHub Actions.

## Global Constraints

- Taskmaster recommendations remain evidence-backed and proposed-only in this PR.
- Single-session repeated friction is valid evidence; high-impact single observations remain valid.
- Embedded recommendation evidence survives ordinary observation retention and is removed only by explicit session/recommendation cleanup.
- Observation writes must target the active workspace that owns the session metadata.
- Use `pnpm` for Node package tasks and preserve the Electron/renderer boundary.

### Task 1: Correct evaluator semantics and persistence

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `crates/orkworksd/src/taskmaster/store.rs`
- Test: inline Rust tests in those files

- [x] Restore qualifying clusters with two observations from one session and add a regression test.
- [x] Preserve prior embedded evidence when rebuilding proposed recommendations.
- [x] Use `now` for newly resurfaced dismissed successors while preserving `created_at` for in-place proposed updates.
- [x] Replace recommendation writes with `harness::integration::atomic_replace` and verify repeated writes through store tests.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::`.

### Task 2: Rebuild persisted recommendations on workspace open

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Test: workspace-open tests in `session_handlers.rs`

- [x] Schedule evaluation after the active `WorkspaceState` is installed and orphan cleanup completes.
- [x] Keep scheduling outside the workspace mutex and use the existing debounce/generation checks.
- [x] Reuse the workspace-open path and existing evaluator scheduling contract for persisted observations.
- [x] Run the focused HTTP and evaluator tests.

### Task 3: Make observation writes workspace-safe and complete final Peon capture

**Files:**
- Modify: `crates/orkworksd/src/http/workflow_observation_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/peon_runtime.rs`
- Test: handler and Peon runtime tests

- [x] Reject observation reports when the session is absent from active workspace metadata.
- [x] Apply the same membership check to Peon-originated observations.
- [x] Persist final-scan workflow observations before finalizing session state.
- [x] Run focused observation tests.

### Task 4: Make deletion failure cleanup retry-safe and refresh documentation

**Files:**
- Modify: `crates/orkworksd/src/runtime/retention.rs`
- Modify: callers in `crates/orkworksd/src/http/session_handlers.rs` and retention cleanup
- Modify: `AGENTS.md`, `docs/agents/architecture.md` if status text is stale
- Test: retention and documentation-triggered checks

- [x] Remove in-memory session handles and tracking when metadata deletion completed but final recommendation cleanup still fails.
- [x] Preserve retry/orphan cleanup behavior for recommendation files.
- [x] Mark Peon and Taskmaster observation/evaluation work as implemented and remove stale refresh-endpoint claims.
- [x] Run `cargo test --manifest-path crates/orkworksd/Cargo.toml` and desktop checks.

### Task 5: Verify and update PR

- [ ] Run `git diff --check`, doc currency, and worktree checks.
- [ ] Push the fix commit to `feat/workflow-observation-feedback-loop`.
- [ ] Confirm all PR checks pass and reply in each actionable review thread with the fix and test evidence.
- [ ] Resolve only the review threads addressed by this change.
