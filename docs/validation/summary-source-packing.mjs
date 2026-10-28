// Documentation-only join assay for the existing empty-output logical fixture.
// This does not validate admission, authority, currentness, native storage or
// arbitrary result/evaluation source codecs. Never import into production.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSummary,validateResponse} from './hierarchy-summary-components.mjs';
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const hash=raw=>createHash('sha256').update(raw).digest('hex');
const same=(a,b)=>assert.deepEqual(a,b,'exact source/display join');
const one=rows=>{assert.equal(rows.length,1,'exactly one logical row');return rows[0];};
const wire=(entry,limit)=>{
  const bytes=Buffer.from(entry.canonicalUtf8);
  same(bytes.length,entry.utf8Bytes);same(hash(bytes),entry.sha256);
  return decodeSummary(bytes,limit);
};
export function verifySourceBridge(store,bridge) {
  const candidate=JSON.parse(store.sources.candidateCanonical),result=JSON.parse(store.sources.resultCanonical);
  same(canonical(candidate),store.sources.candidateCanonical);same(canonical(result),store.sources.resultCanonical);
  same(Object.keys(candidate).sort(),['schemaVersion','assignmentIdentity','predecessor','authorityGeneration','outputs'].sort());
  same(candidate.schemaVersion,1);same(candidate.predecessor,{kind:'no-head'});same(candidate.outputs,[]);
  same(candidate.assignmentIdentity.kind,'root');same(candidate.authorityGeneration.kind,'parent-runtime');
  same(Object.keys(candidate.authorityGeneration).sort(),['generation','kind','sidecarGeneration']);
  for(const key of ['generation','sidecarGeneration'])assert(/^[a-f0-9]{64}$/.test(candidate.authorityGeneration[key]));
  same(candidate.assignmentIdentity,store.profile.assignmentIdentity);same(store.profile.outputs,[]);
  const candidateDigest=hash('orkworks.assignment-result-candidate.v1\n'+store.sources.candidateCanonical);
  same(candidateDigest,store.sources.candidateDigest);
  same(result,{...candidate,candidateDigest,resultRevision:1,timeCostEvidence:[]});
  const resultDigest=hash('orkworks.assignment-result.v1\n'+store.sources.resultCanonical);
  same(resultDigest,store.sources.resultDigest);
  const reference={assignmentIdentity:result.assignmentIdentity,resultRevision:1,resultDigest};
  const key={kind:'assignment-result',identity:reference};
  const state=one(store.states.filter(s=>s.name==='accepted'));
  const rows=state.rows.map(r=>({table:r.table,key:JSON.parse(r.keyCanonical),value:JSON.parse(r.valueCanonical)}));
  const retained=one(rows.filter(r=>r.table==='objects' && r.key.kind==='assignment-result'));
  same(retained.key,key);same(retained.value.digest,resultDigest);same(retained.value.body,result);
  const head=one(rows.filter(r=>r.table==='heads'));
  same(head.key,{kind:'assignment-result',identity:result.assignmentIdentity});same(head.value.target,{key,digest:resultDigest});
  const summary=one(rows.filter(r=>r.table==='summary_state'));
  same(summary.key,result.assignmentIdentity);same(summary.value.row.assignmentIdentity,result.assignmentIdentity);
  same(summary.value.row.schemaVersion,1);same(summary.value.row.state.kind,'current');
  same(summary.value.row.state.material,summary.value.checkpoint.material);
  same(summary.value.row.summaryRevision,summary.value.checkpoint.summaryRevision);
  same(head.value.lastCommit,summary.value.lastCommit);
  same(summary.value.checkpoint.applied,summary.value.lastCommit);
  const request=wire(bridge.rendererRequest,4096),response=wire(bridge.response,4194304);
  same(response.selection.accepted,{kind:'result',reference});
  same(response.selection.summaryRevision,summary.value.row.summaryRevision);
  same(response.detail.material,summary.value.row.state.material);
  same(response.detail.criteria,store.profile.criteria);same(response.detail.rubric,store.profile.rubric);
  same(response.detail.evaluations,[]);same(response.detail.material.overall,'unassessed');
  same(bridge.context.sidecarGeneration,result.authorityGeneration.sidecarGeneration);
  validateResponse(response,request,bridge.context);
  return hash(Buffer.from(bridge.response.canonicalUtf8));
}
