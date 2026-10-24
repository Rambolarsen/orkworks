---
type: Plan
title: Ordered completion relay implementation
description: Build and verify the approved bounded completion queue and source publisher while preserving rollout gates.
tags: [codex, hooks, runtime, protocol]
status: draft
---

# Ordered completion relay implementation

Implement the [approved wire design](../specs/2026-10-24-ordered-completion-relay-design.md)
for [#690](https://github.com/Rambolarsen/orkworks/issues/690). Pass condition:
real publisher subprocesses and the retained Rust consumer agree on contiguous
publication/acknowledgement, and every incomplete/failing transport preserves
conservative attention. No production activation is part of this change.

## Routing and ownership

One round, at most one Luna implementation worker plus two fresh read-only
verifiers (four active agents including the owner). The Python publisher and
Rust consumer consume the same frozen wire contract; neither task consumes the
other task’s implementation. Each writer uses an isolated owned worktree.

- Worker owns only `scripts/codex-completion-report.py` and its `.test.py` under
  `crates/orkworksd/`. Real subprocess tests initialize literal header fixtures;
  observe RED before code, test serial/parallel publication, malformed input,
  URL-free sanitized errors, lock timeout, in-flight ownership, overflow and
  no-fallback poison behavior. No runtime or installation edits.
- Owner owns `runtime/codex_hook_report_relay/completion.rs` and its module
  declaration, cross-boundary tests and docs. Reuse the retained parent directory
  implementation, fs2 flock and descriptor-relative atomic I/O. Test sequence,
  acknowledgement, corruption, stale identity, no-follow/lock replacement,
  in-flight blocking, live fences, overflow and one-use application behavior.
- Owner integrates the worker’s bounded commit and runs actual producer → Rust
  consumer → completion application/store coverage. Legacy identity/approval
  mailbox paths remain unchanged. Existing native and completion gates stay shut.
- Correctness verifier checks current diff and security/concurrency invariants;
  coverage verifier hunts untested failure paths against the written contract.
  Owner reconciles findings once, then runs required checks and completes a PR.

## Delivery

- [x] RED/GREEN publisher and pure consumer tests, with actual durable files.
- [x] Merge and test cross-language transport with the real application/store.
- [x] Update architecture/contracts and record no production callers or rows.
- [x] Run full verify-repo, scoped review and close-out checks.
- [ ] Complete PR lifecycle and owned-worktree cleanup.

## Review and risk treatment

Ready under existing owner approval. Dependencies 3 (cross-process contract and
installed qualification); blast radius 2 (dormant isolated queue); state changes
2 (new private queue, no metadata/schema migration); reversibility 1 (no launch
caller); uncertainty 3 (filesystem durability and native callback completeness).
Total 11. Failures latch assistance off; no old stream is adopted after restart.
Missing upstream callbacks remain an explicit qualification failure, not
something a local counter can certify. No new package dependency is needed.

## Evidence ledger

The initial Rust stream initialization regression failed before its lock/header
were created. The inert consumer then failed sequence acknowledgement and
in-flight blocking checks. A later privacy regression exposed tempfile’s default
umask-dependent directory mode; creation now requests 0700 atomically. Counter
exhaustion and raced queue replacement regressions failed before their guards.
The publisher worker observed RED before implementation, then passed 17 real
subprocess tests. Owner integration review additionally reproduced FIFO-open
hangs and inode replacement adoption; the bounded worker fix pass added
nonblocking regular-file checks, bounded directory scans and null-root rejection.

The combined focused run passed 47 Rust tests plus the publisher integration
suite. A later direct cross-language lock test passed: the real Python publisher
observed the retained Rust flock, could not create its marker during the
application callback, and published sequence 2 only after acknowledgement.
The real publisher → queue → completion application/store test persisted the
permission wait, then cleared only the matched completion to working. The
17-test publisher suite also passed in the ordinary restricted tool sandbox.

Ruling: marker creation/removal use the retained queue lock as well as atomic
state replacement, so a producer cannot appear between the consumer’s marker
check and callback. The consumer validates retained directory/lock identity,
strict bounded JSON, source root/fingerprint/generation and the original queue
inode/bytes before acknowledgement. Failure/panic latches its in-memory fence;
no adoption or reconstruction path exists. Native hook completeness and fresh
launch qualification remain unverified and no production stream is created.
The initial full verification passed; final evidence follows below.

The first full verify-repo run passed all checks. The completeness verifier then
found a material late-failure race: after marker unlink succeeded but directory
sync failed, cleanup released the lock before restoring failure evidence. A
real Python publisher/Rust consumer regression reproduced an application call
through that clean window. Cleanup now publishes poison/restores its marker
before unlocking; restoration also takes the shared lock when needed. The
regression and combined focused run passed (49 Rust tests plus the 17-test
publisher suite). Full verification and current-diff review are being repeated
for this one bounded feedback fix. No production gate changed.


Both fresh verifiers confirmed the corrected code diff without further material
findings. The medium correctness review covers code digest
`8e90a3cfcd69a473a90d137508b185826af11dd53341bebbb18603ad1e289ec0`
against base `75de6d2c31dfa108d039d56dbf08da725481b28b`. The separate
coverage reviewer confirmed the real cross-process regression closes the
late-failure gap. Reviewers inspected code/tests; the owner ran the checks.
This records a current-session review, not execution of an unavailable native
slash tool.

The first final full run failed in the unchanged Codex owned-install trust
fixture (`Unknown` versus `NeedsTrust`), then 77 tests failed on the shared
poisoned fixture mutex. That trust test passed immediately in isolation.
The cause of the intermittent trust result was not established; no unrelated
Taskmaster/trust implementation was changed. The complete rerun passed: 2,197 Rust tests, all subprocess integrations,
OpenCode tests, desktop type/native/tests/build, docs build and close-out
checks. The code digest remained unchanged through review and verification.
Documentation currency warnings for public docs/README need no edits: this
dormant source transport changes no shipped capability or user flow; canonical
architecture, ADR and runtime/harness contracts were updated instead.
