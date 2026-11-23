import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchClarification,checkAnswerAdmission,checkAnswerState} from './research-clarification-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-clarification.py',import.meta.url))]);
test('generator supplies assessment, answer chain, fresh assessment, changed-answer and exhaustion views',()=>{
 const p=JSON.parse(generate());assert.equal(p.effects?.length,8);assert.equal(p.answer.stateAlternatives.length,4);
});
test('missing clarification packaging is rejected',()=>assert.throws(()=>validateResearchClarification({})));

import {readFileSync} from 'node:fs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=readFileSync(new URL('research-clarification-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
const domain=(n,b)=>{const c=clone(b);return hash(n+'\n'+canonical(c));};
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);
 assert.deepEqual(validateResearchClarification(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change,reseal] of [
 ['clarification variant keeps researchNeeded true',p=>{p.assessment.manifest.value.body.decision.researchNeeded=true},null],
 ['question text exceeds its bound',p=>{p.assessment.manifest.value.body.decision.question.questionText='x'.repeat(2049)},null],
 ['five suggestions',p=>{p.assessment.manifest.value.body.decision.question.suggestions.push('fifth')},null],
 ['decision revision drifts from the manifest',p=>{p.assessment.manifest.value.body.decision.inputRevision=3},'assessment'],
 ['manifest digest drifts',p=>{p.assessment.manifest.value.body.inputDigest=hash('drifted')},null],
 ['pending pointer digest mismatch',p=>{p.assessment.pending.requestDigest=hash('drifted')},null],
 ['run keeps the pending binding after the answer',p=>{p.answer.runAfter.value.body.pendingClarification=p.assessment.pending},'answer'],
 ['answer manifest producer is a parent runtime',p=>{p.answer.manifest.value.producer={kind:'parent'}},'answer'],
 ['answer manifest drops the clarification-answer record',p=>{p.answer.manifest.value.body.records.pop()},'answer'],
 ['answers family duplicates the answer body',p=>{p.answer.row.value.body={...p.answer.body}},'answer'],
 ['fresh assessment drops the retained answer entry',p=>{p.freshAssessment.manifest.value.body.records.splice(1,1)},'fresh'],
 ['fresh decision stays a copied assessment',p=>{p.freshAssessment.manifest.value.body.decision.inputRevision=3},'fresh'],
 ['re-ask reuses questionVersion 1',p=>{p.changedAnswer.request.key.questionVersion=1},'reask'],
 ['paused resumeTarget is awaiting_clarification',p=>{p.answer.stateAlternatives.find(x=>x.id==='paused-retained').resumeTarget='awaiting_clarification'},null],
 ['unsupported direct/skip decision flags',p=>{const d=p.assessment.manifest.value.body.decision;d.clarificationNeeded=false;d.researchNeeded=false},'assessment'],
 ['fresh manifest drops a capture record',p=>{const m=p.freshAssessment.manifest.value.body;m.records=m.records.filter(r=>r.kind!=='research-capture');delete m.inputDigest;m.inputDigest=domain('orkworks.preparation-input.v1',m)},'fresh'],
 ['re-ask body embeds questionVersion',p=>{p.changedAnswer.request.value.body.questionVersion=2;const r=p.changedAnswer.request.value.body;delete r.requestDigest;r.requestDigest=domain('orkworks.research-clarification-request.v1',r)},'reask'],
 ['persisted answer text drifts',p=>{p.answer.row.value.answer.answerText='Different policy entirely.'},null],
 ['current-input pointer stays at input-2',p=>{p.answer.runAfter.value.body.currentInput={...p.assessment.runAfter.value.body.currentInput}},'answer'],
 ['prepared input body exceeds 32 KiB',p=>{p.assessment.manifest.value.body.records.push({kind:'research-scope',reference:{task:{workspaceId:'w',runId:'r',planId:'p',planRevision:1,taskId:'t'},scopeDigest:'0'.repeat(64)}})},null],
 ['flat family key loses the run member',p=>{const k=p.assessment.request.key;const flat={...k.run,questionId:k.questionId,questionVersion:k.questionVersion};p.assessment.request.key=flat;'flat'},null],
 ['answer manifest drifts a retained record',p=>{const m=p.answer.manifest.value.body;m.records.find(r=>r.kind==='configuration').reference.configurationDigest='0'.repeat(64);delete m.inputDigest;m.inputDigest=domain('orkworks.preparation-input.v1',m)},'answer'],
]) {test(`mutation ${name} is rejected`,()=>{
 const p=packet();change(p);
 if(reseal==='assessment'){const m=p.assessment.manifest.value.body;delete m.inputDigest;m.inputDigest=domain('orkworks.preparation-input.v1',m);}
 if(reseal==='answer'){const m=p.answer.manifest.value.body;delete m.inputDigest;m.inputDigest=domain('orkworks.preparation-input.v1',m);}
 if(reseal==='fresh'){const m=p.freshAssessment.manifest.value.body;delete m.inputDigest;m.inputDigest=domain('orkworks.preparation-input.v1',m);}
 if(reseal==='reask'){const r=p.changedAnswer.request.value.body;delete r.requestDigest;r.requestDigest=domain('orkworks.research-clarification-request.v1',r);}
 assert.throws(()=>validateResearchClarification(p),e=>{assert.ok(e instanceof Error);return true;});
});}
test('supplied-fact helpers separate retained identical retries from new admissions',()=>{
 const p=packet();
 const receipt=checkAnswerAdmission(p.answer.row,p.assessment.request,
  {authority:'supplied-current-Main',stageContinueRequired:false,currentInputRevision:2,expectedInputRevision:2,
   pending:p.assessment.pending});
 assert.equal(receipt.inputRevision,3);
 const retry=checkAnswerAdmission(p.answer.row,p.assessment.request,
  {authority:'supplied-current-Main',stageContinueRequired:false,retainedIdentical:true,currentnessRequired:false,
   request:{answerId:'answer-1',answerText:'Replace only duplicated rows.'},pending:p.assessment.pending});
 assert.deepEqual(retry,receipt,'identical retry returns the original receipt before live CAS');
 assert.throws(()=>checkAnswerAdmission(p.answer.row,p.assessment.request,
  {authority:'supplied-current-Main',stageContinueRequired:false,runState:'assessing',currentInputRevision:3,expectedInputRevision:2,
   pending:p.assessment.pending}),/stale input rejected/);
 assert.throws(()=>checkAnswerAdmission(p.answer.row,p.assessment.request,
  {authority:'supplied-current-Main',stageContinueRequired:false,runState:'complete',currentInputRevision:2,expectedInputRevision:2,
   pending:p.assessment.pending}),/terminal runs/);
 assert.throws(()=>checkAnswerAdmission(p.answer.row,p.assessment.request,
  {authority:'supplied-current-Main',stageContinueRequired:true,currentInputRevision:2,expectedInputRevision:2,
   pending:p.assessment.pending}),/stage fence/);
});
test('gate-respecting state selection helpers',()=>{
 assert.equal(checkAnswerState({state:'complete'}),'complete');
 assert.equal(checkAnswerState({paused:true,pauseReason:'parent_ended'}),'paused');
 assert.equal(checkAnswerState({paused:true,pauseReason:'sidecar_replaced'}),'paused');
 assert.equal(checkAnswerState({state:'assessing',stageContinueRequired:true}),'paused');
 assert.equal(checkAnswerState({stageContinueRequired:false,blockers:1,allocation:'held'}),'blocked');
 assert.equal(checkAnswerState({stageContinueRequired:false,blockers:0,allocation:'settled',resumeApprovalRequired:true}),'awaiting_resume_approval');
 assert.equal(checkAnswerState({stageContinueRequired:false,blockers:0,allocation:'settled',parent:'supplied-active'}),'assessing');
 assert.throws(()=>checkAnswerState({stageContinueRequired:false,blockers:0,allocation:'settled',parent:'none'}),/active eligible parent/);
});
test('literal bounds and domains are pinned',()=>{
 const p=packet();
 assert.equal(p.assessment.request.key.questionVersion,1);
 assert.equal(p.exhaustion.pairs,32);assert.equal(p.exhaustion.revisions,64);
 assert.equal(p.answer.body.origin.policy,'research-clarification-answer-v1');
});
