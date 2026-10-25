---
type: Design Proposal
title: Hierarchy candidate command manifests and ledger effects
description: Proposed complete logical effect recipes for candidate admission and finalization, with exact resource deltas, bounded lookup rules and remaining producer qualification gates.
tags: [orkworks, hierarchy, storage, candidates, accounting]
status: draft
---

# Hierarchy candidate command manifests and ledger effects

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `5388d85a`.
**Three scoped choices accepted for further design/qualification; no executable plan or runtime approval.**
See the [owner acceptance](#owner-acceptance-and-handoff--2026-10-25).
This follows the [accepted slot metadata](2026-10-25-hierarchy-slot-finalization-design.md#owner-acceptance-and-handoff--2026-10-25)
and [accepted compact codec](2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25).
It changes no canonical source/request/wire object, receipt quota, claim policy or
numerical storage default. #740 remains deferred/no-go until November.

## Outcome and short plan

Replace minimum core-row estimates with one explicit union of source, receipt,
ledger, dependency, summary and control effects for each candidate command.
A builder must be able to derive the entire changed logical row set from exact
old state and validated source intent, without caller-supplied affected lists.
This is a design deliverable: specify the effect recipes, resource and lookup
contracts, demonstrate accounting/denial cases, then review before any dependent
implementation. The recipes are complete only when all named expansions resolve;
an unresolved producer schema or affected set denies the whole command.

Read with the [storage registry and commitments](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation),
[source byte/rehash rules](2026-10-04-assignment-evaluation-design.md#proposed-output-byte-and-rehash-accounting),
[receipt pools](2026-10-04-assignment-evaluation-design.md#receipt-ownership-reservations-and-finite-storage),
[summary lineage](2026-10-23-hierarchy-transaction-storage-design.md#summary-lineage-and-application-checkpoints)
and [publication authority](2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24).

## Choice and scope

Use existing logical tables and closed commands, not a second candidate log,
caller-authored SQL/manifest, generic extension bag or new persistent index.
Discover candidate admission order and generation uniqueness from the assignment's
at most 32 retained admission slots. This avoids duplicating immutable provenance
in a new generation/ordinal counter table. Physical indexes and native query
cost still need qualification; a logical recipe does not certify SQLite work.

Cover candidate admission, one accepted or rejected finalization, and the separate
positive work reservation required before any content pass. Include supersession/
abandonment release effects so their old credits cannot be counted twice.
Evaluation publication, artifact production, proposal custody and purge keep their
own source-owned commands. They are dependencies, not silently added actions.
An artifact-bearing candidate needs its producer's closed artifact custody/index
records before it is eligible; this proposal does not invent those missing records.
Path-only and empty-output effect sets do not establish native finalization authority.

## Exact state, lookup and index contract

Let A be the complete AssignmentIdentity, C the exact candidate ObjectRef, c its
admission ordinal, R the exact accepted ResultRef, and H the pre-command header.
Every key uses the existing strict canonical codec; identities/digests are checked
against retained bytes as well as hashes. No timestamp, largest object digest,
latest artifact or mutable path supplies identity. All reads below consume the
proposed visited-row budget, including unsuccessful seeks and join validation.

For admission, read the 32 possible slot keys (A,10c), c=0..31. Present admission
slots must form one contiguous prefix. Validate each consumed mutation receipt,
claim, full retry key and retained candidate. Each candidate belongs to A, its
receipt slot matches its prefix ordinal, and authority generations are unique.
The next ordinal is the prefix length; 32 rejects a new candidate. A hole,
duplicate generation, missing candidate/proof/lookup or dangling row is unavailable,
not free capacity. All other rows for a retained candidate remain owned until
eligible complete-subject deletion. This is a bounded lookup recipe, not an
assertion that a hash or one slot alone proves inventory integrity.

Resolve an exact authenticated retry first through the accepted full-key lookup.
Only a genuinely new request scans/adopts the next ordinal and checks predecessor.
Recheck the read-set and exact H in the serialized transaction; concurrent builders
cannot both reserve c or the same producer generation. Commit claims preserve the
accepted authority cutoff, not a bypass of this validation.

For finalization, resolve C through its immutable object and exact admission slot,
claim/lookup/receipt, candidate_work and outcome slot. Find c by the same bounded
admission inventory, never resultRevision-1. Validate all ten slot positions:
open work has admission consumed, outcome reserved and eight candidate user credits.
Accepted/rejected work has its original outcome and terminal credit state.
Superseded/abandoned work has no fabricated outcome; eight credits are absent.
Any contradictory state fences the subject. An existing terminal outcome returns
its original bytes without content work or a new transaction. Supersession/
abandonment returns that disposition without scheduling finalization.

For new acceptance, recheck the current result head against C.predecessor; the
new revision is 1 for no-head, otherwise predecessor revision +1, bounded by 32.
Read the exact predecessor result, not a maximum-row election. All immutable old
results remain retained. Resolve present server artifacts only through the exact
assignment/declaration/artifact-ID/version producer index and original immutable
bytes; confirm claimed absence against that same authoritative index. A missing
index/schema is unavailable, not known missing. No arbitrary artifact lookup,
new broad read grant or cross-assignment reuse is introduced.

Dependency membership uses the existing authoritative consumer/target/purpose
index. Every retained consumer whose freshness/material can change participates
in the affected-set expansion below. Old reference edges remain protected when
a head advances. Same-owner result/payload/candidate/receipt joins are retained
as one subject; do not manufacture cross-owner edges merely to count them.
New external artifact/custody edges require closed source-owner mappings and
quotas. An unqualified producer/index keeps that source combination no-go.

## Closed resource ledger subset

Propose the following exact resource tags in quota_usage for this slice. They
are storage metadata outside source digests; other commands cannot use arbitrary
resource strings. Owner is exactly one of:

- `{kind:"assignment",assignmentIdentity:A}`.
- `{kind:"run",workspaceId:A.workspaceId,runId:A.runId}`.
- `{kind:"workspace",workspaceId:A.workspaceId}` across retained runs/epochs.

The last form has no active-epoch filter: namespace rotation cannot release old
usage. The selected physical store must account for every retained epoch it owns;
shared/cross-store custody remains separately gated. Source-owned run identity,
workspace lease and store epoch still require their original proofs.

Key is exactly `{owner,resource}`; body stays the existing
`{used:Counter,reserved:Counter,lastCommit:CommitRef}`. Counters are checked u64
canonical decimal TEXT. Reject negative/overflow deltas, missing/corrupt required
rows or a mismatch with retained ownership. Explicit owner preparation must have
initialized the required zero rows; commands cannot treat missing rows as zero.

| Resource | Owners | Exact usage and admission |
| --- | --- | --- |
| reporting-receipt-bytes | assignment, run, workspace | used is consumed slots times 16,384; reserved is unconsumed slots times 16,384. Sum obeys accepted 17/64/256 MiB pools, shared with all #744 receipt purposes, not a separate candidate allowance |
| assignment-record-bytes | assignment, run, workspace | used is sum of canonical candidate/result/evaluation/disposition object body bytes owned by that subject; reserved is zero for these retained rows. Count each logical occurrence once. Aggregate source caps remain unresolved; no producer becomes eligible until its finite owner caps are reviewed |
| result-payload-bytes | assignment, run, workspace | used is sum of retained result payload row byte lengths, including every present declaration occurrence; reserved is zero in committed accepted payload rows. Prior artifact production and configuration declaration reservations retain their independent producer budget |
| summary-row-bytes | workspace | used is sum of canonical retained SummaryRowV1 byte lengths, including unavailable rows; reserved zero. Use the existing proposed 256 MiB ceiling and 2 MiB per-row limit |
| summary-row-count | workspace | used is number of nonnull retained summary rows, including unavailable rows; reserved zero. Existing proposed ceiling 4,096 |
| summary-lineage-bytes | workspace | used is canonical checkpoint material plus pending before/after material bytes, without counting the same physical logical occurrence twice; reserved zero. Existing proposed 64 MiB ceiling |
| summary-pending-count | workspace | used is pending_material row count, reserved zero. Existing proposed ceiling 4,096 |

All evaluation/user writers must update the same reporting-receipt-bytes owner
rows. Every accepted evaluation or disposition insertion must also add its full
canonical object-body length to assignment-record-bytes at the target assignment,
that assignment's owner run and workspace. A correction inserts a new immutable
revision and charges its entire body; it does not subtract the old revision.
An invalidation adds its disposition body without deleting evaluation bytes.
Reviewer artifact production retains its additional producing-owner budget;
these target-record charges do not replace it. Exact retries add no record charge.
Reconciliation includes every retained candidate/result/evaluation/disposition
object in this ledger, not merely current heads. Only eligible owner purge may
subtract deleted owned bodies; its complete command remains separately gated.
Evaluation/disposition/artifact/purge producers that do not supply these shared
ledger effects and their own complete manifests are ineligible, even if the
candidate command recipes are otherwise ready. This proposal
selects keys/meaning, not a second pool or new aggregate source/payload defaults.
Those finite defaults, producer agreement and physical overhead remain no-go gates.
Database main/journal/scratch ceilings are independently enforced by the qualified
adapter; they are not inferred from these counters or represented as source credits.

For each resource/owner, sum all command deltas first; write that quota row once
iff its final body changes. Manifest/control/index bytes have no source-payload
quota exemption from complete-command accounting. Deletions never provide negative
command-byte credit. Do not charge canonical storage envelope/link bytes to the
512 KiB source-body ceiling; they count in command/main/physical admission instead.

## Complete command effect recipes

The following sets use only actual changed (table,key) rows; repeated references
coalesce to one row with the original before and final after value. No link-only
rewrite is permitted. A deterministic complete intent is rebuilt if the captured
source/read-set changes; it is never patched with caller-guessed effects.

Let:

- Q be exactly all changed quota_usage rows from the resource rules above.
- S be all changed summary_state and pending_material rows from the summary
  expansion below, including the owning subject's checked subjectRevision.
- D be all changed dependencies rows required by the source-owned custody and
  affected-set expansion. Include additions/removals only when their owner/source
  command actually changes membership; merely reading a retained edge changes none.
- P be any old commit_proof rows whose deletion is proven unreferenced in this
  same transaction. Default is empty; keeping old proofs must still fit capacity.
- K be exactly header singleton replacement plus new commit_proof insertion.

The exact changed set is the union of the command core with Q,S,D,P,K.
Changes is that union minus K, sorted uniquely under format 3; before/after row
commitments and excluded links use the accepted v3 domains. Every new/replaced
row's link joins this command; retained unchanged rows keep their old links.
The header predecessor equals H, sequence increments once with overflow denial,
and no other row or physical logical index entry may be mutated off-manifest.

| Command | Exact core before Q,S,D,P,K |
| --- | --- |
| candidate-admission | Insert C object, candidate_work(C), consumed admission slot 10c, nine reserved slots 10c+1..10c+9, one ClaimProofV2 row and one mutation_lookup row |
| candidate-work | Replace candidate_work(C) with checked positive reservation increment and consumedPasses+1, before starting that content pass; no receipt, result, lookup or claim |
| result-finalization accepted | Insert exact result object, every present output's payload row keyed (A,new revision,declarationId), replace/insert result head, replace candidate_work as accepted, consume outcome slot, replace eight user reservations to exact R with null stream |
| result-finalization rejected | Replace candidate_work as rejected, consume outcome with original rejection response, delete eight unused candidate user slots. No result, payload or head write |
| candidate-supersession / candidate-abandonment | Replace candidate_work with exact terminal state and null outcomeReceipt; delete eight unused candidate user slots. Keep reserved outcome and original admission/provenance |

Admission candidate_work starts exactly reservedRehashBytes="0",
consumedPasses="0", state=open, outcomeReceipt=null and lastCommit. No declared
payload is copied or hashed during admission. Do not conflate validation against
committed artifact metadata with a content pass. Existing immutable candidate
bytes cannot be changed under a new receipt or generation.

Acceptance candidate_work preserves its conservative work counters and sets
outcomeReceipt=10c+1. Rejection does the same without a result revision. Neither
refunds rehash charge. Supersession/abandonment preserves counters and outcome
reservation. Terminal replays make no deletion or ledger change. Resume/ownership
transfer must include its own separately reviewed producer effects; this command
recipe is not a replacement for #610/#742 admission/authority transactions.

Use a present-output payload row for every present declaration, including zero
bytes. Missing declarations create no payload row. In this proposed representation,
server-artifact inclusion copies its exact bytes into this result-keyed payload row
and counts them again; it does not silently introduce physical sharing, external
pointers or an exemption for existing immutable bytes. Prior artifact publication
remains charged to its own producer/owner. New copies must fit the complete command,
source cap and qualified scratch/native publication mechanism, or reject.

## Exact ledger deltas

Let Bc/Br be actual canonical candidate/result body lengths, Bp the sum of the
accepted present payload lengths, b=16,384, and n the number of present outputs.
The following deltas apply separately to assignment, owner run and workspace:

| Transition | Receipt used | Receipt reserved | Record used | Payload used |
| --- | ---: | ---: | ---: | ---: |
| New admission | +b | +9b | +Bc | 0 |
| Accepted finalization | +b | -b | +Br | +Bp |
| Rejected finalization | +b | -9b | 0 | 0 |
| Supersession/abandonment of open work | 0 | -8b | 0 | 0 |
| Work reservation or exact terminal retry | 0 | 0 | 0 | 0 |

Thus acceptance changes consumed versus reserved classification but not pool total;
rejection and supersession release eight credits, not admission/outcome history.
Each changed shared pool row appears once, even when several slots move. Work
reservation updates only its candidate-owned counters, not an additional receipt
or a freely refundable global rehash pool. Declared maxRehashBytes is checked
against the exact retained candidate counters before every pass.

Summary resource deltas are computed from complete old/new rows: row-byte and
row-count differences, checkpoint material-byte differences and inserted/deleted
pending before/after bytes/counts. Replacement counts only once in changed rows,
but its entire new encoded key/value contributes command bytes. Reserve capacity
for an unavailable fence and intact lineage before allowing its source change;
if this cannot fit, reject the entire source command.

## Summary and dependency expansion

Every command that changes candidate admission/disposition or accepted result
advances A.subjectRevision exactly once per committed command, regardless of how
many credits or edges it changes. Checked overflow blocks that command. A pure
candidate-work accounting change does not enter the source owner's listed
subjectRevision events and changes no material/subjectRevision. It still commits
its own work ledger proof and participates in pending/health publication fences.

For A and every dependency-affected assignment, compute exact before/after #744
material under the existing aggregate and source-currentness rules. Candidate
status changes alone do not enter SummaryMaterialV1; they may change visible
candidate details and subjectRevision without advancing summaryRevision. Initial
summary admission belongs to explicit subject preparation, never a read or an
arbitrary later candidate. Missing admission/lineage proof is unavailable.

With byte-identical material, keep summaryRevision/checkpoint/pending unchanged
except any required source-owner subjectRevision or visible unavailable/health
fence. For changed material, use exactly one existing branch:

- Install new current row and checkpoint atomically when capacity and lineage
  permit, consuming one checked summary revision.
- Retain the proven checkpoint, mark reconciling and append the exact ordered
  before/after pending transition when current-row capacity is insufficient.
- Preserve terminal exhausted state under the existing summary-exhaustion rule;
  it cannot force a wrap or an impossible increment, nor block an otherwise
  eligible source correction merely because its summary is exhausted.

Include every changed S row and resulting Q row. Do not compact pending history
opportunistically inside candidate commands: use a separate reviewed reconciliation
command. If an affected set, material or currentness cannot be proven, preserve
all required durable unavailable proof under the established fence contract or
deny. A volatile event alone cannot hide a committed old passing summary.

Discover affected consumers from the complete authoritative incoming-dependency
index, including retained proposal/selection/evaluation inputs under their source
mappings. Add/fence all affected summaries in the same source transaction; never
truncate the set at 4,096. Old immutable records/edges remain historical rather
than being retargeted to the new head. An edge consumer without a closed registry,
health-fence representation or quota mapping makes that affected command no-go.
This proposal supplies no fallback generic object or broad run-erasure operation.
Publish the exact affected volatile run-snapshot invalidations after durable commit
under the existing ordered publication contract; events remain best-effort.

## Bytes, rows and manifest closure

Core changed row counts including K, but excluding Q,S,D,P and output payloads,
are the accepted minima: admission 16, acceptance 14, rejection 12 and terminal
supersession/abandonment 11. Candidate-work is 3. Accepted finalization adds n
payload rows. These are union counts, not sums that count overlapping keys twice.

For the isolated illustrative case with no D/P, one owning summary_state row,
no pending/summary-material changes and preinitialized ledgers, admission changes
six Q rows (receipt+record at three scopes), hence 23 rows. Rejection changes
three Q rows plus that S row, hence 16. Acceptance with n=0 changes three receipt
and three record Q rows plus S, hence 21; with positive Bp it also changes three
payload Q rows and n payload rows, hence 24+n before any summary-resource/lineage
changes. A genuinely accepted new result changes material, so those acceptance
subtotals are lower bounds, not complete accepted-result fixtures. The checker
keeps this distinction explicit.

Compute commandBytes from full newly encoded keys/values/links, required deletion
keys, manifest entries and both K rows. Count all new payload bytes and duplicated
logical bytes. For admission, conservative ten-slot allowance is 163,840; for
acceptance, nine replaced-slot allowances are 147,456. Claimed proof/lookup bytes
are included once inside the admission slot allowance but still count as two
changed rows and manifest entries. Other Q/S/D/P/K/source/payload costs are added
in full. No unchanged source row contributes replacement bytes. Physical SQL/
secondary-index/page/journal/scratch overhead is separately unqualified.

Reject if the complete union exceeds proposed 4,096 rows or complete command bytes
exceed 16 MiB, even if each source/slot independently fits. A deleted slot releases
receipt reservation but contributes its key/manifest/ledger/control costs without
negative byte credit. Two newly copied 8 MiB outputs fail before other metadata.
No split transaction, post-commit payload write, free index or cap increase is
selected. A qualified adapter must include any additional logical row it needs
through a reviewed closed registry/manifest update before it can execute this
recipe; physical-only indexes still consume physical budgets and work.

## Bounded accounting and negative evidence

The [fixture catalog](../../validation/hierarchy-candidate-command-vectors.json)
and [checker](../../validation/check-hierarchy-candidate-commands.mjs) exercise
resource conservation, exact changed-key union arithmetic and command rejection
boundaries. They are symbolic logical effect constructions, not canonical full
source/store bytes, admitted producer histories, executable SQL or physical/native
qualification. Full byte-bound source/lineage/artifact/state fixtures remain an
execution gate; no maximum eligible profile is claimed here.

Run from the repository root:

```bash
node docs/validation/check-hierarchy-candidate-commands.mjs
```

| Case | Required outcome |
| --- | --- |
| Admission generation already exists / hole in admission prefix | Reject conflict / fence corruption; no recycled c |
| Authenticated exact retry after predecessor changes | Original receipt before predecessor CAS; no new rows/work/charges |
| Failed pass or restart after reserving work | Keep positive charge; no refund/free retry; a new pass reserves again |
| Pass cannot reserve its positive bound | Atomic terminal budget-exhausted rejection through the rejection recipe, no result |
| Receipt pool fills after candidate admission | Owed finalization needs no additional pool capacity; other command/authority checks still apply |
| Acceptance after predecessor changes | Reject finalization durably under its source-owned rules; no stale head write |
| Candidate terminal release repeated | Original disposition/outcome; no second deletion/refund |
| Missing producer artifact index / unknown dependency consumer | Unavailable/no-go; never infer missing output or omit custody effects |
| New result with no output declarations | Zero payload rows/bytes, but result/receipt/head/summary/ledger effects still count |
| Receipt and source quotas fit but complete command exceeds a cap | Reject whole command; do not drop an effect or borrow released bytes |
| New successor write ownership races internal finalization | Existing native ordering no-go; stored metadata grants no exclusion |

## Bounded review disposition

One read-only correctness review found no contradiction in slot, rehash,
subjectRevision or summary-lineage semantics. Both correctness and builder reviews
identified that the first checker demonstrated only one hard-coded union despite
seven category totals; the fixture catalog now includes table/key effect sets for
all seven constructions and the checker compares each against its recipe, with
missing/extra/duplicate effect denials. The builder review also identified the
cross-writer record-ledger gap; the ledger contract now explicitly requires every
retained evaluation/correction/disposition charge and gates unintegrated producers.
These are applied review dispositions, not a claim of post-edit independent or
native qualification. Existing reviewed contract and fixture bytes stay unchanged.

## Review handoff and remaining gates

The owner accepted the shared ledger keys/meanings, per-result payload-copy policy
and work-ledger subjectRevision choice for further design/qualification, as recorded
below. This is scoped acceptance of those three choices, not blanket acceptance
of every component contract, producer schema or executable command profile.
Existing technical definitions and fixtures remain unchanged.

This closes logical command recipes and identifies exact required producer joins;
it does not close missing artifact/custody/other-domain codecs, broader aggregate
source/payload defaults, complete byte-bound admitted histories, native query/
row/physical/resource/scratch feasibility, finalization/quiescence/recovery versus
writable ownership, shared custody or #740 eligibility. No runtime execution is
eligible until every referenced expansion is qualified and the bounded executable
plan separately approved. Keep #744/#746 open.

Next bounded preparation: bind one source profile to complete
canonical before/after command bytes, summary/lineage and owner-cap rows, then
qualify its storage and finalization ordering under a separately approved plan.
Do not substitute the symbolic accounting checker for that evidence.

## Owner acceptance and handoff — 2026-10-25

After PR #1059 merged and the three proposed choices were presented for owner
review, the owner answered “go ahead”. This accepts the following choices for
further design and qualification against reviewed source
[PR #1059](https://github.com/Rambolarsen/orkworks/pull/1059), merged as
`93c650307b641f5e517838d8f4f9ffee3056799e` (reviewed head
`0092bcce842d15079691fa4d3e597250172f82c9`):

- The shared quota_usage owner/resource keys and meanings in the closed resource
  subset, including evaluation/correction/disposition writers' record-byte charges
  and the requirement to gate producers that omit these shared effects. This
  accepts no new numerical aggregate source/payload default or extra receipt pool.
- A distinct result-keyed payload row for every present output, including zero-byte
  outputs and server-artifact bytes copied into that result. Copies count again
  in that result's payload and complete-command costs; prior artifact-production
  charges remain. No physical-sharing or source-capacity discount is inferred.
- Positive candidate-work reservation updates its candidate-owned work counters
  and commit proof, while remaining outside subjectRevision and summary material.
  Candidate admission/finalization/disposition, user-slot and retained-edge events
  keep their existing source-revision rules; work accounting gains no authority.

The accepted tradeoffs include cross-writer ledger integration before producer
eligibility and additional payload storage/copy cost. Work-budget updates still
participate in the existing pending/health publication fences; omitting a
subjectRevision increment does not permit an uncharged pass or an invisible
source outcome. Neither a quota row nor a stored work ledger/proof supplies native
quiescence, ownership exclusion or permission to finalize.

This record changes no reviewed codec, effect recipe, ledger delta, fixture,
source/request/wire field, quota value or claim/lease policy. It accepts only the
three named choices, not the entire broader storage/component design or an
admitted maximum profile. Symbolic fixtures do not become canonical full-byte
histories or native evidence through acceptance.

Missing artifact/custody/other-domain codecs, finite aggregate source/payload
defaults, exact source-profile command/lineage/owner-cap fixtures, native query/
physical/resource/scratch feasibility, finalization/quiescence/recovery ordering
against writable ownership, shared custody and separately approved executable
plans remain gates. No database, migration, native/model probe or runtime code is
approved. #740 remains deferred/no-go until November; keep #744/#746 open.

Next bounded design task: define one source profile's complete canonical
before/after command bytes, summary/lineage and owner-cap rows, preserving the
remaining authority and native qualification gates. Review new schema/default
choices separately before dependent implementation.

## Exact logical source-profile witness — 2026-10-25

The [empty-output profile proposal](2026-10-25-hierarchy-empty-output-profile-design.md)
pins complete synthetic preparation/admission/acceptance row projections,
manifests, summary checkpoints and shared ledger/cap comparisons. Decoded source
BLOB projection and subject-preparation are new choices for separate review.
Fixture cap witnesses establish no production defaults, approved bootstrap,
admitted history or physical/native eligibility. Existing codecs/catalogs are
unchanged; all producer, authority, custody and executable-plan gates remain.
