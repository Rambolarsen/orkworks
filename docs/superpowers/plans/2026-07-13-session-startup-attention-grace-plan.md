# Session Startup Attention Grace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep a new session idle while its first two seconds of generic terminal startup output are emitted.

**Architecture:** Store a startup-grace deadline locally in each invocation of `start_session_runtime`. Route both in-memory and persisted process attention updates through one predicate so the two views cannot disagree. Explicit harness/agent attention paths remain untouched.

**Tech Stack:** Rust, Tokio `Instant`/`Duration`, existing sidecar runtime tests.

## Global Constraints

- The grace period is exactly two seconds and applies to every harness.
- Terminal output must still be replayed, buffered, and persisted during the grace period.
- Explicit `waiting_for_input`, `blocked`, and other agent/harness attention signals must not be delayed.
- The grace state is runtime-only; do not add metadata fields or API changes.
- Keep Electron and renderer code unchanged.

---

### Task 1: Guard process-derived Working inference during session startup

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:14-17,254-535`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs:630-980`

**Interfaces:**
- Consumes: `tokio::time::Instant`, `std::time::Duration`, `SessionInfo.lifecycle`, and the `has_visible_output` result already computed for each PTY chunk.
- Produces: `should_infer_working(lifecycle: &str, has_visible_output: bool, startup_grace_ends_at: tokio::time::Instant) -> bool`, used for both in-memory `SessionHandle.info` and persisted session metadata updates.

- [ ] **Step 1: Write the failing tests**

Add these unit tests beside the existing `SessionRuntime` tests:

```rust
#[test]
fn startup_grace_keeps_visible_output_idle() {
    assert!(!should_infer_working(
        "alive",
        true,
        tokio::time::Instant::now() + STARTUP_ATTENTION_GRACE,
    ));
}

#[test]
fn visible_output_after_startup_grace_is_working() {
    assert!(should_infer_working(
        "alive",
        true,
        tokio::time::Instant::now() - std::time::Duration::from_millis(1),
    ));
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml startup_grace`

Expected: compilation failure because `should_infer_working` and `STARTUP_ATTENTION_GRACE` do not exist.

- [ ] **Step 3: Implement the minimal shared inference predicate**

At the existing runtime constants, add:

```rust
const STARTUP_ATTENTION_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

fn should_infer_working(
    lifecycle: &str,
    has_visible_output: bool,
    startup_grace_ends_at: tokio::time::Instant,
) -> bool {
    lifecycle == "alive"
        && has_visible_output
        && tokio::time::Instant::now() >= startup_grace_ends_at
}
```

At the beginning of `start_session_runtime`, before awaiting startup state, capture:

```rust
let startup_grace_ends_at = tokio::time::Instant::now() + STARTUP_ATTENTION_GRACE;
```

Move that value into the spawned driver task. Replace both existing conditions that independently check `meta.lifecycle`/`handle.info.lifecycle` plus `has_visible_output` with `should_infer_working(...)`. Do not change output buffering, replay, persistence queues, Peon `last_output`, or explicit attention endpoints.

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml startup_grace`

Expected: both startup grace tests pass.

- [ ] **Step 5: Run relevant runtime regression tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::session_runtime`

Expected: all session runtime tests pass, including resize and backpressure coverage.

- [ ] **Step 6: Commit the implementation**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix(sidecar): ignore startup output for attention"
```

### Task 2: Verify the sidecar integration and documentation currency

**Files:**
- Verify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Verify: `.claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: the shared `should_infer_working` predicate from Task 1.
- Produces: verification evidence that the sidecar and documentation remain consistent.

- [ ] **Step 1: Run the full Rust suite**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: all tests pass.

- [ ] **Step 2: Run the documentation currency check**

Run: `rtk bash .claude/hooks/doc-check.sh`

Expected: no unaddressed documentation triggers, or update the specifically flagged document before completion.

- [ ] **Step 3: Inspect the final diff**

Run: `rtk git diff --check HEAD~1..HEAD && rtk git status --short`

Expected: no whitespace errors and a clean working tree.
