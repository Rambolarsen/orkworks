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

## Review follow-up (substantial cycle 1/3)

Current-head Codex review identified that cancellation after the final successful write could report failure despite a submitted prompt. A deterministic gated-writer regression reproduced the failure. Preserve the successful delivery acknowledgement after every byte has been written, including when cancellation races the return or flush; keep cancellation effective while bytes remain. The pinned portable-pty writers issue unbuffered native writes and have no-op flush methods on Unix and Windows. Full verification and fresh current-head reviews follow before merge.

Linux CI exposed a native-write limitation: the stop event completed, but the input syscall remained blocked after the child and last slave closed. Local Linux reproduction confirmed the failed acknowledgement wait rather than a blocked stop. Keep the uncertain reservation, document that stop does not promise immediate retry on Linux, and run the verified stop/failure/retry regression on macOS. Unix output-backpressure and platform-neutral cancellation regressions continue to run on Linux. Revised macOS full-suite verification passed 1,598 unit tests and four reporter integration tests; fresh independent `/code-review medium` found no actionable issues.

Local Linux targeted delivery tests passed (four eligible tests). The broader container run initially exposed invalid Git mount paths, missing orphan reaping, and an existing wrap-ingestion timeout; those checks passed in prior GitHub CI. A corrected container rerun progressed past Git/reaping checks, but the VM stopped before final results. GitHub CI remains the full Linux merge gate.

## Delivery identity follow-up (substantial cycle 2/3)

Copilot identified lookup-by-session-ID dispatch after reservation and loss of finalization across a same-workspace reopen. Capture the original runtime sender and identity at reservation; use the captured sender and gate live input side effects by identity. Bind finalization to workspace path plus a weak advisory-lease reference, preserving same-workspace reopens while rejecting actual replacements without retaining the old lease. Unleased test stores retain instance identity checks. Regressions delay dispatch across runtime replacement and reopen the live workspace before write acknowledgement.

Current-head review cycle 2: verified and fixed stop publication before startup
subscription and the driver-check/writer-dispatch race. Both regressions failed
before the fixes and passed after. The blocking writer reads the shared watch
at admission; a previously admitted native call remains uncertain until return.
The accepted-input generation-check race also reproduced; input bookkeeping and
resume admission/rollback now share the existing projection gate, held only
through synchronous effects and released before awaiting delivery.

Codex original-store suggestion is only applicable while the original advisory
lease remains active. Same-workspace reopen preserves that lease and is covered
by the new finalization regression. Finalizing after genuine lease release would
violate ADR 0052's exclusive metadata ownership and Taskmaster's rejection of
late/cross-workspace results; ADR 0060's genuine switch closes the old sidecar
before opening the destination. The reacquired-lease regression verifies late
results do not mutate the new owner. Missing-target recovery remains the
documented path for abandoned reservations; no old-store write bypass is added.

Cycle 2 final validation: all 11 delivery regressions and the existing staged
resume-cancellation regression passed. The final full macOS run passed 1,604
unit tests (3 ignored) and 4 reporter integration tests. Rust production build,
format check, diff whitespace check, and documentation build passed. Fresh
independent `/code-review medium` approved the revised diff with no required
changes. Its optional startup write logging suggestion does not affect the
delivery reservation or lifecycle guarantees and is not a merge blocker.
