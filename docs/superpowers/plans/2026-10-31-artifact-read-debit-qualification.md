---
type: Qualification Plan
title: Artifact debit representation and recovery qualification handoff
description: Scoped representation review, source closure and conditional ledger/manifest evidence before separately approved native debit qualification.
tags: [orkworks, artifacts, debit, qualification, recovery]
status: draft
workflow_status: representation-accepted-source-closure-required
---

# Artifact debit representation and recovery qualification handoff

Outcome: exact recoverable debit accounting under the accepted #1184 policy.
Pass condition: accepted command/row/catalog choices and source-valid complete
conditional effects, then independently approved native enforcement evidence.
**Gate 1 representation accepted; Gate 2 source/accounting closure remains
Investigate. No complete transaction fixture, adapter or runtime execution.**
Tracking: [#1210](https://github.com/Rambolarsen/orkworks/issues/1210),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Contract: [debit representation proposal](../specs/2026-10-31-artifact-read-debit-proof-design.md).
Reuse the [artifact Gate 2–4 plan](2026-10-30-evaluation-artifact-qualification.md)
and [accepted authority decision](../../validation/artifact-read-work-authority.md#owner-acceptance-and-command-proof-handoff--2026-10-31).

## Investigation, ownership and boundaries

Root owns the proposal and handoff. The inspected source/protocol decisions are:
external artifact reads have no idempotency key; inherited manifests already bind
exact row changes with claim:null for internal commands; counters need complete
retained debit history to recompute lifetime work. Use one new retained row and
three existing ledger updates, with two existing controls, rather than a new
receipt/claim protocol. Metadata validation before B must not materialize content.
Source completeness and actual permit/commit/fetch/handoff ordering remain gates.

Review topology: one author, then one bounded read-only correctness review and one
independent read-only builder/completeness review of the merged draft. Maximum
three active agents; lower-cost tier first. No concurrent production writer and
no second review round of unchanged artifacts. Review only these new codecs and
handoff, reusing #1184's accepted policy and prior technical review.

## Gate 1 — exact representation decision

- [x] Review the four concrete choices at the contract's end, including the
  independent sequence, 16 KiB complete row bound, direct full-key lookup,
  retained accounting consumers/proofs and single closed format-4 catalog.
- [x] Record owner acceptance/revisions against exact reviewed bytes. Unchanged
  #1184 policy, wire/source/request/receipt domains and quotas need no repeat
  approval; new representation choices do need their scoped decision.

Check: all fields/keys/domains/nullability, recovery comparison and hash/link
exclusions are explicit. Fresh catalog selection cannot adopt an old subset.
No dependent complete-state fixture or executable work before this decision.

## Gate 2 — source and accounting closure

- [ ] Reconcile #1183 source/configuration/preparation and complete consumer registry
  with the new historical debit-row retention joins. Validate original stream,
  target/evaluation/source/custody and zero-ledger history without content prefetch.
- [ ] Establish bounded complete old state and all three lifetime ledger sums,
  direct origin/predecessor proofs and source-owned membership. No missing-row
  initialization, caller-supplied affected set or absent-consumer assumption.

Check: an independent builder can reconstruct exact prestate and B from genuine
accepted sources; no root/synthetic child substitute or symbolic missing index.
Unknown source/retention membership keeps the profile no-go.

## Gate 3 — independently generated conditional packet

- [ ] After applicable decisions, independently generate command/digest/row and
  format-4 manifest bytes for exact six-row success and complete unchanged denials.
  Preserve old fixture catalogs; use complete old/new states, not components alone.
- [ ] Check full-key recovery versus new reread, forced collision/body mismatch,
  stale header/U, individual cap equality/overflow, noncanonical/mixed catalog,
  partial/missing debit/source/proof, no-refund and namespace/retiring-worker cases.
- [ ] Independently count complete keys/values/staging/manifests/controls and all
  native-independent limits, including metadata-only access and bounded recount.
  Compare upper-width construction with the documented 9,238-byte debit row and
  15,976-byte six-row new-value subtotal; these are not complete native costs.

Check: success has four sorted changes plus two controls, correct U→U′ and no
source/receipt/summary side effect; every denial keeps complete old state or the
existing authoritative unavailable fence. No fixture can assert native ordering.

## Gate 4 — separate bounded native plan

Only after applicable source/schema decisions, prepare a separately reviewed and
approved plan naming actual modules and *all* revoke/replace/health/lease owners,
supported SQLite/VFS/OS/filesystem, private scratch stores, exact instrumentation,
finite time/repetition/fault budgets and cleanup. Prove metadata-before-charge,
charge-before-content, permit/revoke and physical disposition, independent stop,
source/lease loss, restart/lost-ack and retained-worker/memory limits. No new
bearer, worker, automatic replay, migration or unbounded scan. #740 and maximal
profile gates remain separate; no executable command or probe is approved here.

## Review and risk disposition

Planning checkpoint: least confidence is complete source/consumer membership and
bounded recovery cost, not field serialization. Investigation tied the retained
row to #1183 provenance and made direct accounting references part of retention;
metadata/content separation and old-command recovery versus fresh reread are
explicit acceptance oracles. A width-only row is not source-valid state.

Local review: Scope/clarity/coverage Ready for Gate 1 written review. Dependencies
4 (source, reporting, storage, authority); blast radius 4 (accounting/recovery);
state changes Unknown (no implementation); reversibility Unknown (no migration/
rollback profile); uncertainty 4 (native/source/maximal profile unqualified).
Sum incomplete. Ready never means runtime approval. Applicable tests are docs
metadata/index/link build and unchanged documentation suites; no new runtime test
or fixture implementation is part of this handoff.

Disposition: Ready for scoped representation review; Investigate for Gates 2–4.
Independent bounded review findings/dispositions are recorded below when complete.


## Bounded independent review — 2026-10-31

One Luna-tier correctness reviewer checked command/ledger/source/manifest and
accepted-policy joins against the cited contracts; one separate Luna-tier builder
reviewer checked schema/recovery/packing/retention and handoff completeness against
#1210. Both reported no concrete defect or missing requirement. No artifact was
edited by a reviewer and no second review round was run. The author checked the
exact width construction independently; these findings establish written-contract
review only, not source-valid fixture or native enforcement evidence.

Reviewed disposition: Ready for Gate 1's one scoped representation decision;
Investigate for source-valid/conditional/native qualification. Existing contracts,
source identities, #1184 policy and fixture catalogs remain unchanged. No new
representation choice is accepted by this review or by publishing the proposal.


Author clarification after the bounded reviews: command admission must prove
poststate recovery/recount eligibility within existing work bounds before adding
a debit, rather than admitting a history known to be unrecoverable. This adds no
quota or supported profile; no independent re-review of that clarification is
claimed. Complete source/native qualification remains gated.


## Gate 1 accepted; source closure remains — 2026-10-31

The owner [accepted the four exact representation choices](../specs/2026-10-31-artifact-read-debit-proof-design.md#owner-acceptance-and-source-closure-handoff--2026-10-31)
from PR #1217, including the complete row bound, retention cost, exact six-row
commit and single catalog. Gate 1 is complete and #1210's specification/decision/
handoff is delivered. Reuse those unchanged bytes and prior reviews; do not
request another decision on the same debit codecs.

The next permitted prerequisite is #1183's separate source/profile review and
source-owner closure, tracked with #1176. Its current child-source proposal and
Gate A remain unaccepted; mutable #742 control/event codecs, real atomic fresh
preparation/attachment, bounded complete consumers and native authority do not
follow from a valid source-record digest. The accepted debit consumer must join
that complete registry before Gate 2 can pass. No synthetic prestate or assumed
absent consumer can open Gate 3.

Gate 2 remains Investigate, Gates 3–4 gated. No conditional complete-state packet,
fixture/checker, database/migration, native/model probe or runtime work ran.
#740, actual source/custody/native and maximal-profile/executable-plan gates remain
separate. This status annotation changes no contract bytes or accepted bounds.
