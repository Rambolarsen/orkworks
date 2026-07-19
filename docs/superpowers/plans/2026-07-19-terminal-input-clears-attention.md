# Terminal Input Clears Attention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Immediately replace stale `Needs you` with process-sourced `Working` after non-empty terminal input is successfully accepted by the runtime command channel, for every harness.

**Architecture:** Retain the terminal action in the command future until its send succeeds. Perform input bookkeeping only from that success path. Persist the working transition atomically before mutating the in-memory session, so metadata and API state agree.

**Tech Stack:** Rust, Tokio MPSC, Axum WebSocket, portable-pty. No new dependencies.

## Global Constraints

- Implement in an agent-owned branch/worktree; this sidecar change requires PR review before merge.
- Do not add harness configuration or change the Claude hook script: behavior is universal.
- Empty input, queue-overflow drops, closed runtime channels, non-alive sessions, and persistence failures must not change attention.
- The successful runtime-channel send is the delivery boundary; do not wait for terminal output.
- Preserve queue ordering/coalescing, label handling, sensitive-input protection, and usage-limit rechecks.
- Verify with `cargo test --manifest-path crates/orkworksd/Cargo.toml`, `cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings`, `bash .claude/hooks/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.

---

## File Structure

- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs` — dispatch-confirmed action handling and persist-before-memory transition.
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs` — regression tests using the existing `test_state_with_runtime_session` fixture.
- Existing design: `docs/superpowers/specs/2026-07-19-terminal-input-clears-attention-design.md` (commit `c2751c1`).

### Task 1: Gate bookkeeping on successful input dispatch

**Files:**

- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:141-167, 680-765`
- Test: `crates/orkworksd/src/runtime/terminal_runtime.rs` (`mod tests`)

**Interfaces:**

- Change `PendingCommandFuture` to `Future<Output = Result<TerminalAction, ()>>`.
- Derive `Clone` for `TerminalAction`.
- `spawn_command_future` returns `Ok(action)` only after `send_runtime_command` or `update_runtime_size` succeeds.

- [ ] **Step 1: Write failing tests**

Add a test for a dropped control receiver showing the input command future resolves to `Err(())`, not a dispatched `TerminalAction::Input`. Add a test confirming `dispatch_terminal_message` still creates `TerminalAction::Input(String::new())` for an empty frame, which the success handler must explicitly ignore.

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml --lib runtime::terminal_runtime::tests -- --nocapture
```

Expected: the send-failure regression fails because the current future erases the dispatched action and bookkeeping happens before it resolves.

- [ ] **Step 2: Return the action after a successful send**

Replace the input arm in `spawn_command_future` with this pattern; use the same pattern for resize and kill:

```rust
TerminalAction::Input(data) => Some(Box::pin(async move {
    crate::runtime::session_runtime::send_runtime_command(
        &state,
        &id,
        crate::runtime::session_runtime::RuntimeCommand::Input(data.clone()),
    ).await?;
    Ok(TerminalAction::Input(data))
})),
```

In the completed-future branch, handle the returned action before dequeuing another action:

```rust
match result {
    Ok(TerminalAction::Input(data)) if !data.is_empty() => {
        record_dispatched_terminal_input(&state, &id, &data);
    }
    Ok(_) => {}
    Err(()) => break,
}
```

Remove both pre-send `record_peon_input_side_effects` calls at the immediate and queued dispatch sites. This prevents state changes for unsent and dropped input.

- [ ] **Step 3: Verify and commit**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml --lib runtime::terminal_runtime::tests -- --nocapture
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
git add crates/orkworksd/src/runtime/terminal_runtime.rs
git commit -m "refactor(terminal): confirm input dispatch before bookkeeping"
```

Expected: terminal-runtime tests pass and Clippy has no warnings.

### Task 2: Persist the universal working transition before exposing it

**Files:**

- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs:239-320`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs:1450-1570`

**Interfaces:**

- Rename `record_peon_input_side_effects` to `pub(crate) fn record_dispatched_terminal_input`, so the sibling `session_runtime` test module can exercise the dispatch-confirmed boundary.
- Keep `record_terminal_input` for parsing/label/work-signal behavior.
- For non-empty input on an alive session, `try_write_session` the following before changing `handle.info`: `observed_status = Some("working")`, `attention = Some("working")`, `metadata_source = "process"`, and `metadata_confidence = 1.0`.

- [ ] **Step 1: Write failing state regressions**

Replace `terminal_input_preserves_observed_attention_in_memory_and_metadata` with `dispatched_terminal_input_replaces_needs_you_in_memory_and_metadata`. Seed both memory and disk with an alive, agent-sourced `needs_you` session; dispatch `"continue\r"`; assert both have working attention, working observed status, and process source. Add a parallel `"y"` case proving single-key input works and an empty-input case asserting the seed remains unchanged.

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml --lib runtime::session_runtime::tests::dispatched_terminal_input_replaces_needs_you_in_memory_and_metadata -- --nocapture
```

Expected: FAIL because current bookkeeping preserves the old attention state.

- [ ] **Step 2: Implement persistence-first state handling**

At the start of `record_dispatched_terminal_input`, return for empty data. Read the metadata record, modify the four working fields only when `meta.lifecycle == "alive"`, then call `try_write_session(&meta)`. On error, log the session ID and return before changing `SessionHandle`. After a successful write, lock sessions and assign the matching four values to the alive handle. Fold any metadata label write into this same fallible path, or make it use `try_write_session` before changing the matching in-memory field.

- [ ] **Step 3: Add persistence-failure coverage**

Create a test workspace whose metadata `sessions` directory is a regular file so `try_write_session` fails. After non-empty dispatched input, assert the in-memory handle still reports `needs_you` and agent source; do not only assert logging.

- [ ] **Step 4: Verify and commit**

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
git add crates/orkworksd/src/runtime/terminal_runtime.rs crates/orkworksd/src/runtime/session_runtime.rs
git commit -m "fix(sidecar): clear attention after terminal input"
```

Expected: all checks pass and documentation/worktree currency findings are resolved.

### Task 3: Review before PR handoff

**Files:**

- Review: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Review: `crates/orkworksd/src/runtime/session_runtime.rs`

- [ ] **Step 1: Request lightweight code review**

Give the reviewer this plan and the implementation range. Require review of universal scope, dispatch-before-bookkeeping ordering, empty/drop/send-failure cases, persistence-before-memory ordering, and preserved terminal queue ordering.

- [ ] **Step 2: Address findings and reverify**

Verify each Critical or Important finding against the codebase, add targeted tests for accepted fixes, then run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
cargo clippy --manifest-path crates/orkworksd/Cargo.toml -- -D warnings
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Expected: all commands pass before PR creation.
