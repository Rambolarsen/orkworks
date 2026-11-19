import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchReview,checkReviewAdmission,checkSummaryReviewReference} from './research-review-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-review.py',import.meta.url))]);
test('generator supplies the two-task plan, corrected selection, review admission, stale/replacement and citation views',()=>{
 const p=JSON.parse(generate());assert.equal(p.effects?.length,8);assert.equal(p.correctedSelection.body.inputRevision,3);
});
test('missing review packaging is rejected',()=>assert.throws(()=>validateResearchReview({})));

import {readFileSync} from 'node:fs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=readFileSync(new URL('research-review-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
const domain=(n,b)=>{const c=clone(b);return hash(n+'\n'+canonical(c));};
function resealSnapshot(p){const s=p.reviewInput.snapshot.value.body;delete s.inputDigest;s.inputDigest=domain('orkworks.research-review-input.v1',s);}
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);
 assert.deepEqual(validateResearchReview(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change,reseal] of [
 ['reviewer scope is a cross-task alias',p=>{p.twoTaskPlan.body.reviewTask.scope={...p.twoTaskPlan.body.reviewTask.scope,task:{...p.twoTaskPlan.body.reviewTask.scope.task,taskId:'review-task'}}},null],
 ['reviewer dependency is a bare id',p=>{p.twoTaskPlan.body.reviewTask.dependsOn='prefix-task'},null],
 ['plan embeds a second reviewer',p=>{p.twoTaskPlan.body.batches=[['prefix-task'],['review-task'],['review-task-2']]},null],
 ['delivery entry embeds an inputDigest',p=>{const ok=p.twoTaskPlan.orderingDenial.settingsOk.deliveryOrder.find(d=>d.kind==='research-review-input');ok.inputDigest=hash('embedded')},null],
 ['composed startup delivers an empty payload',p=>{p.reviewerConfiguration.composedStartup=p.reviewerConfiguration.composedStartup.replace(/\n\{[\s\S]*\}\n$/,'\n{}\n')},null],
 ['evidence adapter drifts from the harness',p=>{p.reviewerConfiguration.startupLeaves.evidence.adapterId='fixture-other-adapter'},'evidence'],
 ['profile digest drifts from the review tools',p=>{p.reviewerConfiguration.startupLeaves.evidence.requestedProfileDigest=hash('drifted')},null],
 ['snapshot carries an unlisted excerpt',p=>{p.reviewInput.snapshot.value.body.citedExcerpts.push({reference:{...p.reviewInput.snapshot.value.body.citedExcerpts[0].reference,id:'e-b'},contentBase64:'U3ludGg=',sizeBytes:5,contentDigest:p.reviewInput.snapshot.value.body.citedExcerpts[0].reference.reference.excerptDigest})},'snapshot'],
 ['snapshot cites bytes outside the retained range',p=>{p.reviewInput.snapshot.value.body.citedExcerpts[0].contentBase64='XXXX';p.reviewInput.snapshot.value.body.citedExcerpts[0].sizeBytes=3},'snapshot'],
 ['outcome keeps consistent verdict with an unverified question',p=>{p.reviewAdmission.outcome.questionStatuses[1].status='unverified'},'outcome'],
 ['family duplicates the outcome body',p=>{p.reviewAdmission.row.value.body={...p.reviewAdmission.outcome}},null],
 ['summary cites a stale review digest',p=>{p.summaryCitation.row.value.body.reviewReferences[0].contentDigest=hash('stale')},'summary'],
 ['summary keeps the superseded report reference',p=>{p.summaryCitation.row.value.body.reportReferences=[p.collectionShapes.expectations[0].value.body.selectedReport]},'summary'],
 ['replacement keeps the researcher task member',p=>{p.staleReview.replacementPlan.task={taskId:'prefix-task'}},'replacement'],
 ['replacement reuses the first reviewer configuration',p=>{p.staleReview.replacementPlan.reviewTask.configuration.configurationDigest=p.reviewerConfiguration.body.configurationDigest},'replacement'],
]) {test(`mutation ${name} is rejected`,()=>{
 const p=packet();change(p);
 if(reseal==='snapshot')resealSnapshot(p);
 if(reseal==='outcome'){const o=p.reviewAdmission.outcome;delete o.reviewDigest;o.reviewDigest=domain('orkworks.research-review-outcome.v1',o);
  const r=p.reviewAdmission.row.value.request;delete r.requestDigest;r.requestDigest=domain('orkworks.research-review-request.v1',r);
  const ev=p.summaryCitation.evidence;ev.contentDigest=o.reviewDigest;ev.sizeBytes=Buffer.byteLength(canonical(o));
  p.summaryCitation.row.value.body.reviewReferences=[ev];const s=p.summaryCitation.row.value.body;
  p.summaryCitation.row.value.digest=domain('orkworks.research-summary.v1',s);}
 if(reseal==='summary'){const s=p.summaryCitation.row.value.body;p.summaryCitation.row.value.digest=domain('orkworks.research-summary.v1',s);}
 if(reseal==='evidence'){const e=p.reviewerConfiguration.startupLeaves.evidence;delete e.evidenceDigest;
  e.evidenceDigest=domain('orkworks.capability-evidence.v1',e);p.reviewerConfiguration.body.harness.evidenceDigest=e.evidenceDigest;
  const rc=p.reviewerConfiguration.body;delete rc.configurationDigest;rc.configurationDigest=domain('orkworks.assignment-configuration.v3',rc);
  p.twoTaskPlan.body.reviewTask.configuration.configurationDigest=rc.configurationDigest;}
 if(reseal==='payload'){const s=p.reviewInput.snapshot.value.body;delete s.inputDigest;s.inputDigest=domain('orkworks.research-review-input.v1',s);
  const payload=canonical(s);p.reviewerConfiguration.composedStartup='Report uncertainty.\n'+p.reviewerConfiguration.body.renderedInstructions+
   '\nORKWORKS RETAINED RESEARCH REVIEW INPUT v1\n'+String(Buffer.byteLength(payload))+'\n'+payload+'\n';}
 if(reseal==='row'){const v=p.reviewAdmission.row.value;delete v.requestDigest;v.requestDigest=domain('orkworks.research-review-request.v1',v.request);}
 if(reseal==='replacement'){const rp=p.staleReview.replacementPlan;delete rp.planDefinitionDigest;rp.planDefinitionDigest=domain('orkworks.research-source-plan.v1',rp);}
 assert.throws(()=>validateResearchReview(p),e=>{assert.ok(e instanceof Error);return true;});
});}
test('supplied-fact helpers separate retained identical retries from new admissions',()=>{
 const p=packet();
 const receipt=checkReviewAdmission(p.reviewAdmission.row,{authority:'supplied-current-child',targetCurrent:true,
  selectedReport:p.reviewAdmission.row.value.request.review.reviewedReport,taskVersion:8});
 assert.equal(receipt.acceptedTaskVersion,8);
 const retryFacts={authority:'supplied-current-child',retainedIdenticalRequest:true,currentnessRequired:false,
  request:p.reviewAdmission.row.value.request};
 const retry=checkReviewAdmission(p.reviewAdmission.row,retryFacts);
 assert.deepEqual(retry,receipt,'authorized identical retry returns the original receipt before live CAS');
 const advanced=checkReviewAdmission(p.reviewAdmission.row,{...retryFacts});
 assert.deepEqual(advanced,receipt,'retry still returns the original receipt with an advanced task version');
 const superseded=checkReviewAdmission(p.reviewAdmission.row,{...retryFacts});
 assert.deepEqual(superseded,receipt,'retry still returns the original receipt after the selection changed');
 assert.throws(()=>checkReviewAdmission(p.reviewAdmission.row,{authority:'supplied-current-child',targetCurrent:true,
  selectedReport:{reportVersion:1},taskVersion:9}),/stale target denied/);
 assert.equal(checkSummaryReviewReference(p.summaryCitation.row.value.body,{requiredReview:'declared',reviewDigest:p.summaryCitation.evidence.contentDigest,reviewCurrent:true,verdict:'consistent'}),'cited');
 assert.throws(()=>checkSummaryReviewReference(p.summaryCitation.row.value.body,{requiredReview:'declared',reviewDigest:p.summaryCitation.evidence.contentDigest,reviewCurrent:true,verdict:'conflict'}),/never satisfies/);
 assert.throws(()=>checkSummaryReviewReference({reviewReferences:[]},{requiredReview:'unknown'}),/proven membership/);
});
test('literal bounds and domains are pinned',()=>{
 const p=packet();
 assert.equal(p.twoTaskPlan.domain,'orkworks.research-source-plan.v1');
 assert.equal(p.reviewerConfiguration.composedStartupLimit,262144);
 assert.equal(p.staleReview.replacementPlan.variant,'reviewer-only-replacement');
 assert.equal(p.summaryCitation.evidence.sourceKind,'research-review');
 assert.equal(p.correctedSelection.body.decision.inputRevision,prefixDecisionRevision());
});
function prefixDecisionRevision(){return JSON.parse(readFileSync(new URL('research-prefix-components.json',import.meta.url))).inputs[1].value.body.decision.inputRevision;}
