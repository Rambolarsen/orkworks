---
type: Implementation Plan
title: SQL recommendation producer revocation plan
description: Approved source-only delivery and verification of legacy operation revocation before SQL cleanup.
tags: [orkworks, sqlite, recommendations, authority, plan]
status: draft
workflow_status: implementation-active
---

# SQL recommendation producer revocation plan

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [proposed revocation contract](../specs/2026-11-08-sql-recommendation-producer-revocation.md).
Base: `02431980407875d6d55f7cd5844f84adeaf8510a`.
Pass condition: while the original shared owner stays alive, a stale legacy
writer cannot recreate recommendations after admitted SQL cleanup, and delivery
settlement and cleanup retain their owners. This is an owner-lifetime-only
prerequisite, not complete producer qualification or reopen safety.
The owner accepted this concrete plan after #1528 with “go ahead” on
2026-11-08. Execute inline under the accepted owner-lifetime boundary.

## Investigation and material choices

The existing shared owner solves per-call contention but has no SQL mode. Fixing
only evaluator apply misses assessment upsert/redaction, packets and rollback.
An ID blacklist is unbounded and cannot identify reused creations; a global epoch
alone does not validate newly captured stale sources. Prefer bounded fail-closed
revocation now, then separately qualify the SQL producer. Reuse the existing
canonical-root owner; no new database, marker, registry or schema.

Least-confident boundary investigated: legacy reads/constructors perform recovery
and migration, so blocking only writes is insufficient. Delivery settlement also
needs mutation authority after PTY acknowledgement, so transition must refuse
while any workspace delivery is in flight and claim/admit under the same short
workspace exclusion as delivery registration. Durable pending assessment records
contain recommendations without selected SQL creation proofs; do not remint them
from reused current IDs. Later producer representation remains a separate gate.

Risk: dependencies3 (application/store/delivery); blast radius3 (all legacy
recommendation operations); state changes3 (monotonic process-local authority,
existing file effects unchanged); reversibility3 (no in-place reset, explicit
qualified workspace reopen required); uncertainty3 (source traced, concurrency
matrix not executed). Total15/25. Dormant routing, explicit denial, no new wire
format, retained permit and targeted race tests bound this work.

## Delivery and acceptance

- [x] Record owner acceptance and the dated ADR0103 clarification before source
  implementation. Reuse approval within this exact scope thereafter.
- [x] Owner admission in `taskmaster/store/operation.rs` and `store.rs`: distinct
  private SQL grant, monotonic mode, nonblocking claims, typed permanent legacy
  refusal, constructor admission before effects and all legacy I/O routes covered.
  Test stale clone/independent open/read recovery/graph/packet/rollback refusal,
  distinct roots, Busy precedence, no new registry retention, and an explicit
  progress-failed/last-owner-drop/reopen counterexample. This first unit must not
  claim to fence the resulting fresh Legacy owner.
- [x] Captured integration in `session_application/sql_writes/deletion.rs` and the
  existing journal composite: under current app admission check no workspace
  delivery in flight, claim once, admit/revoke, hand that exact SQL grant through
  blocking recovery/fresh pass/receipt/progress. Keep bindings stage-specific.
  Test cancellation/expiry/pre-admission conflict leaves Legacy mode unchanged;
  admitted file/SQL errors and caller loss keep revocation and owners intact;
  exact-job retries work and input guards stay free.
- [x] Delivery concurrency in `session_application/recommendation_delivery.rs`
  and scoped tests: successful and failed delivery block transition
  until their existing settlement finishes. Race new delivery registration against
  transition under the shared workspace exclusion; no resend, fabricated success,
  registry clearing, authority-error polling or input-critical waits.
- [x] Update canonical architecture/validation claims, retain startup/producer/
  finalizer/backup/migration/activation gates. Live SQL cleanup stays blocked until
  startup/reopen routing preserves or refuses ownership for pending jobs. Run scoped and full repository
  verification and current-diff manual review. Current-head CI review and the
  existing PR/owned-worktree lifecycle remain merge gates. Production stays JSON.

Owner admission and captured handoff are coupled. A private primitive may be
reviewed separately only if no caller exposes the transition until the full
admission/in-flight tests pass. No generic application refactor or producer
activation belongs in these steps.

## Review disposition and routing

Self-review: Ready for independent source/contract and builder review; no runtime
behavior or test results claimed. Root owns all edits. One read-only round,
maximum two lower-cost `gpt-6-luna` reviewers at high effort: source/authority
correctness and builder/completeness, respectively. Review the same proposal and
plan against source; report material unanswered choices with citations. Apply
bounded findings before requesting the one owner decision. No delegated writers.

Source review finding and disposition: the broad post-cleanup claim exceeded the
weak registry's owner lifetime. Narrow the first-step outcome/pass condition and
record the concrete successful-file-effects/failed-progress/owner-loss/reopen
counterexample. Add its negative qualification test and an explicit live-cleanup
startup/reopen prerequisite. No durable marker, longer-lived registry or new
reopen mechanism is selected. Builder review found no material unanswered
implementation choice; its reviewed interfaces and delivery ordering are unchanged.

Independent review disposition: Ready for owner decision. The builder pass found
no material unanswered choice. Source review found the owner-loss claim issue
above; its bounded follow-up verified that the corrected owner-lifetime scope,
negative reopen case and explicit live-cleanup gate resolve the finding. No
remaining material findings. Both reviews were read-only; no runtime tests,
filesystem interruption results or producer qualification are claimed. The
initial frozen-doc build passed all 1,202 docs tests and metadata/index/link
checks; final corrected-doc verification is recorded with the proposal PR.

## Approved implementation evidence — 2026-11-08

Owned `sql-producer-revocation`, base `a1078baf`. Owner acceptance recorded in
this plan, the design and dated ADR0103 clarification before source edits.
The original two behavioral tests failed: cleanup ran despite a workspace
in-flight delivery, and stale `put` recreated a deleted recommendation. Both
pass with the checked SQL grant and shared atomic mode. A missing-root test
caught constructor mkdir before admission; the wired no-create resolver now
refuses it. The real accept fixture initially used default dead metadata and
correctly received no input; its local fixture now supplies alive/active metadata
without weakening production acceptance.

Scoped checks: all 21 captured recommendation tests and all 122 store tests pass.
Six new application and four new store cases cover delivery success/failure and
SQL-first ordering, cancellation/expiry, owner-loss after failed progress, complete
legacy I/O refusal, Busy/cancel/retry, missing roots and independent aliases/owners.
Existing graph evidence uses independent raw-file assertions after intentional
legacy read refusal, with no writer bypass. Original recovery interruption and
1,024-record/64-descriptor cases remain. Full verification and current-diff review
follow. All live producer/startup/reopen/finalizer/migration gates remain.

Implementation review routing: root is the sole writer. One fresh read-only
round, at most two lower-cost `gpt-6-luna` reviewers at high effort: owner/constructor/
admission correctness and permit/delivery/failure/test completeness. Review the
frozen current source diff; apply bounded material findings with regression
verification. No delegated implementers or other coding harness.

Manual review corrections: the lifetime pass required explicit permanent-authority
refusal after an admitted foreign/corrupt file-stage failure; those existing
cases now assert typed `SqlAuthorityRequired` after cleanup returns and its retained worker releases the permit.
The admission pass rejected a caller-supplied success callback. Admission now
consumes a one-shot opaque proof with a private constructor in the checked
application module, minted only after the cancellation/deadline CAS under the
original guards. No production Clone/Default/Deserialize or fixture constructor
is exposed. Compiler-negative probes reject the old callback (E0308) and direct
construction from the store module (E0451). Temporary probe source is removed;
trusted free fixture construction is test-only. The initial verifier was stopped
before these source edits and is not passing evidence. Reverify the corrected
source and review the bounded changed delta before completion.

Corrected scoped verification: all 122 store tests and all 21 captured
recommendation tests pass. Both original medium reviewers completed the bounded
current-delta follow-up without remaining findings and verified all 14 Rust
source hashes (eight unchanged from the initial review). Final frozen artifact
SHA256 `6d5e87cd0d7533b048db672ced7d6d7e3ce9e8292b545a1df162036d9f35f93c`.
Reviewers performed source review only, not test execution. Final `bash scripts/verify-repo.sh` passed with `RUST_TEST_THREADS=1`: 3,227
Rust unit tests (18 existing ignored), seven platform-applicable integration
tests, 31 reporter tests, one Electron ABI proof, desktop type-check/build and
1,709 tests (three existing skips), plus 1,202 docs tests/build and diff/doc/
worktree checks. No extra filters/skips. The earlier interrupted run is not
passing evidence. Public docs/README reminders were reviewed: this private
source-only barrier changes no installed flow, while canonical architecture,
validation, acceptance and ADR records carry the changed claims. Existing
bundle-size warnings concern unchanged assets. Current-head CI and merge/owned
cleanup evidence are recorded on the implementation PR.
