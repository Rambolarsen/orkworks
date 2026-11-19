import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {validateScopeRejection,checkScopeRetry,nextScopeRevision,measureScopeRow,checkScopeWidths} from './research-scope-rejection-components.mjs';
const raw=()=>readFileSync(new URL('research-scope-rejection-components.json',import.meta.url)),packet=()=>JSON.parse(raw());
const digest=b=>hash('orkworks.research-lifecycle-decision.v1\n'+canonical(b));
// Reseal after semantic mutations: a stale digest is not the intended denial.
function reseal(p){for(const c of p.cases){const r=c.rejection;r.value.digest=digest(r.value.body);r.value.receipt.decisionDigest=r.value.digest;
 if(c.completion){c.completion.value.body.request.trigger.rejection.digest=r.value.digest;c.completion.value.digest=digest(c.completion.value.body);c.completion.value.receipt.decisionDigest=c.completion.value.digest;}
 c.measurements=[r,...(c.completion?[c.completion]:[])].map(measureScopeRow);}}
test('Python regenerates exact UTF-8 LF bytes without JavaScript',()=>{const d=mkdtempSync(join(tmpdir(),'scope-rejection-'));
 try{const out=join(d,'p.json'),r=spawnSync('python3',[fileURLToPath(new URL('generate-research-scope-rejection.py',import.meta.url)),'--output',out],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);assert.deepEqual(readFileSync(out),raw());}finally{rmSync(d,{recursive:true,force:true});}});
test('finite packet validates both alternatives and independent measurements',()=>{const p=packet();assert.equal(p.cases.length,2);assert.deepEqual(validateScopeRejection(p),p.cases.map(c=>c.measurements));});
test('literal contract completes only the eligible original current plan',()=>{const p=packet();validateScopeRejection(p);const [yes,no]=p.cases;
 assert.equal(yes.after.currentPlan.value.body.status,'complete');assert.equal(yes.after.currentPlan.value.body.planVersion,3);
 assert.equal(no.after.currentPlan.value.body.status,'active');assert.equal(no.after.currentPlan.value.body.planVersion,2);assert.equal(no.completion,null);
 for(const c of p.cases){assert.equal(c.after.pendingPlan.value.body.status,'cancelled');assert.equal(c.after.pendingPlan.value.body.planVersion,2);
 assert.equal(c.after.proposedTask.value.body.status,'pending');assert.equal(c.after.proposedTask.value.body.taskVersion,1);assert.equal(c.after.proposedTask.value.body.reservation,null);
 assert.deepEqual(c.after.currentTask,c.source.currentTask);assert.equal(c.after.run.value.body.runVersion,21);assert.equal(c.rejection.value.receipt.planVersion,null);}
 assert.equal(yes.source.currentTask.value.body.status,'reported_complete');assert.equal(yes.source.coordination.value.receipt.acceptedTaskVersion,9);
});
const mutations=[
 ['missing prior-plan input',p=>p.proposal.input.records=p.proposal.input.records.filter(x=>x.kind!=='prior-plan')],
 ['foreign replaced plan',p=>p.proposal.input.records.find(x=>x.kind==='prior-plan').plan.planId='other'],
 ['reused current task',p=>p.proposal.plan.task.taskId='prefix-task'],
 ['wrong group',p=>p.proposal.plan.task.group.groupId='prefix-group'],
 ['wrong source input digest',p=>p.proposal.plan.parentPlanBinding.inputDigest='0'.repeat(64)],
 ['proposed approval smuggled',p=>p.cases[0].rejection.value.body.readSet.pendingPlan.value.body.latestApproval=p.cases[0].rejection.value.body.readSet.currentPlan.value.body.latestApproval],
 ['wrong replaces source',p=>p.cases[0].rejection.value.body.readSet.pendingPlan.value.body.replaces=null],
 ['stale expected proposed version',p=>p.cases[0].rejection.value.body.request.expectedProposedPlanVersion=2],
 ['different completion H',p=>p.cases[0].completion.value.body.readSet.expectedCommit.commitSequence='99'],
 ['poststate used as completion read',p=>p.cases[0].completion.value.body.readSet.run=p.cases[0].after.run],
 ['wrong trigger kind',p=>p.cases[0].completion.value.body.request.trigger.kind='recovery'],
 ['wrong rejection ref key',p=>p.cases[0].completion.value.body.request.trigger.rejection.key.decisionId='other'],
 ['rejected proposal completed instead',p=>p.cases[0].completion.value.body.request.plan=p.proposal.ref],
 ['incomplete work given completion',p=>p.cases[1].completion=structuredClone(p.cases[0].completion)],
 ['missing required compound completion',p=>p.cases[0].completion=null],
 ['cleared input history',p=>p.cases[0].after.run.value.body.currentInput.inputRevision=2],
 ['lost rejected source ref',p=>p.cases[0].after.run.value.body.planReferences.pop()],
 ['task outcome rewritten',p=>p.cases[0].after.currentTask.value.body.coordinationResult=null],
 ['proposed task reused',p=>p.cases[0].after.proposedTask.value.body.reservation={}],
 ['double run increment',p=>p.cases[0].after.run.value.body.runVersion=22],
 ['current version advanced without completion',p=>p.cases[1].after.currentPlan.value.body.planVersion=3],
 ['different paired receipt C',p=>p.cases[0].completion.value.receipt.commit.commitSequence='99'],
 ['lost independent blocker',p=>p.cases[0].after.run.value.body.blockers=[]],
 ['invented current plan approval',p=>p.cases[0].after.currentPlan.value.body.latestApproval=null],
 ['new storage format',p=>p.storageFormat=57]
];
for(const [name,change] of mutations)test('denies '+name,()=>{const p=packet();change(p);reseal(p);assert.throws(()=>validateScopeRejection(p));});
test('changed re-proposal skips retained cancelled revision instead of overwriting it',()=>{const p=packet();assert.equal(nextScopeRevision(p.submissions,p.facts),3);});
test('incomplete submission membership cannot elect a maximum',()=>{const p=packet();assert.throws(()=>nextScopeRevision(p.submissions,{...p.facts,membership:'unknown'}));});
test('duplicate retained PlanRef denies revision allocation',()=>{const p=packet();assert.throws(()=>nextScopeRevision([...p.submissions,p.submissions[1]],p.facts));});
test('submission order cannot be silently sorted or repaired',()=>{const p=packet();assert.throws(()=>nextScopeRevision([...p.submissions].reverse(),p.facts));});
test('64 submitted revisions exhaust the selected plan ID',()=>{const p=packet(),refs=Array.from({length:64},(_,i)=>({...p.submissions[0],planRevision:i+1,planDefinitionDigest:hash(String(i))}));assert.throws(()=>nextScopeRevision(refs,p.facts));});
test('original completing retry returns exact paired receipts despite later terminal live state',()=>{const p=packet(),c=p.cases[0];assert.deepEqual(checkScopeRetry(c,c.rejection.value.body.request,{...p.facts,runState:'cancelled',runVersion:999}),{rejection:c.rejection.value.receipt,completion:c.completion.value.receipt});});
test('original incomplete retry returns rejection only, not a later completion',()=>{const p=packet(),c=p.cases[1];assert.deepEqual(checkScopeRetry(c,c.rejection.value.body.request,p.facts),{rejection:c.rejection.value.receipt,completion:null});});
test('retry denies authority before reading retained rows',()=>{assert.throws(()=>checkScopeRetry(null,null,{authority:'revoked'}),/authority/);});
for(const k of ['context','rate','currentness','membership','sourceProofs','disposition','originalOutcome'])test('unknown '+k+' denies original retry',()=>{const p=packet(),c=p.cases[0];assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,{...p.facts,[k]:'unknown'}));});
test('changed original request conflicts',()=>{const p=packet(),c=p.cases[0],r=structuredClone(c.rejection.value.body.request);r.reason+=' changed';assert.throws(()=>checkScopeRetry(c,r,p.facts));});
test('missing original completion companion is unavailable',()=>{const p=packet(),c=p.cases[0];c.completion=null;assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
test('renaming a case cannot erase its original completing outcome',()=>{const p=packet(),c=p.cases[0];c.id='incomplete';c.completion=null;assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
test('wrong original companion cannot be borrowed from another outcome',()=>{const p=packet(),c=p.cases[0];c.completion.value.receipt.commit.commitSequence='99';assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
for(const [label,mutate] of [
 ['rejection indexed key',c=>c.rejection.key.decisionId='other'],
 ['rejection receipt key',c=>c.rejection.value.receipt.key.decisionId='other'],
 ['rejection receipt run version',c=>c.rejection.value.receipt.runVersion=900],
 ['rejection receipt plan version',c=>c.rejection.value.receipt.planVersion=3],
 ['completion indexed key',c=>c.completion.key.plan.planId='other'],
 ['completion receipt key',c=>c.completion.value.receipt.key.plan.planId='other'],
 ['completion receipt run version',c=>c.completion.value.receipt.runVersion=900],
 ['completion receipt plan version',c=>c.completion.value.receipt.planVersion=900]
])test('retry denies altered '+label,()=>{const p=packet(),c=p.cases[0];mutate(c);assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
test('incomplete retry binds indexed UI key without a companion as a backstop',()=>{const p=packet(),c=p.cases[1];c.rejection.key.decisionId='other';assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
test('unchanged current plan at version ceiling needs no increment on incomplete rejection',()=>{
 const p=packet(),c=p.cases[1],r=c.rejection,original=p.facts.originalOutcomes[r.value.digest];
 r.value.body.readSet.currentPlan.value.body.planVersion=r.value.body.request.expectedPlanVersion=2147483647;
 r.value.digest=digest(r.value.body);r.value.receipt.decisionDigest=r.value.digest;
 assert.deepEqual(checkScopeRetry(c,r.value.body.request,{...p.facts,originalOutcomes:{[r.value.digest]:original}}),{rejection:r.value.receipt,completion:null});
});
test('only outer lastCommit and own receipt.commit are projected out',()=>{const c=packet().cases[0],r=structuredClone(c.rejection),m=measureScopeRow(r);r.value.lastCommit.commitSequence='99';r.value.receipt.commit.commitSequence='99';assert.equal(measureScopeRow(r).chargeBytes,m.chargeBytes);r.value.body.readSet.expectedCommit.commitSequence='999';assert(measureScopeRow(r).chargeBytes>m.chargeBytes);});
test('lower paired ceiling wins over independently valid wrapper sizes',()=>{const r=packet().cases[0].rejection;r.value.body.padding='x'.repeat(600000);assert(measureScopeRow(r).wrapperBytes<2097152);assert.throws(()=>checkScopeWidths([r,r]),/job/);});
test('receipt ceiling cannot hide inside an otherwise small row',()=>{const r=packet().cases[0].rejection;r.value.receipt.padding='x'.repeat(4096);assert.throws(()=>checkScopeWidths([r]),/receipt/);});
test('fresh retained source changes do not weaken exact prior-plan membership',()=>{const p=packet(),input=p.proposal.input;
 input.records=input.records.filter(x=>x.kind!=='prior-plan');delete input.inputDigest;input.inputDigest=hash('orkworks.preparation-input.v1\n'+canonical(input));
 assert.throws(()=>validateScopeRejection(p));
});
test('literal key-plus-projection totals count each copied original snapshot',()=>{
 const p=packet();assert.deepEqual(validateScopeRejection(p),[[{chargeBytes:8683,wrapperBytes:9081},{chargeBytes:8746,wrapperBytes:9144}],[{chargeBytes:8691,wrapperBytes:9089}]]);
});
