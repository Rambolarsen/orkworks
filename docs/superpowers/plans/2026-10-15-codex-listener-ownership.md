---
type: Implementation Plan
title: Codex connected-listener ownership implementation
description: Implement a dormant macOS proof only after approval and resolve the missing native-TUI identity contract before integration.
tags: [orkworks, codex, security, plan]
status: draft
---

# Codex connected-listener ownership implementation

Outcome: advance [#763](https://github.com/Rambolarsen/orkworks/issues/763)'s
listener-identity prerequisite for the [Needs You fix](https://github.com/Rambolarsen/orkworks/issues/690).
Spec: [listener ownership design](../specs/2026-10-15-codex-listener-ownership-design.md).
Status: **local proof approved by the owner on 2026-10-16 and implemented under
[#897](https://github.com/Rambolarsen/orkworks/issues/897); full integration remains
blocked on a native-client identity contract**. The helper is dormant and does
not remove production gates.

## Constraints and compatibility

Preserve the approved native runtime, capability authentication, isolated
execution environment, native approval control, configuration routing, exact
resume, and generation/executable fences in ADR 0076. Keep `VERIFIED` empty and
the metadata-clear gate closed. Initial kernel proof covers macOS arm64 only;
other targets return unavailable. Do not add a relay: upstream TUI reconnection
after sidecar crash invalidates that approach. No Unix transport, shared
daemon, readiness-as-identity, or inherited-socket assumption is introduced.

## Deliverables and dependencies

- [x] **Dormant connected-stream proof, approved 2026-10-16.** Add
  `crates/orkworksd/src/runtime/codex_native/listener_identity.rs`, registered
  by `codex_native.rs`. It consumes the retained owned-child identity and the
  already-connected stream's reversed four-tuple, returning proved ownership
  or unavailable. Kernel ABI details remain private to this owner. First write
  failing tests for an accepting unrelated child; implement exact macOS
  established-socket matching with bounded inventory and complete-size checks.
  Cover positive ownership, wrong tuple/PID/state, truncation, permission
  denial, exit, and checked conversions. Other targets fail closed and compile
  in platform CI. This deliverable does not remove any production gate.
- [ ] **Native-client contract investigation, before integration.** Audit and
  obtain an upstream-reviewed mechanism that proves immutable server process
  and birth identity on the actual TUI stream before every bearer upgrade.
  The pinned TUI retries for 120 seconds; include owner crash and PID reuse.
  No unsupported flag or new upstream behavior may be inferred. Record the
  exact supported version/source and obtain approval of any resulting
  upstream transport or identity amendment before dependent runtime work.
- [ ] **All-client integration, blocked on the previous contract.** Once
  supported, update `runtime/codex_native/launch.rs` and `protocol.rs` so the
  observer upgrades the same proved stream and the TUI receives the reviewed
  immutable identity parameters. Every reconnect repeats proof. Reproduce
  accepting-competitor exposure first, then require zero upgrade bytes for
  observer and actual TUI, including a surviving TUI after sidecar death.
  Verify bounded startup/cancellation/cleanup and conservative attention.
- [ ] **Installed evidence, after implementation and review.** Update
  `runtime/codex_native_diagnostic.rs` and the existing
  [verification record](../verification/2026-10-05-codex-native-approval-status.md)
  only after all-client ownership is established. Exact binary/host evidence
  must include acceptance-before-HTTP, capability authentication, crash and
  reconnect rejection, descendant cleanup, and prompt survival across observer
  disconnect. Model turns/manual interaction retain existing operator approval
  requirements. Keep #690/#788/#789 gates explicit; fake fixtures do not qualify
  installed binaries.

Execute authorized deliverables inline under `executing-plans`, with TDD for
runtime code and deterministic barriers for races. No subagents or additional
coding harnesses are planned. Review a new 0.162.1 initialization contract
separately if needed; it cannot inherit the audited 0.160.0 contract.

## Verification and close-out

Run focused proof tests, scoped Rust build, full tests and fmt check, then
`bash scripts/verify-repo.sh`. Provision the repository Node version and frozen
desktop dependencies for full Rust tests. Run docs checks/build and both
close-out hooks. Security/lifecycle code requires `/code-review medium` and
required CI on the exact PR head. Do not claim #763 or #690 resolved or enable
production startup/clearing from a helper-only implementation.

## Risk and inline review

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 5 | Full startup requires a currently absent native-client contract; local helper stays dormant. |
| Blast radius | 3 | Future native startup/reconnect/shutdown; no rollout in the local slice. |
| State changes | 1 | No persisted schema or metadata migration. |
| Reversibility | 2 | Dormant helper can be reverted; no live conversation is replayed. |
| Uncertainty | Unknown | Fake-server kernel proof passed; no audited all-client crash-safe mechanism exists. |

Assessment incomplete; scores do not grant approval. Disposition:
**Investigate native client support; local proof ready for owner review**.
Review caught the relay-owner crash gap by tracing the actual upstream TUI
reconnect implementation. Do not execute the blocked integration steps.

Original documentation-only handoff checks: OKF validation, 20 OKF tests, docs build and diff
whitespace checks passed. The build reported circular/large-chunk warnings in
unchanged VitePress bundling. The doc-drift hook suggested README review because
an ADR changed; no edit is needed because this draft changes no public feature,
milestone, or availability claim. The worktree hook reported foreign stale or
merged worktrees; they were left untouched. Its initial merged-branch notice
for this fresh branch preceded any commit and was the common-base condition,
not a merged PR. At that handoff, no Rust source changed, so Rust/full-repo code validation was
not run.

## Local proof implementation evidence

The private Rust module retains the connected stream and borrows the unreaped
owned leader; a small SDK-compiled C adapter decodes socket structures without
hand-copied private ABI layouts. Kernel queries run on the blocking pool with
bounded inventory, deadlines and cooperative cancellation. The guard rechecks
leader liveness before exposing that same stream. No production caller,
compatibility entry, installed diagnostic, native TUI or metadata-clear gate is
changed. This cannot establish immutable TUI identity after sidecar death.

Initial behavioral regressions failed before implementation: an accepting
competitor received fixture bytes, an exited leader retained access, and invalid
inventories were accepted. They pass with the proof. Additional coverage checks
inventory growth, ambiguity, denied/partial/oversized replies, deadline and
cancellation, plus the actual SDK decoder's state/protocol/size/port checks.
The focused macOS arm64 CI workflow discovers the expected live-child and SDK
tests before running them; Linux/Windows builds keep the native proof unavailable.
Full repository verification and exact-head medium review follow before merge.

Local close-out: full `scripts/verify-repo.sh` passed, including the supported
macOS proof and SDK decoder, all Rust tests, desktop type/build/tests, docs build,
and diff/doc/worktree checks. The exact CI discovery and test commands also
passed locally. The read-only medium review found no required changes, including
its follow-up on replacement/ambiguity/unsupported tests and native CI routing.
Docs warnings need no public-guide/root-rule edit because this dormant helper
adds no available feature, and the private build dependency changes no universal
agent rule. A foreign merged worktree warning was left to its owner.

## Review cycle 1 of 3

Current-head Codex review identified the accepted-descriptor visibility race:
TCP connect can finish before the child accepts and acquires a connected FD.
A deterministic regression first failed because a complete zero-match snapshot
returned unavailable immediately. The scanner now retries those snapshots with
a bounded five-millisecond pause, within the existing three-attempt and shared
deadline limits. Ambiguous results still fail immediately. The regression also
requires a permanently absent descriptor to stop after exactly three snapshots.
The custom review's desktop IPC finding was a reverse comparison of intervening
main changes from #899, not part of this PR; the GitHub PR file inventory and
prospective merge result confirm those changes are retained without a delta.

Cycle 1 validation: all ten proof tests pass. The first full rerun encountered
the existing readiness-cancellation PID/reaper race tracked in
[#901](https://github.com/Rambolarsen/orkworks/issues/901); the fully qualified
case passed in isolation, then one complete retry of `verify-repo.sh` passed
with 1,995 Rust tests (five ignored), the SDK/reporter/producer suites,
1,322 desktop tests (one skipped), and build/docs/close-out checks. No readiness
or reaper production path was changed. Scoped medium review found no required
changes in the visibility fix.
