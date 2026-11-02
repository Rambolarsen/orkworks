// Documentation-only synthetic component checker. Never import into runtime.
// Supplied history, control snapshots and lists prove neither freshness,
// complete membership, native events nor transactional/authorization eligibility.
import assert from 'node:assert/strict';
import {canonical,hash,exact,text,id,digest,integer,list,sorted,nativePath} from './root-bootstrap-components.mjs';
import {validateRun} from './root-preparation-components.mjs';
import {verifyChildSources} from './child-source-components.mjs';
import {validateSourceComponent} from './root-preparation-source-components.mjs';

const same=(a,b,message='exact control/source join')=>assert.deepEqual(a,b,message);
const tag=(x,xs)=>assert(xs.includes(x),'closed control tag');
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const domain=(d,x)=>hash(d+'\n'+canonical(x));
const bytes=x=>Buffer.byteLength(canonical(x));
const bound=(x,max)=>assert(bytes(x)<=max,'complete component byte ceiling');
const version=x=>integer(x,1,2147483647);
const revision=x=>integer(x,1,64);
const counter=(x,positive=true)=>{
 assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(x)&&BigInt(x)<=18446744073709551615n&&(!positive||BigInt(x)>0n),'lossless u64 Counter');
};
function commit(x){exact(x,'storageEpoch commitSequence changeDigest');digest(x.storageEpoch);digest(x.changeDigest);counter(x.commitSequence);}
const limits={child_control_inputs:65536,child_runtime_attachments:4096,child_run_controls:1048576,child_plan_controls:16384,child_task_controls:16384,child_launch_controls:32768,child_session_states:16384,child_coordination_receipts:16384,child_recovery_decisions:2097152};
const runKey=r=>pick(r,'workspaceId runId');
const taskOf=o=>o.source.configuration.task;
function owner(t,c){
 exact(t,'workspaceId runId planId planRevision taskId');
 for(const k of ['workspaceId','runId','planId','taskId'])id(t[k]);revision(t.planRevision);
 const found=[c.sources.worker,c.sources.reviewer].find(o=>canonical(taskOf(o))===canonical(t));
 assert(found,'retained original task');return found;
}
function sourcePlan(p,c){
 exact(p,'workspaceId runId planId planRevision planDefinitionDigest');digest(p.planDefinitionDigest);
 const found=[c.sources.worker,c.sources.reviewer].find(o=>canonical(o.source.plan)===canonical(p));assert(found,'retained exact plan');return found;
}
function original(x,o,k){same(x,o.source[k],'original '+k);}
function runtimeBody(b,c){
 tag(b.kind,['parent','child']);
 if(b.kind==='parent'){
  exact(b,'schemaVersion kind parent');assert.equal(b.schemaVersion,1);assert.equal(b.parent.kind,'attached');
  validateSourceComponent('attachment',b.parent,{run:c.sources.run,rootAssignmentId:c.rootAssignmentId});
 }else{
  exact(b,'schemaVersion kind assignmentIdentity attachmentId sidecarGeneration launchGeneration');assert.equal(b.schemaVersion,1);
  const o=owner(pick(b.assignmentIdentity,'workspaceId runId planId planRevision taskId'),c);
  original(b.assignmentIdentity,o,'assignmentIdentity');assert.equal(b.attachmentId,o.source.attachmentId);digest(b.sidecarGeneration);digest(b.launchGeneration);
 }
 const p=b.kind==='parent'?b.parent:b;
 return {kind:b.kind,...runKey(c.sources.run),sessionId:b.kind==='parent'?p.parentSessionId:b.assignmentIdentity.childSessionId,
  sidecarGeneration:p.sidecarGeneration,launchGeneration:p.launchGeneration};
}
function runtimeRef(x,c,kind,sessionId){
 exact(x,'kind workspaceId runId sessionId sidecarGeneration launchGeneration attachmentDigest');
 tag(x.kind,['parent','child']);same(pick(x,'workspaceId runId'),runKey(c.sources.run));id(x.sessionId);
 for(const k of ['sidecarGeneration','launchGeneration','attachmentDigest'])digest(x[k]);
 if(kind)assert.equal(x.kind,kind);if(sessionId)assert.equal(x.sessionId,sessionId);
 const matches=(c.runtimeHistory??[]).filter(r=>r.table==='child_runtime_attachments'&&canonical(r.key)===canonical(pick(x,'kind workspaceId runId sessionId sidecarGeneration launchGeneration')));
 assert.equal(matches.length,1,'exact retained synthetic runtime');
 const r=matches[0];same(runtimeBody(r.value.body,c),r.key);assert.equal(domain('orkworks.child-source-runtime-attachment.v1',r.value.body),x.attachmentDigest);
 assert.equal(r.value.digest,x.attachmentDigest);
}
function blockers(xs,c){
 list(xs,16);sorted(xs,x=>x.id);
 for(const b of xs){exact(b,'id kind task detail');id(b.id);tag(b.kind,['launch-interrupted','source-changed','capacity-exceeded','profile-exit-required','control-exhausted','native-unavailable','coordination-failure','recovery-approval-required']);text(b.detail,2048);if(b.task!==null)owner(b.task,c);}
}
function directories(xs,o){
 list(xs,2);sorted(xs,x=>x.kind);
 const w=o.allocation.allocatedWorktree;
 for(const d of xs){exact(d,'kind path object');tag(d.kind,['root','private-git']);nativePath(d.path);
  exact(d.object,'platform mechanismVersion volumeId fileId');
  // Full object descriptors were independently validated in the retained source packet.
  same(d.path,d.kind==='root'?w.canonicalRoot:w.canonicalPrivateGitDirectory);
  same(d.object,d.kind==='root'?w.rootObject:w.privateGitDirectoryObject,'original owned directory descriptor');
 }
}
function prepared(b,c){
 exact(b,'schemaVersion workspaceId runId preparationRevision inputRevision goalDigest bootstrapConfigurationDigest decision records inputDigest');assert.equal(b.schemaVersion,1);
 const r=c.sources.run;same(pick(b,'workspaceId runId goalDigest bootstrapConfigurationDigest'),pick(r,'workspaceId runId goalDigest bootstrapConfigurationDigest'));
 integer(b.preparationRevision,2,64);integer(b.inputRevision,2,64);assert.equal(b.preparationRevision,b.inputRevision);
 const d=b.decision;exact(d,'schemaVersion inputRevision clarificationNeeded researchNeeded reasons unknowns');assert.equal(d.schemaVersion,1);assert.equal(d.inputRevision,b.inputRevision);
 assert.equal(d.clarificationNeeded,false);assert.equal(d.researchNeeded,false);same(d.unknowns,[]);list(d.reasons,16,1);d.reasons.forEach(x=>text(x,2048));
 list(b.records,256,1);sorted(b.records,x=>x.kind+'\0'+canonical(x));
 const configurations=b.records.filter(x=>x.kind==='configuration');
 const reviewing=b.records.some(x=>x.kind==='review-input');const o=reviewing?c.sources.reviewer:c.sources.worker;
 const expected=[{kind:'configuration',reference:o.source.configuration}];
 if(reviewing)expected.push({kind:'configuration',reference:c.sources.worker.source.configuration},{kind:'review-input',reference:o.source.input},
  {kind:'result',reference:c.sources.snapshot.target},...c.sources.snapshot.evidence.map(x=>({kind:'research-report',source:x.source})));
 const prior=b.records.filter(x=>x.kind==='prior-plan');list(prior,1);
 if(prior.length)expected.push({kind:'prior-plan',plan:o.source.plan});
 assert(configurations.length>0);expected.sort((a,b)=>Buffer.compare(Buffer.from(a.kind+'\0'+canonical(a)),Buffer.from(b.kind+'\0'+canonical(b))));
 same(b.records,expected,'complete selected prospective input consumption');
 const body={...b};delete body.inputDigest;digest(b.inputDigest);assert.equal(b.inputDigest,domain('orkworks.preparation-input.v1',body));bound(b,32768);
}
function runControl(b,c){
 exact(b,'schemaVersion profile workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest runVersion state currentInput planReferences currentPlan pendingPlan parentRuntime blockers stageContinueRequired pendingClarification pauseReason resumeTarget');
 assert.equal(b.schemaVersion,1);assert.equal(b.profile,'single-task-retained-data-execution-v1');
 same(pick(b,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),pick(c.sources.run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'));
 version(b.runVersion);tag(b.state,['assessing','awaiting_execution_approval','executing','blocked','paused','awaiting_resume_approval','complete','cancelled']);
 if(b.currentInput!==null){exact(b.currentInput,'workspaceId runId preparationRevision inputRevision inputDigest');same(pick(b.currentInput,'workspaceId runId'),runKey(c.sources.run));integer(b.currentInput.preparationRevision,2,64);integer(b.currentInput.inputRevision,2,64);digest(b.currentInput.inputDigest);
  const inputs=(c.inputHistory??[]).filter(r=>canonical({...r.key,inputDigest:r.value.body.inputDigest})===canonical(b.currentInput));assert.equal(inputs.length,1,'retained prepared input');prepared(inputs[0].value.body,c);
 }
 list(b.planReferences,1024);assert.equal(new Set(b.planReferences.map(canonical)).size,b.planReferences.length);b.planReferences.forEach(p=>sourcePlan(p,c));
 assert(new Set(b.planReferences.map(p=>p.planId)).size<=16);
 for(const p of [b.currentPlan,b.pendingPlan])if(p!==null){sourcePlan(p,c);assert(b.planReferences.some(x=>canonical(x)===canonical(p)),'plan retained in submission history');}
 if(b.parentRuntime!==null)runtimeRef(b.parentRuntime,c,'parent',b.parentSessionId);
 blockers(b.blockers,c);assert.equal(typeof b.stageContinueRequired,'boolean');assert.equal(b.pendingClarification,null);
 if(b.state==='paused'){tag(b.pauseReason,['stage_cancelled','parent_ended','workspace_changed','sidecar_replaced']);tag(b.resumeTarget,['assessing','awaiting_execution_approval','executing']);}
 else{assert.equal(b.pauseReason,null);assert.equal(b.resumeTarget,null);}
}
function planControl(b,c){
 exact(b,'schemaVersion plan planVersion status latestApproval task replaces');assert.equal(b.schemaVersion,1);const o=sourcePlan(b.plan,c);version(b.planVersion);
 tag(b.status,['proposed','active','paused','superseded','complete','cancelled']);same(b.task,taskOf(o));
 if(b.latestApproval!==null)original(b.latestApproval,o,'approval');if(b.status==='active')assert.notEqual(b.latestApproval,null);
 if(b.replaces!==null){sourcePlan(b.replaces,c);const inputs=(c.inputHistory??[]).filter(r=>r.value.body.records.some(x=>x.kind==='prior-plan'&&canonical(x.plan)===canonical(b.replaces)));
  assert(inputs.length>0,'replacement requires retained prepared prior-plan binding');
  // This source packet has only original plans. It cannot establish a replacement
  // plan bound to the prospective input, so that join remains unavailable here.
  assert.equal(o.plan.parentPlanBinding.inputDigest,inputs[0].value.body.inputDigest,'exact replacement source input');
 }
}
function taskControl(b,c){
 exact(b,'schemaVersion task taskVersion status reservation coordinationResult');assert.equal(b.schemaVersion,1);const o=owner(b.task,c);version(b.taskVersion);
 tag(b.status,['pending','launching','running','needs_parent_result','reported_complete','reported_blocked','reported_failed','launch_interrupted']);
 if(b.status==='pending'){assert.equal(b.reservation,null);assert.equal(b.coordinationResult,null);}else{original(b.reservation,o,'reservation');assert(b.taskVersion>=o.reservation.assignmentTaskVersion);}
 if(b.coordinationResult!==null){exact(b.coordinationResult,'requestDigest');digest(b.coordinationResult.requestDigest);
  const receipts=(c.coordinationHistory??[]).filter(r=>canonical(r.key)===canonical(b.task)&&r.value.requestDigest===b.coordinationResult.requestDigest);
  assert.equal(receipts.length,1,'retained original coordination receipt');coordination(receipts[0].value,c);
  assert(receipts[0].value.receipt.acceptedTaskVersion<=b.taskVersion,'coordination accepted version retained');
  if(b.status.startsWith('reported_'))assert.equal(receipts[0].value.request.outcome,({reported_complete:'completed',reported_blocked:'blocked',reported_failed:'failed'})[b.status]);
 }

 if(b.status.startsWith('reported_'))assert.notEqual(b.coordinationResult,null);
}
function launchControl(b,c){
 exact(b,'schemaVersion task reservation admission phase allocation attachmentId spawnRuntime observedDirectories interruptedAt reasonCode');assert.equal(b.schemaVersion,1);const o=owner(b.task,c);original(b.reservation,o,'reservation');
 const a=b.admission;exact(a,'approval parentRuntime expectedTaskVersion assignmentTaskVersion');original(a.approval,o,'approval');runtimeRef(a.parentRuntime,c,'parent',c.sources.run.parentSessionId);
 version(a.expectedTaskVersion);version(a.assignmentTaskVersion);assert.equal(a.expectedTaskVersion+1,a.assignmentTaskVersion);assert.equal(a.assignmentTaskVersion,o.reservation.assignmentTaskVersion,'frozen original reservation version');
 tag(b.phase,['reserved','allocating','allocated','prepared','spawn-pending','running','interrupted','ended','settled-no-child']);
 if(b.allocation!==null)original(b.allocation,o,'allocation');
 if(b.attachmentId!==null)assert.equal(b.attachmentId,o.source.attachmentId);
 if(b.spawnRuntime!==null){exact(b.spawnRuntime,'sidecarGeneration launchGeneration');digest(b.spawnRuntime.sidecarGeneration);digest(b.spawnRuntime.launchGeneration);}
 directories(b.observedDirectories,o);
 if(b.phase==='interrupted'){tag(b.interruptedAt,['allocation','attachment','spawn']);tag(b.reasonCode,['native-failure','source-drift','authority-lost','process-replaced','disposition-unknown']);}
 else{assert.equal(b.interruptedAt,null);assert.equal(b.reasonCode,null);}
 if(['reserved','allocating'].includes(b.phase)){assert.equal(b.allocation,null);assert.equal(b.attachmentId,null);assert.equal(b.spawnRuntime,null);}
 if(b.phase==='allocated'){assert.notEqual(b.allocation,null);assert.equal(b.attachmentId,null);assert.equal(b.spawnRuntime,null);}
 if(['prepared','spawn-pending','running','ended'].includes(b.phase)){assert.notEqual(b.allocation,null);assert.notEqual(b.attachmentId,null);}
 if(b.phase==='prepared')assert.equal(b.spawnRuntime,null);
 if(['spawn-pending','running','ended'].includes(b.phase))assert.notEqual(b.spawnRuntime,null);
 if(b.phase==='settled-no-child'){assert.equal(b.attachmentId,null);assert.equal(b.spawnRuntime,null);}
 if(b.attachmentId!==null)assert.notEqual(b.allocation,null);
 if(b.spawnRuntime!==null)assert.notEqual(b.attachmentId,null);
}
function sessionState(b,c){
 exact(b,'schemaVersion task reservation assignmentIdentity attachmentId sessionId producerRevision state runtime');assert.equal(b.schemaVersion,1);const o=owner(b.task,c);
 original(b.reservation,o,'reservation');original(b.assignmentIdentity,o,'assignmentIdentity');assert.equal(b.attachmentId,o.source.attachmentId);assert.equal(b.sessionId,o.source.assignmentIdentity.childSessionId);
 counter(b.producerRevision);tag(b.state,['prepared','nonterminal','terminal']);
 if(b.state==='prepared')assert.equal(b.runtime,null);
 if(b.state==='nonterminal')assert.notEqual(b.runtime,null);
 if(b.runtime!==null)runtimeRef(b.runtime,c,'child',b.sessionId);
}
function coordination(v,c){
 exact(v,'schemaVersion request requestDigest receipt lastCommit');const b=v.request;
 exact(b,'schemaVersion task reservation childSessionId expectedTaskVersion outcome summary');assert.equal(b.schemaVersion,1);const o=owner(b.task,c);original(b.reservation,o,'reservation');assert.equal(b.childSessionId,o.source.assignmentIdentity.childSessionId);
 version(b.expectedTaskVersion);tag(b.outcome,['completed','blocked','failed']);text(b.summary,2048);digest(v.requestDigest);assert.equal(v.requestDigest,domain('orkworks.child-source-coordination-request.v1',b));
 const r=v.receipt;exact(r,'schemaVersion task reservation childSessionId acceptedTaskVersion commit');assert.equal(r.schemaVersion,1);
 same(pick(r,'task reservation childSessionId'),pick(b,'task reservation childSessionId'));version(r.acceptedTaskVersion);assert.equal(r.acceptedTaskVersion,b.expectedTaskVersion+1);commit(r.commit);same(r.commit,v.lastCommit);bound(r,4096);
}
function recovery(b,c){
 exact(b,'schemaVersion recoveryId task reservation expectedRunVersion expectedTaskVersion reviewContextId sidecarGeneration kind allocation attachmentId readSet observedDirectories blockers reason');assert.equal(b.schemaVersion,1);
 const o=owner(b.task,c);original(b.reservation,o,'reservation');id(b.recoveryId);version(b.expectedRunVersion);version(b.expectedTaskVersion);digest(b.reviewContextId);digest(b.sidecarGeneration);text(b.reason,2048);
 tag(b.kind,['settle-no-child','confirm-original-allocation','confirm-original-attachment','observe-terminal']);if(b.allocation!==null)original(b.allocation,o,'allocation');
 if(b.attachmentId!==null)assert.equal(b.attachmentId,o.source.attachmentId);directories(b.observedDirectories,o);blockers(b.blockers,c);
 const rs=b.readSet;exact(rs,'expectedCommit run task launch session');commit(rs.expectedCommit);
 const kinds={run:'child_run_controls',task:'child_task_controls',launch:'child_launch_controls',session:'child_session_states'};
 for(const [name,table]of Object.entries(kinds)){
  if(name==='session'&&rs.session===null)continue;
  const r=rs[name];exact(r,'table key value');assert.equal(r.table,table);same(r.key,name==='run'?runKey(c.sources.run):b.task);
  validateControlRow({...r,rowDigest:controlRowDigest(r)},c);
  assert.equal(r.value.lastCommit.storageEpoch,rs.expectedCommit.storageEpoch);assert(BigInt(r.value.lastCommit.commitSequence)<=BigInt(rs.expectedCommit.commitSequence),'snapshot origin no later than captured header');
 }
 assert.equal(rs.run.value.body.runVersion,b.expectedRunVersion);assert.equal(rs.task.value.body.taskVersion,b.expectedTaskVersion);same(rs.task.value.body.reservation,b.reservation);same(rs.launch.value.body.reservation,b.reservation);
 if(rs.session!==null){same(rs.session.value.body.reservation,b.reservation);assert.equal(rs.session.value.body.attachmentId,b.attachmentId);}
 for(const blocker of b.blockers){assert(blocker.task!==null);same(blocker.task,b.task);assert(rs.run.value.body.blockers.some(x=>canonical(x)===canonical(blocker)),'exact captured blocker body');}
 same(b.allocation,rs.launch.value.body.allocation);assert.equal(b.attachmentId,rs.launch.value.body.attachmentId);
 if(['settle-no-child','confirm-original-allocation'].includes(b.kind)){assert.equal(b.attachmentId,null);assert.equal(rs.session,null);}
 else{assert.notEqual(b.attachmentId,null);assert.notEqual(rs.session,null);assert.notEqual(b.allocation,null);}
 if(b.kind==='confirm-original-allocation')assert.notEqual(b.allocation,null);
 if(b.kind==='observe-terminal')assert.equal(rs.session.value.body.state,'terminal');
}
/** Projection only. This does not validate excluded links or any owning proof. */
export function controlRowDigest(r){
 assert(Object.hasOwn(limits,r.table),'closed control table');const body=structuredClone(r.value);delete body.lastCommit;
 if(['child_coordination_receipts','child_recovery_decisions'].includes(r.table))delete body.receipt.commit;
 return domain('orkworks.hierarchy-storage-row.v4',{table:r.table,key:r.key,body});
}
export function validateControlRow(r,c){
 exact(r,'table key value rowDigest');assert(Object.hasOwn(limits,r.table),'closed control catalog');digest(r.rowDigest);
 const v=r.value;commit(v.lastCommit);assert.equal(v.schemaVersion,1);bound({key:r.key,value:v},limits[r.table]);
 let key;
 if(r.table==='child_coordination_receipts'){coordination(v,c);key=v.request.task;}
 else{
  const extras=r.table==='child_control_inputs'?' producer':r.table==='child_runtime_attachments'?' digest':r.table==='child_recovery_decisions'?' digest receipt':'';
  exact(v,'schemaVersion body lastCommit'+extras);const b=v.body;
  switch(r.table){
   case 'child_control_inputs':prepared(b,c);runtimeRef(v.producer,c,'parent',c.sources.run.parentSessionId);key=pick(b,'workspaceId runId preparationRevision inputRevision');break;
   case 'child_runtime_attachments':key=runtimeBody(b,c);digest(v.digest);assert.equal(v.digest,domain('orkworks.child-source-runtime-attachment.v1',b));break;
   case 'child_run_controls':runControl(b,c);key=runKey(b);break;
   case 'child_plan_controls':planControl(b,c);key=b.plan;break;
   case 'child_task_controls':taskControl(b,c);key=b.task;break;
   case 'child_launch_controls':launchControl(b,c);key=b.task;break;
   case 'child_session_states':sessionState(b,c);key=b.task;break;
   case 'child_recovery_decisions':{
    recovery(b,c);digest(v.digest);assert.equal(v.digest,domain('orkworks.child-source-recovery-decision.v1',b));key={task:b.task,recoveryId:b.recoveryId};
    const q=v.receipt;exact(q,'schemaVersion recoveryId decisionDigest runVersion taskVersion commit');assert.equal(q.schemaVersion,1);assert.equal(q.recoveryId,b.recoveryId);assert.equal(q.decisionDigest,v.digest);version(q.runVersion);version(q.taskVersion);
    assert.equal(q.runVersion,b.expectedRunVersion+1);assert.equal(q.taskVersion,b.expectedTaskVersion+1);commit(q.commit);same(q.commit,v.lastCommit);assert.equal(q.commit.storageEpoch,b.readSet.expectedCommit.storageEpoch);assert.equal(BigInt(q.commit.commitSequence),BigInt(b.readSet.expectedCommit.commitSequence)+1n);bound(q,4096);break;
   }
  }
 }
 same(r.key,key,'reconstructed full metadata key');assert.equal(r.rowDigest,controlRowDigest(r));return 'unverified-control-components-only';
}
export function validateControlPacket(p,c){
 exact(p,'schemaVersion classification rows');assert.equal(p.schemaVersion,1);assert.equal(p.classification,'synthetic-unverified-child-control-components-only');verifyChildSources(c.sources);
 const ctx={...c,runtimeHistory:p.rows.filter(x=>x.row.table==='child_runtime_attachments').map(x=>x.row),inputHistory:p.rows.filter(x=>x.row.table==='child_control_inputs').map(x=>x.row),coordinationHistory:p.rows.filter(x=>x.row.table==='child_coordination_receipts').map(x=>x.row)};
 const ids=new Set();
 for(const v of p.rows){exact(v,'id row canonical bytes sha256');id(v.id);assert(!ids.has(v.id));ids.add(v.id);
  same(canonical(v.row),v.canonical);assert.equal(bytes(v.row),v.bytes);assert.equal(hash(v.canonical),v.sha256);validateControlRow(v.row,ctx);
 }
 return {classification:'unverified-control-components-only',rows:p.rows.length,tables:new Set(p.rows.map(x=>x.row.table)).size};
}
/** Conditional recount of a supplied synthetic list, never a completeness proof. */
export function capacityComponent(attempts,c,p){
 validateRun(c.sources.run);const o=sourcePlan(p,c),seen=new Set();let runUsage=0,planUsage=0;
 for(const {launch,session}of attempts){validateControlRow(launch,c);assert.equal(launch.table,'child_launch_controls');const b=launch.value.body,k=canonical(b.task);assert(!seen.has(k),'unique original attempts');seen.add(k);
  let held;
  if(b.attachmentId===null){assert.equal(session,null);held=b.phase!=='settled-no-child';}
  else{assert.notEqual(session,null,'attached disposition required');validateControlRow(session,c);assert.equal(session.table,'child_session_states');same(session.key,b.task);assert.equal(session.value.body.attachmentId,b.attachmentId);held=session.value.body.state!=='terminal';}
  if(held){runUsage++;if(canonical(b.task)===canonical(taskOf(o)))planUsage++;}
 }
 integer(c.sources.run.maxParallelChildren,1,16);assert.equal(o.plan.maxParallelChildren,1);
 return {runUsage,planUsage,canReserve:runUsage+1<=c.sources.run.maxParallelChildren&&planUsage+1<=o.plan.maxParallelChildren};
}
/** Ordered classification only; no current authority/grant or resume operation. */
export function resumeComponent(run,launches,c){
 validateControlRow(run,c);assert.equal(run.table,'child_run_controls');const b=run.value.body;
 launches.forEach(x=>{validateControlRow(x,c);assert.equal(x.table,'child_launch_controls');});
 if(['complete','cancelled'].includes(b.state))return 'terminal';
 if(b.stageContinueRequired)return 'stage-continue';
 if(b.blockers.length||launches.some(x=>['reserved','allocating','allocated','spawn-pending','interrupted'].includes(x.value.body.phase)))return 'blocked';
 if(b.pendingPlan!==null)return 'pending-plan';
 if(b.currentPlan!==null)return 'reapproval';
 return 'preparation';
}
/** Supplied inventory only. It does not attest that all store rows were read. */
export function controlInventoryComponent(rows,c){
 const keys=new Set(),runtimeCounts=new Map();let total=0,recoveries=0;
 for(const r of rows){validateControlRow(r,c);const k=r.table+'\0'+canonical(r.key);assert(!keys.has(k),'unique retained inventory keys');keys.add(k);
  const v=structuredClone(r.value);delete v.lastCommit;if(v.receipt)delete v.receipt.commit;
  total+=bytes(r.key)+bytes(v);
  if(r.table==='child_recovery_decisions'){recoveries++;}
  if(r.table==='child_runtime_attachments'){const key=r.key.kind+'\0'+r.key.sessionId;runtimeCounts.set(key,(runtimeCounts.get(key)??0)+1);}
 }
 assert(recoveries<=32,'retained recovery count');for(const count of runtimeCounts.values())assert(count<=64,'retained runtime history count');
 assert(total<=8388608,'run control retained pool');assert(total<=67108864,'workspace control retained pool');
 return {classification:'conditional-control-inventory-only',runBytes:total,workspaceContributionBytes:total};
}
/** Compare retained canonical request bytes only. Current authentication must precede
 * this classification in a separately qualified owner; no operation is performed. */
export function coordinationRetryComponent(retained,request,c){
 validateControlRow(retained,c);assert.equal(retained.table,'child_coordination_receipts');
 same(canonical(request),canonical(retained.value.request),'conflicting original coordination outcome');
 return 'exact-retained-retry-candidate';
}
