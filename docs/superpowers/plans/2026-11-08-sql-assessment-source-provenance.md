---
type: Implementation Plan
title: SQL assessment provenance qualification plan
description: Approved strict provenance data and typed SQL qualification prerequisite without enabling assessment publication.
tags: [orkworks, sqlite, assessments, provenance, plan]
status: draft
workflow_status: implementation-complete
---

# SQL assessment provenance qualification plan

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [accepted source contract](../specs/2026-11-08-sql-assessment-source-provenance.md).
Base: `1ab168d35a2cfd8689d82f984b52a0fad278f765`.
Pass condition: exact original source data survives a format4 pending record,
SQL close/reopen and validation; reused/tombstoned/foreign/unqualified sources
fail visibly without record or recommendation effects. This is a data/validation
prerequisite, not a runnable SQL assessment producer. Owner accepted after #1546 with “keep going” on 2026-11-08;
execute inline within this bounded private scope.

## Investigated uncertainty and scope

The apparent ID-only pending-card gap is broader than the output arrays:
assessment output is currently source-ID-free, while its fingerprint includes
complete observations and recommendation snapshots. Retained observations lack
original creation identity. Current-row backfill would bless old data after ID
reuse. Require an origin-qualified closure; legacy inputs stay Unknown. No live
source-origin producer exists for this contract yet, so do not invent one in
codec tests or call its fixture constructor production qualification.

A second boundary is publication versus deletion: a worker read proves currentness
at its transaction, not through a later external write. The private primitive
cannot confer file authority. Later publication needs same-permit ordering,
provenance-aware cleanup and delivery/action qualification. The first slice has
no legacy store bypass and no production/HTTP/recovery route.

Risk: dependencies3 (assessment shapes/store, SQL worker, future source owners);
blast radius3 (private formats and recovery parsing); state changes3 (additive
format4, old bytes preserved); reversibility3 (dormant source-only code, SQL records
need their new reader, no downgrade rewrite); uncertainty4 (source traced,
origin and publication integrations deliberately unqualified). Total16/25.
Finite independent caps, explicit refusal and no route activation bound the work.

## Bounded implementation after acceptance

- [x] Record the owner's one decision in the design/plan and dated ADR0103
  clarification before source implementation. Existing storage/reopen acceptance
  remains valid; it does not accept this new wire format by itself.
- [x] Add a focused private source-provenance module near assessment_report with
  strict schema/canonical validation, fixed digest bytes and128-pair/16KiB limits.
  Keep parsed data distinct from original-input claims and any application grant.
  Production origin-claim construction remains unavailable; trusted fixture
  construction is cfg(test) only. Golden vectors cover empty/one/maximal short
  pair sets, exact hashes, sorting, duplicate/conflicting IDs, unknown/duplicate
  fields, noncanonical UUID/hash, malformed/oversized IDs and both byte boundaries.
- [x] Extend AssessmentRecord/AssessmentProvenance and AssessmentStore validation
  for format4/optional readiness and exact report/pending/source consistency.
  Existing1–3 serialization/recovery stays unchanged; SQL-specific use validation
  rejects their missing proof without invoking legacy recovery. Cover every state,
  no-proposal, redaction, unsupported versions and raw present/null SQL fields
  in both top-level and nested pending-recommendation provenance on each legacy
  format1–3. Reject those before optional decoding loses field presence; preserve
  all previously accepted legacy shapes and ordinary serialization otherwise.
  Cover missing format4 fields,
  source-copy mismatch,64KiB total overflow, failed save and record byte preservation.
  Round-trip a pending record and a surviving recommendation after latest-report
  replacement; source provenance must not disappear with the report.
- [x] Add typed private capture/validate commands in session_store/worker and a
  focused owning module. Consume original epoch/workspace scope and expected
  pairs; compare the current header to that scope before producing provenance,
  never initialize original scope from current SQL. Check exact live rows within
  one bounded read transaction, reserve complete result bytes and
  keep original deadlines. TDD with real SQL: ID reuse, tombstone, missing row,
  inconsistent deletion job, wrong workspace/epoch, capture-time epoch replacement
  with identical IDs/creations, closed/archived eligibility,
  empty-set header validation, busy/corrupt/unavailable/queue expiry and unchanged
  parsed source bytes after failure. No schema change or raw SQL interface.
- [x] Compose copied-fixture qualification: capture trusted original input claims,
  serialize a format4 pending record, close the worker, reopen, validate the exact
  saved object and compare bytes. Between phases inject source deletion/recreation,
  epoch replacement (including equal source IDs/creations) and inaccessible SQL. Assert no recommendation I/O, no source
  refresh, no pending-state completion and no inference dispatch. Keep legacy
  input refusal explicit even if its output card has no visible session IDs.
- [x] Update canonical assessment/architecture/validation claims with actual
  evidence, run scoped and full repository verification, current-diff manual
  review and required CI, then complete the normal PR/owned cleanup lifecycle.
  Do not claim source-origin producer, publication/deletion race or live startup
  qualification from these primitive tests.

No speculative producer implementation, observation wire upgrade, cache/history
redesign or UI belongs in these steps. A later bounded proposal must define each
actual source-origin owner and publication/cleanup integration before those
routes are added. Reuse this representation decision if accepted; do not ask
again for unchanged bytes merely because downstream qualification remains.

## Review and handoff

Root is the sole writer. One fresh read-only review round, at most two lower-cost
`gpt-6-luna` reviewers at high effort: source/authority correctness and builder/
format/completeness. Review both artifacts against the cited code and accepted
contracts. Report material outcome-changing gaps with citations; mark physical
behavior Unknown. Apply bounded findings before the one owner decision. No
runtime evidence is claimed. Initial self-review: Ready for
independent review, with all origin/publication gates explicitly retained.


## Independent review disposition

Source/authority review: Ready; source facts and data-versus-authority boundaries
match the inspected owners. Runtime and physical behavior remain Unknown.
Builder review found two checklist gaps: explicit capture-time original-scope
comparison, and raw present/null SQL fields nested inside legacy pending records.
The contract/plan now state both checks and tests, including format1's permissive
loader. These clarify the selected fail-closed representation, add no new route
or authority, and need no separate policy choice beyond the one owner decision.
Bounded builder follow-up: Ready; both corrections close the identified gaps,
with no remaining material completeness issue. Final corrected-doc build passed
all1,202 documentation tests, metadata/index validation and VitePress links/build.
Existing circular-chunk/bundle warnings concern unchanged assets. This review
record adds no runtime or physical qualification. The owner accepted the concrete contract after #1546; the private implementation is complete.


## Implementation evidence

The private codec, format4 record boundary and typed worker commands are now
implemented. Focused qualification passed all 84 assessment tests, including the
strict codec and record cases; real SQL source checks passed 3 tests and worker
admission/failure checks passed 2 tests. Independent one/maximal-source digest
vectors supplement the empty vector. Exact 16 KiB provenance and 64 KiB complete
record boundaries accept; one extra byte refuses without replacing prior bytes.
The duplicate-parent legacy source-field regression was observed failing before
the stream scanner fix. A second behavioral regression exposed missing pending
readiness equality/capability checks; format4 now mirrors those existing checks.

Fresh manual medium reviews cover source/authority correctness and SQL worker/
integration completeness. The initial source review was clear; bounded follow-up
confirmed the readiness gap and approved its narrow correction. The final scoped
assessment suite passed 84 tests after that fix; the final normal Rust build and
format check passed. Full repository verification passed 3,255 Rust unit tests
(18 ignored), 7 integration tests, 31 reporter tests, 1 native ABI test, 1,709 desktop
tests (3 skipped), 1,202 documentation tests, desktop/docs builds and currency
checks. The full run began before the narrow readiness correction; the final
focused suite and build cover that correction and the added no-proposal case.
Both manual review scopes are APPROVE on the final code snapshot. Documentation
currency suggestions need no public guide/README edits: canonical architecture,
ADR and validation claims changed, while installed/runtime behavior is unchanged.
The PR conversation records CI, terminal-state and owned cleanup evidence.
