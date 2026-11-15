import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchCompletion} from './research-completion-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-completion.py',import.meta.url))]);
test('generator supplies two completion triggers, two summaries and six UI alternatives',()=>{
 const p=JSON.parse(generate());assert.equal(p.completions?.length,2);assert.equal(p.summaries?.length,2);assert.equal(p.ui?.length,6);
});
test('missing lifecycle packaging is rejected',()=>assert.throws(()=>validateResearchCompletion({})));

import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {checkSummaryAdmission,checkLifecycleRetry,validateCompletionWidths,measureCompletionRow,checkCompletionPredicate,checkContinueTarget} from './research-completion-components.mjs';
const raw=readFileSync(new URL('research-completion-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone;
const domain=(n,b)=>hash(n+'\n'+canonical(b));
const decision=p=>p.ui[3].row;
function reseal(r){r.value.digest=domain('orkworks.research-lifecycle-decision.v1',r.value.body);r.value.receipt.decisionDigest=r.value.digest;}
test('independent regeneration and decoded validation preserve exact oracle bytes',()=>{
 assert.deepEqual(generate(),raw);const p=packet(),before=canonical(p);assert.deepEqual(validateResearchCompletion(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change] of [
 ['selector has second run version',p=>p.initial.value.body.runVersion=1],
 ['selector owns current input',p=>p.initial.value.body.currentInput=p.summaries[0].row.value.body.input],
 ['missing admission selector',p=>delete p.initial],
 ['admission preselects plan',p=>p.initial.value.body.completedResearchPlan=p.completions[0].row.value.body.request.plan],
 ['summary wrong family',p=>p.summaries[0].row.table='summaries'],
 ['summary input loses assessment provenance',p=>{p.summaries[0].row.value.body.input.inputRevision=2;}],
 ['summary changes producer generation',p=>{p.summaries[0].row.value.body.producer.launchGeneration='0'.repeat(64);}],
 ['summary omits report coverage',p=>{p.summaries[0].row.value.body.reportReferences=[];}],
 ['summary flattens required review',p=>{p.summaries[0].row.value.body.reviewReferences=[{}];}],
 ['summary changes report version',p=>{p.summaries[0].row.value.body.reportReferences[0].reportIdentity.reportVersion=2;}],
 ['summary unknown field',p=>p.summaries[0].row.value.body.bearer='invented'],
 ['summary wrong digest domain',p=>{let r=p.summaries[0].row;r.value.digest=domain('orkworks.summary.v1',r.value.body);}],
 ['summary output version reset',p=>p.summaries[1].row.value.body.summaryVersion=1],
 ['summary pointer wrong high-water',p=>p.summaries[1].selectorAfter.value.body.summaryVersion=1],
 ['summary invalidation deletes history',p=>p.invalidation.retainedSummaries.pop()],
 ['summary invalidation resets high-water',p=>p.invalidation.after.value.body.summaryVersion=0],
 ['summary invalidation keeps stale selection',p=>p.invalidation.after.value.body.currentSummary=p.invalidation.before.value.body.currentSummary],
 ['completion records unsupported trigger',p=>{p.completions[0].row.value.body.request.trigger.kind='scope-resolution';reseal(p.completions[0].row);}],
 ['completion trigger uses frozen version',p=>{p.completions[0].row.value.body.request.trigger.acceptedTaskVersion=2;reseal(p.completions[0].row);}],
 ['completion advances run twice',p=>p.completions[0].runAfter.value.body.runVersion=13],
 ['completion wrong plan status literal',p=>p.completions[0].planAfter.value.body.status='completed'],
 ['completion drops live parent',p=>p.completions[0].runAfter.value.body.parentRuntime=null],
 ['incomplete recovery claims completion',p=>p.completions[1].row=p.completions[0].row],
 ['UI wrong family',p=>decision(p).table='run_states'],
 ['UI changed snapshot origin',p=>{let r=decision(p);r.value.body.readSet.run.value.lastCommit=r.value.body.readSet.expectedCommit;reseal(r);}],
 ['UI copied source H',p=>{let r=decision(p);r.value.body.readSet.expectedCommit=r.value.lastCommit;reseal(r);}],
 ['UI snapshot wrong key',p=>{let r=decision(p);r.value.body.readSet.lifecycle.key.runId='foreign';reseal(r);}],
 ['UI missing null read',p=>{let r=decision(p);delete r.value.body.readSet.pendingPlan;reseal(r);}],
 ['UI changed expected version',p=>{let r=decision(p);r.value.body.request.expectedRunVersion=49;reseal(r);}],
 ['UI renderer credentials',p=>{let r=decision(p);r.value.body.request.token='secret';reseal(r);}],
 ['UI receipt wrong coalesced version',p=>decision(p).value.receipt.runVersion=52],
 ['stage cancel removes blocker',p=>p.ui[3].runAfter.value.body.blockers=[]],
 ['stage cancel loses pending decision',p=>p.ui[3].selectorAfter.value.body.pendingStageContinue=null],
 ['stage cancel ends parent',p=>p.ui[3].runAfter.value.body.parentRuntime=null],
 ['continue restores cancelled plan',p=>p.ui[4].runAfter.value.body.currentPlan=p.ui[3].planAfter.key],
 ['continue drops blocker',p=>p.ui[4].runAfter.value.body.blockers=[]],
 ['cancel clears historical gate',p=>p.ui[5].runAfter.value.body.stageContinueRequired=false],
 ['cancel clears unknown native blocker',p=>p.ui[5].runAfter.value.body.blockers=[]],
 ['cancel restores parent authority',p=>p.ui[5].runAfter.value.body.parentRuntime=p.ui[3].runAfter.value.body.parentRuntime],
 ['finish old terminal status',p=>p.ui[0].runAfter.value.body.state='completed'],
 ['finish selects unknown disposition',p=>{p.ui[0].row.value.body.request.disposition='accepted';reseal(p.ui[0].row);}],
 ['run cancel invents finish disposition',p=>p.ui[5].selectorAfter.value.body.finishDisposition='research-only'],
 ['discounted embedded snapshot bytes',p=>p.measurements[3].chargeBytes--],
 ['unqualified assumption omitted',p=>p.assumptions.pop()],
 ['invented catalog format',p=>p.formatVersion=7],
])test('decoded component denies '+name,()=>{const p=packet();change(p);assert.throws(()=>validateResearchCompletion(p));});
const parentFacts={authority:'supplied-current-parent',rate:'supplied-allowed',stageContinueRequired:false};
test('summary exact original key retry precedes live CAS and never reselects a stale output',()=>{
 const p=packet(),x=p.summaries[0],before=canonical(p),pub=clone(x.publication);pub.expectedRunVersion=99;
 assert.deepEqual(checkSummaryAdmission(pub,x.row,{...parentFacts,runVersion:99,summaryVersion:2}),{kind:'original',ref:{...x.row.key,summaryDigest:x.row.value.digest},commit:x.row.value.lastCommit});
 assert.equal(canonical(p),before);pub.summary.findings=['different'];assert.throws(()=>checkSummaryAdmission(pub,x.row,parentFacts),/conflict/);
});
test('summary missing exact key requires next version and live CAS without gaps or reset',()=>{
 const pub=packet().summaries[1].publication;
 assert.deepEqual(checkSummaryAdmission(pub,null,{...parentFacts,runVersion:32,summaryVersion:1}),{kind:'new',summaryVersion:2});
 assert.throws(()=>checkSummaryAdmission(pub,null,{...parentFacts,runVersion:33,summaryVersion:1}),/version/);
 assert.throws(()=>checkSummaryAdmission(pub,null,{...parentFacts,runVersion:32,summaryVersion:0}),/next/);
 assert.throws(()=>checkSummaryAdmission(pub,null,{...parentFacts,runVersion:32,summaryVersion:64}),/exhaust/);
});
test('summary authority, rate and stage gate are checked before original lookup',()=>{
 const x=packet().summaries[0];
 for(const f of [{...parentFacts,authority:'revoked'},{...parentFacts,rate:'denied'},{...parentFacts,stageContinueRequired:true}]){
  assert.throws(()=>checkSummaryAdmission(x.publication,x.row,f),/authority|rate|stage/);
  assert.throws(()=>checkSummaryAdmission(null,null,f),/authority|rate|stage/);
 }
});
test('UI exact retry returns original receipt before terminal/version CAS, using original request',()=>{
 const r=packet().ui[5].row,req=clone(r.value.body.request),before=canonical(r);
 assert.deepEqual(checkLifecycleRetry(r,req,{authority:'supplied-current-Main',context:'supplied-exact',state:'cancelled',runVersion:999}),r.value.receipt);
 for(const f of [{authority:'revoked',context:'supplied-exact'},{authority:'supplied-current-Main',context:'stale'}])assert.throws(()=>checkLifecycleRetry(r,req,f),/authority|context/);
 assert.equal(canonical(r),before);req.reason='changed';assert.throws(()=>checkLifecycleRetry(r,req,{authority:'supplied-current-Main',context:'supplied-exact'}),/conflict/);
 assert.throws(()=>checkLifecycleRetry(null,null,{authority:'revoked',context:'supplied-exact'}),/authority/);
 assert.throws(()=>checkLifecycleRetry(null,null,{authority:'supplied-current-Main',context:'stale'}),/context/);
});
const completeFacts={taskStatus:'reported_complete',reservations:'supplied-attached-settled',allocation:'supplied-no-interruption',scope:'supplied-no-pending-revision',trigger:'coordination'};
test('automatic completion requires every supplied predicate and a supported trigger',()=>{
 assert.equal(checkCompletionPredicate(completeFacts),'eligible');
 for(const change of [{taskStatus:'reported_completed'},{taskStatus:'needs_parent_result'},{reservations:'unknown'},{allocation:'interrupted'},{scope:'pending'}])assert.equal(checkCompletionPredicate({...completeFacts,...change}),'incomplete');
 assert.throws(()=>checkCompletionPredicate({...completeFacts,trigger:'scope-resolution'}),/unsupported/);
});
test('continue and runtime resume preserve stage, runtime, blocker and terminal precedence',()=>{
 const f={state:'paused',stageContinueRequired:true,parent:'supplied-live',blockers:1,allocation:'settled',clarification:null};
 assert.equal(checkContinueTarget(f,'runtime-resume'),'paused');
 assert.equal(checkContinueTarget(f,'ui-continue'),'blocked');
 assert.equal(checkContinueTarget({...f,blockers:0},'ui-continue'),'assessing');
 assert.equal(checkContinueTarget({...f,blockers:0,allocation:'unknown'},'ui-continue'),'blocked');
 assert.throws(()=>checkContinueTarget({...f,parent:'interrupted'},'ui-continue'),/resume/);
 assert.throws(()=>checkContinueTarget({...f,blockers:0,clarification:{}},'ui-continue'),/unsupported/);
 for(const state of ['complete','cancelled'])assert.equal(checkContinueTarget({...f,state},'runtime-resume'),state);
});
test('snapshot, complete body/wrapper/receipt and independent job ceilings win',()=>{
 const p=packet(),r=decision(p);
 assert.doesNotThrow(()=>validateCompletionWidths(r,1048576));assert.throws(()=>validateCompletionWidths(r,1048577),/job/);
 for(const [field,max] of [['run',1048576],['lifecycle',16384],['currentPlan',16384]]){let q=clone(r);q.value.body.readSet[field].value.padding='x'.repeat(max);assert.throws(()=>validateCompletionWidths(q,1),/snapshot/);}
 let q=clone(r);q.value.receipt.padding='x'.repeat(4096);assert.throws(()=>validateCompletionWidths(q,1),/receipt/);
 q=clone(r);q.value.padding='x'.repeat(2097152);assert.throws(()=>validateCompletionWidths(q,1),/wrapper/);
 for(const original of [p.initial,p.summaries[0].row]){q=clone(original);q.value.padding='x'.repeat(16384);assert.throws(()=>validateCompletionWidths(q,1),/wrapper/);}
 q=clone(p.summaries[0].row);q.value.body.findings=['x'.repeat(8192)];assert.throws(()=>validateCompletionWidths(q,1),/body/);
 q=clone(r);let s=q.value.body.readSet.run;s.value.widthOnlyPadding='';s.value.widthOnlyPadding='x'.repeat(1048576-Buffer.byteLength(canonical({key:s.key,value:s.value})));
 assert.equal(Buffer.byteLength(canonical({key:s.key,value:s.value})),1048576);assert.throws(()=>validateCompletionWidths(q,Buffer.byteLength(canonical(q))),/job/);
});
test('control projections exclude only outer new C links and retain original snapshot H/origins',()=>{
 const r=decision(packet()),original=measureCompletionRow(r);assert.equal(typeof original?.chargeBytes,'number');
 let q=clone(r);q.value.lastCommit.changeDigest='0'.repeat(64);q.value.receipt.commit.changeDigest='1'.repeat(64);assert.deepEqual(measureCompletionRow(q),original);
 q=clone(r);q.value.body.readSet.run.value.lastCommit.commitSequence='123';assert(measureCompletionRow(q).chargeBytes>original.chargeBytes);
});
test('UTF-8 LF oracle regenerates under paths with spaces and percent signs',()=>{
 const dir=mkdtempSync(join(tmpdir(),'research completion % '));
 try{for(const n of ['generate-research-completion.py','research-prefix-components.json','research-continuation-components.json'])copyFileSync(fileURLToPath(new URL(n,import.meta.url)),join(dir,n));let out=join(dir,'output %.json');execFileSync('python3',[join(dir,'generate-research-completion.py'),'--output',out]);assert.deepEqual(readFileSync(out),raw);}finally{rmSync(dir,{recursive:true,force:true});}
});

test('terminal cancellation keeps raw gate and blocker but clears paused-only fields',()=>{
 const p=JSON.parse(generate()),b=p.ui[5].runAfter.value.body;
 assert.equal(b.state,'cancelled');assert.equal(b.stageContinueRequired,true);assert.equal(b.blockers.length,1);
 assert.equal(b.pauseReason,null);assert.equal(b.resumeTarget,null);
});
test('literal accepted completion/task/disposition names are independent of derivation helpers',()=>{
 const p=JSON.parse(generate());assert.equal(p.completions[0].source.taskAfter.value.body.status,'reported_complete');
 assert.equal(p.completions[0].planAfter.value.body.status,'complete');assert.equal(p.completions[0].runAfter.value.body.state,'synthesizing');
 assert.deepEqual(p.ui.slice(0,3).map(x=>x.row.value.body.request.disposition),['research-only','no-go','declined-execution']);
 assert.equal(p.completions[1].candidateTask.value.body.status,'needs_parent_result');assert.equal(p.completions[1].row,null);
});
for(const [name,change] of [
 ['17 findings',b=>b.findings=Array(17).fill('finding')],
 ['17 uncertainties',b=>b.uncertainties=Array(17).fill('unknown')],
 ['17 choices',b=>b.assignmentChoices=Array(17).fill({description:'choice',reason:'reason'})],
 ['oversized UTF-8 prose',b=>b.findings=['é'.repeat(1025)]],
 ['duplicate report membership',b=>b.reportReferences.push(clone(b.reportReferences[0]))],
 ['missing report origin',b=>delete b.reportReferences[0].contentDigest],
 ['extra report origin',b=>b.reportReferences[0].inventedOrigin='unqualified'],
])test('finite decoded summary denies '+name+' even with resealed source digest',()=>{
 const p=packet(),r=p.summaries[0].row;change(r.value.body);r.value.digest=domain('orkworks.research-summary.v1',r.value.body);assert.throws(()=>validateResearchCompletion(p));
});

test('independent literal occurrence sizes and representative source digests are pinned',()=>{
 const p=packet();assert.deepEqual(validateResearchCompletion(p).map(x=>[x.chargeBytes,x.wrapperBytes]),
  [[405,605],[2320,2520],[2320,2520],[6726,7122],[5383,5779],[5343,5739],[5408,5804],[6625,7021],[5250,5646],[5463,5859]]);
 assert.equal(p.summaries[0].row.value.digest,'bf871b76a2db60bb4cd1159c9202926ef000c2c54288cdce8c479fa4a448656b');
 assert.equal(p.completions[0].row.value.digest,'531f072bc05ce99730ab34482658c1904a01960c53a2c5d81788b32286c5aed6');
});

test('all supplied local poststate views retain the same new C as their lifecycle receipt',()=>{
 const p=JSON.parse(generate());for(const x of [p.completions[0],...p.ui]){
  for(const name of ['runAfter','selectorAfter','planAfter'])if(x[name])assert.deepEqual(x[name].value.lastCommit,x.row.value.receipt.commit,name);
 }
});
