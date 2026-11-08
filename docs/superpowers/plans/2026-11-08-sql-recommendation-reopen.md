---
type: Implementation Plan
title: SQL recommendation reopen qualification plan
description: Approved bounded constructor refusal and private SQL-owned reopen delivery with pending-job regression evidence.
tags: [orkworks, sqlite, recommendations, startup, plan]
status: draft
workflow_status: implementation-active
---

# SQL recommendation reopen qualification plan

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243),
[#1244](https://github.com/Rambolarsen/orkworks/issues/1244).
Spec: [proposed reopen contract](../specs/2026-11-08-sql-recommendation-reopen.md).
Base: `4e5215db38a3bb7085dc4fa3ce4ca2f1ed4604fe`.
Pass condition: retained known SQL artifacts prevent legacy recommendation
reopening after owner loss, while a separately checked private SQL owner can
retry an exact pending job without legacy constructor effects. Owner acceptance
of this concrete admission mechanism was given after #1537 with “keep going”
on 2026-11-08. Execute inline within the accepted private boundary.

## Investigation and choices

Least-confident boundary: a startup-only pending-job check could deny the first
open yet let independent assessment access create a fresh Legacy owner later.
The generic constructor must refuse known durable evidence; a process-local
mode or strong registry alone cannot establish restart authority. Source traces
show the accepted `format.json`/`session-store` names, the effects inside legacy
open, the existing no-effects binding, bounded discovery and exact-job admission.
The earlier owner-loss counterexample is already an executed negative test.

No new persistent marker or SQL schema. Refuse either known name without parsing
it; SQL ownership always revokes, rather than branching on an empty job page.
Reuse the existing worker/lease/binding and atomic operation owner. Keep the
private SQL opener separate from production authority selection: marker bytes,
complete staging discovery and all startup side effects are still #1244 work.
This distinction prevents a source-only primitive from becoming an activation
bypass. All normal evidence-free JSON behavior is preserved.

Risk: dependencies3 (application, worker, recommendation owner, migration handoff);
blast radius3 (all legacy constructors); state changes3 (existing monotonic mode,
no new durable state); reversibility3 (code revert possible before activation,
no reset of a retained SQL owner); uncertainty3 (source traced, new opener and
artifact-failure matrix not executed). Total15/25. No live-user fixture or
power-loss claim; implementation remains dormant until independent gates pass.

## Coupled delivery and checks

- [x] Record the owner's one acceptance and dated ADR0103 clarification before
  dependent code; preserve the distinct #1244 production routing gate.
- [x] In `taskmaster/store.rs` and focused store tests, add the two-name
  non-following artifact refusal after owner claim and before effects. Preserve
  Busy precedence, error/absence distinction and no-create behavior. TDD: prove
  legacy open currently succeeds beside an empty `session-store`, then cover
  each name/type/error, unchanged bytes, missing root, aliases, independent
  assessment opens and evidence-free legacy behavior.
- [x] In a focused private `session_application/sql_writes` opening module,
  reuse `SessionStore::open`, exact worker/header validation, existing-root
  recommendation binding and retained lease. Add only the minimal typed header
  read needed in `session_store/worker.rs` and its owning module if the current
  interface cannot bind the checked epoch/workspace/root. Do not expose raw SQL.
  Mint a separate opaque opening proof after checks, and claim/revoke the
  original recommendation owner before adoption. Test foreign root/lease/store,
  Busy, zero jobs, all-bits-set jobs, cancellation/worker failure and detached
  adoption. No production/HTTP caller and no legacy preparation inside this path.
- [x] Extend `sql_writes/delete_recommendation_tests/revocation.rs` (or a focused
  reopen test module): copied fixture performs file cleanup, forces SQL progress
  failure, drops every original owner/worker and releases its lease, then starts
  a fresh owned private open. Assert legacy constructor refusal before effects,
  SQL mode before publication, exact pending record reread and successful
  recommendation retry without finalizing row/job. Retain the old counterexample
  only for explicitly evidence-free/unqualified fixtures; do not weaken its
  assertions to hide a remaining supported path. Gate worker/file work to verify
  terminal-critical guards remain free and caller loss retains required owners.
- [ ] Update canonical architecture/validation claims and #1243/#1244 progress.
  Run focused store/worker/application regressions, `bash scripts/verify-repo.sh`,
  current-diff manual review at the documented risk-appropriate effort, required
  CI and complete PR comment inventory. Complete normal PR/owned cleanup. State
  the exact private guarantee and leave production activation closed.

Constructor refusal and private opening are coupled for usable SQL recovery.
Do not land a production route in this slice or generalize unrelated startup
code. Existing job discovery bounds, cancellation budgets, operation/lane and
receipt lifetimes remain authoritative; test only additional behavior and the
real regression rather than duplicating every implementation branch.

## Review and handoff

Root owns every edit. Before owner handoff, one read-only round with at most two
fresh lower-cost `gpt-6-luna` reviewers at high effort: source/authority correctness
and builder/completeness. Review this spec/plan against source and accepted
contracts, with citations and explicit Unknown for unqualified behavior. No
writers delegated. Apply material findings before the one owner decision; do not
repeat review of unchanged artifacts. Ready means ready for owner decision,
not implementation approval or production qualification.

Initial self-review: Ready for independent review. Each contract requirement
maps to constructor, opener or end-to-end tests above. Complete evidence resolver,
marker representation and production adoption are explicitly deferred with no
qualified path claimed. No source changes or new runtime results exist.

Independent builder review: Ready after owner acceptance; no material unanswered
choice. Authority review requested tight two-name scope and questioned the
combined negative evidence. The outcome now names the two artifacts explicitly;
the direct nested application-test link names the actual combined regression,
which preserves its SQL worker while dropping recommendation owners. This is
owner-loss evidence, not a process-restart test. New qualification still must
drop/close the original worker and reacquire the lease as specified above.

Bounded authority follow-up: Ready for owner decision; no remaining material
finding. Reviewer inspected the cited combined test and resolved the evidence
Unknown, and verified the narrowed two-name outcome. Both reviews were source/
contract reading only. Final corrected-doc build passed all 1,202 documentation
tests, metadata/index validation and VitePress link/build checks; existing
circular-chunk/bundle-size warnings concern unchanged assets. This evidence-only
review record adds no behavior or new runtime qualification. At proposal merge, owner acceptance remained pending and no source
implementation or production change was included. Acceptance and subsequent
implementation are recorded below.


## Approved implementation — 2026-11-08

Owner accepted after #1537 with “keep going”; dated ADR clarification precedes
source edits. Root owns `sql-reopen-owner`, base `0ebac776`. Constructor TDD
failed before refusal and passes after it. Private-opening tests passed their
initial six-case matrix; all 125 then-current store tests passed. The combined
reopen test initially failed because the fixture reused its lexical `/var` root
instead of adopting the candidate's canonical `/private/var` root. The fixture
now consumes the returned root/lease and preserves exact application bind checks;
its full worker/lease reopen and pending recommendation retry passes. A physical
root-replacement case correctly made legacy reads unavailable, so its assertion
now expects that exact I/O failure rather than an empty list. Neither correction
weakens authority or recovery checks. Final qualification and review follow.

The opener returns an owned candidate and never adopts AppState. Existing active
context must match the selected path/root/lease; a changed reader/owner/workspace
rejects the result. Future production selection/adoption remains #1244-owned.
Review routing: root sole writer, one fresh read-only whole-branch review with
at most two lower-cost reviewers at high effort, split admission/authority and
worker/cancellation/test completeness. Apply bounded material findings before
PR handoff; manual effort medium for the architecture/lifetime boundary.


Full-verifier compatibility finding: older SQL stale-context fixtures used the
legacy workspace test helper to reopen their SQL root, now correctly refused.
The exact hook regression reproduced `SqlAuthorityRequired` at that helper.
Ten SQL callers now use explicit test-only memory-context replacement: new
metadata reader/organization, cleared active lease/revocation, original path and
recommendation owner retained, and no legacy/file operation. All assertions
remain. A wrong-application fixture uses a separate `foreign-app` root because
its only input to the owner is AppState pointer identity. The original state and
SQL phase assertion remain intact. No production fallback or test-only SQL
constructor bypass was added.

Both medium reviewers cleared these bounded compatibility deltas, verifying all
20 final Rust hashes; the original nine core files are unchanged from initial
review. Final frozen artifact SHA256
`db4dcf31efbe26fc863e35754ed0ba30ed071911a700046378c18c5249827c7f`.
Corrected scoped verification passes 95 native SQL and 88 Peon/runtime SQL tests,
in addition to six opening tests, four constructor tests and 21 recommendation
application tests. The first full verifier was stopped after the fixture failures
and is not passing evidence. A fresh full verifier runs on the corrected tree.


Final local verification passed: `bash scripts/verify-repo.sh` with pinned Node22
and `RUST_TEST_THREADS=1` completed 3,237 Rust unit tests (18 existing ignored),
seven applicable integration tests, 31 reporter tests, one Electron ABI proof,
desktop type-check/build and 1,709 tests (three existing skips), plus 1,202 docs
tests/build and diff/doc/worktree checks. No extra filters or skips. Public-guide
and README reminders were reviewed: this source-only private SQL prerequisite
changes no installed/onboarding claim; canonical architecture, validation and ADR
records carry the new behavior. Existing bundle-size/circular-chunk warnings
concern unchanged assets.

A final platform-only test-module cfg now limits the new opener tests to
macOS/Linux, matching the Unix fixture and cleanup support they consume. The
macOS/Linux compiled body and coverage are unchanged; a focused final-source
compile/test follows, and Windows CI remains the target evidence. The admission
reviewer cleared this bounded cfg delta and checked all 20 final Rust hashes.
The lifetime review remains clear for unchanged runtime/fixture behavior.
Current reviewed source artifact SHA256
`b175c3ab4d5c7c715e6c9dacf96124af7aed00442f8d2517bb6e81134c3dcde3`;
later evidence-only prose does not change those source hashes. Current-head CI,
PR comment inventory, merge and owned cleanup remain separate lifecycle gates.
