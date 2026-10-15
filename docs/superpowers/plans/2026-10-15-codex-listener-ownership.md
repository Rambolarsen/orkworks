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
Status: **local proof ready for owner review; full integration blocked on a
native-client identity contract**. No runtime code has changed.

## Constraints and compatibility

Preserve the approved native runtime, capability authentication, isolated
execution environment, native approval control, configuration routing, exact
resume, and generation/executable fences in ADR 0076. Keep `VERIFIED` empty and
the metadata-clear gate closed. Initial kernel proof covers macOS arm64 only;
other targets return unavailable. Do not add a relay: upstream TUI reconnection
after sidecar crash invalidates that approach. No Unix transport, shared
daemon, readiness-as-identity, or inherited-socket assumption is introduced.

## Deliverables and dependencies

- [ ] **Dormant connected-stream proof, after owner design approval.** Add
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

Documentation handoff checks: OKF validation, 20 OKF tests, docs build and diff
whitespace checks passed. The build reported circular/large-chunk warnings in
unchanged VitePress bundling. The doc-drift hook suggested README review because
an ADR changed; no edit is needed because this draft changes no public feature,
milestone, or availability claim. The worktree hook reported foreign stale or
merged worktrees; they were left untouched. Its initial merged-branch notice
for this fresh branch preceded any commit and was the common-base condition,
not a merged PR. No Rust source changed, so Rust/full-repo code validation was
not run for this documentation-only handoff.
