import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {verifySourceBridge} from './summary-source-packing.mjs';
import {decodeSummary,validateRequest,validateResponse} from './hierarchy-summary-components.mjs';
const read=name=>readFileSync(new URL(name,import.meta.url));
const packet=JSON.parse(read('summary-source-packing.json'));
const store=JSON.parse(read('hierarchy-empty-output-profile.json'));
const sha=raw=>createHash('sha256').update(raw).digest('hex');

test('the source bridge binds exact candidate/result bytes, logical accepted head, summary and approved-context fixture bytes',()=>{
  assert.equal(verifySourceBridge(store,packet.sourceBridge),packet.sourceBridge.response.sha256);
  assert.equal(sha(read('hierarchy-empty-output-profile.json')),packet.inputs.logicalStoreSha256);
  assert.equal(sha(read('hierarchy-summary-vectors.json')),packet.inputs.displayCatalogSha256);
});
for(const [name,mutate] of [
  ['result source byte',s=>s.sources.resultCanonical=s.sources.resultCanonical.replace('target-root','other-root')],
  ['candidate source byte',s=>s.sources.candidateCanonical=s.sources.candidateCanonical.replace('target-root','other-root')],
  ['accepted head',s=>{const r=s.states.at(-1).rows.find(r=>r.table==='heads');const v=JSON.parse(r.valueCanonical);v.target.digest='0'.repeat(64);r.valueCanonical=JSON.stringify(v);}],
  ['retained result object',s=>{const r=s.states.at(-1).rows.find(r=>r.table==='objects' && JSON.parse(r.keyCanonical).kind==='assignment-result');const v=JSON.parse(r.valueCanonical);v.body.resultRevision=2;r.valueCanonical=JSON.stringify(v);}],
  ['summary lineage token',s=>{const r=s.states.at(-1).rows.find(r=>r.table==='summary_state');const v=JSON.parse(r.valueCanonical);v.row.summaryRevision='3';r.valueCanonical=JSON.stringify(v);}],
  ['summary material',s=>{const r=s.states.at(-1).rows.find(r=>r.table==='summary_state');const v=JSON.parse(r.valueCanonical);v.row.state.material.accepted.reference.resultDigest='0'.repeat(64);r.valueCanonical=JSON.stringify(v);}],
  ['missing result head',s=>{s.states.at(-1).rows=s.states.at(-1).rows.filter(r=>r.table!=='heads');}],
  ['duplicate result head',s=>{s.states.at(-1).rows.push(structuredClone(s.states.at(-1).rows.find(r=>r.table==='heads')));}],
]) test(`source bridge rejects ${name} drift`,()=>{
  const altered=structuredClone(store);mutate(altered);assert.throws(()=>verifySourceBridge(altered,packet.sourceBridge));
});
test('source bridge rejects display digest drift instead of rewriting canonical source identities',()=>{
  const bridge=structuredClone(packet.sourceBridge);const r=JSON.parse(bridge.response.canonicalUtf8);
  r.selection.accepted.reference.resultDigest='0'.repeat(64);bridge.response.canonicalUtf8=JSON.stringify(r);
  assert.throws(()=>verifySourceBridge(store,bridge));
});
for(const [kind,entry] of Object.entries(packet.maximumRequests)) test(`maximum-width ${kind} requests fit both bounded hops`,()=>{
  const renderer=decodeSummary(Buffer.from(entry.renderer.canonicalUtf8),4096);
  const main=decodeSummary(Buffer.from(entry.main.canonicalUtf8),4096);
  const context={workspaceId:main.workspaceId,runId:main.runId,sidecarGeneration:main.sidecarGeneration,
    snapshotVersion:renderer.expectedSnapshotVersion,selection:renderer.selection,fresh:true};
  for(const [hop,request] of [['renderer',renderer],['main',main]]){
    assert.equal(validateRequest(request,context,hop),request);
    assert.equal(Buffer.byteLength(entry[hop].canonicalUtf8),entry[hop].utf8Bytes);
    assert.equal(sha(Buffer.from(entry[hop].canonicalUtf8)),entry[hop].sha256);
  }
});
test('all display measurements count complete retained wire bytes and material separately',()=>{
  const catalog=JSON.parse(read('hierarchy-summary-vectors.json'));
  assert.equal(packet.displayMeasurements.length,catalog.vectors.length);
  for(const m of packet.displayMeasurements){
    const v=catalog.vectors.find(v=>v.id===m.id);assert(v);
    for(const field of ['rendererRequest','mainRequest','response'])assert.equal(m[field+'Bytes'],Buffer.byteLength(v[field].canonicalUtf8));
    assert.equal(m.materialBytes,Buffer.byteLength(JSON.stringify(JSON.parse(v.response.canonicalUtf8).detail.material)));
  }
});
test('compact bound charges material, every envelope component and the global effective-dispute reference budget',()=>{
  const b=packet.compactResponseBound;
  assert.equal(b.maxEffectiveDisputeReferences,16*32);
  assert.equal(b.disputeExtraBytes,512*67);
  assert.equal(b.baseWithoutMaterialBytes,b.envelopeBytes+Object.values(b.componentBytes).reduce((a,v)=>a+v,0));
  assert.equal(b.upperBytes,b.baseWithoutMaterialBytes+b.materialBytes+b.disputeExtraBytes);
  assert.equal(b.headroomBytes,4194304-b.upperBytes);
  assert(b.headroomBytes>0);
  // The calculation must include both escaped context arrays, not only material.
  assert(b.componentBytes.criteria>32*2048*6);
  assert(b.componentBytes.rubric>16*2048*6);
  assert(b.componentBytes.outcomes>49*16*64);
  assert(b.componentBytes.findingSources>512*128);
});

test('the retained checkpoint link agrees with the selected finalization commit in this fixture',()=>{
  const changed=structuredClone(store);
  const row=changed.states.at(-1).rows.find(r=>r.table==='summary_state');
  const value=JSON.parse(row.valueCanonical);value.checkpoint.applied.changeDigest='0'.repeat(64);row.valueCanonical=JSON.stringify(value);
  assert.throws(()=>verifySourceBridge(changed,packet.sourceBridge));
});
for(const [kind,domain] of [['criteria','orkworks.criteria-snapshot.v1'],['rubric','orkworks.rubric-snapshot.v1']]) test(`self-consistent substituted ${kind} cannot replace the retained summary snapshot`,()=>{
  const altered=structuredClone(store),bridge=structuredClone(packet.sourceBridge);
  const response=JSON.parse(bridge.response.canonicalUtf8);
  if(kind==='criteria')altered.profile.criteria[0].description='Substituted approved context';
  else altered.profile.rubric.dimensions[0].description='Substituted approved context';
  response.detail[kind]=altered.profile[kind];
  // Explicit sorted object-key spelling for the substituted snapshot. This
  // creates valid source-domain digests, not a malformed-hash rejection.
  const rubric=altered.profile.rubric;
  const snapshot=kind==='criteria'?altered.profile.criteria.map(({id,requirement,description})=>({description,id,requirement})):
    {dimensions:rubric.dimensions.map(({id,description})=>({description,id})),evaluatorRole:rubric.evaluatorRole,id:rubric.id,version:rubric.version};
  const digest=sha(Buffer.from(domain+'\n'+JSON.stringify(snapshot)));
  response.detail.material[kind==='criteria'?'criteriaSnapshotDigest':'rubricSnapshotDigest']=digest;
  const raw=Buffer.from(JSON.stringify(response));
  bridge.response={canonicalUtf8:raw.toString(),utf8Bytes:raw.length,sha256:sha(raw)};
  assert.doesNotThrow(()=>validateResponse(response,JSON.parse(bridge.rendererRequest.canonicalUtf8),bridge.context));
  assert.throws(()=>verifySourceBridge(altered,bridge));
});
