---
type: Qualification Plan
title: Evaluation-artifact producer qualification handoff
description: Gated owner decision, source closure and conditional/native evidence sequence for immutable evaluation-artifact custody and publication.
tags: [orkworks, evaluation, artifacts, qualification, custody]
status: draft
workflow_status: source-and-authority-closure-required
---

# Evaluation-artifact producer qualification handoff

Outcome: qualify exact evaluation/artifact/receipt/custody/accounting publication
without granting a source or authority fallback. Pass condition: owner-accepted
closed producer contracts and an independently checked complete-state/effect
packet, followed only when authorized by a separately approved executable/native
plan. **Ready for written review, not fixture/adaptor or runtime execution.**

Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176),
[#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Spec: [artifact producer proposal](../specs/2026-10-30-evaluation-artifact-producer-design.md).
Baseline: [conditional coherent review history](../../validation/coherent-review-history.md).
Inspection base: `5caab58d`; no implementation method is selected here.

## Investigation and ownership

One author owns the proposal and handoff; no concurrent production writer.
The investigated uncertainties are resolved as design choices or explicit gates:

- Canonical object BLOBs already hold exact evaluation bytes. Proposed content
  sharing therefore needs strict artifact/evaluation joins and reciprocal custody,
  not an inferred external blob store or physical-copy quota discount.
- Existing single-subject purge cannot break protected cross-owner references.
  Propose an explicit no-purge connected slice; joint deletion remains separate.
- Existing source/read request schemas do not supply aggregate lifetime read work.
  Propose a finite precharge command and leave its excluded authority/native fence
  gated; the accepted write claim-first exception does not cover it.
- The accepted source record and result payload pools do not pay reviewer artifact
  production or metadata. New producer resources need scoped owner acceptance and
  explicit fresh upstream preparation; missing rows cannot initialize at zero.
- The format-3 closed registry rejects unknown kinds/tables but has no artifact
  profile marker. Propose distinct format-4 header/manifest/row domains with one
  closed catalog, so missing extension rows cannot select an old base profile.
  Complete reader support/fresh preparation and exact compatibility fixtures
  remain required; no implicit migration or old-domain reinterpretation.

Review routing: after the draft, one read-only correctness reviewer checks the
accepted-contract/custody/authority claims; one separate read-only builder reviewer
checks unanswered schema/ordering/limit/recovery decisions. Root is the sole writer.
Maximum three active agents, one bounded review round, lower-cost tier first.
Record dispositions here; do not execute dependent work from a reviewer summary.

## Gate 1 — scoped owner decision

- [x] Review the five concrete choices in the proposal's final section. Reconcile
  artifact content sharing/reciprocal custody and saturation cost, registry
  compatibility, new producer/read quotas, excluded read-work command and memory
  policy with #741/#744/#746; change owning contracts only for the accepted scope.
- [x] Record acceptance or revisions against exact reviewed bytes. Draft/merge
  authorization does not accept new defaults or authorize fixture/adaptor work.

Check: each new row/field/domain/purpose/resource/command/default has a named
decision; existing request/source/receipt and claim/lease definitions are preserved.
Stop before dependent fixture or code implementation until that decision exists.

## Gate 2 — source and authority closure

- [ ] Supply exact retained v3 producer configuration, frozen child allocation/
  attachment and typed input/output/result/snapshot/evidence proof. Attribute all
  target/reviewer preparation and zero-ledger initialization to real source owners;
  resolve every consumer registry/affected-set expansion used by the profile.
- [ ] Resolve the read-work reservation's excluded authority/lease/revocation cutoff
  with an enforceable mechanism/evidence obligation or an explicitly owner-reviewed
  policy amendment. No write claim reuse, precheck-only proof or stop delay.
- [ ] Close no-purge/custody and loader/recovery rules with complete bounded version/
  index/ledger membership; name complete command, row/query and memory cost bounds.

Check: no missing codec, absent assumed consumer, supplied authority boolean,
unbounded scan or source-cap reservation is being used as proof. Unknown means
unavailable/no-go. New source or authority choices need their own scoped review.

## Gate 3 — independently generated conditional complete-state packet

- [ ] Only after the applicable decisions above, pin canonical artifact metadata,
  stream mapping, raw/source digests and row projections; accepted versions must
  use the unchanged source/request/receipt codecs. Keep old goldens untouched.
- [ ] Produce complete old/new source-owned states and exact manifests for initial
  review, correction, user invalidation and read-work charge. Include both owner
  ledgers, protected paired credits, custody/evidence consumers, subject/summary/
  snapshot effects, proof/header and every index; no symbolic omitted Q/S/D.
- [ ] Check exact authorized retry, changed retry, stale/frozen/wrong-owner/source,
  corrupt/missing mapping/ledger/proof, each budget boundary, metadata/command
  overflow, lost response and no-purge outcomes. Independently verify commitments,
  complete changed rows/bytes and conservative maximum-width packing or denial.

Check: all no-go branches preserve complete old state or the existing authoritative
unavailable fence; a source-content arithmetic packet is not transaction evidence.
No native/database/model invocation is authorized by this conditional packet.

## Gate 4 — separate executable/native qualification plan

- [ ] Name actual owners/modules, supported adapter/tool versions, private scratch
  stores, OS/filesystem evidence, finite repetition/time/fault budgets and cleanup.
  Prove source/current authority/lease and commit/rollback/response ordering,
  bounded memory/query work, native durability and ordinary stop responsiveness.
- [ ] Reconcile #740, shared custody and maximal profile gates before runtime scope;
  use TDD and required repository checks only within the separately approved plan.

No rollout, migration or rollback procedure is selected here; implementation has
not started. Future persistence changes and reversibility remain Unknown.

## Review and risk disposition

For this documentation deliverable: Dependencies 4 (three source owners plus
storage/authority); Blast radius 4 (cross-owner retention and metadata actions);
State changes Unknown (no executable persistence design); Reversibility Unknown
(no migration/rollback qualification); Uncertainty 4 (source/native fences remain
unqualified). Sum incomplete. These ratings are not approval thresholds.

Fresh bounded review dispositions (one round):

- Correctness review identified the missing persisted profile discriminator.
  Resolved in the proposal by new-store-only format 4 with distinct header/
  manifest/row version/domains, closed profile selection and explicit missing/
  unknown-table denial. This is a new owner-review choice, not acceptance or a
  completed native compatibility test.
- Builder review identified ambiguity in the 4 MiB memory scope. Resolved as
  per sidecar instance, shared across selected/retiring workspace buffers, with
  one active-or-retiring materializer and no host-global ceiling claim.

The author checked the revised decisions against the cited source/header/worker
contracts and clarified snapshot invalidation for detail-only source corrections. No second independent review of these revisions is claimed. Existing
technical content remains proposed; code/fixture execution is still gated.

Disposition: **Ready for the next written owner-review gate; Investigate for
executable qualification.** No runtime, schema/default acceptance or unqualified
code plan follows.


## Gate 1 accepted; Gate 2 investigated — 2026-10-30

The [scoped acceptance](../specs/2026-10-30-evaluation-artifact-producer-design.md#owner-acceptance-and-gate-2-handoff--2026-10-30)
completes Gate 1 without changing technical definitions or expanding authority.
The [bounded source audit](../../validation/evaluation-artifact-source-closure.md)
separates accepted logical contracts from retained producer/consumer evidence and
physical reservation ordering. Its next deliverables are
[#1183](https://github.com/Rambolarsen/orkworks/issues/1183) (closed child source and
preparation records) and [#1184](https://github.com/Rambolarsen/orkworks/issues/1184)
(read-work authority ordering). These specification investigations may proceed
within their own scope; no new codec or cutoff is accepted by naming the issues.

Current disposition: **Gate 1 complete; Gate 2 Investigate; Gates 3–4 gated.**
The previous ready-for-owner-review disposition is historical. No component
arithmetic/hash, synthetic child identity, final response check or advisory-lock
existence passes Gate 2. No dependent fixture, code, migration or probe ran.


## Read-work Gate 2 handoff — 2026-10-31

The [#1184 investigation](../../validation/artifact-read-work-authority.md) traces the actual nearby owners, separates
durable debit permission from content access and bounded response commitment, and
records complete charge/denial/recovery oracles. It finds no qualified physical
commit/revocation fence in the inspected integration. Its narrowly scoped internal
debit-permit alternative is **proposed, not accepted**; the existing cutoff and
no-go remain. Owner decision, exact proof/command codecs, #1183 source closure and
a separately approved bounded native plan still precede dependent transactions.
Gate 2 remains Investigate; no #744 claim reuse, fixture, native probe, database or
runtime implementation follows.
