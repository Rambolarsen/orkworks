---
type: Implementation Plan
title: Explicit branch document recovery
description: Preserve invalid branch records before a confirmed snapshot-bound reset.
tags: [orkworks, plans, recovery, branch-documents]
status: stable
---

# Explicit branch document recovery

Implement the owner-approved [recovery contract](../../../specs/session-plan-review.md#branch-document-recovery-amendment)
for [#1782](https://github.com/Rambolarsen/orkworks/issues/1782) on the owned
`branch-recovery-spec-1782` branch in PR #1847. Pass condition: a confirmed
reset preserves exact invalid bytes, clears both shared slots safely and allows
normal discovery, with stale/unsafe/failing operations covered by tests.

## Delivery and ownership

The current agent implements sequentially in this worktree, using TDD and one
whole-diff review. No second coding harness or concurrent writer is needed.

- [x] Store: add a focused `branch_documents_store/recovery.rs` module for
  bounded safe snapshots, per-store opaque IDs, exact-byte preservation and
  reset under the existing per-key lock. Stage/sync outside global guards;
  expose a final-admission callback around replacement only, followed by
  directory sync. Invalidate old prepared generations at visible publication.
  Test malformed/unsupported/identity-invalid, valid/missing/unsafe refusal,
  changed bytes, preserved copies, admission rejection and sync failures.
- [x] Application/projection/HTTP: expose additive `recoveryId` only for a safe
  invalid record, bound per row to existing `LocationInputs`. Retain the actual
  workspace lease throughout the action. Final admission compares lease/store,
  fresh Git context and location inputs under the existing short fence. Add
  authenticated POST `/sessions/:id/plan-document-reset`, requiring only a
  closed `{recoveryId}` body. Test stale location/adoption/record, no input-fence
  retention during backup, authentication and no path/branch injection.
- [x] Desktop: extend the existing plan transport/preload/window types and
  inspector with confirmed **Reset saved document choices**, cancellation,
  branch-sharing explanation, disabled pending controls, accessible errors and
  refresh after success or uncertain failure. Reuse generation cancellation;
  keep Electron/renderer types independent. Test transport validation and real
  rendered inspector interaction, including cancel, stale refresh and retry.
- [ ] Close-out: update canonical architecture/user guidance, this plan and
  acceptance status. Run focused tests then `scripts/verify-repo.sh` with Node22
  and CI `RUST_TEST_THREADS=4`; run final manual medium review for this new
  authenticated mutation and persistence ordering. Finish current-head review,
  CI, recommendation inventory and normal PR merge/owned-tree cleanup.

## Boundaries and review

No schema migration, automatic quarantine, general cleanup or restore/import UI.
Do not alter committed-patch success semantics. Original record and documents
remain untouched before a durable preserved copy; post-publication failure may
already have reset storage. Existing legacy references cannot recreate pins.

Review: **Ready**. Spec coverage maps to store, boundary and inspector tests
above. Dependencies 2 (existing Rust/Electron/renderer boundary); blast radius 3
(shared branch choices); state change 3 (explicit reset with preserved bytes);
reversibility 3 (manual original-byte recovery, no automatic restore); uncertainty
2 (native durable writer already verified in #1841). Total 13/25. Address risk
with exact-snapshot admission, retained lease, injected failure ordering and
native Windows CI. The owner approved the full contract; no repeat gate needed.


## Verification evidence (before integration)

Owner-approved implementation is complete. Final `/code-review medium` approved
the code diff against `6154e4e0fcfbf72ce445661f8cebf9ef6dd6e56e`, SHA256
`cdca6fdc235a71f43cb737044ad0365b853dfebcda1f15f8551cf7d1ad9251c4`,
with no remaining findings. TDD exposed and fixed absent recovery, transient
staging permission exposure and historical FIFO blocking before their fixes.

The final focused branch-document run passed all 55 tests. Desktop transport
and static rendering passed 14 tests; the real Electron recovery interaction
passed confirmation, cancel, stale refresh, duplicate-click and error retry.
The recovery code remained unchanged when rebased onto current main. Both
recovery and compact-menu Electron scenarios passed after preserving their
combined registration. The complete Node22 / CI-four-thread verifier ran: 3,593 Rust tests passed,
three failed and 21 were ignored. Two provider fixtures assumed unused port
49999, occupied by the host Codex runtime (tracked in #1862). The unchanged
SQL-origin byte-limited rewrite test expired under suite load, then passed in
an exact isolated run; no production deadline or assertion was changed.

All post-Rust verifier steps passed: 31 reporter tests, 1,776 desktop tests
(five skipped), 2,056 documentation tests, type-check/builds, documentation and
worktree checks. Clean required Rust CI and native Windows recovery tests also
passed. Final PR comments, CI and integration evidence are recorded in
[PR #1847](https://github.com/Rambolarsen/orkworks/pull/1847). No power-loss
hardware qualification is claimed.
