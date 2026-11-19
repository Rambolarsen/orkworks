import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchReview,checkReviewAdmission,checkSummaryReviewReference} from './research-review-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-review.py',import.meta.url))]);
test('generator supplies the two-task plan, review admission, stale/replacement and citation views',()=>{
 const p=JSON.parse(generate());assert.equal(p.effects?.length,8);assert.equal(p.reviewInput.excerptCount,1);
});
test('missing review packaging is rejected',()=>assert.throws(()=>validateResearchReview({})));

import {readFileSync} from 'node:fs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=readFileSync(new URL('research-review-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);
 assert.deepEqual(validateResearchReview(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change,expected] of [
 ['reviewer scope is a cross-task alias',p=>{p.twoTaskPlan.body.reviewTask.scope={...p.twoTaskPlan.body.reviewTask.scope,task:{...p.twoTaskPlan.body.reviewTask.scope.task,taskId:'review-task'}}},'rejected'],
 ['reviewer dependency is a bare id',p=>{p.twoTaskPlan.body.reviewTask.dependsOn='prefix-task'},'rejected'],
 ['plan embeds a second reviewer',p=>{p.twoTaskPlan.body.batches=[['prefix-task'],['review-task'],['review-task-2']]},'unavailable'],
 ['snapshot carries an unlisted excerpt',p=>{p.reviewInput.snapshot.value.body.citedExcerpts.push({...p.reviewInput.snapshot.value.body.citedExcerpts[0],reference:{...p.reviewInput.snapshot.value.body.citedExcerpts[0].reference,id:'e-b'}})},'unlisted'],
 ['outcome status covers a foreign question',p=>{p.reviewAdmission.outcome.questionStatuses.push({questionId:'q-x',status:'consistent'})},'denied'],
 ['family duplicates the outcome body',p=>{p.reviewAdmission.row.value.body={...p.reviewAdmission.outcome}},'rejected'],
 ['summary cites a stale review digest',p=>{p.summaryCitation.row.value.body.reviewReferences[0].contentDigest=hash('stale')},'denied'],
 ['replacement keeps the researcher task member',p=>{p.staleReview.replacementPlan.task={taskId:'prefix-task'}},'rejected'],
 ['ordering entry embeds an inputDigest',p=>{const e=p.twoTaskPlan.orderingDenial.settingsOk.deliveryOrder.find(d=>d.kind==='research-review-input');e.inputDigest=hash('embedded')},'rejected'],
]) {test(`mutation ${name} reseals and is denied`,()=>{
 const p=packet();change(p);
 if(name.includes('digest')||name.includes('stale')||name.includes('unlisted')||name.includes('foreign')){
  // digest-bearing mutations: recompute the containing digest so only the join fails
  const snap=p.reviewInput.snapshot.value.body;snap.inputDigest=hash('orkworks.research-review-input.v1\n'+canonical({...snap,inputDigest:null}).replace('"inputDigest":null',''));
 }
 assert.throws(()=>validateResearchReview(p),expected=>{assert.ok(expected instanceof Error);return true;});
});}
test('supplied-fact helpers separate admission order from currentness',()=>{
 const p=packet();
 const receipt=checkReviewAdmission(p.reviewAdmission.row,{authority:'supplied-current-child',targetCurrent:true,
  selectedReport:p.reviewAdmission.row.value.request.review.reviewedReport,taskVersion:8});
 assert.equal(receipt.acceptedTaskVersion,8);
 assert.throws(()=>checkReviewAdmission(p.reviewAdmission.row,{authority:'supplied-current-child',targetCurrent:true,
  selectedReport:{reportVersion:1},taskVersion:8}),/stale target denied/); assert.equal(checkSummaryReviewReference(p.summaryCitation.row.value.body,{requiredReview:'declared',reviewDigest:p.summaryCitation.evidence.contentDigest,reviewCurrent:true,verdict:'consistent'}),'cited');
 assert.throws(()=>checkSummaryReviewReference(p.summaryCitation.row.value.body,{requiredReview:'declared',reviewDigest:p.summaryCitation.evidence.contentDigest,reviewCurrent:true,verdict:'conflict'}),/never satisfies/);
 assert.throws(()=>checkSummaryReviewReference({reviewReferences:[]},{requiredReview:'unknown'}),/proven membership/);
});
test('literal bounds and domains are pinned',()=>{
 const p=packet();
 assert.equal(p.twoTaskPlan.domain,'orkworks.research-source-plan.v1');
 assert.equal(p.reviewerConfiguration.composedStartupLimit,262144);
 assert.equal(p.staleReview.replacementPlan.variant,'reviewer-only-replacement');
 assert.equal(p.summaryCitation.evidence.sourceKind,'research-review');
});
