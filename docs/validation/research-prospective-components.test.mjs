import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {validateProspective,measureProspectiveRow,checkProspectiveWidths} from './research-prospective-components.mjs';
import {checkScopeRetry} from './research-scope-rejection-components.mjs';
const raw=()=>readFileSync(new URL('research-prospective-components.json',import.meta.url)),packet=()=>JSON.parse(raw());
function generated(){const d=mkdtempSync(join(tmpdir(),'prospective components % '));try{
 const output=join(d,'oracle.json'),r=spawnSync('python3',[fileURLToPath(new URL('generate-research-prospective.py',import.meta.url)),'--output',output],{encoding:'utf8'});
 assert.equal(r.status,0,r.stderr);return readFileSync(output);
}finally{rmSync(d,{recursive:true,force:true});}}
test('independent generator retains A collecting beside unreserved non-collecting B',()=>{
 const p=JSON.parse(generated());assert.equal(p.views?.[0]?.collection?.value?.body?.task?.taskId,'prefix-task');
 assert.equal(p.views?.[0]?.expectations?.[1]?.value?.body?.task?.taskId,'scope-revision-task-2');
 assert.equal(p.views?.[0]?.expectations?.[1]?.value?.body?.collecting,false);
});
test('empty evidence cannot be certified as prospective membership',()=>{assert.throws(()=>validateProspective({}));});

const clone=structuredClone,fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const inputDigest=b=>{const x=clone(b);delete x.inputDigest;return hash('orkworks.preparation-input.v1\n'+canonical(x));};
const lifecycleDigest=b=>hash('orkworks.research-lifecycle-decision.v1\n'+canonical(b));
function reseal(p){
 for(const v of [p.views[1],...p.cases.flatMap(c=>[c.before,c.after])]){v.input.value.body.inputDigest=inputDigest(v.input.value.body);v.run.value.body.currentInput=fields(v.input.value.body,'workspaceId runId preparationRevision inputRevision inputDigest');}
 for(const c of p.cases){const r=c.rejection;r.value.body.readSet.run=clone(c.before.run);r.value.body.request.input=clone(c.before.run.value.body.currentInput);r.value.digest=lifecycleDigest(r.value.body);r.value.receipt.decisionDigest=r.value.digest;
  if(c.completion){const x=c.completion;x.value.body.readSet=clone(r.value.body.readSet);x.value.body.request.input=clone(r.value.body.request.input);x.value.body.request.trigger.rejection.digest=r.value.digest;x.value.digest=lifecycleDigest(x.value.body);x.value.receipt.decisionDigest=x.value.digest;}}
 p.facts.originalOutcomes=Object.fromEntries(p.cases.map(c=>[c.rejection.value.digest,{commit:c.rejection.value.lastCommit,completionDigest:c.completion?.value.digest??null}]));
}
function fresh(p,fn){for(const v of [p.views[1],...p.cases.flatMap(c=>[c.before,c.after])])fn(v.input.value.body);}
test('Python regenerates exact UTF-8 LF oracle bytes with no JavaScript call',()=>{assert.deepEqual(generated(),raw());});
test('finite derived packet validates both original rejection alternatives and measurements',()=>{const p=packet();assert.deepEqual(validateProspective(p),p.measurements);});
test('literal roles and versions preserve one collecting task across submission/report/rejection',()=>{
 const p=packet();validateProspective(p);const [s,v]=p.views;
 assert.equal(p.submissionBefore.collection.value.body.collectionVersion,2);assert.equal(s.collection.value.body.collectionVersion,3);assert.equal(v.collection.value.body.collectionVersion,4);
 assert.equal(v.input.value.body.inputRevision,4);assert.equal(v.input.value.body.decision.inputRevision,3);assert.equal(v.input.value.producer.kind,'child');assert.equal(p.originalAssessment.value.producer.kind,'parent');
 for(const c of p.cases){assert.equal(c.after.collection.value.body.collectionVersion,5);assert.deepEqual(c.after.collection.value.body.task,s.collection.value.body.task);
  assert.deepEqual(c.after.expectations,v.expectations);assert.equal(c.after.proposedTask.value.body.taskVersion,1);assert.deepEqual(c.after.currentTask,c.before.currentTask);
  assert.equal(c.after.run.value.body.runVersion,22);assert.deepEqual(c.after.run.value.body.currentInput,v.run.value.body.currentInput);}
 assert.equal(p.cases[0].after.currentPlan.value.body.planVersion,3);assert.equal(p.cases[1].after.currentPlan.value.body.planVersion,2);
});
const mutations=[
 ['second collecting task',p=>p.views[0].expectations[1].value.body.collecting=true],
 ['retargeted collecting reservation',p=>p.views[0].collection.value.body.task=p.views[0].expectations[1].key],
 ['fabricated B reservation',p=>p.views[0].expectations[1].value.body.reservation=p.views[0].expectations[0].value.body.reservation],
 ['fabricated B selected report',p=>p.views[0].expectations[1].value.body.selectedReport=p.views[1].expectations[0].value.body.selectedReport],
 ['foreign collection owner',p=>p.views[0].expectations[1].value.body.collection.runId='other'],
 ['missing proposed pointer',p=>p.views[0].run.value.body.pendingPlan=null],
 ['wrong original prior plan',p=>p.views[0].pendingPlan.value.body.replaces=null],
 ['false means retired without source disposition',p=>p.views[0].memberships.retired=p.views[0].memberships.prospective],
 ['unsupported pending lifecycle status',p=>p.views[0].pendingPlan.value.body.status='rejected'],
 ['rejected plan recast as proposed',p=>p.cases[0].after.pendingPlan.value.body.status='proposed'],
 ['missing expectation owner',p=>p.views[0].expectations.pop()],
 ['extra expectation owner',p=>p.views[0].expectations.push(clone(p.views[0].expectations[1]))],
 ['skipped submission collection increment',p=>p.views[0].collection.value.body.collectionVersion=2],
 ['skipped report collection increment',p=>p.views[1].collection.value.body.collectionVersion=3],
 ['double report run increment',p=>p.views[1].run.value.body.runVersion=22],
 ['report changes task coordination version',p=>p.views[1].currentTask.value.body.taskVersion++],
 ['report approves pending plan',p=>p.views[1].pendingPlan.value.body.status='active'],
 ['report rewrites frozen B input',p=>p.views[1].pendingPlan.value.body.plan.planDefinitionDigest='0'.repeat(64)],
 ['report changes B expectation',p=>p.views[1].expectations[1].value.body.expectationVersion=2],
 ['missing original parent assessment',p=>p.originalAssessment=null],
 ['forged original assessment producer',p=>p.originalAssessment.value.producer=p.views[1].input.value.producer],
 ['child authors fresh B assessment',p=>fresh(p,b=>b.decision.inputRevision=4)],
 ['A scope substituted for B decision scope',p=>fresh(p,b=>b.decision.scope=p.views[1].expectations[0].value.body.scope)],
 ['dropped prior-plan input',p=>fresh(p,b=>b.records=b.records.filter(x=>x.kind!=='prior-plan'))],
 ['dropped A report input',p=>fresh(p,b=>b.records=b.records.filter(x=>x.kind!=='research-report'))],
 ['report assigned B configuration',p=>fresh(p,b=>b.records.find(x=>x.kind==='research-report').source.reportIdentity.configurationDigest=p.views[0].expectations[1].value.body.configuration.configurationDigest)],
 ['cross-scope capture substitution',p=>fresh(p,b=>b.records.find(x=>x.kind==='research-capture').reference=p.views[0].expectations[0].value.body.scope)],
 ['rejection retires A',p=>p.cases[0].after.expectations[0].value.body.collecting=false],
 ['completion releases collecting membership',p=>p.cases[0].after.collection.value.body.active=false],
 ['rejection mutates unchanged B expectation',p=>p.cases[0].after.expectations[1].value.body.expectationVersion=2],
 ['rejection skips collection role version',p=>p.cases[1].after.collection.value.body.collectionVersion=4],
 ['rejection rolls current input back',p=>p.cases[1].after.input=clone(p.views[0].input)],
 ['rejection clears independent blocker',p=>p.cases[0].after.run.value.body.blockers=[]],
 ['fresh Main request uses stale run version',p=>p.cases[0].rejection.value.body.request.expectedRunVersion=20],
 ['invented storage admission',p=>p.storageFormat=57]
];
for(const [name,mutate] of mutations)test('denies '+name+' after resealing',()=>{const p=packet();mutate(p);reseal(p);assert.throws(()=>validateProspective(p));});
test('fresh rejection retry returns its original pair without rechecking later live input',()=>{
 const p=packet();validateProspective(p);for(const c of p.cases)assert.deepEqual(checkScopeRetry(c,c.rejection.value.body.request,{...p.facts,currentInput:'later',runVersion:999}),{rejection:c.rejection.value.receipt,completion:c.completion?.value.receipt??null});
});
test('revoked Main denies before original-row lookup',()=>{assert.throws(()=>checkScopeRetry(null,null,{authority:'revoked'}),/authority/);});
test('missing original outcome proof cannot elect completion from a case label',()=>{const p=packet(),c=p.cases[0];c.id='incomplete';c.completion=null;assert.throws(()=>checkScopeRetry(c,c.rejection.value.body.request,p.facts));});
test('original rejection bytes conflict instead of becoming a new request',()=>{const p=packet(),c=p.cases[1],request=clone(c.rejection.value.body.request);request.input.inputRevision=3;assert.throws(()=>checkScopeRetry(c,request,p.facts));});
test('lower wrapper ceiling denies oversized input despite generic row room',()=>{const r=packet().views[1].input;r.value.body.padding='x'.repeat(32768);assert.throws(()=>checkProspectiveWidths([r]));});
test('collection body cannot hide an extra allowance',()=>{const r=packet().views[0].collection;r.value.body.padding='x'.repeat(16384);assert.throws(()=>checkProspectiveWidths([r]));});
test('only actual outer commit origins are projected out of occurrence charge',()=>{const r=packet().views[1].input,m=measureProspectiveRow(r);r.value.lastCommit.commitSequence='999';assert.equal(measureProspectiveRow(r).chargeBytes,m.chargeBytes);r.value.producer.attachmentDigest+='00';assert(measureProspectiveRow(r).chargeBytes>m.chargeBytes);});

test('new report registry request captures live A taskVersion8, not frozen assignment2',()=>{const p=packet();validateProspective(p);assert.equal(p.reportOriginal.request.expectedTaskVersion,8);assert.equal(p.reportOriginal.receiptRow.value.expectedTaskVersion,8);assert.deepEqual(p.reportRetry.retained.input,p.views[1].input);});
for(const [label,mutate] of [
 ['report retry creates a new manifest',p=>p.reportRetry.retained.input.value.body.inputRevision=5],
 ['report retry increments collection',p=>p.reportRetry.retained.collection.value.body.collectionVersion=5],
 ['report retry borrows another response',p=>p.reportRetry.receipt.response.acceptedHead.reportVersion=2],
 ['report receipt claims frozen CAS',p=>p.reportOriginal.receiptRow.value.expectedTaskVersion=2],
 ['report receipt key loses stream owner',p=>p.reportOriginal.receiptRow.key.stream.taskId='other']
])test('denies '+label,()=>{const p=packet();mutate(p);assert.throws(()=>validateProspective(p));});

test('standalone input width permits revision64 but denies paired revision65',()=>{const r=packet().views[1].input;r.value.body.inputRevision=r.value.body.preparationRevision=64;checkProspectiveWidths([r]);r.value.body.inputRevision=r.value.body.preparationRevision=65;assert.throws(()=>checkProspectiveWidths([r]));});
test('standalone record-count width permits256 occurrences but denies257',()=>{const r=packet().views[1].input;r.value.body.records=Array.from({length:256},()=>({}));checkProspectiveWidths([r]);r.value.body.records.push({});assert.throws(()=>checkProspectiveWidths([r]));});
test('finite local delta measurements match independent literal Python results',()=>{const p=packet();assert.deepEqual(validateProspective(p).localDeltas,[469, 708, -464, -232]);});

test('captured H stays charged while only each own outer commit is excluded',()=>{const r=packet().cases[0].rejection,m=measureProspectiveRow(r);r.value.lastCommit.commitSequence='99999';r.value.receipt.commit.commitSequence='99999';assert.equal(measureProspectiveRow(r).chargeBytes,m.chargeBytes);r.value.body.readSet.expectedCommit.commitSequence='99999';assert(measureProspectiveRow(r).chargeBytes>m.chargeBytes);});
test('independent literal input and paired lifecycle occurrences count retained producer and H copies',()=>{const p=packet(),m=validateProspective(p);assert.deepEqual([m.views[0][6],m.views[1][6]],[{chargeBytes:3109,wrapperBytes:3309},{chargeBytes:3861,wrapperBytes:4062}]);assert.deepEqual(m.rejections,[[{chargeBytes:8732,wrapperBytes:9130},{chargeBytes:8759,wrapperBytes:9157}],[{chargeBytes:8740,wrapperBytes:9138}]]);});
