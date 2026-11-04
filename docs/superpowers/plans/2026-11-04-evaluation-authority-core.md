---
type: Implementation Plan
title: Non-I/O evaluation authority core
description: Approved first-unit execution plan for private in-memory claim, close, response and disposition ordering without runtime or storage adoption.
tags: [orkworks, hierarchy, evaluation, authority, implementation]
status: draft
workflow_status: active
---

# Non-I/O evaluation authority core

Tracking: [#1376](https://github.com/Rambolarsen/orkworks/issues/1376), under #1176 / #738.
Spec: [reviewed authority-owner design](../specs/2026-11-04-evaluation-publication-authority-design.md).

## Approval and pass condition

After PR #1372, reviewed head 67630684b00e514f2e8e6dc39cb8416138707d42,
merged ae417f10f461f6fec5c0e11a7ef2b542a59a08fb, the owner answered “lets keep goin”
to the explicit approval request for the new owner design and its non-I/O first
unit. This approves inline implementation and tests here. It does not approve
AppState/lifecycle wiring, live source admission, SQL, storage activation or probes.

Pass: deterministic tests prove the private core's exact claim/close/response and
settlement invariants; production cannot create synthetic bindings/readiness;
required repository checks and current-diff review pass. This proves no source
truth, complete transaction, actual stop latency, native lease or runtime profile.

## One coupled unit and interfaces

- Create `crates/orkworksd/src/taskmaster/publication_authority.rs` and its
  `publication_authority/tests.rs`; add only a private module declaration in
  taskmaster/mod.rs. No production caller or additional dependency.
- Authority owns one Arc-held inner Mutex state, fresh 32-byte getrandom identity,
  last issued u64 sequence and at most one pending exact claim. Entropy happens
  outside the latch; errors/poison/exhaustion fail closed without resets.
- Fixed workspace/namespace/run scope handles and immutable binding handles
  belong to one owner. Scope close and binding Unissued→Open→Closed are monotonic;
  closing Unissued blocks late registration. No session-ID map or per-receipt log.
- First-unit binding context and intent bytes are immutable opaque synthetic
  values. Their scope/binding/readiness factories are private `cfg(test)` seams;
  no production caller can supply an eligibility boolean or substitute them for
  actual retained P/T/configuration/target/runtime proof. Future source adoption
  must introduce qualified construction under a separate plan.
- `register_source_binding(&Binding)`, `close_binding(&Binding)` and
  `close_scope(&Scope)` compare exact owner/reference under the latch.
  `select_workspace(&Scope)` closes/detaches the old workspace scope; a retained
  pending claim still occupies the sole slot after a switch.
- `try_claim(ReadyEvaluation)` consumes readiness and returns a non-Clone,
  non-serializable ClaimedEvaluation with exact immutable intent and existing
  internal ClaimIdentity. Denials consume no sequence; successful claims do,
  including later rollback. A foreign owner/scope/binding cannot alias a claim.
- `settle(&ClaimedEvaluation, Disposition)` accepts only the exact installed
  claim: Unknown retains/fences it; Committed/RolledBack retire it without
  reopening any binding/scope. The disposition seam is synthetic/private; it
  certifies no SQL outcome. Dropping a token or waiter has no settlement effect.
- `commit_response(ReadyResponse)` returns its exact immutable buffer after the
  separate binding/scope cutoff. A committed buffer may deliver after close;
  an unresolved uncertain claim fences its affected workspace. This is the
  authority cutoff only, not a read-content/source admission implementation.
- No allocation, content copy/hash, external callback, resource destruction,
  nested core lock, channel send, I/O or backend work under the latch. Move any
  retired Arc/resource out of the guard before dropping it. Scope ancestry is
  fixed-depth; no traversal over the session inventory.

## Execution and verification

- [x] Write behavioral tests and minimal compiling API stubs; run the focused
  suite and observe expected failures before implementing authority behavior.
- [x] Implement terminal registration/close and owner/scope identity, exact sole
  claim/checked sequence, independent response cutoff and definitive settlement.
  Keep synthetic factories test-only and types private.
- [x] Cover revoke-first/claim-first; close-before-register; stale close versus
  replacement; each enclosing scope; foreign owner/workspace; changed readiness
  and a second request; token/waiter drop; Unknown→proven settlement; rollback
  sequence holes; workspace switch; response before/after close; injected entropy
  failure, poisoned state and u64 exhaustion. Use barriers/channels with bounded
  test waits to prove close can finish while a fake worker is parked outside the
  core; do not use sleeps as ordering evidence or assert the fake backend itself.
- [x] Run fmt/build/focused tests, then `bash scripts/verify-repo.sh` under Node 22
  with frozen desktop/docs dependencies and host fs-ext ABI before Rust tests.
  Inspect diff/caller inventory, docs/worktree checks and baseline warnings.
- [x] One independent fresh read-only Luna whole-branch `/code-review medium`
  (security-sensitive in-memory ordering): approved, no findings. PR/CI and owned
  cleanup follow the repository lifecycle; their terminal evidence is in the PR.
  Keep #1176 open.

## Plan review and risk

Self-reviewed once against the approved first-unit oracles: each lifecycle,
identity, lifetime, failure and response requirement has a behavioral test above;
actual source/native/ordinary adoption is excluded, not guessed. Scope/intent
factories remain synthetic instead of inventing another source codec. Dependencies
2 (std Arc/Mutex/atomics and existing getrandom); blast radius 1 (private dormant
module); durable state changes 1 (none); reversibility 1 (remove unused module);
uncertainty 3 (new concurrency logic, mitigated by deterministic races and review).
Native/adoption risk remains Unknown outside this unit. Ready for the approved
bounded implementation; no additional design or execution approval required
within this boundary. Root is the sole writer; no implementer delegation.

## Execution ledger

Task 1: implementation, full verification and current-code review complete. Initial red run had
20 expected failures and one passing entropy-refusal guard. The first 21 tests
then passed. An additional retained-owner test failed while the claim held only
a Weak owner reference; the exact claim now retains Arc ownership. Final focused
suite: 23 passed, none ignored. Simultaneous contenders install only one claim
and consume one sequence; deterministic close/disposition tests park the fake
worker outside the core.

Ruling: synthetic immutable source/intent buffers and all binding/readiness
factories stay in private test seams; they are not a new source codec or live
eligibility proof. Claim owner retention is part of the approved lifetime, not
an extra authority. The unit proves per-owner exclusion; sidecar-wide sole-owner
construction, real lifecycle/source/lease and storage adoption remain gated.


Full `bash scripts/verify-repo.sh` passed: Rust fmt/build, 2,852 unit tests
(17 existing ignored) plus integration suites, 20 OpenCode reporter tests,
desktop type/native/tests/build (1,689 passed, three existing skipped), 1,202 docs
tests/site build, diff and documentation/worktree checks. Baseline compiler/Node/
VitePress warnings are unrelated. User/onboarding and architecture currency
warnings need no public change because the module is private and dormant, with
no runtime/API/metadata boundary change. Foreign worktrees remain untouched.

Independent `/code-review medium` against e89fdc4fedaee691297710d2e52547f5110e831e
and staged tree 2c62c4dc74bd85f2859dd2f78c7e1002276f77fd approved with no findings.
The parent ran tests/build; the reviewer inspected the diff/tests and source
boundary. Final code blobs are checked against that reviewed tree before commit;
only this documentation result record changes afterward. No source/native/store
qualification or actual ordinary stop-latency claim follows.


## Namespace response-fence correction — 2026-11-04

Tracking: [#1384](https://github.com/Rambolarsen/orkworks/issues/1384), after the
first unit merged in #1379. A deterministic probe found that closing namespace
N1 with a pending claim still allowed a prepared response in N2 under the same
workspace. The response cutoff compares workspace but not namespace ancestry.
This violates the [accepted namespace-rotation requirement](../specs/2026-10-24-hierarchy-authority-ordering-design.md#lifecycle-and-ownership-decisions);
it is a bounded correction within the approved non-I/O unit, not a new policy.

Root owns this coupled fix in publication_authority.rs and its tests:

- Add a regression that denies N2 responses until N1's claim reaches proven
  commit or rollback, with new claims still blocked by the sole pending slot.
  Preserve same-namespace ready responses, including from a sibling run, and
  the existing uncertain-disposition fence.
- Compare exact immutable namespace ancestry at the current response cutoff,
  alongside the existing workspace check. Use the fixed-depth scope structure;
  perform no allocation, content work, callback or destruction under the latch.
- Run focused authority tests, required full repository verification and a
  fresh current-diff manual review at medium effort for authority ordering.
  No production caller/factory, AppState/lifecycle, SQL, native or source grant
  is added; the storage admission fence stays unchanged.

Plan delta reviewed once: scope and checks cover the reproduced response gap
and preserve sibling-run behavior without changing claim/settlement semantics.
Dependencies 1 (existing scope ancestry); blast radius 1 (private dormant core);
durable state changes 1 (none); reversibility 1 (local code revert); uncertainty
2 (reproduced, qualified only as in-memory ordering). Sum 6. Ready for this bounded
correction; actual source/native/runtime adoption remains outside its approval.


### Correction execution evidence

The repository regression first failed at the rotated-namespace unavailable
assertion (24 passed, one failed), then all 25 focused authority tests passed.
Independent current-diff `/code-review medium` found no actionable issues;
reviewed Rust blobs are `d6f94d7a4807a0d66775b9c45991a304a711099d` and
`60fbf8c8aabcdc5b686c6d08164405cef484f214`, unchanged after review.

Required `bash scripts/verify-repo.sh` passed: Rust formatting/build, 2,864 unit
tests (17 ignored), seven integration tests, 20 reporter tests, desktop type
check, native Electron test, 1,689 desktop tests (three skipped), desktop build,
1,202 docs tests, docs build, diff, documentation and worktree checks.
The first attempt was interrupted after stack inspection identified an existing
Peon fixture guard/shutdown deadlock, tracked separately in
[#1387](https://github.com/Rambolarsen/orkworks/issues/1387). That fixture passed
alone and in the successful full retry; the initial attempt is not passing evidence.

Documentation warnings need no public edit: this dormant private core changes
no user/API/installer claim. The pre-commit worktree warning reflects the owned
branch still sharing its base commit; cleanup follows this PR's terminal state.
Existing compiler, Node color and bundle warnings are unrelated to this diff.
No production caller or runtime/source/native/store qualification was added.
