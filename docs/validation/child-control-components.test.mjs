// Every component is synthetic; this suite never supplies native or SQL evidence.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {validateControlRow,controlRowDigest,validateControlPacket,capacityComponent,resumeComponent,controlInventoryComponent,coordinationRetryComponent} from './child-control-components.mjs';
const packet=JSON.parse(readFileSync(new URL('./child-control-vectors.json',import.meta.url)));
const sources=JSON.parse(readFileSync(new URL('./child-source-vectors.json',import.meta.url)));
const context={sources,rootAssignmentId:'root-1',runtimeHistory:packet.rows.filter(v=>v.row.table==='child_runtime_attachments').map(v=>v.row),inputHistory:packet.rows.filter(v=>v.row.table==='child_control_inputs').map(v=>v.row),coordinationHistory:packet.rows.filter(v=>v.row.table==='child_coordination_receipts').map(v=>v.row)};
const row=id=>structuredClone(packet.rows.find(v=>v.id===id).row);
// Independent test encoding, not the checker under test.
const wire=x=>Array.isArray(x)?'['+x.map(wire).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+wire(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const domain=(d,x)=>sha(d+'\n'+wire(x));
function rehash(r){
  const v=r.value;
  if(r.table==='child_control_inputs'){const b={...v.body};delete b.inputDigest;v.body.inputDigest=domain('orkworks.preparation-input.v1',b);}
  if(r.table==='child_runtime_attachments')v.digest=domain('orkworks.child-source-runtime-attachment.v1',v.body);
  if(r.table==='child_coordination_receipts')v.requestDigest=domain('orkworks.child-source-coordination-request.v1',v.request);
  if(r.table==='child_recovery_decisions'){v.digest=domain('orkworks.child-source-recovery-decision.v1',v.body);v.receipt.decisionDigest=v.digest;}
  const b=structuredClone(v);delete b.lastCommit;if(b.receipt)delete b.receipt.commit;
  r.rowDigest=domain('orkworks.hierarchy-storage-row.v4',{table:r.table,key:r.key,body:b});return r;
}

test('all independently generated variants remain component-only',()=>{
  assert.deepEqual(validateControlPacket(packet,context),{classification:'unverified-control-components-only',rows:45,tables:9});
});
for(const v of packet.rows)test('canonical component '+v.id,()=>{
  assert.equal(wire(v.row),v.canonical);assert.equal(Buffer.byteLength(v.canonical),v.bytes);assert.equal(sha(v.canonical),v.sha256);
  assert.equal(controlRowDigest(v.row),v.row.rowDigest);
  assert.equal(validateControlRow(v.row,context),'unverified-control-components-only');
});
const rejects=[
 ['run-blocked','unknown run field',r=>r.value.body.grant=true],
 ['run-blocked','stale root binding',r=>r.value.body.runDefinitionDigest='0'.repeat(64)],
 ['run-blocked','unknown current plan',r=>r.value.body.currentPlan.planId='foreign'],
 ['run-paused','pause loses target',r=>r.value.body.resumeTarget=null],
 ['run-blocked','unsupported clarification',r=>r.value.body.pendingClarification={}],
 ['run-blocked','unknown blocker',r=>r.value.body.blockers[0].kind='anything'],
 ['run-blocked','foreign blocker task',r=>r.value.body.blockers[0].task.taskId='foreign'],
 ['run-blocked','run version overflow',r=>r.value.body.runVersion=2147483648],
 ['plan-active','active has no approval',r=>r.value.body.latestApproval=null],
 ['plan-active','foreign task',r=>r.value.body.task.taskId='foreign'],
 ['plan-active','unbound replacement',r=>r.value.body.replaces=r.value.body.plan],
 ['task-pending','pending reserves a child',r=>r.value.body.reservation=sources.worker.source.reservation],
 ['task-running','missing original reservation',r=>r.value.body.reservation=null],
 ['task-reported_complete','outcome digest missing',r=>r.value.body.coordinationResult=null],
 ['launch-reserved','reserved already allocated',r=>r.value.body.allocation=sources.worker.source.allocation],
 ['launch-allocated','allocated missing source',r=>r.value.body.allocation=null],
 ['launch-prepared','prepared has proposed runtime',r=>r.value.body.spawnRuntime={sidecarGeneration:'c'.repeat(64),launchGeneration:'d'.repeat(64)}],
 ['launch-running','running lacks original outcome',r=>r.value.body.spawnRuntime=null],
 ['launch-running','recaptured assignment version',r=>{r.value.body.admission.expectedTaskVersion=4;r.value.body.admission.assignmentTaskVersion=5;}],
 ['launch-running','foreign parent runtime',r=>r.value.body.admission.parentRuntime.sessionId='foreign'],
 ['launch-running','foreign allocation',r=>r.value.body.allocation.allocationId='other'],
 ['launch-running','adopted directory object',r=>r.value.body.observedDirectories[0].object.fileId='foreign'],
 ['launch-interrupted','missing disposition reason',r=>r.value.body.reasonCode=null],
 ['launch-settled-no-child','settled child has attachment',r=>r.value.body.attachmentId='other'],
 ['session-prepared','prepared has runtime',r=>r.value.body.runtime=row('session-nonterminal').value.body.runtime],
 ['session-nonterminal','nonterminal no runtime',r=>r.value.body.runtime=null],
 ['session-terminal','live version replaces frozen identity',r=>r.value.body.assignmentIdentity.taskVersion=5],
 ['session-terminal','rounded producer counter',r=>r.value.body.producerRevision=9007199254740992],
 ['session-terminal','counter overflow',r=>r.value.body.producerRevision='18446744073709551616'],
 ['parent-runtime','reserved parent as runtime',r=>{r.value.body.parent.kind='reserved';delete r.value.body.parent.sidecarGeneration;delete r.value.body.parent.launchGeneration;}],
 ['child-runtime','foreign generation key',r=>r.key.launchGeneration='e'.repeat(64)],
 ['child-runtime','foreign child assignment',r=>r.value.body.assignmentIdentity.childSessionId='foreign'],
 ['worker-prepared-input','unsupported clarification need',r=>r.value.body.decision.clarificationNeeded=true],
 ['worker-prepared-input','input revision not advanced',r=>{r.value.body.inputRevision=1;r.value.body.decision.inputRevision=1;r.key.inputRevision=1;}],
 ['reviewer-prepared-input','missing consumed configuration',r=>r.value.body.records=r.value.body.records.filter(x=>x.kind!=='configuration')],
 ['reviewer-prepared-input','duplicate consumed record',r=>r.value.body.records.push(r.value.body.records[0])],
 ['reviewer-prepared-input','transitive extra report',r=>{const x=structuredClone(r.value.body.records.find(x=>x.kind==='research-report'));x.source.reportIdentity.reportId='foreign';r.value.body.records.push(x);}],
 ['coordination-completed','receipt wrong session',r=>r.value.receipt.childSessionId='foreign'],
 ['coordination-completed','receipt wrong accepted version',r=>r.value.receipt.acceptedTaskVersion=4],
 ['coordination-completed','outcome unknown',r=>r.value.request.outcome='success'],
 ['coordination-completed','receipt owns another commit',r=>r.value.receipt.commit.changeDigest='0'.repeat(64)],
 ['recovery-settle-no-child','no-child with session snapshot',r=>r.value.body.readSet.session=row('recovery-observe-terminal').value.body.readSet.session],
 ['recovery-confirm-original-allocation','allocation absent',r=>r.value.body.allocation=null],
 ['recovery-confirm-original-attachment','session missing',r=>r.value.body.readSet.session=null],
 ['recovery-observe-terminal','terminal observed as live',r=>r.value.body.readSet.session.value.body.state='nonterminal'],
 ['recovery-observe-terminal','wrong captured task version',r=>r.value.body.readSet.task.value.body.taskVersion++],
 ['recovery-observe-terminal','arbitrary snapshot table',r=>r.value.body.readSet.run.table='run_states'],
 ['recovery-observe-terminal','quoted snapshot bytes',r=>r.value.body.readSet.launch.value=wire(r.value.body.readSet.launch.value)],
 ['recovery-observe-terminal','snapshot lost origin',r=>delete r.value.body.readSet.launch.value.lastCommit],
 ['recovery-observe-terminal','future snapshot sequence',r=>r.value.body.readSet.launch.value.lastCommit.commitSequence='3'],
 ['recovery-observe-terminal','cross epoch origin',r=>r.value.body.readSet.launch.value.lastCommit.storageEpoch='0'.repeat(64)],
 ['recovery-observe-terminal','blocker detail substituted',r=>r.value.body.blockers[0].detail='different'],
 ['recovery-observe-terminal','receipt wrong digest',r=>r.value.receipt.recoveryId='other'],
];
for(const [id,name,mutate] of rejects)test('rejects rehashed '+name,()=>{const r=row(id);mutate(r);rehash(r);assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});});

test('excluded new commit links do not erase embedded historical commits',()=>{
 const r=row('recovery-observe-terminal'),original=r.rowDigest;
 r.value.lastCommit.commitSequence='4';r.value.receipt.commit.commitSequence='4';assert.equal(controlRowDigest(r),original);
 r.value.body.readSet.launch.value.lastCommit.commitSequence='1';assert.notEqual(controlRowDigest(r),original);
});
test('positive u64 bridge and commit counters remain lossless',()=>{
 const r=row('session-terminal');r.value.body.producerRevision='18446744073709551615';r.value.lastCommit.commitSequence='18446744073709551615';rehash(r);assert.equal(validateControlRow(r,context),'unverified-control-components-only');
});
test('complete key/value wrapper is charged and bounded',()=>{
 const r=row('run-blocked');r.value.body.blockers[0].detail='界'.repeat(683);rehash(r);assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
for(const [name,sessionId,want] of [['unattached interrupted',null,1],['prepared','session-prepared',1],['live after report','session-nonterminal',1],['ended','session-terminal',0]])test('conditional capacity '+name,()=>{
 const l=row(sessionId==='session-prepared'?'launch-prepared':sessionId==='session-terminal'?'launch-ended':sessionId?'launch-running':'launch-interrupted');
 assert.deepEqual(capacityComponent([{launch:l,session:sessionId?row(sessionId):null}],context,sources.worker.source.plan),{runUsage:want,planUsage:want,canReserve:want===0});
});
test('settled no-child frees capacity only as conditional supplied disposition',()=>assert.equal(capacityComponent([{launch:row('launch-settled-no-child'),session:null}],context,sources.worker.source.plan).runUsage,0));
test('ordinary resume holds capacity without changing original spawn generation',()=>{
 const s=row('session-nonterminal');s.value.body.runtime.launchGeneration='e'.repeat(64);
 // The matching retained runtime is supplied separately; the original spawn is unchanged.
 const h=row('child-runtime');h.value.body.launchGeneration='e'.repeat(64);h.key.launchGeneration='e'.repeat(64);rehash(h);s.value.body.runtime.attachmentDigest=h.value.digest;rehash(s);
 const c={...context,runtimeHistory:[...context.runtimeHistory,h]};assert.equal(capacityComponent([{launch:row('launch-running'),session:s}],c,sources.worker.source.plan).canReserve,false);
});
test('capacity rejects incomplete attached disposition',()=>assert.throws(()=>capacityComponent([{launch:row('launch-running'),session:null}],context,sources.worker.source.plan),{name:'AssertionError'}));
test('capacity rejects duplicate attempts rather than counting a child twice',()=>assert.throws(()=>capacityComponent([{launch:row('launch-interrupted'),session:null},{launch:row('launch-interrupted'),session:null}],context,sources.worker.source.plan),{name:'AssertionError'}));
for(const [name,change,launches,want] of [
 ['stage gate',()=>{},[row('launch-interrupted')],'stage-continue'],
 ['blocker',r=>r.stageContinueRequired=false,[],'blocked'],
 ['unsettled',r=>{r.stageContinueRequired=false;r.blockers=[];},[row('launch-interrupted')],'blocked'],
 ['approved',r=>{r.stageContinueRequired=false;r.blockers=[];},[],'reapproval'],
 ['terminal',r=>r.state='cancelled',[],'terminal']
])test('conditional resume priority '+name,()=>{const r=row('run-blocked');change(r.value.body);rehash(r);assert.equal(resumeComponent(r,launches,context),want);});
test('independent generator reproduces complete packet',()=>{
 const dir=mkdtempSync(join(tmpdir(),'child-control-'));
 try{const dest=join(dir,'vectors.json');const r=spawnSync('python3',[fileURLToPath(new URL('./generate-child-control-vectors.py',import.meta.url)),'--output',dest],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);assert.deepEqual(readFileSync(dest),readFileSync(new URL('./child-control-vectors.json',import.meta.url)));}finally{rmSync(dir,{recursive:true,force:true});}
});

// These tests catch a stale run-ceiling join, forgotten repeated byte charge,
// mutable-row alias discount or conflicting original outcome classification.
test('capacity rejects a tampered retained run ceiling',()=>{
 const c=structuredClone(context);c.sources.run.maxParallelChildren=2;
 assert.throws(()=>capacityComponent([],c,c.sources.worker.source.plan),{name:'AssertionError'});
});
test('retained inventory counts every canonical key and projected value occurrence',()=>{
 const rows=[row('launch-running'),row('session-nonterminal'),row('coordination-completed')];
 const expected=rows.reduce((n,r)=>{const v=structuredClone(r.value);delete v.lastCommit;if(v.receipt)delete v.receipt.commit;return n+Buffer.byteLength(wire(r.key))+Buffer.byteLength(wire(v));},0);
 assert.deepEqual(controlInventoryComponent(rows,context),{classification:'conditional-control-inventory-only',runBytes:expected,workspaceContributionBytes:expected});
});
test('retained inventory rejects duplicate mutable keys',()=>assert.throws(()=>controlInventoryComponent([row('launch-running'),row('launch-ended')],context),{name:'AssertionError'}));
test('original coordination byte retry is a candidate only, never authentication',()=>{
 const r=row('coordination-completed');
 assert.equal(coordinationRetryComponent(r,structuredClone(r.value.request),context),'exact-retained-retry-candidate');
 const request=structuredClone(r.value.request);request.summary='Changed original outcome';
 assert.throws(()=>coordinationRetryComponent(r,request,context),{name:'AssertionError'});
});
test('recovery successor must advance the captured header sequence',()=>{
 const r=row('recovery-observe-terminal');r.value.lastCommit.commitSequence='2';r.value.receipt.commit.commitSequence='2';rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
for(const length of [2048,2049])test('coordination summary UTF-8 bound '+length,()=>{
 const r=row('coordination-completed');r.value.request.summary='x'.repeat(length);rehash(r);
 if(length===2048)assert.equal(validateControlRow(r,context),'unverified-control-components-only');else assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
for(const reason of ['stage_cancelled','parent_ended','workspace_changed','sidecar_replaced'])for(const target of ['assessing','awaiting_execution_approval','executing'])test('pause preserves explicit gate across '+reason+'/'+target,()=>{
 const r=row('run-paused');r.value.body.pauseReason=reason;r.value.body.resumeTarget=target;rehash(r);
 assert.equal(validateControlRow(r,context),'unverified-control-components-only');assert.equal(resumeComponent(r,[],context),'stage-continue');
});
for(const stage of ['allocation','attachment','spawn'])for(const reason of ['native-failure','source-drift','authority-lost','process-replaced','disposition-unknown'])test('interruption tag '+stage+'/'+reason,()=>{
 const r=row('launch-interrupted');r.value.body.interruptedAt=stage;r.value.body.reasonCode=reason;if(stage==='spawn')r.value.body.attachmentId=sources.worker.source.attachmentId;rehash(r);assert.equal(validateControlRow(r,context),'unverified-control-components-only');
});

for(const count of [32,33])test('conditional retained recovery count '+count,()=>{
 const rows=Array.from({length:count},(_,i)=>{const r=row('recovery-settle-no-child');r.key.recoveryId='recovery-'+i;r.value.body.recoveryId=r.key.recoveryId;r.value.receipt.recoveryId=r.key.recoveryId;return rehash(r);});
 if(count===32)assert.equal(controlInventoryComponent(rows,context).classification,'conditional-control-inventory-only');else assert.throws(()=>controlInventoryComponent(rows,context),{name:'AssertionError'});
});
for(const count of [64,65])test('conditional retained runtime count '+count,()=>{
 const rows=Array.from({length:count},(_,i)=>{const r=row('child-runtime');r.key.launchGeneration=i.toString(16).padStart(64,'0');r.value.body.launchGeneration=r.key.launchGeneration;return rehash(r);});
 if(count===64)assert.equal(controlInventoryComponent(rows,context).classification,'conditional-control-inventory-only');else assert.throws(()=>controlInventoryComponent(rows,context),{name:'AssertionError'});
});

test('task outcome cannot pin an arbitrary well-formed digest',()=>{
 const r=row('task-reported_complete');r.value.body.coordinationResult.requestDigest='0'.repeat(64);rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
test('task completion cannot point to a retained failed outcome',()=>{
 const r=row('task-reported_complete');r.value.body.coordinationResult.requestDigest=row('coordination-failed').value.requestDigest;rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
for(const stage of ['attachment','spawn'])test('interruption at '+stage+' cannot forget its allocation',()=>{
 const r=row('launch-interrupted');r.value.body.interruptedAt=stage;r.value.body.allocation=null;r.value.body.observedDirectories=[];rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
test('spawn interruption cannot forget its prepared attachment',()=>{
 const r=row('launch-interrupted');r.value.body.interruptedAt='spawn';rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
test('run capacity ceiling still binds a different retained plan',()=>{
 assert.deepEqual(capacityComponent([{launch:row('launch-interrupted'),session:null}],context,sources.reviewer.source.plan),{runUsage:1,planUsage:0,canReserve:false});
});
test('rehashed different run definition cannot supply an old plan ceiling',()=>{
 const c=structuredClone(context);c.sources.run.maxParallelChildren=2;const b={...c.sources.run};delete b.runDefinitionDigest;
 c.sources.run.runDefinitionDigest=domain('orkworks.orchestration-run.v1',b);
 assert.throws(()=>capacityComponent([],c,c.sources.worker.source.plan),{name:'AssertionError'});
});
test('running cannot cite an unretained original spawn generation',()=>{
 const r=row('launch-running');r.value.body.spawnRuntime.launchGeneration='f'.repeat(64);rehash(r);
 assert.throws(()=>validateControlRow(r,context),{name:'AssertionError'});
});
test('running phase and terminal ordinary snapshot are an inconsistent recount',()=>assert.throws(()=>capacityComponent([{launch:row('launch-running'),session:row('session-terminal')}],context,sources.worker.source.plan),{name:'AssertionError'}));
