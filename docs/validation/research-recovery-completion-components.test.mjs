import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateRecoveryCompletion,checkPositiveSettlement,checkPairedRetry,measureRecoveryCompletion,checkCompoundWidths} from './research-recovery-completion-components.mjs';
import {canonical,hash} from './root-bootstrap-components.mjs';
const raw=()=>readFileSync(new URL('research-recovery-completion-components.json',import.meta.url));
const packet=()=>JSON.parse(raw());
const digest=(name,b)=>hash(name+'\n'+canonical(b));
// Reseal a changed decision so semantic joins, rather than just a stale hash,
// must detect the changed source. Measurements are independently rechecked.
function reseal(p){
 p.recovery.value.digest=digest('orkworks.research-source-recovery-decision.v1',p.recovery.value.body);
 p.recovery.value.receipt.decisionDigest=p.recovery.value.digest;
 p.lifecycle.value.body.request.trigger.decisionDigest=p.recovery.value.digest;
 p.lifecycle.value.digest=digest('orkworks.research-lifecycle-decision.v1',p.lifecycle.value.body);
 p.lifecycle.value.receipt.decisionDigest=p.lifecycle.value.digest;
 p.measurements=[p.recovery,p.lifecycle].map(measureRecoveryCompletion);
}
test('generator emits independently reproducible source packet',()=>{
 const dir=mkdtempSync(join(tmpdir(),'recovery-completion-'));
 try{const out=join(dir,'packet.json');const r=spawnSync('python3',[fileURLToPath(new URL('generate-research-recovery-completion.py',import.meta.url)),'--output',out],{encoding:'utf8'});
  assert.equal(r.status,0,r.stderr);assert.deepEqual(readFileSync(out),raw());
 }finally{rmSync(dir,{recursive:true,force:true});}
});
test('packet includes both original records and checked occurrence measurements',()=>{
 const p=packet();assert.equal(p.recovery.table,'research_recovery_decisions');assert.equal(p.lifecycle.table,'research_lifecycle_records');
 assert.deepEqual(validateRecoveryCompletion(p),p.measurements);assert.equal(p.measurements.length,2);
});
test('literal contract preserves frozen and historical versions during repair',()=>{
 const p=packet();validateRecoveryCompletion(p);
 assert.equal(p.terminal.session.value.body.assignmentIdentity.taskVersion,2);
 assert.equal(p.turn.value.body.acceptedTaskVersion,8);
 assert.equal(p.coordination.value.receipt.acceptedTaskVersion,10);
 assert.equal(p.recovery.value.body.expectedTaskVersion,11);
 assert.equal(p.recovery.value.receipt.taskVersion,12);
 assert.equal(p.after.task.value.body.status,'reported_complete');
 assert.equal(p.after.run.value.body.runVersion,15);assert.equal(p.after.plan.value.body.planVersion,3);
 assert.equal(p.after.run.value.body.state,'blocked');
 assert.deepEqual(p.after.run.value.body.blockers.map(x=>x.kind),['recovery-approval-required']);
});
test('eligible selected settlement returns exact removable blockers and shared outcome',()=>{
 const p=packet();assert.deepEqual(checkPositiveSettlement(p,p.facts),{kind:'eligible',removed:['a-disposition'],runVersion:15,taskVersion:12,planVersion:3});
});
// Each mutation names a missing source/projection check. Resealing prevents a
// mere digest comparison from hiding missing semantic checks.
const mutations=[
 ['wrong readiness',p=>p.interruption.task.value.body.status='reported_failed'],
 ['invented early result',p=>p.interruption.task.value.body.coordinationResult={requestDigest:'a'.repeat(64)}],
 ['unknown first spawn',p=>p.interruption.launch.value.body.spawnRuntime=null],
 ['wrong interruption kind',p=>p.interruption.launch.value.body.interruptedAt='allocation'],
 ['interruption after coordination',p=>p.interruption.task.value.lastCommit=p.coordination.value.lastCommit],
 ['retokened original turn',p=>p.turn.value.body.acceptedTaskVersion=12],
 ['retokened original result',p=>p.coordination.value.receipt.acceptedTaskVersion=12],
 ['failed parent outcome',p=>p.coordination.value.request.outcome='failed'],
 ['missing complete bindings',p=>p.bindings.bindings=[]],
 ['duplicate blocker binding',p=>p.bindings.bindings.push(structuredClone(p.bindings.bindings[0]))],
 ['wrong binding body',p=>p.bindings.bindings[0].blocker.detail='same task is not proof'],
 ['unresolved binding',p=>p.bindings.bindings[0].disposition='supplied-unresolved'],
 ['unknown binding',p=>p.bindings.bindings[0].disposition='supplied-unknown'],
 ['wrong binding reservation',p=>p.bindings.bindings[0].reservation.reservationId='other'],
 ['copied current interruption task',p=>p.bindings.bindings[0].interruptionTask=p.terminal.task],
 ['missing captured blocker',p=>p.recovery.value.body.blockers.pop()],
 ['mutable frozen identity',p=>p.terminal.session.value.body.assignmentIdentity.taskVersion=11],
 ['stale expected task version',p=>p.recovery.value.body.expectedTaskVersion=10],
 ['nonterminal bridge',p=>p.terminal.session.value.body.state='running'],
 ['wrong terminal runtime',p=>p.terminal.session.value.body.runtime.launchGeneration='0'.repeat(64)],
 ['reconstructed lifecycle read',p=>p.lifecycle.value.body.readSet.run=p.after.run],
 ['different captured H',p=>p.lifecycle.value.body.readSet.expectedCommit.commitSequence='99'],
 ['wrong trigger source',p=>p.lifecycle.value.body.request.trigger.recoveryId='other'],
 ['cleared independent approval blocker',p=>p.after.run.value.body.blockers=[]],
 ['rewritten original result',p=>p.after.task.value.body.coordinationResult=null],
 ['missing task version advance',p=>p.after.task.value.body.taskVersion=11],
 ['double run advance',p=>p.after.run.value.body.runVersion=16],
 ['rewritten terminal bridge',p=>p.after.session.value.body.producerRevision='5'],
 ['unsettled launch poststate',p=>p.after.launch.value.body.phase='interrupted'],
 ['new allocation during repair',p=>p.after.launch.value.body.allocation.allocationId='replacement'],
 ['different receipt C',p=>p.lifecycle.value.receipt.commit.commitSequence='100'],
 ['wrong completed selector',p=>p.after.selector.value.body.completedResearchPlan=null],
 ['physical profile smuggled into packet',p=>p.storageFormat=57],
];
for(const [name,mutate] of mutations)test('denies '+name,()=>{const p=packet();mutate(p);reseal(p);assert.throws(()=>validateRecoveryCompletion(p));});
for(const key of ['authority','rate','originalSpawn','nativeSettlement','sourceProofs','membership','currentness','disposition','planEligibility','grantFencing','capacity'])
 test('supplied '+key+' fact cannot be unknown',()=>{const p=packet();p.facts[key]='unknown';assert.throws(()=>checkPositiveSettlement(p,p.facts));});
test('cancellation-first denies a new completion',()=>{const p=packet();p.facts.runState='cancelled';assert.throws(()=>checkPositiveSettlement(p,p.facts));});
test('original paired retry ignores advanced live versions and exhausted new count',()=>{
 const p=packet();assert.deepEqual(checkPairedRetry(p,p.recovery.value.body,{...p.facts,runState:'cancelled',runVersion:900,newDecisionCount:32}),
  {recovery:p.recovery.value.receipt,completion:p.lifecycle.value.receipt});
});
test('retry authenticates before original lookup',()=>{
 const p=packet();assert.throws(()=>checkPairedRetry(null,p.recovery.value.body,{...p.facts,authority:'revoked'}),/authority/);
});
test('retry revalidates shown original context before lookup',()=>{
 const p=packet();assert.throws(()=>checkPairedRetry(null,p.recovery.value.body,{...p.facts,context:'stale'}),/context/);
});
test('changed original request conflicts',()=>{const p=packet(),b=structuredClone(p.recovery.value.body);b.reason+='changed';assert.throws(()=>checkPairedRetry(p,b,p.facts));});
test('missing original companion never rechecks live completion',()=>{const p=packet();p.lifecycle=null;assert.throws(()=>checkPairedRetry(p,p.recovery.value.body,p.facts));});
test('wrong original companion outcome denies retry',()=>{const p=packet();p.lifecycle.value.receipt.commit.commitSequence='99';assert.throws(()=>checkPairedRetry(p,p.recovery.value.body,p.facts));});
test('projection excludes only outer outcome links and own receipt commit',()=>{
 const p=packet(),r=structuredClone(p.recovery),m=measureRecoveryCompletion(r);r.value.lastCommit.commitSequence='999';r.value.receipt.commit.commitSequence='999';
 assert.equal(measureRecoveryCompletion(r).chargeBytes,m.chargeBytes);
 r.value.body.readSet.expectedCommit.commitSequence='999';assert(measureRecoveryCompletion(r).chargeBytes>m.chargeBytes);
});
test('width-only alternatives reject the smaller joint ceiling even below wrapper caps',()=>{
 const p=packet(),r=structuredClone(p.recovery);r.value.body.padding='x'.repeat(600000);
 assert(measureRecoveryCompletion(r).wrapperBytes<2097152);
 assert.throws(()=>checkCompoundWidths([r,r]),/paired job/);
});
test('receipt ceiling cannot be hidden by a small whole wrapper',()=>{
 const p=packet();p.recovery.value.receipt.padding='x'.repeat(4096);assert.throws(()=>checkCompoundWidths([p.recovery,p.lifecycle]),/receipt/);
});
test('literal independently measured charges keep both copied H and run snapshots',()=>{
 const p=packet();assert.deepEqual(validateRecoveryCompletion(p),[{chargeBytes:12103,wrapperBytes:12499},{chargeBytes:7333,wrapperBytes:7729}]);
 const r=structuredClone(p.lifecycle),before=measureRecoveryCompletion(r).chargeBytes;
 delete r.value.body.readSet.run;assert(measureRecoveryCompletion(r).chargeBytes<before);
});
