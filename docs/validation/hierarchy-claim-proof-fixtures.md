---
type: Contract Fixtures
title: Hierarchy claim-proof action fixtures and combined byte budgets
description: Reproducible child/UI proof vectors, maximum local field widths, Unicode request reconstruction and conservative combined receipt/proof budget results.
tags: [orkworks, hierarchy, authority, fixtures, storage, validation]
status: draft
---

# Hierarchy claim-proof action fixtures and combined byte budgets

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `fc8a37e3f6cca1079c4579cd643f0a6867deb999`; proof proposal
merged in PR #1038. Evidence date: 2026-10-25. **Encoding/binding measurements
only; proposed schema remains unaccepted and every native profile unqualified.**

The [machine-readable catalog](hierarchy-claim-proof-vectors.json) contains 16
independent positive encoding vectors and 17 negative cases/oracles. It follows
the [claim-proof/format-2 proposal](../superpowers/specs/2026-10-25-hierarchy-claim-proof-design.md),
[#744 source/request/receipt schemas](../superpowers/specs/2026-10-04-assignment-evaluation-design.md#action-specific-request-objects)
and the [storage logical-row accounting](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#complete-command-accounting-and-payload-placement).
No new action, field, authority, digest domain or numeric limit is selected here.
The accepted claim/revoke and retained-lease tradeoff is unchanged.

## What was measured

Python and Node independently calculated canonical source/request/proof/claim-row
bytes and hashes from the catalog. Node also reconstructed all five action
requests from retained source/receipt/proof data, checked reviewer/UI and record
bindings, recomputed each measured budget term, distinguished artifact raw-content
hashes from evaluation record-domain hashes, and tested changed slot/authority
bindings. Python checked duplicate-key and unpaired-surrogate literals plus
claim/commit Counter type/range denial. The programs below reproduce that evidence.

These are not runtime endpoint, SQLite, filesystem, latch-ordering, crash,
producer-capability, power-loss or storage-headroom tests. No database was opened
and no coding tool/model or native capability was probed. Request/source copies
in this test catalog make fixtures self-contained; they do not authorize a second
raw request log in production. Placeholder references/generations supply no
approved assignment, retained result/predecessor history, grant or eligibility.
Each vector is a separate hypothetical case, not one batch to load into a store;
repeated synthetic claim identities across cases are not permitted in a real
sidecar lifetime.

## Coverage and width boundaries

- Child/root candidates include no-head and exact revision predecessors.
- Child evaluation includes initial/no-head and correction/revision variants,
  with a separate exact reviewer assignment/run from the target.
- User create includes empty and invalidated prior slots. The expected prior
  user slot stays distinct from the newly issued userReviewId, which is removed
  with reporterSource when reconstructing the original evaluationDraft.
- User correction preserves the issued stream and exact evaluation predecessor;
  user invalidation covers both user and child streams, including a complete
  target evaluation whose source-domain digest is independently checked.
- Small-source rationale text includes composed/decomposed accents, an emoji,
  quote, backslash, newline, tab, NUL and U+2028. Canonical UTF-8 preserves scalar
  values and normalization differences; mandatory JSON escaping remains exact.
- Width cases use the published local 128-byte ASCII ID ceiling, 64-character
  digests/generations/epochs, maximum plan/task/result revisions, 20-digit claim
  and commit Counter strings, and 2,048-byte source rationale/correction text.
  User stream revisions stay at two; child correction revisions stay at twelve.
  Run IDs use the narrower #742 `<64-hex epoch>-<runSequence>` format, whose
  maximum 63-bit run sequence is 19 digits, rather than filling a run ID to 128.

These are maximum **local field-width** cases, not claims that all source maxima
are simultaneously realizable, approved or supported by a coding-tool profile.
Tighter native identity/binding rules and source admission quotas still win.
One criterion/dimension is represented, so this does not exercise maximum source
array counts or fill a 512 KiB source record. ProofV1's own closed strings are
ASCII; Unicode exercises the inherited source/request reconstruction, not a
new Unicode identity encoding. Large claim counters are TEXT and use Python int
or Node BigInt for validation, never a rounded JavaScript Number.

## Combined logical slot accounting

The existing slot cap is **16,384 bytes**, including stored receipt owner/routing/
key/response and proof metadata. Source bodies have independent caps. Measure
raw canonical UTF-8 component values, with no trailing newline and without the
hash-domain prefix. This is logical admission accounting under the current
uncompressed row representation, not SQLite page/file/memory measurement.

The catalog's `expected.knownSlotComponentBytes` sums:

1. Claim proof BLOB, canonical ClaimIdentity primary-key value, 64 proof-digest
   ASCII bytes and its outer CommitRef value.
2. Original wire receipt BLOB, its ClaimProofRef value, receipt-slot lastCommit
   and mutation ReceiptBinding commit values (three CommitRefs total with #1).
3. Receipt-slot owner AssignmentIdentity key value, result/stream values,
   consumed-state/purpose values and mutation kind/protocol values.
4. The mutation's stored source-record reference. For evaluations this is the
   exact existing ObjectRef `{key:{kind:"assignment-evaluation",identity:EvaluationReference},digest:evaluationDigest}`.
   Here the reference and digest names stand for their complete typed values.
   Candidate/disposition
   tuple-key envelopes remain unclosed, so count the known canonical assignment
   or full nested EvaluationReference value plus revision/digest component bytes,
   omitting the outer tuple/ObjectRef delimiters and names. The nested reference
   serialization is fully counted, not reduced to its individual leaf values. This is deliberately a lower subtotal, not a chosen tuple encoding.

Repeated identity values in different required key/row/field positions count
each time. Copying a value into another field is not a quota discount. Actual
ID interning/compression or replacing full logical records with new compact
references would require a separately reviewed schema/representation decision;
this calculation neither selects nor assumes that alternative.

`knownSlotBytes` is the sum above. `remainingSlotBytes = 16384 - knownSlotBytes`
is only an upper bound on space left for **all unmeasured metadata**, not an
admission allowance. Missing terms include complete scoped retry key/routing,
slot/tuple/row/BLOB envelopes, every key/index/alias byte and other required
storage fields. Commit manifest/proof-control changes, source/artifact/payload,
dependency, summary/lineage and quota rows also consume the separate full-command
and changed-row budgets, rather than disappearing into a receipt reservation.
No source/receipt/index representation is silently truncated to fit.

| Independent vector | Proof BLOB | Wire receipt | Known slot subtotal | Maximum remaining for all unmeasured metadata |
| --- | ---: | ---: | ---: | ---: |
| `root-candidate-initial` | 1,907 | 343 | 4,008 | 12,376 |
| `root-candidate-correction-max` | 3,591 | 456 | 6,862 | 9,522 |
| `child-candidate-initial` | 2,229 | 343 | 4,546 | 11,838 |
| `child-candidate-correction-max` | 4,981 | 456 | 9,180 | 7,204 |
| `child-evaluation-initial` | 3,935 | 959 | 8,606 | 7,778 |
| `child-evaluation-correction` | 4,045 | 959 | 8,716 | 7,668 |
| `child-evaluation-correction-max` | 9,173 | 2,009 | 19,153 | **over by 2,769** |
| `user-create-empty-slot` | 2,348 | 397 | 5,610 | 10,774 |
| `user-create-replacement` | 2,474 | 397 | 5,736 | 10,648 |
| `user-create-replacement-max` | 5,346 | 620 | 11,659 | 4,725 |
| `user-correction` | 2,539 | 398 | 5,804 | 10,580 |
| `user-correction-max` | 5,409 | 621 | 11,723 | 4,661 |
| `user-invalidation` | 2,618 | 402 | 5,826 | 10,558 |
| `user-invalidation-max` | 5,488 | 625 | 11,745 | 4,639 |
| `child-stream-invalidation` | 3,464 | 825 | 7,942 | 8,442 |
| `child-stream-invalidation-max` | 7,766 | 1,763 | 17,439 | **over by 1,055** |

Two width cases already exceed the slot before missing overhead:

- Child evaluation correction: **19,153 bytes**, at least **2,769 over** the cap.
- Child-stream invalidation: **17,439 bytes**, at least **1,055 over** the cap.

Under the current logical row contract these cases must reject/roll back before
acceptance. A proof BLOB fitting by itself does not make its whole slot fit.
The result does not justify spending another receipt slot, borrowing future user
credits, dropping lineage/authority bindings, charging an unbounded side table,
or raising the accepted cap. Invalidating a child stream uses that child's
protected invalidation slot, not the user-stream pool. Its capacity reservation
still does not promise metadata/physical writability for every source maximum.

A nonnegative remaining value also proves no eligible profile: missing terms
must fit exactly, with all independent source, command, row and native limits.
For example, the max user-create vector leaves 4,725 bytes, but has no bound yet
for its complete lookup/key/index/storage overhead. No fixture here measures
SQLite B-tree pages, journal growth, disk allocation or allocator/scratch costs.
The 16 MiB command cap also includes all other publication effects; these slot
subtotals cannot certify a transaction's whole command or fan-out budget.

## Negative cases and limits of execution

The catalog separates literal/mutation checks from written runtime oracles.
The supplied programs execute Counter type/range, duplicate-key, scalar-UTF-8,
overlong-ID, changed-request-digest and changed-proof/authority assays. They do
not implement or invoke a sidecar validator. Missing-proof, swapped-claim,
revoked-retry, purge and SQL rollback entries remain expected behavior for later
runtime/native tests, not passing test results. The existing proposed failure
oracles and all unqualified gates remain authoritative.

## Reproduce with Python

Save the following block as `/private/tmp/check-claim-fixtures.py`, then run
`rtk proxy python3 /private/tmp/check-claim-fixtures.py docs/validation/hierarchy-claim-proof-vectors.json`
from the repository root. It uses only Python's standard library.

```python
import json, hashlib, re, sys
from pathlib import Path

def unique_pairs(pairs):
    out={}
    for k,v in pairs:
        if k in out: raise ValueError('duplicate key')
        out[k]=v
    return out

def canonical(x):
    return json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode('utf-8')

def counter(x):
    return isinstance(x,str) and re.fullmatch('[1-9][0-9]*',x) and int(x)<=2**64-1

def digest(domain,x):
    return hashlib.sha256(domain.encode()+b'\n'+canonical(x)).hexdigest()

c=json.loads(Path(sys.argv[1]).read_text(),object_pairs_hook=unique_pairs)
domains={'candidate':'orkworks.assignment-result-candidate.v1','evaluation':'orkworks.assignment-evaluation.v1','disposition':'orkworks.assignment-disposition.v1'}
for v in c['vectors']:
    e=v['expected']; p=v['proof']; d=p['decision']; r=v['receipt']; s=v['source']
    assert len(canonical(s))==e['sourceCanonicalBytes']
    assert digest(domains[v['sourceKind']],s)==e['sourceDigest']
    assert digest('orkworks.assignment-report-request.v1',v['request'])==e['requestDigest']==d['requestDigest']==r['requestDigest']
    assert digest('orkworks.hierarchy-claim-proof.v1',p)==e['proofDigest']
    assert len(canonical(p))==e['proofCanonicalBytes']
    row={'table':'claim_proofs','key':d['claimIdentity'],'body':{'body':p,'proofDigest':e['proofDigest']}}
    assert digest('orkworks.hierarchy-storage-row.v2',row)==e['claimRowDigest']
    assert len(canonical(row))==e['claimRowCommitmentBytes']
    assert len(canonical(r))==e['wireReceiptBytes']
    assert sum(e['knownSlotComponentBytes'].values())==e['knownSlotBytes']
    assert 16384-e['knownSlotBytes']==e['remainingSlotBytes']
    assert counter(d['claimIdentity']['sequence'])
    assert counter(v['outerCommitExample']['commitSequence'])
for n in c['negativeCases']:
    if n['kind']=='raw-claim-identity-json':
        try: json.loads(n['raw'],object_pairs_hook=unique_pairs)
        except ValueError: pass
        else: raise AssertionError('accepted duplicate key')
    if n['kind']=='raw-text-json':
        try: json.loads(n['raw']).encode('utf-8')
        except UnicodeEncodeError: pass
        else: raise AssertionError('accepted lone surrogate')
    if n['kind'] in ('claim-sequence','commit-sequence'):
        assert not counter(n['value'])
print('Python verified golden byte/hash values, exact integer-string bounds, duplicate-key and surrogate literal rejection. No database or runtime opened.')
```

## Reproduce with Node

Save this block as `/private/tmp/check-claim-fixtures.mjs`, then run
`rtk proxy node /private/tmp/check-claim-fixtures.mjs docs/validation/hierarchy-claim-proof-vectors.json`.
It uses Node built-ins and opens no database or network connection. Its canonical
serializer is a fixture checker for these closed shapes, not a production
replacement for the reviewed lossless source codec.

```javascript
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const catalog=JSON.parse(readFileSync(process.argv[2],'utf8'));
const canon=x=>Array.isArray(x)?'['+x.map(canon).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canon(x[k])).join(',')+'}':JSON.stringify(x);
const bytes=x=>Buffer.byteLength(canon(x));
const hash=(domain,x)=>createHash('sha256').update(domain+'\n'+canon(x)).digest('hex');
const raw=x=>createHash('sha256').update(canon(x)).digest('hex');
const domains={candidate:'orkworks.assignment-result-candidate.v1',evaluation:'orkworks.assignment-evaluation.v1',disposition:'orkworks.assignment-disposition.v1'};
const ref=(s,d)=>Object.fromEntries(['assignmentIdentity','resultRevision','resultDigest','reviewerIdentity','evaluationRevision'].map(k=>[k,s[k]]).concat([['evaluationDigest',d]]));
const result=s=>Object.fromEntries(['assignmentIdentity','resultRevision','resultDigest'].map(k=>[k,s[k]]));
function counter(x){assert.equal(typeof x,'string');assert(/^[1-9][0-9]*$/.test(x));assert(BigInt(x)<=18446744073709551615n);}
function identity(x){
 for(const [k,v] of Object.entries(x))if(k.endsWith('Id')){assert.equal(typeof v,'string');assert(/^[A-Za-z0-9._-]{1,128}$/.test(v));}
 assert(/^[a-f0-9]{64}-[1-9][0-9]{0,18}$/.test(x.runId));assert(BigInt(x.runId.slice(65))<=9223372036854775807n);
 if(x.kind==='child'){assert(x.planRevision>=1&&x.planRevision<=64);assert(x.taskVersion>=1&&x.taskVersion<=2147483647);}
}
assert.equal(catalog.vectors.length,16);
const actions=new Set();
for(const v of catalog.vectors){
 const {source:s,proof:p,receipt:r,request:q,expected:e}=v;const d=p.decision;actions.add(q.action);
 assert.equal(bytes(s),e.sourceCanonicalBytes);assert.equal(hash(domains[v.sourceKind],s),e.sourceDigest);
 let rebuilt={schemaVersion:1,action:r.action,idempotencyKey:r.idempotencyKey};
 if(q.action==='orkworks.result.submit.v1')rebuilt.candidate=s;
 else if(q.action==='orkworks.user-evaluation.create.v1'){const draft=structuredClone(s);delete draft.reviewerIdentity;delete draft.reporterSource;rebuilt.evaluationDraft=draft;rebuilt.expectedUserSlot=d.precondition.expectedUserSlot;assert.equal(s.reviewerIdentity.kind,'user');assert.equal(s.reporterSource.kind,'user');if(rebuilt.expectedUserSlot.kind==='invalidated')assert.notEqual(s.reviewerIdentity.userReviewId,rebuilt.expectedUserSlot.userReviewId);}
 else if(q.action==='orkworks.user-evaluation.invalidate.v1'){rebuilt.disposition=s;rebuilt.resultDigest=d.precondition.evaluation.resultDigest;assert.deepEqual(ref(v.targetEvaluation,hash(domains.evaluation,v.targetEvaluation)),d.precondition.evaluation);assert.equal(s.evaluationDigest,d.precondition.evaluation.evaluationDigest);}
 else rebuilt.evaluation=s;
 assert.equal(canon(rebuilt),canon(q));assert.equal(bytes(q),e.requestCanonicalBytes);
 assert.equal(hash('orkworks.assignment-report-request.v1',rebuilt),e.requestDigest);assert.equal(d.requestDigest,e.requestDigest);assert.equal(r.requestDigest,e.requestDigest);
 assert.deepEqual(d.subject,s.assignmentIdentity);identity(d.subject);counter(d.claimIdentity.sequence);counter(v.outerCommitExample.commitSequence);assert.equal(d.workspaceId,d.subject.workspaceId);
 if(d.authority.kind==='producer'){
  identity(d.authority.assignmentIdentity);assert.equal(d.authority.authorityGeneration.sidecarGeneration,d.claimIdentity.sidecarGeneration);
  if(q.action==='orkworks.result.submit.v1'){assert.deepEqual(d.authority.assignmentIdentity,d.subject);assert.deepEqual(s.authorityGeneration,d.authority.authorityGeneration);assert.deepEqual(s.predecessor,d.precondition.predecessor);}
  else {assert.equal(q.action,'orkworks.evaluation.submit.v1');assert.deepEqual(s.reviewerIdentity.assignmentIdentity,d.authority.assignmentIdentity);assert.notEqual(canon(d.subject),canon(d.authority.assignmentIdentity));assert.deepEqual(s.reporterSource.authorityGeneration,d.authority.authorityGeneration);}
 }else {assert.deepEqual(d.authority,{kind:'workspace-ui'});assert(q.action.startsWith('orkworks.user-evaluation.'));}
 if(v.sourceKind==='candidate')assert.deepEqual(p.record,{kind:'candidate',assignmentIdentity:s.assignmentIdentity,candidateDigest:e.sourceDigest});
 if(v.sourceKind==='evaluation'){assert.deepEqual(p.record,{kind:'evaluation',reference:ref(s,e.sourceDigest)});assert.equal(s.reporterSource.kind,q.action==='orkworks.evaluation.submit.v1'?'child':'user');if(q.action!=='orkworks.user-evaluation.create.v1')assert.deepEqual(d.precondition.predecessor,s.predecessor);assert(s.evaluationRevision<=(s.reviewerIdentity.kind==='child'?12:2));}
 if(v.sourceKind==='disposition'){assert.equal(p.record.dispositionDigest,e.sourceDigest);assert.deepEqual(p.record.evaluation,d.precondition.evaluation);assert.equal(s.actor,'user');}
 const pd=hash('orkworks.hierarchy-claim-proof.v1',p);assert.equal(pd,e.proofDigest);assert.equal(bytes(p),e.proofCanonicalBytes);
 const row={table:'claim_proofs',key:d.claimIdentity,body:{body:p,proofDigest:pd}};
 assert.equal(bytes(row),e.claimRowCommitmentBytes);assert.equal(hash('orkworks.hierarchy-storage-row.v2',row),e.claimRowDigest);
 assert.equal(bytes(r),e.wireReceiptBytes);assert.equal(r.recordDigest,e.sourceDigest);assert.equal(r.recordKind,v.sourceKind);
 if(q.action==='orkworks.evaluation.submit.v1'){assert.equal(r.artifact.contentDigest,raw(s));assert.equal(r.artifact.sizeBytes,bytes(s));assert.equal(r.artifact.version,s.evaluationRevision);assert.notEqual(r.artifact.contentDigest,r.recordDigest);}else assert.equal(r.artifact,null);
 const pref={claimIdentity:d.claimIdentity,proofDigest:pd};
 let rv=null,sv=null,purpose,rm;
 if(v.sourceKind==='candidate'){purpose='candidate-admission';rm=bytes(d.subject)+128;}
 else if(v.sourceKind==='evaluation'){rv=result(s);sv=s.reviewerIdentity;purpose=q.action==='orkworks.evaluation.submit.v1'?'child-evaluation':'user-evaluation';rm=bytes({key:{kind:'assignment-evaluation',identity:ref(s,e.sourceDigest)},digest:e.sourceDigest});}
 else {rv=result(d.precondition.evaluation);sv=s.reviewerIdentity;purpose=s.reviewerIdentity.kind==='child'?'child-invalidation':'user-invalidation';rm=bytes(d.precondition.evaluation)+1+64;}
 const parts={proofBlob:bytes(p),proofPrimaryKey:bytes(d.claimIdentity),proofDigestAscii:64,threeCommitLinkValues:3*bytes(v.outerCommitExample),wireReceiptBlob:bytes(r),receiptClaimReference:bytes(pref),slotOwnerKeyIdentity:bytes(d.subject),slotResultValue:bytes(rv),slotStreamValue:bytes(sv),receiptRecordReferenceMinimum:rm,slotStateValue:bytes('consumed'),slotPurposeValue:bytes(purpose),receiptKindValue:bytes('mutation'),receiptProtocolValue:bytes('assignment-report-v1')};
 assert.deepEqual(parts,e.knownSlotComponentBytes);const sum=Object.values(parts).reduce((a,b)=>a+b,0);assert.equal(sum,e.knownSlotBytes);assert.equal(16384-sum,e.remainingSlotBytes);
 if(v.sourceKind==='evaluation'){const normalized=structuredClone(s);normalized.rating.rationale=normalized.rating.rationale.normalize('NFC');if(v.profile.startsWith('short'))assert.notEqual(hash(domains.evaluation,normalized),e.sourceDigest);}
 if(v.profile.startsWith('maximum')&&v.sourceKind==='evaluation')assert.equal(Buffer.byteLength(s.rating.rationale),2048);
}
assert.equal(actions.size,5);
for(const n of catalog.negativeCases){
 if(n.kind==='claim-sequence'||n.kind==='commit-sequence')assert.throws(()=>counter(n.value));
 if(n.kind==='id')assert(Buffer.byteLength(n.value)>128);
 if(n.kind==='raw-text-json'){const x=JSON.parse(n.raw);assert.notEqual(Buffer.from(x).toString('utf8'),x);}
 if(n.id==='changed-user-slot'){const v=catalog.vectors.find(v=>v.id===n.seed);const changed=structuredClone(v.request);changed.expectedUserSlot.userReviewId=n.value;assert.equal(changed.idempotencyKey,v.request.idempotencyKey);assert.notEqual(hash('orkworks.assignment-report-request.v1',changed),v.expected.requestDigest);}
 if(n.id==='authority-class-switch'){const v=catalog.vectors.find(v=>v.id===n.seed);const p=structuredClone(v.proof);p.decision.authority=n.value;assert.notEqual(hash('orkworks.hierarchy-claim-proof.v1',p),v.expected.proofDigest);assert.equal(v.action,'orkworks.evaluation.submit.v1');assert.notEqual(p.decision.authority.kind,'producer');}
}
const over=catalog.vectors.filter(v=>v.expected.remainingSlotBytes<0);assert.deepEqual(over.map(v=>[v.id,v.expected.knownSlotBytes]),[['child-evaluation-correction-max',19153],['child-stream-invalidation-max',17439]]);
console.log(`Verified ${catalog.vectors.length} fixture source/request/proof/row hashes, request reconstruction and byte subtotals across ${actions.size} actions. Encoding/binding assays only; no runtime/SQL/native authority test. Two logical overflows confirmed.`);
```

## Handoff and next decision

This pass supplies the missing five-action/reviewer/UI/Unicode/field-width
encoding coverage and measured combined-slot lower subtotals. It also identifies
a concrete packing problem in the proposed full-identity representation. Schema
acceptance is still pending; a hash match or successful fixture check is not an
accepted implementation or proof of historical claim timing.

Next bounded design work is to reconcile proof/receipt representation with the
16 KiB cap: review compact, exact immutable bindings or a restricted eligible
identity profile, retaining every authority/source/predecessor join. Any changed
schema/profile/limit needs explicit review and acceptance; this pass chooses
none. Close the remaining scoped-key/tuple/index encodings and complete command/
row maxima before an executable plan. Native resource/durability/lease/writer/
health/finalization evidence, #740 eligibility, shared custody and separately
approved qualification/runtime plans remain gates. Keep #744/#746 open.

## Compact-binding follow-up — 2026-10-25

The [compact exact-binding proposal](../superpowers/specs/2026-10-25-hierarchy-compact-bindings-design.md) responds to #1044's two 16 KiB receipt-slot overflows. Its proof-v2/storage-v3 encoding preserves
all 16 original full bindings by exact expansion and measures at most 9,503
declared logical metadata bytes across three rows, including a lookup row and
Base64 wire response. Original proof-v1/format-2 vectors remain unchanged.
This is a new schema proposal requiring review, not acceptance, a migration,
physical storage qualification, native authority evidence or runtime approval.
Whole-command, reserved/finalization and native platform gates remain open.

## Compact codec acceptance — 2026-10-25

The owner [accepted #1049's exact compact codecs](../superpowers/specs/2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25)
for further design/qualification: proof-v2 exact local references, the three
claimed-mutation metadata rows and full-key retry lookup, canonical original
receipt bytes, scoped format-3 commitments and logical-byte accounting. This
supersedes acceptance-pending wording only for those named choices. Earlier
proof-v1/format-2 encodings and fixture bytes remain historical and unchanged;
no mixed parsing or implicit upgrade is accepted.

Source/request/wire fields, quotas and accepted claim/lease policy are unchanged.
No physical/whole-command qualification, remaining domain/reserved/finalization
schema/default acceptance, migration, probe, executable plan or runtime approval
follows. The linked record lists the remaining gates and next bounded metadata/
budget design work. #740 stays deferred/no-go until November; keep #744/#746 open.
