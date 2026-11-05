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


## Debit policy decision and representation handoff — 2026-10-31

The owner [accepted #1184's exact internal debit-permit tradeoff](../../validation/artifact-read-work-authority.md#owner-acceptance-and-command-proof-handoff--2026-10-31).
The written investigation and reviewed handoff are complete. This resolves the
policy-choice portion of Gate 2's read-work item, not the enforcement/complete-state
check or Gate 2 as a whole. Reuse the existing old/new charge and failure oracles
under the now-accepted permit-first policy. Unchanged technical choices need no
repeat owner decision.

Remaining sequence within the existing gates:

- [ ] [#1210](https://github.com/Rambolarsen/orkworks/issues/1210): propose exact
  internal command, nonsecret permit/commit proof, recovery lookup and closed
  format-4 commitment/accounting representation; obtain scoped acceptance of
  those new choices before dependent transaction fixtures.
- [ ] Close #1183 source/preparation/consumer/custody evidence and all applicable
  complete-command bounds, then independently qualify the complete old/new packet.
- [ ] Prepare the separately approved bounded native plan after applicable
  contracts close; prove actual transition participation, commit/disposition,
  lease/retirement and ordinary stop independence. Policy acceptance supplies
  no native mechanism or complete source proof.

Status remains Gate 2 Investigate, Gates 3–4 gated. No fixture/checker, database/
migration, native/model probe or runtime implementation is delivered or authorized.
The #744 claim scope/proof and all source/global limits stay unchanged; #740 and
maximum-profile qualification remain independent.


## Exact debit representation review prepared — 2026-10-31

The [#1210 proposal](../specs/2026-10-31-artifact-read-debit-proof-design.md) and
[bounded qualification handoff](2026-10-31-artifact-read-debit-qualification.md)
prepare the exact command/row/recovery/catalog decision under the unchanged
accepted #1184 policy. Their new representation, 16 KiB metadata bound and
historical accounting-consumer/proof retention still require scoped owner review.
Source-valid complete-state fixtures follow only after applicable acceptance and
#1183 source/consumer/preparation closure; native enforcement and separately
approved executable plans remain independent. Gate 2 stays Investigate.


## Debit representation decision complete — 2026-10-31

The owner [accepted #1210's exact representation choices](../specs/2026-10-31-artifact-read-debit-proof-design.md#owner-acceptance-and-source-closure-handoff--2026-10-31)
from PR #1217. Its written specification, scoped decision and reviewed handoff are
complete, including retained debit-accounting consumers and direct proofs. This
resolves the representation-choice portion of Gate 2; it passes neither the source/
consumer/preparation check nor actual native authority/lease/retirement evidence.

#1183's retained-source/profile choices still need their own scoped decision and
source-owner closure. Mutable source/control codecs and bounded complete consumer
membership are not supplied by accepting debit bytes. Continue from that existing
source handoff before any complete conditional transaction packet. Gate 2 remains
Investigate, Gates 3–4 gated; #740 and maximal-profile/executable-plan gates remain
independent. No fixture/checker, database, probe, migration or runtime work follows.


## Initial evaluation effect derivation — 2026-11-04

[#1363](https://github.com/Rambolarsen/orkworks/issues/1363) derives one
[initial child evaluation/artifact recipe](../specs/2026-10-30-evaluation-artifact-producer-design.md#initial-child-evaluation-publication-derivation--2026-11-04)
from the accepted contracts. It names retained target/reviewer/no-head/mapping
reads, the ten-row minimum core, global target-result child admission order,
separate target record/receipt and producer artifact charges, and complete
membership/summary/proof obligations. The core count is not a full transaction
or maximum-profile witness. No new field, command codec/tag, limit or authority
policy is selected, so existing decisions are reused without repeat approval.

The live board has closed #1183/#1184/#1210 after their bounded deliverables;
older unchecked handoff items above are historical and do not reopen them.
Their closure and component evidence do not pass this plan's Gate 2. Current
catalog work requires the complete 33-family profile and source-owned command
closure; #1356 now makes the incomplete kernel test-only and refuses production
admission before I/O. The logical recipe adds no exception to that fence.

Gate 1 remains complete; Gate 2 remains Investigate; Gates 3–4 stay gated.
Keep #1176 open for actual source/current authority, complete physical membership,
command effects/bounds and native recovery/disposition qualification. No complete
conditional packet or executable SQL/runtime plan is claimed by this update.

One bounded independent read-only Luna review checked the added derivation against
its owning contracts and found no factual mismatch or missing accepted obligation.
The author clarified d as the declaration ID resolved through retained configuration;
no new representation or decision resulted. This review qualifies the derivation,
not any source, physical or native mechanism named by it.


## Correction and invalidation effect derivation — 2026-11-04

[#1368](https://github.com/Rambolarsen/orkworks/issues/1368) extends the
[accepted-effect map](../specs/2026-10-30-evaluation-artifact-producer-design.md#correction-and-child-stream-invalidation-derivation--2026-11-04)
to a same-child-stream correction and Electron-authorized invalidation of that
stream. It derives exact current-head/predecessor/frozen/mapping reads, the
nine-row correction and four-row invalidation minimum cores, global child
admission ordinals versus per-stream revisions, and original first-write credit
consumption with protected correction credits. Target record/receipt charges,
producer artifact deltas, unchanged payload/read-work pools and complete
membership/summary/proof obligations stay distinct.

These are minimum cores and obligations, not full changed-set or packing
witnesses. No accepted policy, codec, limit, command kind or authority changes.
Gate 1 remains complete; Gate 2 remains Investigate; Gates 3–4 remain gated.
Keep #1176 open for actual source/current authority, full physical membership,
command/budget and native recovery/disposition qualification. The #1356 admission
fence is unchanged; no dependent fixture, SQL, runtime or native/model probe is
authorized by this derivation.


One bounded independent read-only Luna round checked the new material from two
perspectives: correctness against the accepted identity/CAS/slot/custody/ledger
contracts, and completeness of the builder's obligations and qualification
boundary. Both found no material mismatch or gap. The minimum core counts remain
logical derivations, not evidence of physical query/index work, full changed-set
size or packing fit. No unchanged artifact or accepted choice was reopened.


## Concrete authority-owner proposal — 2026-11-04

[#1369](https://github.com/Rambolarsen/orkworks/issues/1369) prepares a
[concrete initial-evaluation authority owner](../specs/2026-11-04-evaluation-publication-authority-design.md)
against actual ordinary lifecycle/prompt/runtime/lease seams. Existing claim-first
semantics remain accepted; separate owner/two-phase close/identity/token choices
and the proposed non-I/O first unit are pending scoped owner decision. That unit
would test exact claim/revoke/response/disposition ordering without AppState or
lifecycle wiring, SQL, live authority issuance or storage activation. Prepare its
concise execution plan only after the concrete decision. Source/native/index/
complete-command and Gate 2–4 qualifications remain open; #1176 stays open.


## Current source-owner and membership audit — 2026-11-04

[#1375](https://github.com/Rambolarsen/orkworks/issues/1375) appends a
[pinned source-owner/lookup audit](../../validation/evaluation-artifact-source-closure.md#current-source-owner-and-membership-audit--2026-11-04)
after the three publication effect maps and #1372's pending authority proposal.
It distinguishes supplied byte validation, isolated test mechanics and ordinary
session ownership from an actual eligible hierarchy source/publication owner.
It names the coherent header-bound stream/slot/retry/ledger/source/proof and
forward/reverse consumer lookup obligations without selecting SQL/index DDL.

The scoped owner decision on #1372 still precedes planning its proposed non-I/O
core. Actual source producers and ordinary-session bridges, complete physical
membership/profile, full old/new command/budget and native recovery/disposition
remain separate closure work. Existing Gate 1 decisions and closed prerequisite
issues are unchanged; Gate 2 stays Investigate and Gates 3–4 remain gated.
No complete conditional fixture, database, runtime or native/model work follows.


One bounded independent read-only Luna round checked the appended audit from
source-correctness and builder-completeness perspectives. Both found no material
error, missing accepted obligation or invented decision. The inspection remains
bounded rather than an exhaustive writer census, database/index qualification or
native result; its negative searches are not impossibility proofs. No unchanged
technical artifact or accepted choice was reopened.


## Authority core complete; close-only proposal — 2026-11-04

The owner accepted the bounded first unit; [PR #1379](https://github.com/Rambolarsen/orkworks/pull/1379)
merged the private non-I/O authority core and 23 deterministic tests, closing
#1376. Earlier pending-owner wording above is historical for that first unit.
No production source/runtime/storage integration or complete profile follows.

[#1380](https://github.com/Rambolarsen/orkworks/issues/1380) now proposes
[optional close-only lifecycle plumbing](../specs/2026-11-04-evaluation-publication-authority-design.md#close-only-lifecycle-proposal--2026-11-04)
with a source-backed terminal-root inventory and conditional coupled plan.
Every production carrier would remain None, with no constructor or attachment;
this requests a new scoped decision for call-site routing/tests only. Actual
source/ordinary bridge, resource/catalog/native and active adoption gates remain.
Gate 2 stays Investigate; #1176 stays open. #740 itself is completed research,
with its downstream evidence limitations preserved rather than reopened.


## Source-to-runtime admission proposal — 2026-11-05

The authority core and PR #1396's close-only lifecycle routing are now merged;
all production bindings remain unattached. [#1397](https://github.com/Rambolarsen/orkworks/issues/1397)
prepares the [private source-admission design](../specs/2026-11-05-evaluation-source-admission-design.md)
for original child preparation, actual runtime/ordinary-session bridge and
registration freshness under participating source writers. It is a proposal,
not an actual source publisher, physical-commit mechanism or executable plan.

Existing Gate 1 choices remain accepted. Gate 2 stays Investigate and Gates 3–4
remain gated: original source/ordinary preparation atomicity, complete physical
membership/profile, resource bounds, source authority at commit and native
recovery still require their qualified mechanisms. Keep #1176 open and #740
closed; no new fixture, code, database, adapter or probe is authorized here.


## Joint original preparation proposal — 2026-11-05

[#1406](https://github.com/Rambolarsen/orkworks/issues/1406) prepares the
[concrete original publisher architecture](../specs/2026-11-05-joint-child-preparation-design.md):
one qualified rollback transaction across ordinary and hierarchy databases,
private typed source/ordinary owners, complete preparation before spawn and a
versioned ordinary-lineage commitment. The no-ATTACH/worker, schema/profile and
retention changes are proposed explicitly, with SQL availability coupling.
Neither this proposal nor the prior admission proposal is accepted by continued
drafting or merge. Existing accepted contracts remain in force.

Gate 2 stays Investigate; Gates 3–4 remain gated. No source claim-first exception,
actual source publisher, full catalog/physical membership, source commit cutoff,
resource/native qualification, database/migration or runtime activation follows.
Keep #1176 open and #740 closed research.
