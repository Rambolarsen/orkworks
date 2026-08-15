# Session Startup Finalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure a session deleted or cancelled while its runtime starts finalizes without rolling a live generation back.

**Architecture:** `ResumeAdmission` rolls back only before PTY spawn. Startup records its irreversible boundary before PID registration. One helper routes a post-spawn abort through generation-owned finalization only when the same generation is already `ending`; ordinary startup errors retain the existing error path.

**Tech Stack:** Rust, Axum, Tokio, portable-pty, sidecar integration tests.

## Global Constraints

- Preserve ADR 0041 generation ownership; a stale generation does no cleanup or persistence.
- Do not change HTTP shapes, persisted metadata, or harness definitions.
- Keep `resume_in_progress` until the existing terminal finalizer releases it.
- Process fixtures must work on Windows and Unix.

---

### Task 1: Transfer resume rollback ownership at PTY spawn

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs:389-485,631-740`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:673-780`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Consumes: `ResumeAdmission::drop`, `SessionRuntime::run_generation()`, and `start_session_runtime`.
- Produces: a runtime-only marker that makes rollback a no-op after the claimed generation spawns its PTY.

- [ ] **Step 1: Write the failing cancellation regression**

Use a portable long-running resumed process. Spawn `resume_session`, wait until `session_pids` contains the ID, abort the handler task, then assert the replacement generation remains registered and prior metadata/terminal size are not restored. Kill it and assert normal finalization releases its claim.

- [ ] **Step 2: Verify RED**

Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml cancelled_resume_after_spawn_keeps_live_runtime -- --nocapture`. It must fail because `ResumeAdmission::drop` removes or restores the live generation.

- [ ] **Step 3: Implement the smallest ownership transfer**

Set the generation-scoped marker immediately after successful `spawn_command`, before PID registration. In `ResumeAdmission::drop`, roll back only when that exact claimed generation has not crossed the marker. Do not clear `resume_in_progress` here.

- [ ] **Step 4: Verify GREEN**

Re-run the focused command from Step 2; it must pass.

### Task 2: Finalize deleted post-spawn startup aborts

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:736-780`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

**Interfaces:**
- Consumes: `set_session_status_for_generation`, `clear_ended_session_tracking`, and `schedule_session_ending_finalization`.
- Produces: one helper used by rejected `running`, reader-clone, and writer-take setup failures.

- [ ] **Step 1: Write the failing delete-during-startup regression**

Start a resumable session, delete it during `INITIAL_RESIZE_GRACE`, and use a bounded timeout to assert metadata and handle lifecycle become `ended`, status is `killed`, PID/Peon tracking is gone, and the claim is false.

- [ ] **Step 2: Verify RED**

Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_during_startup_finalizes_same_generation -- --nocapture`. It must time out or show the generation stuck in `ending`.

- [ ] **Step 3: Implement centralized post-spawn cleanup**

Kill/wait the child, then schedule the existing finalizer with `killed` only when the same generation owns an `ending` handle. Apply this helper to all three post-spawn failure sites. Leave non-ending errors for the existing handler error transition.

- [ ] **Step 4: Verify GREEN**

Re-run the focused command from Step 2; it must pass.

### Task 3: Make the stale-resume fixture portable and verify

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs:2467-2480`

- [ ] **Step 1: Replace the POSIX-only fake executable**

Use the suite's cross-platform child helper, or equivalent target-specific commands, for `resume_session_replaces_unattached_ended_stale_handle`.

- [ ] **Step 2: Run focused tests**

Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_` and `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml delete_during_startup_finalizes_same_generation`.

- [ ] **Step 3: Complete verification and handoff**

Run `rtk cargo build --manifest-path crates/orkworksd/Cargo.toml`, `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`, `git diff --check`, `bash .claude/hooks/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`. Commit only the plan and scoped Rust files, push the existing PR branch, then request a fresh lifecycle review.
