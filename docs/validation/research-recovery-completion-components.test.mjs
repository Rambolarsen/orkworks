import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchRecoveryCompletion} from './research-recovery-completion-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-recovery-completion.py',import.meta.url))]);
test('generator provides two positive alternatives and preserves the old incomplete recovery',()=>{
 const p=JSON.parse(generate());assert.equal(p.cases?.length,2);assert.equal(p.negative?.expected,'deny-incomplete-task');
});
test('missing positive completion packaging is rejected',()=>assert.throws(()=>validateResearchRecoveryCompletion({})));

import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {checkLateDispositionBindings,checkPairedRecoveryRetry,validatePairedRecoveryWidths} from './research-recovery-completion-components.mjs';
import {validateContinuationWidths} from './research-continuation-components.mjs';
import {validateCompletionWidths} from './research-completion-components.mjs';
const raw=readFileSync(new URL('research-recovery-completion-components.json',import.meta.url));
const packet=()=>JSON.parse(raw),clone=structuredClone,size=x=>Buffer.byteLength(canonical(x));
const domain=(n,b)=>hash(n+'\n'+canonical(b));
function reseal(x){const r=x.recovery,l=x.completion;r.value.digest=domain('orkworks.research-source-recovery-decision.v1',r.value.body);r.value.receipt.decisionDigest=r.value.digest;l.value.body.request.trigger.decisionDigest=r.value.digest;l.value.digest=domain('orkworks.research-lifecycle-decision.v1',l.value.body);l.value.receipt.decisionDigest=l.value.digest;}
test('independent regeneration and decoded checking retain every oracle byte',()=>{
 assert.deepEqual(generate(),raw);let p=packet(),before=canonical(p);assert.deepEqual(validateResearchRecoveryCompletion(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change] of [
 ['missing original known spawn',x=>x.recovery.value.body.readSet.launch.value.body.spawnRuntime=null],
 ['first-spawn field changed',x=>x.recovery.value.body.readSet.launch.value.body.spawnRuntime.launchGeneration='0'.repeat(64)],
 ['wrong original reservation',x=>x.recovery.value.body.reservation.reservationId='foreign'],
 ['new recovery kind',x=>x.recovery.value.body.kind='confirm-new-spawn'],
 ['child recovery cast',x=>x.recovery.table='child_recovery_decisions'],
 ['current turn loses readiness',x=>x.sources.interruption.task.value.body.status='launch_interrupted'],
 ['task result inferred from terminal',x=>x.recovery.value.body.readSet.task.value.body.status='needs_parent_result'],
 ['original result rewritten',x=>x.sources.coordination.value.request.outcome='failed'],
 ['coordination skips interruption version',x=>x.sources.coordination.value.request.expectedTaskVersion=8],
 ['coordination receipt retokened',x=>x.sources.coordination.value.receipt.acceptedTaskVersion=12],
 ['turn receipt retokened',x=>x.sources.turn.value.body.acceptedTaskVersion=12],
 ['frozen assignment retokened',x=>x.recovery.value.body.readSet.session.value.body.assignmentIdentity.taskVersion=12],
 ['recovery expected version is frozen',x=>x.recovery.value.body.expectedTaskVersion=2],
 ['recovery expected version is old result',x=>x.recovery.value.body.expectedTaskVersion=10],
 ['recovery expected run drift',x=>x.recovery.value.body.expectedRunVersion=13],
 ['session is only nonterminal',x=>x.recovery.value.body.readSet.session.value.body.state='nonterminal'],
 ['current producer revision reset',x=>x.recovery.value.body.readSet.session.value.body.producerRevision='1'],
 ['original allocation omitted',x=>x.recovery.value.body.allocation=null],
 ['original attachment omitted',x=>x.recovery.value.body.attachmentId=null],
 ['captured blocker subset',x=>x.recovery.value.body.blockers=[]],
 ['original source H replaced with C',x=>x.recovery.value.body.readSet.expectedCommit=x.recovery.value.lastCommit],
 ['historical snapshot origin retokened',x=>x.recovery.value.body.readSet.launch.value.lastCommit=x.recovery.value.lastCommit],
 ['lifecycle prestate rebuilt from repaired run',x=>x.completion.value.body.readSet.run=x.after.run],
 ['lifecycle wrong trigger kind',x=>x.completion.value.body.request.trigger.kind='coordination'],
 ['lifecycle wrong source recovery ID',x=>x.completion.value.body.request.trigger.recoveryId='foreign'],
 ['lifecycle wrong family',x=>x.completion.table='run_states'],
 ['missing pending-plan null',x=>delete x.completion.value.body.readSet.pendingPlan],
 ['task not advanced for launch repair',x=>x.after.task.value.body.taskVersion=11],
 ['task advanced twice',x=>x.after.task.value.body.taskVersion=13],
 ['immutable result lost',x=>x.after.task.value.body.coordinationResult=null],
 ['launch remains interrupted',x=>x.after.launch.value.body.phase='interrupted'],
 ['launch interruption flag remains',x=>x.after.launch.value.body.interruptedAt='spawn'],
 ['launch gets another allocation',x=>x.after.launch.value.body.allocation.allocationId='new'],
 ['plan not completed',x=>x.after.plan.value.body.status='active'],
 ['plan version advanced twice',x=>x.after.plan.value.body.planVersion=4],
 ['run version advanced twice',x=>x.after.run.value.body.runVersion=16],
 ['parent planning revoked by plan completion',x=>x.after.run.value.body.parentRuntime=null],
 ['current input replaced by synthesis',x=>x.after.run.value.body.currentInput.inputRevision=4],
 ['second current run owner',x=>x.after.run.table='run_states'],
 ['completion pointer absent',x=>x.after.lifecycle.value.body.completedResearchPlan=null],
 ['bridge producer revision advanced by observation',x=>x.after.sessionRetained.value.body.producerRevision='5'],
 ['read-only bridge origin retokened',x=>x.after.sessionRetained.value.lastCommit=x.recovery.value.lastCommit],
 ['recovery receipt run disagrees',x=>x.recovery.value.receipt.runVersion=16],
 ['recovery receipt task is old result',x=>x.recovery.value.receipt.taskVersion=10],
 ['completion receipt plan disagrees',x=>x.completion.value.receipt.planVersion=4],
 ['poststate C differs',x=>x.after.plan.value.lastCommit.commitSequence='9'],
])test('positive source denies '+name+' with both decision digests resealed',()=>{const p=packet(),x=p.cases[0];change(x);reseal(x);assert.throws(()=>validateResearchRecoveryCompletion(p));});
test('completion retains independent approval instead of clearing all captured blockers',()=>{
 const p=packet();p.cases[1].after.run.value.body.blockers=[];assert.throws(()=>validateResearchRecoveryCompletion(p));
});
test('old incomplete recovery remains an exact negative source',()=>{
 const p=packet();p.negative.row=p.cases[0].completion;assert.throws(()=>validateResearchRecoveryCompletion(p));
});
for(const [name,change] of [
 ['missing binding',x=>x.bindings.bindings.pop()],
 ['extra binding',x=>x.bindings.bindings.push(clone(x.bindings.bindings[0]))],
 ['wrong blocker body',x=>x.bindings.bindings[0].blocker.detail='different'],
 ['unresolved predicate',x=>x.bindings.bindings[0].disposition='supplied-unresolved'],
 ['unknown predicate',x=>x.bindings.bindings[0].disposition='supplied-unknown'],
 ['wrong reservation',x=>x.bindings.bindings[0].reservation.reservationId='foreign'],
 ['wrong source family',x=>x.bindings.bindings[0].interruptionLaunch.table='child_launch_controls'],
 ['retokened event origin',x=>x.bindings.bindings[0].interruptionTask.value.lastCommit=x.recovery.value.body.readSet.expectedCommit],
 ['interruption after coordination',x=>x.bindings.bindings[0].interruptionLaunch.value.lastCommit=x.sources.coordination.value.lastCommit],
 ['captured later task used as interruption',x=>x.bindings.bindings[0].interruptionTask=x.recovery.value.body.readSet.task],
 ['unknown binding field',x=>x.bindings.bindings[0].proof=true],
 ['changed H',x=>x.bindings.expectedCommit=x.recovery.value.lastCommit],
 ['omitted entire captured blocker',x=>x.recovery.value.body.blockers.pop()],
 ['independent approval added to removable set',x=>{x.bindings.bindings[0].blocker=clone(x.recovery.value.body.blockers[2]);}],
 ['unsorted correspondence',x=>x.bindings.bindings.reverse()],
])test('stipulated relation denies '+name,()=>{const x=packet().cases[1];change(x);assert.throws(()=>checkLateDispositionBindings(x.recovery,x.bindings,x.sources));});
test('exact correspondence removes only covered disposition blockers and is read-only',()=>{
 const p=packet(),before=canonical(p);
 assert.deepEqual(checkLateDispositionBindings(p.cases[0].recovery,p.cases[0].bindings,p.cases[0].sources),[]);
 assert.deepEqual(checkLateDispositionBindings(p.cases[1].recovery,p.cases[1].bindings,p.cases[1].sources),[p.cases[1].recovery.value.body.blockers[2]]);
 assert.equal(canonical(p),before);
});
const UI={authority:'supplied-current-Main',context:'supplied-exact'};
test('paired recovery retry checks Main authority/context before existing originals and live CAS',()=>{
 const x=packet().cases[0],before=canonical(x);
 assert.deepEqual(checkPairedRecoveryRetry(x.recovery,x.completion,x.recovery.value.body,{...UI,runVersion:999,taskVersion:999,state:'complete'}),{recovery:x.recovery.value.receipt,completion:x.completion.value.receipt});
 assert.equal(canonical(x),before);
 for(const f of [{...UI,authority:'revoked'},{...UI,context:'stale'}])assert.throws(()=>checkPairedRecoveryRetry(x.recovery,x.completion,x.recovery.value.body,f),/authority|context/);
 let b=clone(x.recovery.value.body);b.reason='changed';assert.throws(()=>checkPairedRecoveryRetry(x.recovery,x.completion,b,UI),/conflict/);
 assert.throws(()=>checkPairedRecoveryRetry(x.recovery,null,x.recovery.value.body,UI),/companion/);
 let l=clone(x.completion);l.value.body.request.trigger.recoveryId='foreign';assert.throws(()=>checkPairedRecoveryRetry(x.recovery,l,x.recovery.value.body,UI),/digest|trigger/);
 l=clone(x.completion);l.value.receipt.commit.commitSequence='99';assert.throws(()=>checkPairedRecoveryRetry(x.recovery,l,x.recovery.value.body,UI),/C/);
});
test('the paired lower job bound includes both records and all supplied binding copies',()=>{
 const x=packet().cases[0],floor=size({recovery:x.recovery,completion:x.completion,bindings:x.bindings});
 assert.doesNotThrow(()=>validatePairedRecoveryWidths(x.recovery,x.completion,x.bindings,floor));
 assert.throws(()=>validatePairedRecoveryWidths(x.recovery,x.completion,x.bindings,floor-1),/floor/);
 assert.throws(()=>validatePairedRecoveryWidths(x.recovery,x.completion,x.bindings,1048577),/job/);
 let r=clone(x.recovery),l=clone(x.completion);
 for(const read of [r.value.body.readSet.run,l.value.body.readSet.run]){read.value.widthOnlyPadding='x'.repeat(600000);}
 assert.doesNotThrow(()=>validateContinuationWidths(r,1048576));assert.doesNotThrow(()=>validateCompletionWidths(l,1048576));
 assert.throws(()=>validatePairedRecoveryWidths(r,l,x.bindings,1048576),/floor|job/);
 let b=clone(x.bindings);b.bindings[0].interruptionRun.value.widthOnlyPadding='x'.repeat(1048576);
 assert.throws(()=>validatePairedRecoveryWidths(x.recovery,x.completion,b,1048576),/snapshot/);
 b=clone(x.bindings);b.bindings=Array(17).fill(b.bindings[0]);assert.throws(()=>validatePairedRecoveryWidths(x.recovery,x.completion,b,1048576),/binding/);
});
test('literal statuses, versions and original/new C links match the accepted contract',()=>{
 const p=JSON.parse(generate());for(const x of p.cases){
  assert.equal(x.sources.turn.value.body.acceptedTaskVersion,8);assert.equal(x.sources.coordination.value.receipt.acceptedTaskVersion,10);
  assert.equal(x.recovery.value.body.expectedTaskVersion,11);assert.equal(x.recovery.value.receipt.taskVersion,12);
  assert.equal(x.after.task.value.body.status,'reported_complete');assert.equal(x.after.plan.value.body.status,'complete');
  assert.equal(x.after.plan.value.body.planVersion,3);assert.equal(x.after.run.value.body.runVersion,15);
  assert.equal(x.after.sessionRetained.value.body.producerRevision,'4');assert.equal(x.after.sessionRetained.value.body.assignmentIdentity.taskVersion,2);
  assert.deepEqual(x.recovery.value.body.readSet.run,x.completion.value.body.readSet.run);
  for(const k of ['run','task','launch','plan','lifecycle'])assert.deepEqual(x.after[k].value.lastCommit,x.recovery.value.receipt.commit);
  assert.deepEqual(x.after.sessionRetained,x.recovery.value.body.readSet.session);
 }
 assert.deepEqual(p.cases.map(x=>x.after.run.value.body.state),['synthesizing','blocked']);
});
test('UTF-8 LF generation is stable under paths with spaces and percent signs',()=>{
 const dir=mkdtempSync(join(tmpdir(),'positive recovery % '));
 try{for(const n of ['generate-research-recovery-completion.py','research-prefix-components.json','research-completion-components.json'])copyFileSync(fileURLToPath(new URL(n,import.meta.url)),join(dir,n));let out=join(dir,'output %.json');execFileSync('python3',[join(dir,'generate-research-recovery-completion.py'),'--output',out]);assert.deepEqual(readFileSync(out),raw);}finally{rmSync(dir,{recursive:true,force:true});}
});

test('binding cannot accept a correlated source and binding cast to another family',()=>{
 const x=packet().cases[0];x.sources.interruption.launch.table='child_launch_controls';x.bindings.bindings[0].interruptionLaunch.table='child_launch_controls';
 assert.throws(()=>checkLateDispositionBindings(x.recovery,x.bindings,x.sources),/family/);
});
test('original turn version must precede interrupted readiness even with consistent turn bytes',()=>{
 const x=packet().cases[0],t=x.sources.turn;t.value.body.expectedTaskVersion=9;t.value.body.acceptedTaskVersion=10;t.value.digest=domain('orkworks.research-source-turn-completion.v1',t.value.body);
 assert.throws(()=>checkLateDispositionBindings(x.recovery,x.bindings,x.sources),/version/);
});
test('correlated captured and binding arrays cannot bypass canonical blocker order',()=>{
 const x=packet().cases[1];x.recovery.value.body.readSet.run.value.body.blockers.reverse();x.recovery.value.body.blockers.reverse();x.bindings.bindings.reverse();
 assert.throws(()=>checkLateDispositionBindings(x.recovery,x.bindings,x.sources),/order/);
});
for(const [name,change,pattern] of [
 ['foreign recovery family',x=>x.recovery.table='child_recovery_decisions',/family/],
 ['foreign recovery lookup key',x=>x.recovery.key.recoveryId='foreign',/key/],
 ['mismatched lifecycle expected version',x=>{x.completion.value.body.request.expectedRunVersion=13;reseal(x);},/version/],
 ['mismatched original receipt key',x=>x.completion.value.receipt.key.plan.planId='foreign',/key/],
])test('paired retry denies '+name+' after current Main checks',()=>{
 const x=packet().cases[0];change(x);assert.throws(()=>checkPairedRecoveryRetry(x.recovery,x.completion,x.recovery.value.body,UI),pattern);
});

test('independent literal occurrence sizes and source digests are pinned',()=>{
 const p=packet();assert.deepEqual(validateResearchRecoveryCompletion(p),[
  {id:'disposition-only',recovery:{chargeBytes:11480,wrapperBytes:11876},completion:{chargeBytes:7022,wrapperBytes:7418},bindingsBytes:7555,suppliedComponentBytes:26963},
  {id:'independent-approval-retained',recovery:{chargeBytes:12665,wrapperBytes:13061},completion:{chargeBytes:7608,wrapperBytes:8004},bindingsBytes:16020,suppliedComponentBytes:37199}]);
 assert.equal(p.cases[0].recovery.value.digest,'ff9b8ebd70090bf605abdc68f92c90cee91ce1eb13bb7fe8160ca4f88fe71ebc');
 assert.equal(p.cases[0].completion.value.digest,'448f81784e158f254a549c9abccb2e63ed39e6d3a476f8ba9d5b75091dcfcc40');
});

for(const [name,change] of [
 ['turn family',x=>x.sources.turn.table='wrong'],
 ['coordination family',x=>x.sources.coordination.table='wrong'],
 ['turn source key',x=>x.sources.turn.key.taskId='foreign'],
 ['coordination source key',x=>x.sources.coordination.key.taskId='foreign'],
 ['turn task body',x=>x.sources.turn.value.body.task.taskId='foreign'],
 ['coordination request task',x=>x.sources.coordination.value.request.task.taskId='foreign'],
 ['turn original reservation',x=>x.sources.turn.value.body.reservation.reservationId='foreign'],
 ['coordination original reservation',x=>x.sources.coordination.value.request.reservation.reservationId='foreign'],
 ['turn source schema',x=>x.sources.turn.value.schemaVersion=2],
 ['coordination source schema',x=>x.sources.coordination.value.schemaVersion=2],
])test('binding validates '+name+' independently of source/binding agreement',()=>{
 const x=packet().cases[0];change(x);x.sources.turn.value.digest=domain('orkworks.research-source-turn-completion.v1',x.sources.turn.value.body);
 x.sources.coordination.value.requestDigest=domain('orkworks.research-source-coordination-request.v1',x.sources.coordination.value.request);
 assert.throws(()=>checkLateDispositionBindings(x.recovery,x.bindings,x.sources));
});
for(const [name,change] of [
 ['task version',x=>{x.recovery.value.body.expectedTaskVersion=99;x.recovery.value.receipt.taskVersion=100;}],
 ['plan version',x=>{x.completion.value.body.request.expectedPlanVersion=99;x.completion.value.receipt.planVersion=100;}],
 ['plan identity',x=>{for(const ref of [x.completion.key.plan,x.completion.value.body.key.plan,x.completion.value.body.request.plan,x.completion.value.receipt.key.plan])ref.planId='foreign';}],
])test('paired retry joins resealed '+name+' to the captured original row',()=>{
 const x=packet().cases[0];change(x);reseal(x);assert.throws(()=>checkPairedRecoveryRetry(x.recovery,x.completion,x.recovery.value.body,UI));
});
