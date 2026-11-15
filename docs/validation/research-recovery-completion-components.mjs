// Finite offline examples only; no event dispatcher, publisher or authority.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
const clone=structuredClone,same=(a,b,label)=>assert.deepEqual(a,b,label);
const size=x=>Buffer.byteLength(canonical(x)),digest=(name,b)=>hash(name+'\n'+canonical(b));
const names=['research-prefix-components.json','research-continuation-components.json','research-completion-components.json'];
const raw=names.map(n=>readFileSync(new URL(n,import.meta.url)));
const prefix=JSON.parse(raw[0]),a=prefix.chain.assignment,task=prefix.chain.startup.scope.task,plan=a.plan;
const fields=(x,ks)=>Object.fromEntries(ks.split(' ').map(k=>[k,x[k]]));
const runKey=fields(task,'workspaceId runId');
const [parent,child]=prefix.runtimeHistory.map(r=>({...r.key,attachmentDigest:r.value.digest}));
const source=prefix.chain.startup.context.run;
const runFields=fields(source,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest');
const input=fields(prefix.inputs[0].value.body,'workspaceId runId preparationRevision inputRevision inputDigest');
const origin=n=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED positive recovery label '+n)});
function snapshot(r,table,key,body,n){
 exact(r,'table key value');same(r.table,table,'typed snapshot table');same(r.key,key,'typed snapshot key');
 exact(r.value,'schemaVersion body lastCommit');assert.equal(r.value.schemaVersion,1);same(r.value.body,body,'complete source-derived body');same(r.value.lastCommit,origin(n),'supplied original origin');
 const cap=table==='research_run_controls'?1048576:table==='research_launch_controls'?32768:16384;
 assert(size(r)<=cap,'snapshot width');
}
function run(v,blockers){return {schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',...runFields,runVersion:v,state:'blocked',currentInput:input,
 planReferences:[plan],currentPlan:plan,pendingPlan:null,parentRuntime:parent,blockers,stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null};}
function taskBody(v,status,result=null){return {schemaVersion:1,task,taskVersion:v,status,reservation:a.reservation,coordinationResult:result};}
function record(r,table,key,domain,receipt){
 exact(r,'table key value');assert.equal(r.table,table);same(r.key,key,'original record key');exact(r.value,'schemaVersion body digest receipt lastCommit');assert.equal(r.value.schemaVersion,1);
 assert.equal(r.value.digest,digest(domain,r.value.body),'body source domain');same(r.value.receipt,receipt,'original outcome receipt');same(r.value.lastCommit,origin(8),'shared supplied C');
 assert(size({key:r.key,value:r.value})<=2097152,'2 MiB decision wrapper');assert(size(r.value.receipt)<=4096,'4 KiB receipt');
}
export function measureRecoveryCompletion(r){
 const v=clone(r.value);delete v.lastCommit;delete v.receipt.commit;
 return {chargeBytes:size({key:r.key,value:v}),wrapperBytes:size({key:r.key,value:r.value})};
}
export function checkCompoundWidths(rows){
 for(const r of rows){assert(size({key:r.key,value:r.value})<=2097152,'decision wrapper');assert(size(r.value.receipt)<=4096,'receipt width');}
 const measurements=rows.map(measureRecoveryCompletion);
 assert(measurements.reduce((n,m)=>n+m.wrapperBytes,0)<=1048576,'smaller paired job ceiling');return measurements;
}
// Checks exactly this finite selected example. It deliberately does not decode
// arbitrary wire records, discover source membership or elect a current owner.
export function validateRecoveryCompletion(p){
 exact(p,'inputHashes assumptions turn interruption coordination terminal recovery lifecycle bindings after facts measurements');
 same(p.inputHashes,Object.fromEntries(names.map((n,i)=>[n,hash(raw[i])])),'unchanged raw documentation inputs');
 same(p.assumptions,[
  'decoded hypothetical source snapshots, not a legal history or current store',
  'UI/native/authority/source/proof/currentness/membership/outcome facts supplied and unqualified',
  'symbolic CommitRefs hash labels only; no header, manifest, row/change hash or storage profile',
  'full effects, ledgers, grant fences, capacity and physical/job/command fit remain unqualified'],'qualification boundary');
 const blockers=[{id:'a-disposition',kind:'launch-interrupted',task,detail:'Supplied late disposition interruption of the proven original spawn.'},
  {id:'b-approval',kind:'recovery-approval-required',task,detail:'Independent approval gate remains unresolved.'}];
 const worktree=prefix.chain.allocation.allocatedWorktree;
 const observed=[{kind:'private-git',path:worktree.canonicalPrivateGitDirectory,object:worktree.privateGitDirectoryObject},{kind:'root',path:worktree.canonicalRoot,object:worktree.rootObject}];
 const launch={schemaVersion:1,task,reservation:a.reservation,admission:{approval:a.approval,parentRuntime:parent,
  expectedTaskVersion:prefix.chain.reservation.expectedTaskVersion,assignmentTaskVersion:a.assignmentIdentity.taskVersion},
  phase:'interrupted',allocation:a.allocation,attachmentId:a.attachmentId,spawnRuntime:fields(child,'sidecarGeneration launchGeneration'),observedDirectories:observed,interruptedAt:'spawn',reasonCode:'disposition-unknown'};
 const turnBody={schemaVersion:1,task,reservation:a.reservation,childSessionId:a.assignmentIdentity.childSessionId,expectedTaskVersion:7,acceptedTaskVersion:8,
  evidence:{kind:'ui_action',runtime:child,reviewContextId:hash('UNQUALIFIED positive turn context'),sidecarGeneration:child.sidecarGeneration}};
 const t=p.turn;exact(t,'table key value');same(t.table,'research_turn_completions');same(t.key,task);
 exact(t.value,'schemaVersion body lastCommit digest');assert.equal(t.value.schemaVersion,1);same(t.value.body,turnBody,'original UI turn, not native proof');
 assert.equal(t.value.digest,digest('orkworks.research-source-turn-completion.v1',turnBody));same(t.value.lastCommit,origin(3));assert(size(t)<=65536);
 exact(p.interruption,'run task launch');
 snapshot(p.interruption.run,'research_run_controls',runKey,run(12,blockers),4);
 snapshot(p.interruption.task,'research_task_controls',task,taskBody(9,'needs_parent_result'),4);
 snapshot(p.interruption.launch,'research_launch_controls',task,launch,4);
 const q=p.coordination;exact(q,'table key value');same(q.table,'research_coordination_receipts');same(q.key,task);exact(q.value,'schemaVersion request requestDigest receipt lastCommit');assert.equal(q.value.schemaVersion,1);
 const request={schemaVersion:1,task,reservation:a.reservation,childSessionId:a.assignmentIdentity.childSessionId,expectedTaskVersion:9,outcome:'completed',summary:'Explicit synthetic parent outcome after the disposition interruption.'};
 same(q.value.request,request,'original explicit completed request');const rd=digest('orkworks.research-source-coordination-request.v1',request);assert.equal(q.value.requestDigest,rd);
 same(q.value.receipt,{schemaVersion:1,task,reservation:a.reservation,childSessionId:request.childSessionId,acceptedTaskVersion:10,commit:origin(5)});same(q.value.lastCommit,origin(5));assert(size(q)<=16384);
 exact(p.terminal,'session task run');
 const session={schemaVersion:1,task,reservation:a.reservation,assignmentIdentity:a.assignmentIdentity,attachmentId:a.attachmentId,sessionId:request.childSessionId,producerRevision:'4',state:'terminal',runtime:child};
 snapshot(p.terminal.session,'research_session_states',task,session,6);
 const result={requestDigest:rd};snapshot(p.terminal.task,'research_task_controls',task,taskBody(11,'reported_complete',result),6);
 snapshot(p.terminal.run,'research_run_controls',runKey,run(14,blockers),6);
 const b=p.recovery.value.body;
 exact(b,'schemaVersion recoveryId task reservation expectedRunVersion expectedTaskVersion reviewContextId sidecarGeneration kind allocation attachmentId readSet observedDirectories blockers reason');
 same(b,{schemaVersion:1,recoveryId:'late-disposition-terminal',task,reservation:a.reservation,expectedRunVersion:14,expectedTaskVersion:11,
  reviewContextId:hash('UNQUALIFIED positive recovery context'),sidecarGeneration:parent.sidecarGeneration,kind:'observe-terminal',allocation:a.allocation,attachmentId:a.attachmentId,
  readSet:{expectedCommit:origin(7),run:p.terminal.run,task:p.terminal.task,launch:p.interruption.launch,session:p.terminal.session},observedDirectories:observed,blockers,
  reason:'Settle the retained original disposition only; no launch or result rewrite.'},'exact captured original recovery');
 record(p.recovery,'research_recovery_decisions',{task,recoveryId:b.recoveryId},'orkworks.research-source-recovery-decision.v1',
  {schemaVersion:1,recoveryId:b.recoveryId,decisionDigest:p.recovery.value.digest,runVersion:15,taskVersion:12,commit:origin(8)});
 const lb=p.lifecycle.value.body,selector={schemaVersion:1,run:runKey,summaryVersion:0,completedResearchPlan:null,currentSummary:null,pendingStageContinue:null,finishDisposition:null};
 const planBody={schemaVersion:1,plan,planVersion:2,status:'active',latestApproval:a.approval,task,replaces:null};
 exact(lb,'schemaVersion key request readSet');assert.equal(lb.schemaVersion,1);same(lb.key,{kind:'plan-completion',plan});
 same(lb.request,{schemaVersion:1,kind:'plan-completion',run:runKey,expectedRunVersion:14,input,plan,expectedPlanVersion:2,
  trigger:{kind:'recovery',task,recoveryId:b.recoveryId,decisionDigest:p.recovery.value.digest}},'original recovery pins completion');
 exact(lb.readSet,'expectedCommit run lifecycle currentPlan pendingPlan');same(lb.readSet.expectedCommit,b.readSet.expectedCommit);same(lb.readSet.run,b.readSet.run,'same original pre-trigger run');
 assert.equal(lb.readSet.pendingPlan,null);snapshot(lb.readSet.lifecycle,'research_lifecycle_state',runKey,selector,1);snapshot(lb.readSet.currentPlan,'research_plan_controls',plan,planBody,2);
 record(p.lifecycle,'research_lifecycle_records',lb.key,'orkworks.research-lifecycle-decision.v1',
  {schemaVersion:1,key:lb.key,decisionDigest:p.lifecycle.value.digest,runVersion:15,planVersion:3,commit:origin(8)});
 const bindings=p.bindings;exact(bindings,'schemaVersion expectedCommit bindings');assert.equal(bindings.schemaVersion,1);same(bindings.expectedCommit,b.readSet.expectedCommit);
 assert(bindings.bindings.length>=1&&bindings.bindings.length<=16,'bounded complete bindings');
 const covered=b.blockers.filter(x=>x.kind==='launch-interrupted');
 same(bindings.bindings.map(x=>x.blocker),covered,'exact sorted complete captured launch blockers');
 for(const x of bindings.bindings){
  exact(x,'blocker reservation interruptionRun interruptionTask interruptionLaunch disposition');same(x.reservation,b.reservation);
  same(x.interruptionRun,p.interruption.run);same(x.interruptionTask,p.interruption.task);same(x.interruptionLaunch,p.interruption.launch);
  assert.equal(x.disposition,'supplied-resolved','each stipulated original native settlement required');
 }
 exact(p.after,'run task launch session plan selector');
 snapshot(p.after.run,'research_run_controls',runKey,{...run(15,[blockers[1]]),currentPlan:null},8);
 snapshot(p.after.task,'research_task_controls',task,taskBody(12,'reported_complete',result),8);
 snapshot(p.after.launch,'research_launch_controls',task,{...launch,phase:'ended',interruptedAt:null,reasonCode:null},8);
 same(p.after.session,p.terminal.session,'recovery never rewrites ordinary bridge');
 snapshot(p.after.plan,'research_plan_controls',plan,{...planBody,planVersion:3,status:'complete'},8);
 snapshot(p.after.selector,'research_lifecycle_state',runKey,{...selector,completedResearchPlan:plan},8);
 const measurements=[p.recovery,p.lifecycle].map(measureRecoveryCompletion);same(p.measurements,measurements,'independent UTF-8 occurrence projection');
 // A necessary lower-bound check only; omitted owners/effects mean this is not
 // a full paired-job packing witness or a reconstructed resource ledger.
 checkCompoundWidths([p.recovery,p.lifecycle]);
 return measurements;
}
function authorized(f){
 assert.equal(f.authority,'supplied-current-Main','current authority before lookup');assert.equal(f.rate,'supplied-allowed','current rate before lookup');
 assert.equal(f.currentness,'supplied-qualified','current scope/owner before lookup');assert.equal(f.sourceProofs,'supplied-qualified','original complete proofs');
 assert.equal(f.membership,'supplied-qualified','complete original membership');assert.equal(f.disposition,'supplied-committed','known original paired disposition');
 assert.equal(f.context,'supplied-exact','exact shown original recovery context before lookup');
}
export function checkPositiveSettlement(p,facts){
 authorized(facts);validateRecoveryCompletion(p);
 for(const k of ['originalSpawn','nativeSettlement','planEligibility','grantFencing','capacity'])assert.equal(facts[k],'supplied-qualified','supplied '+k+' required');
 assert.equal(facts.runState,'blocked','selected nonterminal prestate required');
 return {kind:'eligible',removed:p.bindings.bindings.map(x=>x.blocker.id),runVersion:15,taskVersion:12,planVersion:3};
}
export function checkPairedRetry(p,body,facts){
 authorized(facts);assert(p?.recovery&&p?.lifecycle,'original companion required');
 same(body,p.recovery.value.body,'original request conflict');validateRecoveryCompletion(p);
 return {recovery:clone(p.recovery.value.receipt),completion:clone(p.lifecycle.value.receipt)};
}
