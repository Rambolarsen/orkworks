---
type: Design Proposal
title: Compact exact hierarchy claim and receipt bindings
description: Accepted proof-local assignment references and reconstructable retry keys for further design and qualification, with unchanged runtime gates.
tags: [orkworks, hierarchy, authority, storage, recovery]
status: draft
workflow_status: accepted-for-design-and-qualification
---

# Compact exact hierarchy claim and receipt bindings

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `b4328f0995f3b5e67d3991e0059074478df75894`.
**Scoped codecs accepted for further design/qualification; not executable.**
See the [owner acceptance record](#owner-acceptance-and-handoff--2026-10-25).
The original reviewed proposal follows
[#1044](https://github.com/Rambolarsen/orkworks/pull/1044)'s two receipt-slot
overflows. It changes the proposed storage representation, not the
[accepted scoped claim policy](2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24).
No runtime implementation, database creation, migration, native/model probe,
quota increase or broader storage-schema/default acceptance is authorized by this document.
#740 remains deferred/no-go until November. Finalization authority and its
ordering against writable ownership remain separately unqualified.

Read with the [original claim-proof proposal](2026-10-25-hierarchy-claim-proof-design.md),
[storage contract](2026-10-24-hierarchy-storage-contract-design.md),
[canonical reporting records](2026-10-04-assignment-evaluation-design.md#canonical-records)
and [original fixture evidence](../../validation/hierarchy-claim-proof-fixtures.md).
Source schemas/digests, ReportingMutationReceipt fields, action IDs, authorization,
CAS, slot reservation/purpose, retention and source quota rules are unchanged.

## Choice and boundary

Store each complete assignment once inside its immutable proof. Use local
integer references elsewhere in that proof. Replace repeated full receipt
bindings with a proof reference and the original wire response bytes. A single
lookup row locates a receipt by a digest of its full retry key, then reconstructs
and compares the full key before returning anything. Neither a digest nor a
local index becomes an authority or equality shortcut.

This needs no shared identity dictionary, process cache, extra request log or
cross-owner storage dependency. A proof remains independently expandable.
Embedding abbreviated identities in canonical source records would change their
digests; retaining a second raw request would repeat bounded source bodies.
Increasing the 16,384-byte slot or borrowing another slot would change capacity
semantics. Those alternatives are not part of this proposal.

The measurements below close only the declared logical bytes of three metadata
rows for an accepted claimed mutation. They do not qualify SQLite overhead,
other rows in a command, reservation/finalization encodings or native authority.

## Exact proof-local representation

Use the original strict canonical serializer and field bounds. `C(x)` means its
canonical UTF-8 JSON bytes, with keys ordered by UTF-8 bytes. Reject duplicate
or unknown keys/tags, noncanonical encodings, wrong nullability and integer/string
substitutions. Full AssignmentIdentity, AuthorityGeneration, ClaimIdentity,
Counter, predecessor and source-reference types keep their original definitions.
Hashes are SHA-256 of the exact ASCII domain, one LF, then `C(value)`.

ClaimProofV2 has exactly these fields:

```text
{schemaVersion:2, assignments:[AssignmentIdentity...],
 decision:{claimIdentity,storageEpoch,admissionEpoch,action,requestDigest,
           authority,precondition},
 record}
```

The table contains one or two full identities. Entry 0 is exactly the target
subject. Entry 1 exists only for a distinct child reviewer required by child
evaluation or UI invalidation of that child stream. All entries must be used;
duplicates, extra/unused entries, cross-workspace entries and self-review are
invalid. The sidecar constructs this deterministic table before claiming and
freezes it with the decision. A later server-issued userReviewId can fill the
user-create record but cannot alter the table or decision.

The omitted decision.workspaceId is exactly assignments[0].workspaceId; the
omitted decision.subject is exactly assignments[0]. No binding is discarded.
The following are closed local forms:

| Form | Exact fields |
| --- | --- |
| LocalResultRef | assignment: 0, resultRevision, resultDigest |
| LocalReviewerIdentity, child | kind: child, assignment: 1 |
| LocalReviewerIdentity, user | kind: user, userReviewId |
| LocalEvaluationReference | assignment: 0, resultRevision, resultDigest, reviewerIdentity: local reviewer, evaluationRevision, evaluationDigest |
| Producer authority | kind: producer, assignment: 0 for result submit or 1 for child evaluation, authorityGeneration |
| UI authority | kind: workspace-ui |
| Candidate record | kind: candidate, assignment: 0, candidateDigest |
| Evaluation record | kind: evaluation, reference: local evaluation reference |
| Disposition record | kind: disposition, evaluation: local evaluation reference, dispositionRevision: 1, dispositionDigest |

`assignment` is a JSON integer, never a string or caller-supplied alias. The four
precondition variants retain exactly their original tags/fields: result-predecessor
keeps its predecessor; evaluation-predecessor uses a local result and reviewer
with the unchanged predecessor; user-slot uses a local result and unchanged
expectedUserSlot; evaluation-head uses a local evaluation reference.

Expand every local reference to the exact full identity, restore workspaceId
and subject, and change schemaVersion to 1. The result must satisfy all of the
original ClaimProofV1 action-specific semantic joins, source/request digest
checks, reviewer/subject roles, authority-generation and predecessor checks.
A decoder cannot accept an invalid action variant merely because its indices
are in range. The fixtures compare canonical expanded bytes to the unchanged
full proofs, rather than only comparing their hashes.

ProofDigestV2 uses `orkworks.hierarchy-claim-proof.v2`. ClaimProofRefV2 has exactly
claimIdentity and proofDigest, as before; the store format determines the proof
codec/domain. It is retained decision evidence, never a credential or independent
proof that the latch preceded revocation. Hashes cannot authenticate history
against same-user replacement or reconstruct a missing claim.

## Proposed storage format 3 and compatibility

The store header and ChangeManifestV3 use schemaVersion 3. Manifest fields and
command-kind requirements are the same as the v2 proposal, with claimed commands
referring strictly to ClaimProofV2. The change domain becomes
`orkworks.hierarchy-storage-change.v3`; the row domain becomes
`orkworks.hierarchy-storage-row.v3`. CommitRef keeps its field shape but resolves
only through this format's manifest codec/domain. Claim records use proof domain
v2; source and request domains do not change. Add mutation_lookup to the closed
logical table registry. Each manifest change keeps exactly table, key,
beforeDigest and afterDigest, sorted uniquely by UTF-8 table name then canonical
logical key bytes. For mutation_lookup the logical key is the digest JSON string
(the physical key remains its raw 64 ASCII bytes). Every changed proof, slot and
lookup row is listed; header/latest and the manifest's own commit_proof row keep
the original control-effect equations rather than self-hashed changes.

Compute unchanged source/request digests, then complete proof/proofDigest,
then the three row commitments, then manifest digest, and only then attach
excluded outer CommitRefs. Validate those links separately. Link attachment
cannot change hashes; substitution cannot pass the commit joins.

A format-3 reader accepts only its declared codecs, never a mixed proof-v1/v2
store, fallback expansion, implicit conversion or recomputed legacy digest.
The previous proof-v1/format-2 documents and golden bytes remain historical
proposals, unchanged. No deployed store or migration is claimed. Other domain
rows, DDL, resource/lease limits and full manifest qualification remain gated.

## Three closed claimed-mutation metadata rows

All three rows are written atomically with the original source, dependency,
summary and manifest effects. Keys are canonical JSON BLOBs except the explicitly
ASCII lookup key. The following structures are complete for this action class;
no receipt-specific alias, pointer or hidden index metadata may escape accounting.

### Claim proof

`claim_proofs` key is `C(ClaimIdentity)`. Value is exactly:

```text
{body:ClaimProofV2, proofDigest:ProofDigestV2, commit:CommitRef}
```

The row commitment envelope is `{table:"claim_proofs",key:ClaimIdentity,body}`,
where body is the stored value excluding only its outer commit. Commit and proof
identity/digest must join the same manifest and consumed receipt. The source
record reference occurs once logically in proof.record; it is not repeated in
a new canonical-record pointer field elsewhere.

### Consumed receipt slot

`receipt_slots` key is `C({assignmentIdentity:assignments[0],slotId})`, with
slotId a JSON integer 0 through 1087. The consumed claimed-mutation value is:

```text
{schemaVersion:3,state:"consumed",purpose,
 receipt:{kind:"mutation",protocol:"assignment-report-v1",
          claim:ClaimProofRefV2,responseBase64,commit:CommitRef},
 lastCommit:CommitRef}
```

Purpose is the existing candidate-admission, child-evaluation, user-evaluation,
user-invalidation or child-invalidation purpose. The slot ordinal must match the
unchanged purpose/stream/revision allocation, not merely lie in range. Derive
its exact result, stream, owning run/epoch, accepted record and request binding
from the proof and validated original receipt/source. No duplicate scopedKey,
requestDigest, result, stream or record field is stored here.

The sidecar serializes the successful ReportingMutationReceipt once as `C(receipt)`,
stores **standard padded RFC 4648 Base64** of those exact bytes, and emits those
same decoded bytes as the wire response. Receipt fields and source semantics
are unchanged. Decode must strictly round-trip to the identical Base64 string,
valid UTF-8, strict receipt shape and identical canonical JSON; whitespace,
URL-safe alphabet, missing padding and alternate encodings are rejected. All
Base64 bytes, quotes and field names count; encoded length is 4 times the ceiling
of decoded byte length divided by 3. No source body is added to the receipt.
Child artifact.contentDigest remains the raw SHA-256 of canonical evaluation
bytes, distinct from its source-domain record digest.

The row envelope is `{table:"receipt_slots",key:SlotKeyV3,body}`, excluding only
lastCommit and receipt.commit from body to avoid commitment cycles. Keep the
claim reference and responseBase64 inside the commitment. The original decoded
receipt must exactly match proof action, request digest and source/reviewer/
revision/artifact joins; do not trust an opaque response alone. Reconstruct the
complete original request from the retained source and frozen precondition,
including the receipt's idempotencyKey, using the original action-specific rules,
and verify the unchanged request-domain digest. No raw request log is added.

Reserved slots and internal candidate-finalization receipts are separate paths,
with their existing distinct authority and source requirements. This proposal
does not supply their complete encoding or physical budget and cannot be used
as a missing-claim fallback for a producer/UI mutation.

### Mutation lookup

`mutation_lookup` has exactly one row per newly accepted mutation/consumed slot,
with no aliases. Key is the raw 64 ASCII characters of RetryKeyDigestV1; value is:

```text
{schemaVersion:3,claim:ClaimProofRefV2,slotId}
```

The complete row commitment envelope is
`{table:"mutation_lookup",key:RetryKeyDigestV1,body:value}`; every value field is
committed. Its claim resolves the subject slot key. All proof, slot, manifest,
commit and original response joins must validate before use. Missing/corrupt
rows are unavailable, not a reason to elect a latest record, reclaim a slot,
repair a grant or mint authority. This adds one logical row to the original
proof-plus-slot proposal and must be included in changed-row/command limits.

## Reconstructable full retry key

RetryKeyV1 is a strict closed tagged union. Every variant has schemaVersion 1,
workspaceId, admissionEpoch, action and idempotencyKey. Additional fields are:

| kind | Additional exact fields |
| --- | --- |
| producer-result | assignmentIdentity: full producer/subject identity, authorityGeneration |
| producer-evaluation | reviewerAssignment: full child identity, authorityGeneration, target: full ResultRef |
| workspace-ui | subject: full ResultRef |

RetryKeyDigestV1 uses `orkworks.assignment-report-key.v1`. The full tuple is
constructed transiently from authenticated incoming authority and request; it
is not another persisted raw request. Reconstruct the stored tuple from the
expanded immutable proof and decoded original receipt's idempotencyKey. For UI
create/correct/invalidate, derive the exact result subject from the frozen
precondition; do not include the newly issued userReviewId, expectedUserSlot,
current head, process/token identity or claim ID. A changed expected slot/stream
changes the complete request digest, not this retry identity. A different
subject, generation, action or authority kind changes the full key.

After strict request validation, current authentication and rate checks, use
the digest only to locate a candidate row. Compare `C(incoming full key)` to
`C(reconstructed stored full key)` byte-for-byte. Equality plus equal complete
requestDigest returns the exact original decoded response, before mutable CAS,
without another claim, slot or lookup row. Equality with a changed request digest
returns the existing 409 idempotency conflict. A different full key at the same
digest is a bounded safe conflict/unavailable result: fail closed, do not
overwrite, chain collision entries, disclose another owner's response or treat
it as a retry. A forced collision test can exercise this comparison without
claiming to find an actual SHA-256 collision.

Producer revocation denies even receipt reads; a historical proof never renews
authority. UI token renewal or sidecar restart in the same admission namespace
keeps its retry identity. Do not require the current sidecar generation to equal
an old UI claim's generation. Stored producer authority/claim generations still
must agree under the original proof rules. Admission namespace changes retain
the existing denial/retention rules.

## Retention and unchanged claim/lease boundary

Keep proof, lookup, slot and all required source/manifest validation material
under the same target assignment/run ownership and existing dependency fences.
No global identity catalog is needed. Safe subject purge removes only eligible
owned rows and their lookup dependencies atomically; completion, token renewal,
restart, namespace rotation or worker retirement cannot release reservations or
orphan retry rows. Source quotas receive no physical-sharing discount.

Exactly one already accepted #744 mutation attempt may win its claim before
revoke and commit afterward; queued/later requests lose bearer authority
immediately. Revoke winning first denies the claim. The one active-or-retiring
worker/claim fence, potentially indefinite old-lease hold, independent ordinary
stop/PTY cleanup, unsettled namespace/final-finish conflict and actual lease-loss
denial remain unchanged. Launch/approval/control/cleanup, #742 late research
reports, shared learning and internal finalization gain no exception.

## Byte evidence and limits

The [compact fixture catalog](../../validation/hierarchy-compact-claim-vectors.json)
pins the unchanged seed catalog SHA-256
`b078842638d904e3944c5d4b9192ba98b365963f0be982e936de1d947ccd15a6`.
Its 16 independent cases preserve the original sources, requests, receipt fields
and full proofs by reference. It records compact proof/key/row hashes, a v3
initialization manifest digest and complete encoded key/value byte counts.
Fixture duplication is test data, not a proposed production request log.

For each claimed mutation the exact declared logical charge is:

```text
len(C(proofKey)) + len(C(proofValue)) +
len(C(slotKey)) + len(C(slotValue)) +
64 + len(C(lookupValue))
```

This includes all three complete keys/values, JSON delimiters, counters,
CommitRefs, names and Base64 expansion. It excludes no field of those rows.
Shared manifests and all source/ledger/reservation/summary/dependency effects
still count independently in whole-command/main/changed-row accounting; the
manifest cannot become an uncharged raw claim/request store.

| Case | Previous full binding bytes | Compact declared bytes | Remaining in 16,384 |
| --- | ---: | ---: | ---: |
| Maximum child evaluation correction | 19,153 | 9,503 | 6,881 |
| Maximum child stream invalidation | 17,439 | 9,038 | 7,346 |
| User invalidation maximum-identity fixture | 11,745 | 6,515 | 9,869 |

The first two previous totals are the #1044 overflow findings. The compact
maximum across all 16 vectors and width constructions is **9,503 bytes**.
The user-invalidation width construction is 6,517 bytes (9,867 remaining);
its global slotId width accounts for the two extra bytes over the fixture.
The script below recomputes every total and digest, expands all 16 proofs to
byte-identical original full bindings, and checks duplicate table entries,
wrong index types and retry-key identity distinctions.

Width constructions use the already bounded maximum ASCII local IDs, full
assignment/run identity widths, 20-digit counters, global slotId 1087 and a
six-digit child artifact.sizeBytes (524288). These are **field-width bounds,
not admitted artifact/ledger states**: the synthetic wire size need not equal
a seed's actual canonical source size, and the global maximum slot ordinal need
not belong to that vector's purpose. They prove encoding width only. All variable
source text remains outside these metadata rows; no maximal eligible native
profile, slot-allocation history or complete command is claimed.

The slot budget is reserved logical capacity, not filesystem availability.
Native SQL/index/cell/page/journal/allocator overhead and any additional required
receipt-specific implementation metadata must be qualified separately against
the remaining allowance, or the implementation remains no-go. No cap increase,
extra credit, relaxed source quota or assumed physical sharing closes that gap.

## Reproduce without a database

Save the following block as `/private/tmp/check-compact.mjs`, then run from the
repository root:

```bash
node /private/tmp/check-compact.mjs docs/validation/hierarchy-compact-claim-vectors.json docs/validation/hierarchy-claim-proof-vectors.json
```

The checker is fixture arithmetic and structural expansion, not a production
strict parser, authenticated lookup, native race test or exhaustive fuzz test.
The original negative/source/request cases remain applicable; their passing
checks do not qualify the proposed new storage runtime.

```js
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const path=process.argv[2],seedPath=process.argv[3];
const data=JSON.parse(readFileSync(path,'utf8')),seedBytes=readFileSync(seedPath),seed=JSON.parse(seedBytes);
const canon=x=>Array.isArray(x)?'['+x.map(canon).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canon(x[k])).join(',')+'}':JSON.stringify(x);
const bytes=x=>Buffer.byteLength(canon(x));
const hash=(d,x)=>createHash('sha256').update(d+'\n'+canon(x)).digest('hex');
assert.equal(createHash('sha256').update(seedBytes).digest('hex'),data.seedCatalogSha256);
function fields(x,names){assert.deepEqual(Object.keys(x).sort(),names.split(' ').sort());}
function expand(p){
 fields(p,'schemaVersion assignments decision record');assert.equal(p.schemaVersion,2);assert(p.assignments.length>=1&&p.assignments.length<=2);
 const a=p.assignments,used=new Set();assert.equal(new Set(a.map(canon)).size,a.length);assert(a.every(x=>x.workspaceId===a[0].workspaceId));
 const get=i=>{assert.equal(typeof i,'number');assert(Number.isInteger(i)&&i>=0&&i<a.length);used.add(i);return a[i];};
 const reviewer=r=>r.kind==='child'?{kind:'child',assignmentIdentity:get(r.assignment)}:structuredClone(r);
 const result=r=>{assert.equal(r.assignment,0);return {assignmentIdentity:get(r.assignment),resultRevision:r.resultRevision,resultDigest:r.resultDigest};};
 const evaluation=e=>({...result(e),reviewerIdentity:reviewer(e.reviewerIdentity),evaluationRevision:e.evaluationRevision,evaluationDigest:e.evaluationDigest});
 const d=structuredClone(p.decision);fields(d,'claimIdentity storageEpoch admissionEpoch action requestDigest authority precondition');
 if(d.authority.kind==='producer'){fields(d.authority,'kind assignment authorityGeneration');d.authority.assignmentIdentity=get(d.authority.assignment);delete d.authority.assignment;}else fields(d.authority,'kind');
 const q=d.precondition;let pre;
 if(q.kind==='result-predecessor')pre=q;
 else if(q.kind==='evaluation-predecessor')pre={kind:q.kind,result:result(q.result),reviewerIdentity:reviewer(q.reviewerIdentity),predecessor:q.predecessor};
 else if(q.kind==='user-slot')pre={kind:q.kind,result:result(q.result),expectedUserSlot:q.expectedUserSlot};
 else {assert.equal(q.kind,'evaluation-head');pre={kind:q.kind,evaluation:evaluation(q.evaluation)};}
 d.workspaceId=a[0].workspaceId;d.subject=get(0);d.precondition=pre;
 let r;const qrec=p.record;
 if(qrec.kind==='candidate'){assert.equal(qrec.assignment,0);r={kind:'candidate',assignmentIdentity:get(qrec.assignment),candidateDigest:qrec.candidateDigest};}
 else if(qrec.kind==='evaluation')r={kind:'evaluation',reference:evaluation(qrec.reference)};
 else {assert.equal(qrec.kind,'disposition');r={kind:'disposition',evaluation:evaluation(qrec.evaluation),dispositionRevision:1,dispositionDigest:qrec.dispositionDigest};}
 assert.equal(used.size,a.length);return {schemaVersion:1,decision:d,record:r};
}
function retryKey(full,wire){
 const d=full.decision,base={schemaVersion:1,workspaceId:d.workspaceId,admissionEpoch:d.admissionEpoch,action:d.action,idempotencyKey:wire.idempotencyKey};
 if(d.authority.kind==='producer')return d.action==='orkworks.result.submit.v1'?{...base,kind:'producer-result',assignmentIdentity:d.authority.assignmentIdentity,authorityGeneration:d.authority.authorityGeneration}:{...base,kind:'producer-evaluation',reviewerAssignment:d.authority.assignmentIdentity,authorityGeneration:d.authority.authorityGeneration,target:d.precondition.result};
 const q=d.precondition,r=q.kind==='evaluation-head'?q.evaluation:q.result;
 return {...base,kind:'workspace-ui',subject:{assignmentIdentity:r.assignmentIdentity,resultRevision:r.resultRevision,resultDigest:r.resultDigest}};
}
function purpose(v){return v.action==='orkworks.result.submit.v1'?'candidate-admission':v.action==='orkworks.evaluation.submit.v1'?'child-evaluation':v.action==='orkworks.user-evaluation.invalidate.v1'?(v.source.reviewerIdentity.kind==='child'?'child-invalidation':'user-invalidation'):'user-evaluation';}
function measure(v,p,sid,wire=v.receipt){
 assert(Number.isInteger(sid)&&sid>=0&&sid<=1087);
 const pd=hash('orkworks.hierarchy-claim-proof.v2',p),ref={claimIdentity:p.decision.claimIdentity,proofDigest:pd},key=retryKey(expand(p),wire),kd=hash('orkworks.assignment-report-key.v1',key),commit=v.outerCommitExample;
 const response=Buffer.from(canon(wire)).toString('base64');assert.equal(Buffer.from(response,'base64').toString(),canon(wire));
 const pk=p.decision.claimIdentity,pv={body:p,proofDigest:pd,commit};
 const sk={assignmentIdentity:p.assignments[0],slotId:sid};
 const sv={schemaVersion:3,state:'consumed',purpose:purpose(v),receipt:{kind:'mutation',protocol:'assignment-report-v1',claim:ref,responseBase64:response,commit},lastCommit:commit};
 const lv={schemaVersion:3,claim:ref,slotId:sid};
 const parts={proofKey:bytes(pk),proofValue:bytes(pv),slotKey:bytes(sk),slotValue:bytes(sv),lookupKey:64,lookupValue:bytes(lv)};
 const projected=structuredClone(sv);delete projected.lastCommit;delete projected.receipt.commit;
 return {proofDigest:pd,retryKey:key,retryKeyDigest:kd,parts,total:Object.values(parts).reduce((a,b)=>a+b,0),replyBytes:bytes(wire),proofBytes:bytes(p),proofRowDigest:hash('orkworks.hierarchy-storage-row.v3',{table:'claim_proofs',key:pk,body:{body:p,proofDigest:pd}}),slotRowDigest:hash('orkworks.hierarchy-storage-row.v3',{table:'receipt_slots',key:sk,body:projected}),lookupRowDigest:hash('orkworks.hierarchy-storage-row.v3',{table:'mutation_lookup',key:kd,body:lv})};
}
assert.equal(data.vectors.length,16);
for(const v of data.vectors){
 const s=seed.vectors.find(x=>x.id===v.id);assert(s);assert.equal(v.seedProofDigest,s.expected.proofDigest);assert.equal(canon(expand(v.compactProof)),canon(s.proof));
 const m=measure(s,v.compactProof,v.slotId);assert.equal(canon(m.retryKey),canon(v.retryKey));
 for(const k of ['proofDigest','retryKeyDigest','parts','total','replyBytes','proofBytes','proofRowDigest','slotRowDigest','lookupRowDigest'])assert.deepEqual(m[k],v.expected[k]);
 assert.equal(16384-m.total,v.expected.remaining);assert(m.total<16384);
 const wrong=structuredClone(v.compactProof);wrong.assignments.push(structuredClone(wrong.assignments[0]));assert.throws(()=>expand(wrong));
 const bad=structuredClone(v.compactProof);if(bad.record.kind==='candidate')bad.record.assignment='0';else if(bad.record.kind==='evaluation')bad.record.reference.assignment='0';else bad.record.evaluation.assignment='0';assert.throws(()=>expand(bad));
}
for(const b of data.widthBounds){const s=seed.vectors.find(x=>x.id===b.seedVector),v=data.vectors.find(x=>x.id===b.seedVector),m=measure(s,v.compactProof,b.slotId,b.wireReceipt);assert.deepEqual(m.parts,b.expected.parts);assert.equal(m.total,b.expected.total);assert.equal(16384-m.total,b.expected.remaining);}
const max=Math.max(...data.widthBounds.map(x=>x.expected.total));assert.equal(max,9503);
const create=data.vectors.find(x=>x.id==='user-create-replacement');const full=expand(create.compactProof),wire=seed.vectors.find(x=>x.id===create.id).receipt;
const changed=structuredClone(full);changed.decision.precondition.expectedUserSlot.userReviewId='different-prior';assert.equal(canon(retryKey(changed,wire)),canon(retryKey(full,wire)));const changedCompact=structuredClone(create.compactProof);changedCompact.decision.precondition.expectedUserSlot.userReviewId='different-prior';assert.notEqual(hash('orkworks.hierarchy-claim-proof.v2',changedCompact),hash('orkworks.hierarchy-claim-proof.v2',create.compactProof));
const different=structuredClone(full);different.decision.precondition.result.resultDigest='0'.repeat(64);assert.notEqual(canon(retryKey(different,wire)),canon(retryKey(full,wire))); // Full-key comparison must deny a forced colliding index hit.
assert.equal(hash('orkworks.hierarchy-storage-change.v3',data.initializationManifest),data.initializationDigest);
console.log('Node verified 16 exact binding expansions, retry-key/row/proof hashes, closed-row charges and width constructions; maximum declared logical material 9503 bytes. No SQL/native authority test.');
```

## Required decisions and remaining gates

Historical review handoff from #1049; the later scoped acceptance below resolves
its named codec decision only. All other listed qualification gates remain.

Owner review must accept or revise the concrete proof-v2/storage-v3 codecs,
canonical receipt encoding, reconstructable full-key lookup, row commitments
and accounting boundary before dependent implementation. The accepted claim
tradeoff does not automatically accept these schemas. Then reconcile the
remaining domain/reserved/finalization schemas, full-command budgets and writer
coverage; qualify native lease/claim/revoke/recovery and physical storage limits
on each supported platform. No implementation or probe starts from a passing
fixture alone. The [qualification register](../../validation/hierarchy-writer-storage-qualification.md)
remains the authoritative no-go inventory.

## Owner acceptance and handoff — 2026-10-25

The owner answered “go ahead”, then “keep going”, after the merged
[#1049 concrete schema](https://github.com/Rambolarsen/orkworks/pull/1049)
and the explicit next step to review and accept it. This records acceptance of
that reviewed representation **for further design and qualification**. It is
separate from the proposal's merge and from the earlier authority-policy
acceptance. Reviewed source is `23471c84016627f09fba57399b4e6ae2fd85d846`,
whose exact PR head was `dcfe7d6a123e2af34c4743386e7d695c0bd23dab`.

Accepted scope is limited to:

- ClaimProofV2's deterministic proof-local identity table, exact expansion and
  inherited full action/source/authority/predecessor checks, plus proof domain v2.
- The declared claimed-mutation claim_proofs, consumed receipt_slots and single
  mutation_lookup rows, their closed key/value encodings and commitment rules.
- Canonical original ReportingMutationReceipt bytes stored as standard padded
  Base64 and emitted/retried byte-for-byte, with unchanged wire fields.
- RetryKeyV1's exact producer/UI tuples and key domain, current authentication
  before lookup, reconstructed full-key comparison, changed-request conflicts
  and fail-closed digest collisions; no alias, grant restoration or raw log.
- Format-3 header/manifest/row version and domains, the closed registry extension,
  exact change coverage/order and excluded-link validation for those rows,
  strict no-mixed-codec/no-implicit-migration compatibility boundary.
- The complete declared three-row logical-byte accounting boundary and its
  limited fixture evidence; every additional implementation cost still needs
  accounting and qualification against the unchanged applicable limits.

The original sections from Choice and boundary through the executable fixture
block remain byte-identical to the reviewed proposal. Its 16 vectors still
expand exactly to the unchanged full seed proofs; the largest declared metadata
charge is 9,503 bytes. This accepts neither a physical storage bound nor a
maximal eligible source/ledger profile. Source/request domains and wire fields,
CAS/slot purpose/reservation/retention semantics and the accepted claim/lease/
retirement tradeoff are unchanged. Proof-v1/format-2 goldens remain historical
bytes, not an accepted alternate runtime codec or migration path.

This settles only the listed codec choices. Other domain schemas, reserved and
internal-finalization encodings, whole-command/row/index/resource bounds,
writer/producer coverage, lease/claim/revoke/recovery and finalization-versus-
ownership ordering, native platform durability and physical overhead remain
open. No broader numerical storage default, eligible adapter, database creation,
migration, native/model probe, executable plan or runtime implementation is
approved. #740 remains deferred/no-go until November; shared-learning custody
and separate component/implementation approvals remain independent gates.
Keep #744/#746 open.

Next bounded design work: specify the remaining reserved-slot and internal
finalization metadata with their separate authority/retention joins, then account
for those rows and the new lookup row in complete-command/row/reservation limits.
This is preparation for a separate review, not permission to implement or probe.

## Reserved-slot and internal-outcome follow-up — 2026-10-25

The [slot/finalization proposal](2026-10-25-hierarchy-slot-finalization-design.md)
adds closed proposed reserved and internal-outcome variants with separate
authority/source/retention joins, encoding fixtures and command-row/reservation
accounting. It preserves the accepted claimed-mutation codec. This is a new
proposal for separate review, not owner acceptance, complete-command/native
qualification or runtime authorization. Existing goldens and limits are unchanged.

## Reserved-slot and internal-outcome acceptance — 2026-10-25

The owner [accepted the scoped slot/finalization metadata](2026-10-25-hierarchy-slot-finalization-design.md#owner-acceptance-and-handoff--2026-10-25)
from reviewed PR #1053 for further design/qualification: the two closed variants,
exact ownership/stream/ordinal and retention joins, original canonical response
encoding, atomic credit transitions and independent logical accounting boundary.
This supersedes earlier pending-review status for these named encodings only.
Existing technical definitions, fixture bytes, source/wire schemas, quota values
and claim/lease policy are unchanged; internal finalization gains no claim.

Complete command/source/ledger/index effects, physical/native/resource and
finalization-versus-writable-ownership qualification, shared custody and separate
executable-plan approvals remain gates. No database, migration, probe or runtime
work is approved. #740 remains deferred/no-go until November; keep #744/#746 open.
Next is the complete candidate admission/finalization command-manifest proposal.
