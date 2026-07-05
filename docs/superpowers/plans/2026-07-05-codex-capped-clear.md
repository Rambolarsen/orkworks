# Codex Capped Clear Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clear a live Codex session's stale capped badge after a real continue/resume cycle, while keeping remembered/offline sessions from inheriting another live session's capped state.

**Architecture:** Keep the fix inside the Rust session aggregation path. Track a fresh post-input scan origin for capped live sessions so the badge only clears after new output is observed, and stop fanning harness-level capped state back into remembered sessions while leaving provider-level capacity rows unchanged.

**Tech Stack:** Rust, Axum handlers, existing session aggregation tests in `crates/orkworksd/src/http/session_handlers.rs`

---

### Task 1: Lock remembered-session behavior with a failing test

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions() {
    // Build one live capped Codex session and one remembered Codex session.
    // Assert the live session is capped and the remembered one stays uncapped.
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions -- --exact`
Expected: FAIL because `list_sessions` currently propagates harness capped state into remembered sessions.

- [ ] **Step 3: Write minimal implementation**

```rust
// Only copy harness capped state back onto live sessions whose own
// at_usage_limit field is sourced from active runtime state.
if !harness_capped.is_empty() {
    for info in &mut infos {
        if info.memory_state != MemoryState::Live {
            continue;
        }
        // existing harness propagation...
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_does_not_mark_remembered_sessions_capped_from_other_live_sessions -- --exact`
Expected: PASS

### Task 2: Lock live Codex capped-clearing behavior with failing tests

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: `crates/orkworksd/src/http/session_handlers.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn list_sessions_clears_live_capped_after_fresh_post_input_output_without_new_limit() {
    // Seed a live Codex session with a latched cap and a scan origin.
    // Simulate fresh output after the origin that does not contain the limit text.
    // Assert the returned session is no longer capped.
}

#[tokio::test]
async fn list_sessions_keeps_live_capped_when_fresh_post_input_output_still_contains_limit() {
    // Same setup, but fresh output still contains the limit text.
    // Assert the returned session remains capped.
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_clears_live_capped_after_fresh_post_input_output_without_new_limit list_sessions_keeps_live_capped_when_fresh_post_input_output_still_contains_limit -- --exact`
Expected: FAIL because the sticky latch only resets on OrkWorks resume today.

- [ ] **Step 3: Write minimal implementation**

```rust
// Add a capped re-check origin to SessionHandle.
// When the user sends terminal input on a capped capacity-detecting session,
// record the current buffer origin.
// In list_sessions, if fresh output exists after that origin:
// - clear the latch when no new limit text appears in the fresh slice
// - keep the latch when a new limit text still appears there
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_clears_live_capped_after_fresh_post_input_output_without_new_limit list_sessions_keeps_live_capped_when_fresh_post_input_output_still_contains_limit -- --exact`
Expected: PASS

### Task 3: Run focused regression coverage and repo close-out checks

**Files:**
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`

- [ ] **Step 1: Run the targeted Rust session tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml list_sessions_ -- --nocapture`
Expected: PASS for the session list regression coverage, including the existing pending-capacity tests.

- [ ] **Step 2: Run doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: either no flagged docs or a concrete list to address before finishing.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-07-05-codex-capped-clear.md \
  crates/orkworksd/src/http/session_handlers.rs \
  crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "fix: clear stale codex capped status"
```
