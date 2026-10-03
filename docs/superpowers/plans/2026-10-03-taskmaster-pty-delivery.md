# Taskmaster PTY delivery implementation plan

> **For agentic workers:** Execute inline with the executing-plans workflow. Each step is a verification checkpoint.

**Goal:** Fix #728's circular PTY wait without permitting duplicate delivery.
**Architecture:** One blocking write in flight; the async driver continues output and stop handling. Keep the write acknowledgement until actual completion and reject queued input after cancellation.
**Tech Stack:** Rust, Tokio, portable-pty; no new dependencies.
**Spec:** [ADR 0075](../../adr/0075-responsive-pty-input-delivery.md), [Taskmaster knowledge](../../../specs/taskmaster-knowledge.md), [MVP](../../../specs/orkworks-mvp.md).

## Global constraints

- Explicit user approval for each handoff; no automatic redispatch.
- Preserve PTY/runtime ownership, ordered input and resize, bounded queues, generation checks, and recommendation evidence.
- Never infer completion from timeout, child exit, or terminal text.

## Uncertainty and blind spots

Least confident: cancellation of a blocked native write. macOS stack samples show both master input and Codex stdout blocked, with the reader waiting on a full queue. Moving writes to the blocking pool breaks that cycle. Stop can kill the child, but cannot universally interrupt a native write if another process retains its slave; keep the reservation until the operation actually returns.

Project blind spot: an HTTP timeout is not a delivery cancellation. The existing in-flight delivery guard must survive it. Tests hold the original request pending, reject a duplicate, stop the target, and retry only after the failed writer acknowledgement.

## Task 1: Responsive delivery and verified retry

**Files:** `crates/orkworksd/src/runtime/session_runtime.rs` owns sequential writes and driver progress; `crates/orkworksd/src/session_application.rs` tests the real recommendation lifecycle; `docs/agents/architecture.md` describes the runtime.
**Interfaces:** Keep `send_runtime_input`, `RuntimeCommand`, and recommendation APIs unchanged.

- [x] Add real-PTY tests using a raw-mode child that writes more output than the driver queue before reading the remaining input, and a child that does not read at all. Both self-terminate on an alarm so a failing test cannot hang the suite.
- [x] Run `rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml pty_delivery -- --nocapture`; observe output-progress and stop failures against the current synchronous writer.
- [x] Move exclusive writer ownership into one blocking task per input. Store the pending task in the driver, poll it alongside output and stop, and defer ordered input/resize until it finishes. Use a shared cancellation flag checked between write calls; retain the acknowledgement in the blocking task. Reject pending commands once stopping.
- [x] Run the regression command again; require delivery success under pressure and failed handoff rollback after stop, followed by exactly one successful explicit retry.
- [x] Run all Rust tests, build, formatting, and the docs build. Perform an independent diff-scoped code review at medium effort because this changes concurrency and lifecycle handling.
- [ ] Commit and open one PR closing #728; inventory checks and review channels, resolve feedback, merge only when required checks and review pass, then clean the owned worktree.

## Verification evidence

The real-PTY output and stop regressions failed against the synchronous implementation and passed after the driver change. The HTTP cancellation regression failed before delivery finalization was detached and passed afterward. All 1,596 Rust unit tests and four reporter integration tests passed; three unit tests remain ignored. Rust build/format checks, the docs build, 17 Node checks, reporter shell diagnostics, and branch-policy checks passed. Independent `/code-review medium` found no actionable issues.
