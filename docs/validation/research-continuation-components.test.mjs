import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchContinuation} from './research-continuation-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-continuation.py',import.meta.url))]);
test('Python derives all turn evidence variants and original recovery kinds',()=>{
  const p=JSON.parse(generate());
  assert.equal(p.turns?.length,3);assert.equal(p.coordinations?.length,3);assert.equal(p.recoveries?.length,4);
});
test('empty component packaging is rejected',()=>assert.throws(()=>validateResearchContinuation({})));

import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {validateContinuationWidths,checkCoordinationRetry,checkRecoveryInventory} from './research-continuation-components.mjs';
const retained=readFileSync(new URL('research-continuation-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const turn=p=>p.turns[0].gate;
const recovery=p=>p.recoveries[0].row;
function resealTurn(p,index=0){const r=p.turns[index].gate;r.value.digest=domain('orkworks.research-source-turn-completion.v1',r.value.body);}
function resealRecovery(p,index=0){const r=p.recoveries[index].row;r.value.digest=domain('orkworks.research-source-recovery-decision.v1',r.value.body);r.value.receipt.decisionDigest=r.value.digest;}
test('independent regeneration and checker preserve all component bytes',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),before=canonical(p);
  assert.deepEqual(validateResearchContinuation(p),p.measurements);assert.equal(canonical(p),before);
});
for(const [name,change] of [
  ['child turn family',p=>{turn(p).table='child_turn_completions';}],
  ['wrong turn source domain',p=>{const r=turn(p);r.value.digest=domain('orkworks.child-source-turn-completion.v1',r.value.body);}],
  ['unknown evidence kind',p=>{turn(p).value.body.evidence.kind='report-receipt';resealTurn(p);}],
  ['foreign child generation',p=>{turn(p).value.body.evidence.runtime.launchGeneration='0'.repeat(64);resealTurn(p);}],
  ['frozen version used as live version',p=>{turn(p).value.body.expectedTaskVersion=2;turn(p).value.body.acceptedTaskVersion=3;resealTurn(p);}],
  ['missing task readiness advance',p=>{p.turns[0].taskAfter.value.body.taskVersion=7;}],
  ['turn creates task outcome',p=>{p.turns[0].taskAfter.value.body.status='reported_complete';}],
  ['extra turn source field',p=>{turn(p).value.body.token='invented';resealTurn(p);}],
  ['wrong UI sidecar',p=>{p.turns[1].gate.value.body.evidence.sidecarGeneration='0'.repeat(64);resealTurn(p,1);}],
  ['terminal evidence without terminal bridge',p=>{p.turns[2].gate.value.body.evidence.session.value.body.state='nonterminal';resealTurn(p,2);}],
  ['terminal evidence copied origin',p=>{p.turns[2].gate.value.body.evidence.session.value.lastCommit=turn(p).value.lastCommit;resealTurn(p,2);}],
  ['missing coordination turn gate',p=>{delete p.coordinations[0].gate;}],
  ['child coordination family',p=>{p.coordinations[0].row.table='child_coordination_receipts';}],
  ['wrong coordination source domain',p=>{const r=p.coordinations[0].row;r.value.requestDigest=domain('orkworks.child-source-coordination-request.v1',r.value.request);}],
  ['report receipt substitutes for turn',p=>{p.coordinations[0].gate.table='report_receipts';}],
  ['coordination changes frozen reservation',p=>{p.coordinations[0].row.value.request.reservation.reservationId='foreign';}],
  ['conflicting parent outcome',p=>{p.coordinations[0].row.value.request.outcome='failed';}],
  ['coordination releases live child',p=>{p.coordinations[0].capacityReleased=true;}],
  ['child recovery family',p=>{recovery(p).table='child_recovery_decisions';}],
  ['wrong recovery source domain',p=>{const r=recovery(p);r.value.digest=domain('orkworks.child-source-recovery-decision.v1',r.value.body);r.value.receipt.decisionDigest=r.value.digest;}],
  ['recovery expected run mismatch',p=>{recovery(p).value.body.expectedRunVersion=19;resealRecovery(p);}],
  ['recovery frozen/live mismatch',p=>{recovery(p).value.body.expectedTaskVersion=2;resealRecovery(p);}],
  ['recovery child snapshot cast',p=>{recovery(p).value.body.readSet.run.table='child_run_controls';resealRecovery(p);}],
  ['recovery snapshot foreign key',p=>{recovery(p).value.body.readSet.task.key.taskId='foreign';resealRecovery(p);}],
  ['recovery copied snapshot origin',p=>{const b=recovery(p).value.body;b.readSet.run.value.lastCommit=b.readSet.expectedCommit;resealRecovery(p);}],
  ['omitted original blocker',p=>{recovery(p).value.body.blockers.pop();resealRecovery(p);}],
  ['changed original blocker body',p=>{recovery(p).value.body.blockers[0].detail='different';resealRecovery(p);}],
  ['unknown recovery kind',p=>{recovery(p).value.body.kind='retry-spawn';resealRecovery(p);}],
  ['no-child invents attached session',p=>{recovery(p).value.body.readSet.session=p.recoveries[2].row.value.body.readSet.session;resealRecovery(p);}],
  ['allocation confirmation omits allocation',p=>{p.recoveries[1].row.value.body.allocation=null;resealRecovery(p,1);}],
  ['attachment confirmation omits attachment',p=>{p.recoveries[2].row.value.body.attachmentId=null;resealRecovery(p,2);}],
  ['terminal observation omits terminal bridge',p=>{p.recoveries[3].row.value.body.readSet.session=null;resealRecovery(p,3);}],
  ['unknown outcome called rollback',p=>{p.policies[4].expected='rollback/retry';}],
  ['new format selection',p=>{p.formatVersion=7;}],
  ['missing native assumption',p=>{p.assumptions.splice(1,1);}],
  ['duplicate alternative case label',p=>{p.recoveries[1].id=p.recoveries[0].id;}],
  ['extra unbounded alternative',p=>{p.turns.push(structuredClone(p.turns[0]));}],
  ['discounted embedded snapshots',p=>{p.measurements.recoveries[0].chargeBytes--;}],
])test('component denies '+name,()=>{const p=packet();change(p);assert.throws(()=>validateResearchContinuation(p));});
test('original coordination retry precedes live CAS, after supplied authority/rate checks',()=>{
  const r=packet().coordinations[0].row,request=structuredClone(r.value.request);
  assert.deepEqual(checkCoordinationRetry(r,request,{authority:'supplied-current',rate:'supplied-allowed',liveTaskVersion:99}),r.value.receipt);
  request.summary='changed';assert.throws(()=>checkCoordinationRetry(r,request,{authority:'supplied-current',rate:'supplied-allowed',liveTaskVersion:99}),/conflict/);
  assert.throws(()=>checkCoordinationRetry(null,null,{authority:'supplied-revoked',rate:'supplied-allowed',liveTaskVersion:99}),/authority/);
  assert.throws(()=>checkCoordinationRetry(null,null,{authority:'supplied-current',rate:'supplied-denied',liveTaskVersion:99}),/rate/);
});
test('new recovery requires unique key and room within the supplied 32-per-run inventory',()=>{
  const task=recovery(packet()).key.task;
  const keys=Array.from({length:32},(_,n)=>({task,recoveryId:'retained-'+n}));
  const next={task,recoveryId:'new'};
  assert.doesNotThrow(()=>checkRecoveryInventory(keys.slice(0,31),next));
  assert.throws(()=>checkRecoveryInventory(keys,next),/32/);
  assert.throws(()=>checkRecoveryInventory([keys[0],keys[0]],next),/duplicate/);
  assert.throws(()=>checkRecoveryInventory([keys[0]],keys[0]),/original retry/);
});
test('complete wrapper, receipt, snapshot and independent lower job limits apply',()=>{
  const p=packet(),r=recovery(p);
  assert.doesNotThrow(()=>validateContinuationWidths(r,1048576));
  assert.throws(()=>validateContinuationWidths(r,1048577),/job/);
  for(const [name,max] of [['run',1048576],['task',16384],['launch',32768]]){
    const q=structuredClone(r);q.value.body.readSet[name].value.padding='x'.repeat(max);
    assert.throws(()=>validateContinuationWidths(q,1),/snapshot/);
  }
  const receipt=structuredClone(p.coordinations[0].row);receipt.value.receipt.padding='x'.repeat(4096);
  assert.throws(()=>validateContinuationWidths(receipt,1),/receipt/);
  for(const [q,max] of [[structuredClone(p.turns[0].gate),65536],[structuredClone(p.coordinations[0].row),16384],[structuredClone(r),2097152]]){
    q.value.padding='x'.repeat(max);assert.throws(()=>validateContinuationWidths(q,1),/wrapper/);
  }
  const q=structuredClone(p.turns[2].gate);q.value.body.evidence.session.value.padding='x'.repeat(16384);
  assert.throws(()=>validateContinuationWidths(q,1),/snapshot/);
});
test('generator input/output under spaces and percent signs preserves UTF-8 LF bytes',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research continuation % '));
  try{
    for(const name of ['generate-research-continuation.py','research-prefix-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const output=join(dir,'out %.json');execFileSync('python3',[join(dir,'generate-research-continuation.py'),'--output',output]);assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});

import {checkRecoveryRetry} from './research-continuation-components.mjs';
test('original recovery retry precedes live CAS and count, after supplied current UI authority',()=>{
  const r=recovery(packet()),b=structuredClone(r.value.body),before=canonical(r);
  assert.deepEqual(checkRecoveryRetry(r,b,{authority:'supplied-current-UI',liveRunVersion:99,liveTaskVersion:99}),r.value.receipt);
  assert.equal(canonical(r),before);
  b.reason='changed';assert.throws(()=>checkRecoveryRetry(r,b,{authority:'supplied-current-UI',liveRunVersion:99,liveTaskVersion:99}),/conflict/);
  assert.throws(()=>checkRecoveryRetry(null,null,{authority:'supplied-revoked',liveRunVersion:99,liveTaskVersion:99}),/authority/);
});
test('literal independent source measurements and representative digests are pinned',()=>{
  const p=packet();assert.deepEqual(validateResearchContinuation(p),{
    turns:[{chargeBytes:1341,wrapperBytes:1541},{chargeBytes:1504,wrapperBytes:1704},{chargeBytes:2878,wrapperBytes:3078}],
    coordinations:[{chargeBytes:1540,wrapperBytes:1936},{chargeBytes:1538,wrapperBytes:1934},{chargeBytes:1537,wrapperBytes:1933}],
    recoveries:[{chargeBytes:8213,wrapperBytes:8609},{chargeBytes:9078,wrapperBytes:9474},{chargeBytes:11445,wrapperBytes:11841},{chargeBytes:11949,wrapperBytes:12345}]});
  assert.equal(turn(p).value.digest,'dea5ab50c7ca5da773857c421bbbf56fd62e835708b766ffb4b10827f4ee9631');
  assert.equal(recovery(p).value.digest,'7189a0b60877686986594cf8383b353ac3a8ad61a02f5ddf8f6d2879c28c4d24');
});
test('width-only recovery run may fit its snapshot and still exceed lower complete-job limit',()=>{
  const q=structuredClone(recovery(packet())),s=q.value.body.readSet.run;
  s.value.widthOnlyPadding='';s.value.widthOnlyPadding='x'.repeat(1048576-Buffer.byteLength(canonical({key:s.key,value:s.value})));
  assert.equal(Buffer.byteLength(canonical({key:s.key,value:s.value})),1048576);
  assert(Buffer.byteLength(canonical({key:q.key,value:q.value}))<2097152);
  assert.throws(()=>validateContinuationWidths(q,Buffer.byteLength(canonical(q))),/job/);
});
