// Fixed conditional source-history checks; no live authority or storage writes.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {verifyReviewHistory} from './coherent-review-history.mjs';
const read=name=>readFileSync(new URL(name,import.meta.url));
const packet=JSON.parse(read('coherent-review-history.json')),r2=JSON.parse(read('r2-report-vectors.json'));
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
function reseal(entry,value,domain){
  entry.canonicalUtf8=canonical(value);entry.utf8Bytes=Buffer.byteLength(entry.canonicalUtf8);entry.sha256=sha(entry.canonicalUtf8);
  if(domain)entry.domainDigest=sha(domain+'\n'+entry.canonicalUtf8);
}
const change=(p,index,fn)=>{const e=p.history[index].record,v=JSON.parse(e.canonicalUtf8);fn(v);reseal(e,v,index===2?'orkworks.assignment-disposition.v1':'orkworks.assignment-evaluation.v1');};
for(const [name,digest] of Object.entries(packet.inputs))test(`${name}: exact unchanged source input`,()=>assert.equal(sha(read(name)),digest));
test('independent Python history bytes, exact joins, artifact digests and retained charges',()=>{
  assert.deepEqual(verifyReviewHistory(packet,r2),packet.measurements);
});
for(const [name,mutate,reason] of [
 ['placeholder predecessor',p=>change(p,1,e=>e.predecessor.evaluationDigest='c'.repeat(64)),/predecessor/],
 ['stale invalidation',p=>change(p,2,e=>{e.evaluationRevision=1;e.evaluationDigest=p.history[0].record.domainDigest;}),/invalidation head/],
 ['wrong result digest',p=>change(p,0,e=>e.resultDigest='0'.repeat(64)),/result join/],
 ['wrong snapshot',p=>change(p,0,e=>e.criteriaSnapshotDigest='0'.repeat(64)),/snapshot/],
 ['child impersonates user',p=>change(p,0,e=>e.reporterSource={kind:'user'}),/reporter/],
 ['wrong authority generation',p=>change(p,0,e=>e.reporterSource.authorityGeneration.generation='0'.repeat(64)),/reporter/],
 ['empty correction reason',p=>change(p,1,e=>e.correctionReason=''),/text/],
 ['duplicate revision',p=>change(p,1,e=>e.evaluationRevision=1),/revision/],
 ['child invalidates stream',p=>change(p,2,e=>e.actor='child'),/invalidation actor/],
 ['different reviewer',p=>change(p,1,e=>e.reviewerIdentity.assignmentIdentity.childSessionId='foreign'),/reviewer/],
 ['cross-workspace reviewer',p=>p.bindings.reviewerAssignment.workspaceId='foreign',/workspace/],
 ['self review',p=>p.bindings.reviewerAssignment=structuredClone(p.bindings.targetAssignment),/independence/],
 ['root target',p=>p.bindings.targetAssignment.kind='root',/child target/],
 ['insufficient approved read budget',p=>p.bindings.reviewInput.maxReadBytes--,/read budget/],
 ['undeclared evidence',p=>p.bindings.reviewInput.evidenceSources.pop(),/evidence selection/],
 ['wrong evaluation declaration',p=>p.bindings.outputContract.declarations[0].kind='verification_report',/output contract/],
 ['post-invalidation correction',p=>p.history.push(structuredClone(p.history[1])),/frozen/],
 ['missing initial record',p=>p.history.shift(),/initial|revision/],
])test(`rehashed source history rejects ${name}`,()=>{
  const p=structuredClone(packet);mutate(p);assert.throws(()=>verifyReviewHistory(p,r2),reason);
});
test('raw artifact digest cannot be replaced by evaluation-domain digest',()=>{
  const p=structuredClone(packet),entry=p.history[1].receipt,v=JSON.parse(entry.canonicalUtf8);
  v.artifact.contentDigest=p.history[1].record.domainDigest;reseal(entry,v);
  assert.throws(()=>verifyReviewHistory(p,r2),/artifact/);
});
test('changed request cannot hide behind a valid record digest',()=>{
  const p=structuredClone(packet),entry=p.history[1].request,v=JSON.parse(entry.canonicalUtf8);
  v.evaluation.resultDigest='0'.repeat(64);reseal(entry,v,'orkworks.assignment-report-request.v1');
  assert.throws(()=>verifyReviewHistory(p,r2),/request/);
});
test('artifact read must return the exact immutable revision',()=>{
  const p=structuredClone(packet),entry=p.history[1].artifactRead,v=JSON.parse(entry.canonicalUtf8);
  v.evaluation=JSON.parse(p.history[0].record.canonicalUtf8);reseal(entry,v);
  assert.throws(()=>verifyReviewHistory(p,r2),/artifact read/);
});
test('a head-only recount cannot remove retained initial evaluation bytes',()=>{
  const m=verifyReviewHistory(packet,r2);
  assert(BigInt(m.phases[1].targetRecordBytes)>BigInt(m.phases[0].targetRecordBytes));
  assert.equal(BigInt(m.phases[2].reviewerArtifactBytes),BigInt(m.phases[1].reviewerArtifactBytes));
});

test('child correction keeps its extra invalidation credit protected after user invalidates the stream',()=>{
  const m=verifyReviewHistory(packet,r2);
  assert.deepEqual(m.phases.map(p=>[p.childReceiptUsedBytes,p.childReceiptReservedBytes]),[
    ['16384','16384'],['32768','32768'],['49152','16384']]);
  assert.deepEqual(m.phases[2].childSlots,[
    {slotId:320,purpose:'child-evaluation',state:'consumed'},
    {slotId:321,purpose:'child-invalidation',state:'consumed'},
    {slotId:322,purpose:'child-evaluation',state:'consumed'},
    {slotId:323,purpose:'child-invalidation',state:'reserved'}]);
});
