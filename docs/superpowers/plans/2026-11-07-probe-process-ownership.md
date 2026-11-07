---
type: Implementation Plan
title: Owned short probe implementation
description: Implement the approved process ownership and cleanup contract for issue 694.
tags: [probes, process-ownership, implementation]
status: draft
workflow_status: active
---

# Owned short probe implementation

Deliver [the approved #694 contract](../specs/2026-11-07-probe-process-ownership-design.md).
Owner approval: 2026-11-07 “go ahead” after #1511’s concrete approval request.
Pass condition: migrated help/version/path probes clean their native owned set,
preserve caller policies and pass Unix/Windows synthetic fixtures and provider
regressions. Same-user escape limits remain explicit.

## Owners and interfaces

Root owns `probe_process.rs` and its Unix/runtime tests, three caller migrations,
`main.rs` shutdown hook and documentation. One isolated lower-tier Windows worker
owns `process_job.rs`, the provider Job re-export and Windows probe fixture file.
Root integrates its commit. Maximum two implementers plus one fresh final reviewer,
one review/fix round. No installed-tool or hosted inference calls.

`ProbePolicy { max_bytes, reject_overflow, require_success }` selects capture
policy; `run(&mut std::process::Command, policy, Duration)` returns bounded
`CapturedOutput { stdout, stderr }` or typed `ProbeError`. A testable
`ProbeService::new()` owns 64 active/retired permits and one bounded reaper.
Windows `ProcessJob::new/attach_and_resume/terminate` retains existing semantics;
shared `peek_exit(&Child)` observes exit without invalidating the process handle.

## Delivery

- [x] Reproduce non-exec descendant survival through an existing caller; retain
  fixture-owned cleanup so failing tests leave no process behind.
- [x] Implement process owner, cap admission, async pipes, non-consuming Unix
  observation, cleanup settlement/drop/reaper and one-second shutdown. Test
  success, cap/overflow, timeout, cancellation, I/O setup failure, leader exit,
  saturation, unconfirmed cleanup and escaped-session limit; sentinel survives.
- [x] Extract Windows Job primitive without provider behavior changes; add
  native synthetic wrapper/descendant and setup-failure fixtures.
- [x] Migrate detect version/help, Codex isolation and OpenCode path probes;
  preserve exact environment, parsing, diagnostics, cache/generation and resume.
- [ ] Update ADR 0072/runtime/harness/user docs and approval record. Verify Rust
  build/tests/fmt and full repository verifier, docs tests/build, scoped review
  and PR CI. Close #694 after final evidence and merge; clean owned worktrees.

## Review

Ready after self-review against each contract section. Dependencies 3 (three
known callers/platform primitive), blast radius 3 (short probes), state change 1
(no durable schema), reversibility 2 (revert source; no migration), uncertainty 3
(native platform fixtures required). Total 12/25. Review focus: no reaping before
Unix group signal, cancellation without repoll, partial Windows setup, exhausted
cleanup budget, and saturation retaining permits. Each has a fixture owner above.

## Evidence ledger

- Isolated worktree reported; base c32a36cc. Contract approval recorded. Only
  synthetic fixtures are authorized; ordinary host-access limits remain.

- Regression: existing version timeout left its non-exec descendant writing
  heartbeats (RED), then passed after shared ownership (GREEN). Prefix capture
  exposed pipe handles dropping before cleanup; retained them until termination.
  Darwin also returns EPERM for an unreaped zombie-only group. Native fixture
  verified WNOWAIT retains identity and exact-group proc_listpids returns the
  original zombie leader. Accept that case only after a bounded exact-group
  query finds no member other than the observed exited leader; all other EPERM
  cases remain unconfirmed. This implements the contract's anchored absent-set
  case without acquiring cleanup authority from a PID scan.

- Expanded native tests: 13 macOS ownership cases pass, including setup failure,
  cleanup unconfirmed, retained capacity, lost leader identity, remaining-budget
  settlement, successful descendant cleanup and the explicit setsid escape.
  The capacity test failed before retaining the permit until owner removal.
- Fresh `/code-review medium` approved the current code after one fix wave:
  serialize Windows shutdown/Job attachment and replace Windows blocking stdio
  adapters with overlapped named pipes. Reviewed code fingerprint SHA-256
  `b0af11591a413437880d685280ec55d12565ed5cd7b51b04a868683b8162a94d`.
  Framing: binary diff from c32a36cc over Rust source and workflows, then each
  untracked probe_process.rs, probe_process/tests.rs, probe_process/windows_pipe.rs
  repository-relative path, NUL and bytes in that order. Native platform execution
  remains the implementer's CI gate; ordinary same-user escape limits are accepted.
- Windows GNU cross-check compiled the actual shared Job, probe runner, overlapped
  pipes and all Windows fixture/test modules with pinned dependency versions.
  This is compile evidence, not native Windows execution. Docs tests: 1242 pass.

- First full Rust run: 3173 passed, 18 ignored, one unchanged image-context lock
  fixture failed with EWOULDBLOCK at ImageBundle::publish; exact isolated rerun
  passed. Full verifier rerun uses four test threads. Do not count the first run
  as green. Docs build passed; existing chunk warnings do not concern this diff.
  Doc currency warning requests README review: its product/architecture overview
  remains accurate, so no README edit is needed for this private runtime boundary.

- Final startup/shutdown edge regression failed before the fix (shutdown before
  first service use still attempted spawn), then passed after shutdown publishes
  and closes the shared service. All 14 ownership tests pass. Scoped manual
  review approved this two-file delta from 677bd1ab, SHA-256
  `9d7bb8ad25683cb288a90086f0a905dbfe6611478060730ce2ccda7e1263a887`
  over `git diff --binary 677bd1ab -- crates/orkworksd/src/probe_process.rs
  crates/orkworksd/src/probe_process/tests.rs`. No contract change.
