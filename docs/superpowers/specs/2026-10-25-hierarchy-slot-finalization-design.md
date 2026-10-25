---
type: Design Proposal
title: Hierarchy reserved slots and internal finalization metadata
description: Proposed format-3 reserved credit and internal outcome encodings, exact retention joins and conservative complete-command accounting.
tags: [orkworks, hierarchy, storage, recovery, receipts]
status: draft
---

# Hierarchy reserved slots and internal finalization metadata

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `4df4078b`.
**Scoped metadata schema accepted for further design/qualification; not executable.**
See the [owner acceptance](#owner-acceptance-and-handoff--2026-10-25).
This continues the [accepted compact codec](2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25).
It supplies the two remaining receipt-slot variants, not a new authority class.
No database, migration, native/model probe or runtime implementation is authorized.
#740 remains deferred/no-go until November.

Read with the [storage mapping](2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation),
[receipt reservations](2026-10-04-assignment-evaluation-design.md#receipt-ownership-reservations-and-finite-storage),
[candidate finalization](2026-10-04-assignment-evaluation-design.md#candidate-submission-and-sidecar-finalization)
and [transaction limits](2026-10-23-hierarchy-transaction-storage-design.md#proposed-finite-limits).
The accepted 1,088-slot, 17/64/256 MiB receipt pools, 16 KiB per-slot bound,
source records/digests, wire receipts and claim/lease tradeoff are unchanged.
The wider storage limits remain proposed.

## Decision and alternatives

Use a closed reserved variant and a distinct consumed internal-outcome variant
in format 3's existing receipt_slots table. Keep the accepted consumed claimed-
mutation variant byte-for-byte unchanged. Reserved capacity has an explicit
source owner; internal outcomes name the exact admitted candidate and original
finalization response. Neither variant contains a producer/UI claim or retry key.

A fabricated claim would extend the accepted action class to internal work.
A second finalization lookup/log would duplicate the already unique candidate
outcome mapping. A nullable generic receipt envelope would obscure which source
joins are required. These alternatives are excluded from this proposal.
The owner accepted this scoped choice as recorded below.

## Strict encodings and commitments

Use the existing strict canonical serializer, ID/Digest/Counter bounds, full
AssignmentIdentity/ReviewerIdentity/ResultRef and ObjectRef. Stored bytes must
round-trip exactly; reject duplicate/unknown fields, tags, wrong nullability,
noncanonical integers and unknown versions. Slot keys remain exactly
`{assignmentIdentity,slotId}`, with slotId a JSON integer 0..1087. All workspace,
assignment, source identity and ordinal checks are required in addition to hashes.

The format-3 receipt_slots value is a closed union selected by state and, for
consumed values, receipt.kind. No other variant or extension field is admitted.
The row commitment remains SHA-256 of `orkworks.hierarchy-storage-row.v3`, one
LF and canonical `{table:"receipt_slots",key:SlotKeyV3,body}`. Exclude only
lastCommit and the consumed receipt's commit from body, exactly as the accepted
codec specifies. Retain every ownership/purpose/response field. Link attachment
cannot change the hash; wrong links still fail the manifest joins.

### Reserved slot

Exactly:

```text
{schemaVersion:3,state:"reserved",purpose,reservation,lastCommit:CommitRef}
```

There is no receipt property. reservation is exactly one of:

| kind | Exact fields besides kind |
| --- | --- |
| candidate | candidateDigest: Digest |
| result | resultRevision: 1..32, resultDigest: Digest, stream: ReviewerIdentity or null |

The slot key supplies the complete owning assignment; omission from reservation
is not permission to resolve by digest alone. A candidate reservation resolves
the exact assignment-result-candidate object under that assignment and its
admission receipt. A result reservation resolves the exact assignment-result
object under that assignment; its candidateDigest resolves the admitted candidate
and outcome receipt. No latest-head substitution is allowed.

Reserved purpose is candidate-outcome, user-evaluation, user-invalidation or
child-invalidation. Candidate-admission and child-evaluation are reserved and
consumed in the same transaction and have no committed reserved variant.
Candidate reservations permit only candidate-outcome and the eight pre-result
user credits. A candidate-outcome slot never becomes a result reservation.
Result reservations permit only user-evaluation, user-invalidation and
child-invalidation. Unspent user-write slots have null stream. User invalidation
may have null stream until its paired initial write earmarks it. Child invalidation
always has the exact child reviewer stream. Earmarked user invalidation uses
only a user reviewer identity. Null does not mean reusable across subjects.

Use the unchanged ordinal mapping: candidate c=0..31 owns admission 10c,
outcome 10c+1 and user pairs 10c+2+2u / 10c+3+2u for u=0..3. Child evaluation
ordinal j=0..11 for result r owns 320+2q / 321+2q, q=12(r-1)+j.
The candidate ordinal is admission order, never resultRevision-1: rejected
candidates consume ordinals without producing result revisions.

A correction's extra invalidation credit remains reserved and unspendable.
Derive spendability from the exact paired consumed write and retained stream
history: only the stream's first-write pair may invalidate it. A second/changed
stream cannot adopt an unused correction credit. No extra spendability flag or
new stream index is introduced. The eventual bounded query/index implementation
must still account for every lookup and prove this join.

### Consumed internal outcome

Exactly:

```text
{schemaVersion:3,state:"consumed",purpose:"candidate-outcome",
 receipt:{kind:"candidate-finalization",candidate:ObjectRef,
          responseBase64,commit:CommitRef},lastCommit:CommitRef}
```

candidate.key.kind is assignment-result-candidate; candidate.key.identity is
exactly `{assignmentIdentity,candidateDigest}`. Its identity and digest
resolve the exact full candidate object and source-domain digest. The candidate's
assignment equals the slot-key assignment, and slotId is its unique 10c+1.
The same assignment's candidate_work row has state accepted or rejected and
outcomeReceipt equal to this slotId. Its candidate reference is identical.
The receipt commit and lastCommit join the same result-finalization manifest.
No result/evaluation/disposition mutation may use this branch as a missing-claim
fallback. No row is written to claim_proofs or mutation_lookup for this outcome.

responseBase64 is standard padded Base64 of the **original canonical** #744
finalization receipt, with exactly schemaVersion:1, candidateDigest, outcome,
result and reasonCode. Decoded bytes must strictly round-trip through Base64,
UTF-8 and the unchanged canonical receipt schema. Accepted requires an exact
ResultRef and null reason; rejected requires null result and bounded nonsecret
ID reasonCode. No output bytes, authority field or new wire field is added.
Serialize once and retain those original bytes for every receipt read.
The reason-code vocabulary remains the source owner's separate implementation
contract; this proposal neither selects codes nor treats free text as an ID.

For acceptance, resolve the exact result object and check its assignment,
candidateDigest, generation and predecessor against the candidate, with only
source-authorized confirmed missing outputs allowed to differ. Validate its
source digest and retained snapshot/dependency joins. Historical receipt reads
do not require that result to remain the current head. Rejection creates no
result/revision/head. Supersession and abandonment manufacture no outcome receipt;
the candidate-outcome slot remains reserved while its history is retained.

### Authority is separate from outcome evidence

An admitted candidate is immutable provenance, not a surviving producer grant.
Finalization uses the source-owned sidecar path: artifact-only requires immutable
approved artifact bindings; path-backed requires the exact terminal producer,
revoked write authority, recorded user quiescence and freshly revalidated source
worktree/output bindings. Both recheck the candidate predecessor and source caps.
After restart, missing fresh recovery evidence blocks finalization. A stored
receipt, reserved slot, valid digest, run completion or retired producer claim
cannot supply that authority or renew it.

No new finalization authority token, quiescence-record codec or ownership-ordering
mechanism is selected here. Those records/joins must be closed and qualified by
#610/#742/#744 before execution. In particular, finalization versus resumed or
successor writable ownership remains no-go: the accepted producer/UI claim-first
exception does not apply to this internal path. Copying/rehashing, ledger work
and durable rejection retain their own source-owned rules. A metadata fixture
cannot demonstrate historical authorization, native timing or lease ownership.

## Atomic transitions, retry and retention

| Transition | Required receipt-slot effects in the owning transaction |
| --- | --- |
| Candidate admission | Insert/consume admission plus nine reserved rows: one outcome and eight user credits. All ten use the same monotonic c and exact candidate. Charge ten full slots at assignment/run/workspace scope |
| Accepted finalization | Consume its existing outcome; replace eight candidate user reservations with exact result reservations and null streams. Publish result/snapshots/head, candidate_work and all dependency/summary effects together. No new receipt reservation/charge |
| Rejected finalization | Consume outcome with rejected response, set candidate_work rejected and remove the eight unused candidate user rows, reducing all three receipt pools atomically. Preserve admission, outcome and candidate ordinal |
| Supersession / explicit abandonment | Source-authorized terminal transition; remove eight unused pre-result user rows with ledger effects. Keep admission and reserved outcome, with no invented receipt. Never release accepted-result credits |
| First user write | Consume one user-write credit, earmark that pair's invalidation reservation to the issued user stream in the same commit |
| User correction | Consume another write; its paired extra invalidation credit stays null/unspendable. Existing first-write earmark is unchanged |
| Child write/correction | Create/consume its evaluation slot and reserve its paired child-invalidation slot for the exact reviewer. Only first accepted revision's pair is spendable |
| Eligible invalidation | Consume only the stream's first-write paired credit; the accepted claimed-mutation codec/lookup applies, not the internal-outcome codec |

For internal retry, locate candidate_work by the exact retained candidate ObjectRef
and its unique outcome slot, under current UI inspection/source-owned internal
access rules. An existing accepted/rejected outcome returns its original decoded
receipt without new inspection, revision, slot, lookup, proof or ledger charge.
A reserved outcome for open work is pending, not success. Superseded/abandoned
remains that terminal disposition, not pending executable work. Missing or
contradictory object/work/slot/proof joins make the subject unavailable; never
recover by choosing another candidate or latest result.

Retain candidate, admission proof/lookup/receipt, candidate_work, its remaining
reserved/consumed rows, outcome response and all required manifest/source proofs
under the same assignment/run ownership. Keep exact result/evaluation/stream
sources needed for reservation eligibility. Token refresh, restart, namespace
rotation and run completion release nothing. Purge removes eligible owned rows,
lookups and unused proofs atomically only under the existing incoming-dependency,
closed-subject, observed subjectRevision and shared-custody gates. No hash or
receipt reconstructs purged history. Native SQL recovery must establish complete
old/new state; inconsistent partial credit transfer/release stays unavailable.

## Command bytes, row counts and reservations

There are three independent checks:

1. Charge each retained/reserved slot **16,384 bytes** to every applicable
   receipt pool; actual serialized row size never discounts that reservation.
2. The complete per-slot encoded metadata and required physical overhead must
   fit its 16 KiB bound. Reserved/internal rows have no lookup/proof rows of their
   own; existing admission provenance is retained under its own charged slot.
   The accepted mutation path still charges proof + slot + one lookup together.
3. Charge every newly written/replaced key/value/link and every required deletion,
   manifest/control, source/payload/head/ledger/dependency/summary/lineage effect
   to the **proposed** 16 MiB / 4,096-row complete-command limits independently.

Use exact bytes when qualified, or a conservative full-slot charge for every
inserted/replaced slot including the associated claimed proof/lookup once.
Do not charge those associated rows a second time in bytes when already covered
by that allowance; they still each count as a changed row and appear in changes.
Do not subtract released capacity/deleted payload bytes from command admission.
A deletion has no replacement value but its key, manifest entry, ledger and
control effects remain charged. Existing unchanged rows add no changed-value
bytes; any new reference/link/index does. All new manifest fields/keys/digests
and both header/new-proof control rows count. Shared identical bytes in multiple
logical rows count in each row. SQLite physical/index/journal/scratch overhead
is separately unqualified and cannot be inferred from the logical sum.

The following counts are **minimum mandatory effects**, not complete eligible
commands; add every omitted source-specific row and byte. Header + new
commit_proof contributes two rows in each entry. No table below waives closure
of the source/ledger/index schemas or the manifest's exact change set.

| Command | Mandatory core rows | Minimum changed rows | Receipt reservation delta |
| --- | --- | ---: | ---: |
| Candidate admission | 10 slots, claim proof, mutation lookup, candidate object, candidate_work, 2 control | 16 | +163,840 bytes at each of assignment/run/workspace |
| Accepted finalization | outcome + 8 transferred user slots, result object, candidate_work, result head, 2 control | 14 | 0 |
| Rejected finalization | outcome, 8 slot deletions, candidate_work, 2 control | 12 | -131,072 bytes at each scope; no negative command credit |
| Supersession / abandonment | 8 user-slot deletions, candidate_work, 2 control | 11 | -131,072 bytes at each scope; outcome remains reserved |
| Child evaluation revision | 2 slots, claim proof, lookup, evaluation object, evaluation head, 2 control | 8 | +32,768 bytes at each target scope |

Here supersession/abandonment assumes an open candidate whose eight credits still
exist; a replay of an already terminal disposition is not eight more deletions.
Result objects, payload rows, artifact publication, heads, rubric/evidence edges,
quota resources, subjectRevision/summary/checkpoint/pending rows, old-proof deletion
and any additional indexes all count where changed. Let R be the actual changed
row set, including deletions/control; require |R| ≤4096. Each distinct (table,key)
counts once in R even when several operations change it inside one transaction.
The manifest includes precisely R minus header/new-proof control effects.
A 4,096-subject correction plus even one other row exceeds the bound.

For a conservative candidate admission, ten full-slot allowances alone are
163,840 bytes, leaving at most 16,613,376 for every other effect. Those allowances
already include the admission proof/lookup's metadata bytes, not their row counts.
For accepted finalization, nine replaced full slots cost 147,456 bytes, leaving
at most 16,629,760 for all new payload/source/head/ledger/dependency/summary/proof
and control effects. Two new 8 MiB outputs still fail before those metadata costs.
These are arithmetic upper remainders, not promised capacity or complete profiles.
A reserved pool being full does not require new receipt admission for finalization,
but command/physical/source/authority failure still rejects safely.

The [fixture catalog](../../validation/hierarchy-slot-finalization-vectors.json)
contains synthetic reserved/outcome rows, canonical response bytes, row hashes,
exact encoded key/value charges and independent row/reservation arithmetic.
The seven constructions measure 1,702–5,431 encoded key/value bytes; the
accepted-outcome construction is 5,431 bytes and the child-invalidation
reservation is 3,029. These are not exhaustive maximum-profile bounds.
Its checker below is encoding/accounting evidence only. It does not supply a
complete command manifest, native SQL/index overhead or an admitted ledger state.

## Reproduce the bounded encoding evidence

Save the following block as `/private/tmp/check-slot-finalization.mjs` and run
from this repository root with `node /private/tmp/check-slot-finalization.mjs`.
The checker exercises these fixtures, not a production strict parser or database.

<!-- slot-finalization-checker -->

```js
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const path='docs/validation/hierarchy-slot-finalization-vectors.json';
const data=JSON.parse(readFileSync(path,'utf8'));
assert.equal(createHash('sha256').update(readFileSync('docs/validation/'+data.seedCatalog)).digest('hex'),data.seedCatalogSha256);
const canon=x=>Array.isArray(x)?'['+x.map(canon).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canon(x[k])).join(',')+'}':JSON.stringify(x);
const bytes=x=>Buffer.byteLength(canon(x));
const hash=x=>createHash('sha256').update('orkworks.hierarchy-storage-row.v3\n'+canon(x)).digest('hex');
for(const v of data.vectors){
 const body=structuredClone(v.value);delete body.lastCommit;
 if(body.state==='consumed'){
  delete body.receipt.commit;
  const b=Buffer.from(body.receipt.responseBase64,'base64');
  assert.equal(b.toString('base64'),body.receipt.responseBase64);
  assert.equal(b.toString('utf8'),canon(v.response));
  assert.equal(v.response.candidateDigest,body.receipt.candidate.digest);
  assert.equal(v.response.outcome==='accepted',v.response.result!==null);
  assert.equal(v.response.outcome==='accepted',v.response.reasonCode===null);
 }
 const charge=bytes(v.key)+bytes(v.value);
 assert.equal(charge,v.expected.bytes);assert(charge<16384);
 assert.equal(hash({table:'receipt_slots',key:v.key,body}),v.expected.rowDigest);
 const changed=structuredClone(body);changed.purpose='different-purpose';
 assert.notEqual(hash({table:'receipt_slots',key:v.key,body:changed}),v.expected.rowDigest);
}
assert.deepEqual(data.accounting,{slotBytes:16384,assignmentSlots:1088,assignmentBytes:17825792,candidateSlots:10,candidateBytes:163840,releasedUserSlots:8,releasedBytes:131072,childSlots:2,childBytes:32768,admissionCoreRows:16,acceptedFinalizationCoreRows:14,rejectedFinalizationCoreRows:12,terminalDispositionCoreRows:11,childWriteCoreRows:8,commandBytes:16777216,admissionRemainder:16613376,finalizationSlotAllowance:147456,finalizationRemainder:16629760});
assert.equal(32*10+32*12*2,data.accounting.assignmentSlots);
assert.equal(1088*16384,data.accounting.assignmentBytes);
assert.equal(10*16384,data.accounting.candidateBytes);
assert.equal(8*16384,data.accounting.releasedBytes);
assert.equal(2*16384,data.accounting.childBytes);
assert.equal(10+1+1+1+1+2,data.accounting.admissionCoreRows);
assert.equal(9+1+1+1+2,data.accounting.acceptedFinalizationCoreRows);
assert.equal(1+8+1+2,data.accounting.rejectedFinalizationCoreRows);
assert.equal(8+1+2,data.accounting.terminalDispositionCoreRows);
assert.equal(2+1+1+1+1+2,data.accounting.childWriteCoreRows);
assert.equal(16777216-163840,data.accounting.admissionRemainder);
assert.equal(9*16384,data.accounting.finalizationSlotAllowance);
assert.equal(16777216-147456,data.accounting.finalizationRemainder);
console.log(`Verified ${data.vectors.length} reserved/outcome encoding fixtures and core accounting; no native/full-command qualification.`);
```

## Transition and negative oracles

These are written acceptance cases for later implementation, not executed SQL,
source-authentication, race/recovery or native tests:

| Case | Required result |
| --- | --- |
| c differs from resultRevision-1 after a rejection | Outcome/user slots still join admission c; child evaluation pairs use exact accepted result r |
| Same candidate digest under another assignment | Deny full-identity mismatch; never digest-only lookup |
| Reserved child credit has null/user stream | Reject; it cannot be rebound to a reviewer later |
| Corrected stream tries its correction-paired invalidation credit | Deny; only retained first-write pair is eligible |
| Crash during eight-credit transfer/release | Complete old/new transaction only; partial owner/ledger/proof state fences reads/writes |
| Outcome retry after head changes | Original exact response; no head CAS, second receipt or revision |
| Stored rejected response has result or no reason | Reject schema/join; cannot infer acceptance from candidate_work alone |
| Mutation has no claim and uses internal outcome kind | Deny action/source mismatch; no finalization fallback |
| Revoked producer/run ends with pending candidate | No reporting grant survives; separate fresh internal proofs decide finalization eligibility |
| Resume/successor gets writable ownership during finalization | Remains native-ordering no-go; neither receipt nor old quiescence supplies exclusion |
| Superseded/abandoned candidate without outcome | Keep reserved outcome; no accepted/rejected response is manufactured |
| Receipt pool full / command exceeds byte or row cap | No new receipt charge for owed outcome, but complete transaction fails safely under its independent limit |
| Candidate/result/proof/ledger missing or ambiguous on recovery | Unavailable; no maximum-revision election, inferred refund or new authority |

## Review handoff and unresolved gates

The owner accepted the two slot variants, unchanged wire response encoding,
exact candidate/result/stream joins, retention and independent row/byte/reservation
accounting for further design/qualification, as recorded below. The existing accepted mutation codec
and its golden fixtures are unchanged. This document does not accept the whole
format-3 store or numerical storage defaults.

Before an executable plan: close finalization/quiescence/recovery authority
records and native ordering against writable ownership; all remaining source,
ledger, summary, index and command schemas; exact complete-command/maximum-profile
fixtures with physical overhead; writer/producer/native durability/resource
qualification; #745 custody and #740 eligibility; then separately approve the
bounded executable plan. The next preparation is a complete candidate admission/
finalization command manifest and exact source-specific ledger/index effects,
not a live database or model probe. Keep #744/#746 open.

## Owner acceptance and handoff — 2026-10-25

After the scoped review and explicit decision request, the owner answered
“go ahead”, accepting this metadata proposal for further design and qualification.
The reviewed source is [PR #1053](https://github.com/Rambolarsen/orkworks/pull/1053),
merged as `8f88e0dc68c31805cd1ff599e3159b10cd56b77c`.

Accepted scope:

- The closed format-3 reserved slot and consumed internal candidate-outcome
  variants, their commitment exclusions and exact assignment/candidate/result/
  stream/ordinal joins.
- Original canonical finalization response Base64 encoding, retry resolution,
  protected correction credits and atomic credit transfer/release/retention.
- Separate receipt reservation, per-slot fit and complete-command accounting,
  including mandatory core row counts and conservative full-slot byte allowances.

The review found no blocking findings against the accepted contracts. Fresh
execution verified all seven new encoding/accounting fixtures and all sixteen
existing compact-binding expansions/hashes/charges. These are synthetic encoding
checks, not production strict-parser, admitted-history or native evidence.

This settles the named metadata choice only. It changes no source/request/wire
field, quota, claim/lease policy or technical fixture. Internal finalization
acquires no producer/UI claim or grandfathered authority. Protected unused
correction credits remain charged until eligible owner deletion. Logical fit
provides no physical writability or maximum-profile guarantee.

Complete source/ledger/index command manifests and exact full-command accounting,
physical overhead, finalization/quiescence/recovery authority and native ordering
against writable ownership, writer/resource/durability qualification, shared
custody and separately approved executable plans remain open. No database,
migration, native/model probe or runtime implementation is approved. #740 remains
deferred/no-go until November; keep #744/#746 open.

Next bounded design task: close complete candidate admission/finalization command
manifests and exact source-specific ledger/index effects. Review the new proposal
separately before dependent implementation.

## Candidate command-manifest follow-up — 2026-10-25

The [candidate command proposal](2026-10-25-hierarchy-candidate-command-manifests-design.md)
expands minimum row counts into source/receipt/quota/dependency/summary/control
sets, with bounded admission-inventory lookup, exact resource deltas and symbolic
accounting cases. Its new resource keys, payload-copy policy and source-revision
choices require separate owner review. Existing definitions and fixtures remain
unchanged; no complete byte-bound/native profile or runtime approval is claimed.

## Scoped candidate-command choices accepted — 2026-10-25

The owner [accepted three #1059 choices](2026-10-25-hierarchy-candidate-command-manifests-design.md#owner-acceptance-and-handoff--2026-10-25)
for further design/qualification: shared ledger keys/meanings and cross-writer
charges, per-result copies of every present output, and work-budget updates that
do not advance subjectRevision or summary material. This supersedes pending-review
status for those choices only. Definitions, recipes, fixture bytes, quota values,
source/wire fields and claim/lease rules are unchanged.

Missing producer/custody codecs, finite aggregate source/payload defaults, exact
full-byte command/lineage/owner-cap fixtures, native/physical/resource and
finalization-versus-writable-ownership qualification, shared custody and separate
executable-plan approvals remain gates. No database, migration, probe or runtime
work is approved. #740 remains deferred/no-go until November; keep #744/#746 open.
Next is a bounded source-profile canonical command-fixture proposal.
