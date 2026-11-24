---
type: Implementation Plan
title: Private pair bootstrap state tests
description: Conditional offline state-owner and fake-API test slice for accepted build and startup custody, without SQLite calls or runtime wiring.
tags: [orkworks, sqlite, hierarchy, bootstrap, qualification]
status: draft
workflow_status: planning-ready-execution-gated
---

# Private pair bootstrap state tests

Outcome: implement and test the private bootstrap transition owner that a later
qualified startup adapter can use, without installing a router or qualifying a
pair. Pass means deterministic tests exercise that same owner through an in-memory
fake boundary; a second standalone eligibility model does not satisfy this plan.
Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).

**Ready as a bounded planning handoff; Investigate before implementation.**
[ADR 0119](../../adr/0119-pair-build-startup-custody.md) and the
[accepted recipe](../specs/2026-11-24-pair-build-startup-design.md#scoped-build-and-startup-acceptance--2026-11-24)
settle the build/startup choice for further qualification. This plan does not
request that choice again. The existing
[qualification gates](2026-11-24-pair-resource-assurance-qualification.md)
remain binding: Gate 1 source closure and bounded slice execution authorization
must precede code. A fake boundary cannot close unknown native behavior.

## Boundaries and prerequisites

Inspection base: `74ed8719c069d7b415f9e32dc2abf4bfd7bad8a7`.
The planned private `session_store::joint_preparation` owner is not yet present.
[Hierarchy Connection::open](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs)
continues rejecting the unqualified profile before I/O.

- No real SQLite configuration, initialization, allocation callback, SQL, open,
  database, fixture, target metadata retry, native/compiler/model probe or budget
  renewal. No main.rs, build flags, Cargo dependency or release workflow edits.
- No source permission, production eligibility token, generic Connection, public
  callback table, caller-supplied qualified boolean or persisted startup state.
- No arena allocator, pointer-routing implementation, EOF ledger, VFS adapter,
  worker integration or ordinary consumer migration. These retain separate owners
  and gates; fake events cannot establish their memory or lifetime guarantees.
- Keep all [ADR 0118](../../adr/0118-pair-resource-assurance.md) capacities and
  source/cutoff/newness/catalog/initializer/recovery/durability/activation gates.
  Do not add restrictive defaults or silently treat missing build facts as known.
- Actual shared build identity, configuration-writer custody, cache/lookaside
  routes, native adapter and ordinary compatibility remain unresolved in the
  [source map](../../validation/pair-resource-enforcement-coverage.md).

Before execution, record the exact source-closure evidence and authorized test
surface on #1176. Neither merging this plan nor a successful fake test satisfies
Gate 1. If a narrower sequencing exception is desired, resolve it explicitly
against the parent gates before editing; this plan grants no such exception.

## One coupled deliverable: private transition owner and regression tests

Planned files:

| File | Responsibility |
| --- | --- |
| `crates/orkworksd/src/session_store/joint_preparation/bootstrap_state.rs` | Private phases, recorded outcomes and ordered bootstrap effects; no FFI implementation |
| `crates/orkworksd/src/session_store/joint_preparation/bootstrap_state/tests.rs` | In-memory fake effects and transition/event assertions |
| `crates/orkworksd/src/session_store/joint_preparation/mod.rs` | Private module declaration only |
| `crates/orkworksd/src/session_store.rs` | Test-only private module wiring until runtime integration is separately authorized |

If another task creates the parent owner first, extend its actual private module
rather than duplicate it. Keep the entire dormant subtree under test-only wiring
for this slice. Do not expose it to callers just to avoid unused-code warnings.

The transition owner consumes private structured build/configuration observations
and ordered fake API outcomes. Observations distinguish missing, conflicting and
matched evidence, including source/dependency/artifact association, effective
macro absence (defined-zero is insufficient), THREADSAFE/core-mutex custody and
absence of unsupported overrides or earlier unknown writers. Synthetic facts in
tests are explicitly fake, not actual verified-build witnesses. This slice checks
classification and sequencing; collecting/authenticating real facts remains a
later source-owned adapter task.

The effect boundary covers only capture of the saved allocator, router-install
outcome and initialization outcome. Keep it private and unimplemented for real
SQLite in this slice. Preserve distinguishable known rejection and unknown
installation outcome: if installation might have taken effect, retain the saved
owner and mark failure sticky rather than freeing storage or retrying. Fake
handles identify all seven saved methods plus original pAppData, without creating
or calling C pointers. The owner records their retained association across later
failure; it does not claim that fake handles prove stable native addresses.

Use the accepted phases Unexamined, UnavailableWithoutRouter,
SavedBeforeInstall, DelegatingInstalled, InitializedDelegating and
FailedAfterInstall. Record whether a getter was attempted and could have filled
defaults; unavailable does not mean pristine. InitializedDelegating permits only
further qualification, with pair availability still denied by independent gates.
An ambiguous install is conservatively retained as FailedAfterInstall. No reset,
restore, shutdown, unregister or implicit retry effect exists in this boundary.

- [ ] Write failing tests for the cases below against the actual transition owner.
  Run the selected tests to establish a meaningful assertion failure, then add
  minimal transition logic and make them pass. Do not count a filtered zero-test
  run or a build error as the red/green result.
- [ ] Implement ordered outcomes and retained saved-owner identity in the private
  owner. Reuse its transitions directly in tests, with no parallel model oracle.
- [ ] Review the exact diff for forbidden FFI/runtime calls, public admission seams,
  unsupported defaults and accidental loss of retained state. Document what
  remains unproven before handing off native integration.

### Required regression oracles

| Input or failure | Expected result |
| --- | --- |
| Missing/mismatched artifact or source association; macro defined as zero; unsupported cache/thread/allocator configuration; unknown earlier writer | UnavailableWithoutRouter, no fake API effects, no qualified output |
| Matched synthetic observations | Exactly capture, install, initialize in order, with no pair context |
| Capture failure or unknown result | No install/init; retain exact attempted/default-fill observation; no restore |
| Successful capture followed by known install rejection | Saved association and getter effects recorded; no initialization or reset; unavailable without an installed router |
| Unknown install outcome | Saved association retained, FailedAfterInstall, no initialization or retry |
| Successful install followed by failed/unknown initialization | FailedAfterInstall, original saved owner retained; repeated attempt emits no effects |
| Successful initialization | InitializedDelegating, pair/source admission still unavailable; repeat attempt does not install or initialize again |
| Fake lifecycle event after successful install or later failure | Same saved method/pAppData identity remains available for later adapter delegation; bootstrap itself never invokes xInit/xShutdown |

The last oracle checks ownership bookkeeping only. Real callback ABI, concurrency,
non-arena growth denial, unknown pointer provenance, no unwinding/recursion,
process-lifetime addresses and partial native initialization require the later
router/native qualification. Do not add fake pointer arithmetic and call it arena
or cross-context provenance proof.

## Verification and execution handoff

Before code, load the Rust scoped guide, rust-skills and TDD; execute inline when
authorized. Run `cargo test --locked --offline -p orkworksd bootstrap_state`, expecting all
new named cases and a nonzero test count; a missing cached dependency is a stop,
not permission to fetch. Then run `cargo fmt --all -- --check` and the required full
`bash scripts/verify-repo.sh`. The execution authorization must explicitly include
Rust dependency compilation and existing full-suite tests that open databases;
otherwise do not start the slice or substitute a partial suite for required checks.
This is separate from native qualification and does not renew spent probe batches.
Record actual commands, counts, failures and any unavailable prerequisite.

Before PR handoff, request substantial-code review and complete the required
current-diff `/code-review low`; use repository CI routing and close-out checks.
The present documentation-only plan requires docs build, docs/site tests,
doc-check and worktree-check, without executing the proposed Rust tests.

## One-time plan review

Disposition: **Ready for planning handoff; Investigate for execution.** Scope,
owner boundaries and regression oracles cover the accepted fake-API state slice.
The plan distinguishes getter default fill, uncertain installation, retained
post-install failure and qualified admission. No new architecture acceptance or
native exception is inferred. The next action is source-only closure of the
remaining build/configuration routes, then explicit bounded execution authority.

| Risk dimension | Rating and evidence |
| --- | --- |
| Dependencies | 4: shared startup, linked build identity and later native/router owners |
| Blast radius | 2 for the dormant test-only slice; actual startup integration remains separately high impact |
| State changes | 1: proposed slice changes no durable or process-global SQLite state |
| Reversibility | 1: isolated dormant code can be reverted; native rollback is outside this slice |
| Uncertainty | 4: actual build/custody/native behavior is unqualified; fake tests cannot resolve it |

Total 12 is a judgment aid, not an approval threshold. Execution remains gated
regardless of score. #1176 remains open and production remains unavailable.
