# PTY Output Backpressure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound terminal-runtime memory when PTY output outpaces websocket or metadata persistence consumers.

**Architecture:** Replace the two unbounded internal queues in `start_session_runtime(...)` with bounded Tokio channels. Keep the existing runtime event model, but make the driver await persistence capacity so backpressure reaches the PTY reader instead of accumulating unbounded in memory.

**Tech Stack:** Rust, Tokio, portable-pty, Axum-sidecar runtime tests

## Global Constraints

Use `pnpm` for Node package management if any frontend tooling is needed.
Keep the change scoped to stabilization/correctness for issue `#117`; no product-scope change.
Preserve existing terminal history, replay, and kill semantics while bounding memory.
Add the failing test before changing production runtime code.

---

### Task 1: Add failing runtime coverage for bounded backpressure

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Test: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: `start_session_runtime(...)`, `SessionRuntime::live(...)`, `RuntimeCommand::Kill`
- Produces: runtime tests that fail until bounded-channel constants and flow control exist

- [ ] **Step 1: Write the failing test**

Add runtime-focused tests that exercise a flooding child process and expect the runtime to terminate cleanly after kill while using the bounded internal queue setup.

- [ ] **Step 2: Run test to verify it fails**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml backpressure -- --nocapture`
Expected: FAIL because the bounded-channel helpers/constants or test assertions do not match current unbounded behavior.

- [ ] **Step 3: Write minimal implementation**

Introduce the bounded-channel constants and the smallest runtime changes needed for the new tests to compile and fail for the right reason.

- [ ] **Step 4: Run test to verify the failure is the intended one**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml backpressure -- --nocapture`
Expected: FAIL on the bounded-flow-control assertion, not on a compile error or unrelated runtime error.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "test: pin terminal runtime backpressure behavior"
```

### Task 2: Implement bounded PTY and persistence flow control

**Files:**
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Verify: `crates/orkworksd/src/runtime/session_runtime.rs`

**Interfaces:**
- Consumes: the failing backpressure tests from Task 1
- Produces: bounded driver/persistence channels and passing runtime behavior under flood + kill

- [ ] **Step 1: Replace internal unbounded channels with bounded channels**

Define named queue-capacity constants and apply them to the PTY reader -> driver queue and driver -> persistence queue.

- [ ] **Step 2: Keep kill responsive while enforcing backpressure**

Ensure the runtime can still observe `kill_rx` promptly while the driver awaits persistence capacity for a flood of output.

- [ ] **Step 3: Preserve existing persistence tail-flush behavior**

Keep the exit/error branches flushing the remaining persistence tail after the bounded queue change.

- [ ] **Step 4: Run focused tests**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml backpressure -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run broader runtime verification**

Run: `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml terminal_runtime session_runtime -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix: bound terminal runtime PTY output buffering"
```
