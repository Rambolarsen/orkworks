---
type: Implementation Plan
title: SQL-bound recommendation journal recovery plan
description: Proposed delivery and fault evidence for exact-job recommendation recovery before SQL rollout.
tags: [orkworks, sessions, sqlite, recommendations, recovery, plan]
status: draft
workflow_status: owner-review-pending
---

# SQL-bound recommendation journal recovery plan

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [proposed recovery contract](../specs/2026-11-07-sql-recommendation-journal-recovery.md).
Base: `418cacba3b7b3f9b9ce90b1ab6d59d11c1fecf42`.

Pass condition: a captured SQL cleanup can recover its own interrupted journal,
keep all other jobs and unrelated bytes intact, and persist bit `0x10` only after
successful file completion. Production stays JSON. Owner acceptance of the new
protocol representation precedes implementation. Root owns all edits; execution
is inline using `executing-plans` and TDD, with fresh bounded read-only review.

## Investigation and decisions

Existing header epoch/workspace and exact tombstone read avoid a SQL schema bump.
A reserved digest-bound directory name is necessary before the first manifest:
legacy recovery otherwise removes missing-manifest directories recursively.
The proposed v3 envelope prevents optional-v2-field/old-reader ambiguity.
The existing anchored engine supplies bounds, original-byte recovery validation
and constant descriptor use; extend it instead of building a second engine.
See the spec for canonical encoding, compatibility, failure and ownership rules.

Risk assessment: dependencies 3 (SQL/application/recommendation owners); blast
radius 3 (shared legacy recovery entry); state changes 5 (destructive external
cleanup); reversibility 5 (deleted payloads require complete-backup recovery);
uncertainty 3 (source paths inspected, new crash matrix unexecuted). Total 19/25.
Mitigations: dormant routing, fail-closed legacy preflight, immutable exact
identity, retained operation/lane/lease, unchanged caps and fault qualification.

## Delivery

- [ ] Obtain one owner decision on the concrete protocol and record it in the
  design/ADR; do not treat generic continuation as approval of unseen codecs.
- [ ] SQL identity and codec: extend `session_store/deletion_recovery.rs` and
  its typed worker result with immutable header/job identity from one transaction.
  Add a focused private codec beside `taskmaster/store/cleanup/removal/journal.rs`.
  Verify exact UTF-8 length-prefix/name vectors, strict v3 roundtrips, canonical
  epoch enforcement (reject uppercase/compact/braced spellings without header
  rewrite), wrong header/
  job denial, existing read limits and reopen behavior. No public raw SQL access.
- [ ] Bound publication/recovery: extend the existing `removal.rs`, `journal.rs`
  and `journal/protocol.rs` owners with private SQL-specific preparation and a
  combined recover/inspect/publish operation returning `JournalReceipt`. Keep
  legacy v2 entry points distinct and reuse descriptor-relative primitives.
  Tests in focused store modules must cover every spec state, repeated crashes,
  graph/hash/name/identity mismatch, exact caps and 64-descriptor behavior.
- [ ] Compatibility routing: update `taskmaster/store.rs` legacy recovery and
  ordinary operation admission so all names are preflighted before effects and
  reserved SQL journals block both startup and existing-instance operations.
  Test manifest-less and malformed SQL entries mixed with committed v1/v2 data;
  assert all bytes unchanged on refusal. Keep legacy-only regressions passing.
- [ ] Captured integration: use selected immutable identity in
  `session_application/sql_writes/deletion.rs`, replacing the current blanket
  pending-journal refusal with exact-job recovery. Add tests beside
  `delete_recommendation_tests.rs` for epoch/creation/owner changes, SQL failures,
  partial progress, caller loss, uninterrupted permit lifetime and real input
  acknowledgement while recovery stalls. Never finish a row/job here.
- [ ] Update architecture/validation claims to match executed evidence. Run
  scoped tests, full `bash scripts/verify-repo.sh`, docs/build checks and current
  manual review. Re-review only material changed code/contracts. PR CI and the
  existing maintainer merge/owned-worktree cleanup gates remain applicable.

The codec/SQL-read deliverable can be reviewed separately from destructive
integration if diff size warrants. The publication, compatibility and captured
routing changes must not be exposed independently through production callers.
Each implementation PR remains tracked on #1243 with its remaining gates named.

## Review focus and release exclusions

Pin manifest-less directory authority before backup publication; prove mixed
journal preflight has no effects; retain backup evidence across repeated recovery;
keep the operation permit across the complete composite call; reject foreign
store epochs and creation reuse before any unlink. These are delivery checks,
not optional follow-ups. Exact finalization and stale-producer revocation remain
separate pre-activation work. No live data, native launch or installer change.

## Review disposition

Self-review: Ready for independent review and owner decision; implementation
blocked on that decision. Source assumptions are cited in the design. No runtime
fault, performance, filesystem power-loss or platform evidence is claimed by
this documentation task. Independent builder review: Ready, no material questions. Identity/compatibility
review required explicit epoch canonicalization and an explicit finalizer gate;
both source facts were verified and clarified in the design and delivery checks.
Disposition: Ready for owner decision; source-only implementation remains gated.
Frozen docs install, 1,202 documentation tests and metadata/index/link build,
whitespace/doc/worktree checks passed, including the final clarified-doc build.
This records one independent review round; no post-clarification fresh technical
review or executed runtime fault qualification is claimed.

Review routing: root is the sole writer. One read-only review round, maximum two
lower-cost `gpt-6-luna` reviewers at high effort with fresh contexts: one verifies
identity/compatibility claims against source; one checks crash/ownership coverage
and whether a builder must guess a material contract. Apply bounded findings
before handing off the concrete proposal. No delegated writers or coding harness.
