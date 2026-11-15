// Finite decoded documentation examples and stipulated-fact checks only.
// No wire decoder, runtime reducer, publisher, credential or native validator.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateResearchCompletion,validateCompletionWidths,measureCompletionRow} from './research-completion-components.mjs';
import {validateContinuationWidths} from './research-continuation-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const priorRaw=readFileSync(new URL('research-completion-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw),prior=JSON.parse(priorRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),domain=(n,b)=>hash(n+'\n'+canonical(b));
const pick=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const a=prefix.chain.assignment,task=prefix.chain.startup.scope.task,plan=a.plan,sourceRun=prefix.chain.startup.context.run;
const key=pick(sourceRun,'workspaceId runId'),input=pick(prefix.inputs[1].value.body,'workspaceId runId preparationRevision inputRevision inputDigest');
const [parent,child]=prefix.runtimeHistory.map(x=>({...x.key,attachmentDigest:x.value.digest}));
const w=prefix.chain.allocation.allocatedWorktree;
const directories=[{kind:'private-git',path:w.canonicalPrivateGitDirectory,object:w.privateGitDirectoryObject},{kind:'root',path:w.canonicalRoot,object:w.rootObject}];
const origin=(label,n)=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED positive recovery label: '+label)});
function row(table,key,body,C){return {table,key:clone(key),value:{schemaVersion:1,body:clone(body),lastCommit:clone(C)}};}
function expectedCase(name){
 const retained=name==='independent-approval-retained',T=origin(name+'-turn',3),I=origin(name+'-interruption',4),Q=origin(name+'-coordination',5),O=origin(name+'-ordinary-terminal',6),H=origin(name+'-H',7),C=origin(name+'-compound',8);
 const blockers=[{id:'a-disposition',kind:'launch-interrupted',task,detail:'Supplied original late disposition uncertainty.'}];
 if(retained)blockers.push({id:'b-disposition',kind:'launch-interrupted',task,detail:'Supplied second original disposition predicate.'},{id:'c-approval',kind:'recovery-approval-required',task,detail:'Independent approval blocker must remain.'});
 const run=(version,state,changes={})=>({schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',...pick(sourceRun,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
  runVersion:version,state,currentInput:input,planReferences:[plan],currentPlan:plan,pendingPlan:null,parentRuntime:parent,blockers:clone(blockers),stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null,...changes});
 const taskBody=(version,status,result=null)=>({schemaVersion:1,task,taskVersion:version,status,reservation:a.reservation,coordinationResult:result});
 const launch={schemaVersion:1,task,reservation:a.reservation,admission:{approval:a.approval,parentRuntime:parent,expectedTaskVersion:prefix.chain.reservation.expectedTaskVersion,assignmentTaskVersion:a.assignmentIdentity.taskVersion},
  phase:'interrupted',allocation:a.allocation,attachmentId:a.attachmentId,spawnRuntime:pick(child,'sidecarGeneration launchGeneration'),observedDirectories:directories,interruptedAt:'spawn',reasonCode:'disposition-unknown'};
 const interruption={run:row('research_run_controls',key,run(12,'blocked'),I),task:row('research_task_controls',task,taskBody(9,'needs_parent_result'),I),launch:row('research_launch_controls',task,launch,I)};
 const turnBody={schemaVersion:1,task,reservation:a.reservation,childSessionId:a.assignmentIdentity.childSessionId,expectedTaskVersion:7,acceptedTaskVersion:8,
  evidence:{kind:'ui_action',runtime:child,reviewContextId:hash('Unqualified original turn: '+name),sidecarGeneration:child.sidecarGeneration}};
 const turn=row('research_turn_completions',task,turnBody,T);turn.value.digest=domain('orkworks.research-source-turn-completion.v1',turnBody);
 const request={schemaVersion:1,task,reservation:a.reservation,childSessionId:a.assignmentIdentity.childSessionId,expectedTaskVersion:9,outcome:'completed',summary:'Explicit hypothetical parent outcome; no native or quality proof.'};
 const D=domain('orkworks.research-source-coordination-request.v1',request);
 const coordination={table:'research_coordination_receipts',key:task,value:{schemaVersion:1,request,requestDigest:D,receipt:{schemaVersion:1,task,reservation:a.reservation,childSessionId:a.assignmentIdentity.childSessionId,acceptedTaskVersion:10,commit:Q},lastCommit:Q}};
 const currentRun=row('research_run_controls',key,run(14,'blocked'),O),currentTask=row('research_task_controls',task,taskBody(11,'reported_complete',{requestDigest:D}),O);
 const session=row('research_session_states',task,{schemaVersion:1,task,reservation:a.reservation,assignmentIdentity:a.assignmentIdentity,attachmentId:a.attachmentId,sessionId:a.assignmentIdentity.childSessionId,producerRevision:'4',state:'terminal',runtime:child},O);
 const state={schemaVersion:1,run:key,summaryVersion:0,completedResearchPlan:null,currentSummary:null,pendingStageContinue:null,finishDisposition:null};
 const selector=row('research_lifecycle_state',key,state,origin(name+'-selector',1));
 const planBody={schemaVersion:1,plan,planVersion:2,status:'active',latestApproval:a.approval,task,replaces:null};
 const planBefore=row('research_plan_controls',plan,planBody,origin(name+'-plan',1)),recoveryId='positive-'+name;
 const body={schemaVersion:1,recoveryId,task,reservation:a.reservation,expectedRunVersion:14,expectedTaskVersion:11,reviewContextId:hash('Unqualified shown settlement: '+name),sidecarGeneration:parent.sidecarGeneration,
  kind:'observe-terminal',allocation:a.allocation,attachmentId:a.attachmentId,readSet:{expectedCommit:H,run:currentRun,task:currentTask,launch:interruption.launch,session},observedDirectories:directories,blockers:clone(blockers),reason:'Supplied proven original ownership and current terminal/native settlement.'};
 const RD=domain('orkworks.research-source-recovery-decision.v1',body);
 const recovery={table:'research_recovery_decisions',key:{task,recoveryId},value:{schemaVersion:1,body,digest:RD,receipt:{schemaVersion:1,recoveryId,decisionDigest:RD,runVersion:15,taskVersion:12,commit:C},lastCommit:C}};
 const lifecycleKey={kind:'plan-completion',plan};
 const lifecycleRequest={schemaVersion:1,kind:'plan-completion',run:key,expectedRunVersion:14,input,plan,expectedPlanVersion:2,trigger:{kind:'recovery',task,recoveryId,decisionDigest:RD}};
 const lifecycleBody={schemaVersion:1,key:lifecycleKey,request:lifecycleRequest,readSet:{expectedCommit:H,run:clone(currentRun),lifecycle:selector,currentPlan:planBefore,pendingPlan:null}};
 const LD=domain('orkworks.research-lifecycle-decision.v1',lifecycleBody);
 const completion={table:'research_lifecycle_records',key:lifecycleKey,value:{schemaVersion:1,body:lifecycleBody,digest:LD,receipt:{schemaVersion:1,key:lifecycleKey,decisionDigest:LD,runVersion:15,planVersion:3,commit:C},lastCommit:C}};
 const bindings={schemaVersion:1,expectedCommit:H,bindings:blockers.filter(b=>b.kind==='launch-interrupted').map(b=>({blocker:clone(b),reservation:a.reservation,interruptionRun:clone(interruption.run),interruptionTask:clone(interruption.task),interruptionLaunch:clone(interruption.launch),disposition:'supplied-resolved'}))};
 const remaining=blockers.filter(b=>b.kind!=='launch-interrupted');
 const after={run:row('research_run_controls',key,run(15,retained?'blocked':'synthesizing',{currentPlan:null,blockers:remaining}),C),
  task:row('research_task_controls',task,taskBody(12,'reported_complete',{requestDigest:D}),C),launch:row('research_launch_controls',task,{...launch,phase:'ended',interruptedAt:null,reasonCode:null},C),
  plan:row('research_plan_controls',plan,{...planBody,planVersion:3,status:'complete'},C),lifecycle:row('research_lifecycle_state',key,{...state,completedResearchPlan:plan},C),sessionRetained:clone(session)};
 const sources={turn,interruption,coordination,coordinationTask:row('research_task_controls',task,taskBody(10,'reported_complete',{requestDigest:D}),Q),coordinationRun:row('research_run_controls',key,run(13,'blocked'),Q)};
 return {id:name,sources,recovery,completion,bindings,after};
}
export function validateResearchRecoveryCompletion(p){
 exact(p,'prefixSha256 priorCompletionSha256 assumptions negative cases measurements');
 assert.equal(p.prefixSha256,hash(prefixRaw));assert.equal(p.priorCompletionSha256,hash(priorRaw));validateResearchPrefix(prefix);validateResearchCompletion(prior);
 same(p.assumptions,[
  'independent decoded components, not a store, legal history or publisher',
  'original spawn/turn/result and late interruption occurrence are stipulated source facts',
  'Main authority/context, native ownership/quiescence, terminal bridge and complete membership/proofs supplied, not qualified',
  'symbolic H/C, actual atomic effects, capacity, ledger balances and complete physical/job fit unqualified'],'stipulated source boundary');
 same(p.negative,prior.completions[1],'unchanged negative recovery');assert.equal(p.cases.length,2);
 const measurements=[];
 for(const [n,name] of ['disposition-only','independent-approval-retained'].entries()){
  const x=p.cases[n];same(x,expectedCase(name),'complete independent source-derived alternative');
  same(checkLateDispositionBindings(x.recovery,x.bindings,x.sources),x.after.run.value.body.blockers,'only source-bound blockers clear');
  checkPairedRecoveryRetry(x.recovery,x.completion,x.recovery.value.body,{authority:'supplied-current-Main',context:'supplied-exact'});
  const suppliedComponentBytes=size({recovery:x.recovery,completion:x.completion,bindings:x.bindings});validatePairedRecoveryWidths(x.recovery,x.completion,x.bindings,suppliedComponentBytes);
  measurements.push({id:name,recovery:measureCompletionRow(x.recovery),completion:measureCompletionRow(x.completion),bindingsBytes:size(x.bindings),suppliedComponentBytes});
 }
 same(p.measurements,measurements,'independent occurrence and supplied-component widths');return measurements;
}
// No source discovery or native proof is performed. These are exact stipulated
// relation checks for already-decoded inputs, not a general admission API.
export function checkLateDispositionBindings(r,b,sources){
 exact(b,'schemaVersion expectedCommit bindings');assert.equal(b.schemaVersion,1);
 const d=r.value.body,rs=d.readSet;same(b.expectedCommit,rs.expectedCommit,'original binding H');
 for(const [name,family,valueFields] of [['turn','research_turn_completions','schemaVersion body digest lastCommit'],['coordination','research_coordination_receipts','schemaVersion request requestDigest receipt lastCommit']]){
  const source=sources[name];exact(source,'table key value');assert.equal(source.table,family,'fixed original source family');same(source.key,d.task,'original source task key');
  exact(source.value,valueFields);assert.equal(source.value.schemaVersion,1,'original source schema');
 }
 const turnBody=sources.turn.value.body,coord=sources.coordination.value;
 exact(turnBody,'schemaVersion task reservation childSessionId expectedTaskVersion acceptedTaskVersion evidence');
 exact(coord.request,'schemaVersion task reservation childSessionId expectedTaskVersion outcome summary');
 for(const body of [turnBody,coord.request]){
  assert.equal(body.schemaVersion,1,'original source body schema');same(body.task,d.task,'original source task');same(body.reservation,d.reservation,'original source reservation');
  assert.equal(body.childSessionId,rs.session.value.body.sessionId,'original source child');
 }
 assert.equal(sources.turn.value.digest,domain('orkworks.research-source-turn-completion.v1',turnBody),'original turn source digest');
 assert.equal(coord.requestDigest,domain('orkworks.research-source-coordination-request.v1',coord.request),'original coordination source digest');
 exact(coord.receipt,'schemaVersion task reservation childSessionId acceptedTaskVersion commit');assert.equal(coord.receipt.schemaVersion,1);
 same(coord.receipt.task,d.task,'original coordination receipt task');same(coord.receipt.reservation,d.reservation,'original coordination receipt reservation');
 assert.equal(coord.receipt.childSessionId,coord.request.childSessionId,'original coordination receipt child');
 same(rs.task.value.body.coordinationResult,{requestDigest:coord.requestDigest},'current task retains original coordination result');
 const ids=rs.run.value.body.blockers.map(x=>x.id);
 for(let n=1;n<ids.length;n++)assert(Buffer.compare(Buffer.from(ids[n-1]),Buffer.from(ids[n]))<0,'unique canonical blocker order');
 const captured=rs.run.value.body.blockers.filter(x=>canonical(x.task)===canonical(d.task));same(d.blockers,captured,'complete captured task blocker set');
 const target=captured.filter(x=>x.kind==='launch-interrupted');
 assert(b.bindings.length>=1&&b.bindings.length<=16,'1..16 binding limit');
 same(b.bindings.map(x=>x.blocker),target,'exact ordered complete blocker coverage');
 assert.equal(new Set(b.bindings.map(x=>x.blocker.id)).size,b.bindings.length,'unique blocker binding');
 for(const x of b.bindings){
  exact(x,'blocker reservation interruptionRun interruptionTask interruptionLaunch disposition');
  same(x.reservation,d.reservation,'original binding reservation');assert.equal(x.disposition,'supplied-resolved','unresolved/unknown supplied predicate');
  for(const [field,source,table,k] of [['interruptionRun','run','research_run_controls',pick(d.task,'workspaceId runId')],['interruptionTask','task','research_task_controls',d.task],['interruptionLaunch','launch','research_launch_controls',d.task]]){
   const read=x[field];exact(read,'table key value');assert.equal(read.table,table,'fixed research snapshot family');same(read.key,k,'complete interruption key');
   exact(read.value,'schemaVersion body lastCommit');assert.equal(read.value.schemaVersion,1);same(read,sources.interruption[source],'original typed interruption snapshot');
  }
  const I=x.interruptionLaunch.value.lastCommit,T=sources.turn.value.lastCommit,Q=sources.coordination.value.lastCommit,H=rs.expectedCommit;
  same(x.interruptionRun.value.lastCommit,I,'one original interruption origin');same(x.interruptionTask.value.lastCommit,I,'one original interruption origin');
  for(const c of [T,I,Q])assert.equal(c.storageEpoch,H.storageEpoch,'original source epoch');
  assert(BigInt(T.commitSequence)<BigInt(I.commitSequence)&&BigInt(I.commitSequence)<BigInt(Q.commitSequence)&&BigInt(Q.commitSequence)<=BigInt(H.commitSequence),'strict original turn/interruption/coordination/H order');
  assert(x.interruptionRun.value.body.blockers.some(y=>canonical(y)===canonical(x.blocker)),'original blocker body at interruption');
  const turn=sources.turn.value.body,coord=sources.coordination.value,iv=x.interruptionTask.value.body.taskVersion;
  assert.equal(turn.acceptedTaskVersion,turn.expectedTaskVersion+1,'original turn version advance');
  assert(turn.acceptedTaskVersion<iv&&iv<=coord.request.expectedTaskVersion,'original turn/interruption version order');
  assert.equal(coord.receipt.acceptedTaskVersion,coord.request.expectedTaskVersion+1,'original coordination version advance');
  assert(rs.task.value.body.taskVersion>=coord.receipt.acceptedTaskVersion,'current version preserves older original result');
  same(x.interruptionTask.value.body.reservation,d.reservation,'original interrupted task reservation');
  assert.equal(x.interruptionTask.value.body.status,'needs_parent_result','original readiness at interruption');assert.equal(x.interruptionTask.value.body.coordinationResult,null,'outcome not yet accepted');
  const launch=x.interruptionLaunch.value.body;assert.equal(launch.phase,'interrupted');assert.equal(launch.interruptedAt,'spawn');assert.equal(launch.reasonCode,'disposition-unknown');assert.notEqual(launch.spawnRuntime,null,'known original spawn');
  same(launch.reservation,d.reservation,'original launch reservation');same(launch.allocation,d.allocation,'original allocation');same(launch.attachmentId,d.attachmentId,'original attachment');
 }
 const removed=new Set(target.map(x=>x.id));return clone(rs.run.value.body.blockers.filter(x=>!removed.has(x.id)));
}
export function checkPairedRecoveryRetry(r,l,body,facts){
 assert.equal(facts.authority,'supplied-current-Main','current Main authority before original lookup');assert.equal(facts.context,'supplied-exact','exact current context before original lookup');
 same(body,r.value.body,'original recovery conflict');assert(l!==null,'missing original completion companion');
 const d=r.value.body,lb=l.value.body;
 assert.equal(r.table,'research_recovery_decisions','original recovery family');assert.equal(l.table,'research_lifecycle_records','original completion family');
 same(r.key,{task:d.task,recoveryId:d.recoveryId},'original recovery lookup key');
 const lifecycleKey={kind:'plan-completion',plan:lb.request.plan};same(l.key,lifecycleKey,'original completion lookup key');same(lb.key,lifecycleKey,'original completion body key');
 same(l.value.receipt.key,lifecycleKey,'original completion receipt key');assert.equal(r.value.receipt.recoveryId,d.recoveryId,'original recovery receipt key');
 assert.equal(lb.request.kind,'plan-completion');same(lb.request.run,pick(d.task,'workspaceId runId'),'original request run key');
 assert.equal(lb.request.expectedRunVersion,d.expectedRunVersion,'shared expected run version');
 assert.equal(d.readSet.run.value.body.runVersion,d.expectedRunVersion,'captured run version');
 assert.equal(d.readSet.task.value.body.taskVersion,d.expectedTaskVersion,'captured task version');
 const capturedPlan=lb.readSet.currentPlan;assert(capturedPlan!==null,'original completion requires captured plan');
 same(capturedPlan.key,lb.request.plan,'captured original plan key');same(capturedPlan.value.body.plan,lb.request.plan,'captured original plan body');
 same(lb.readSet.run.value.body.currentPlan,lb.request.plan,'original current plan pointer');
 same(pick(lb.request.plan,'workspaceId runId planId planRevision'),pick(d.task,'workspaceId runId planId planRevision'),'original task plan identity');
 assert.equal(capturedPlan.value.body.planVersion,lb.request.expectedPlanVersion,'captured plan version');
 same(lb.request.input,d.readSet.run.value.body.currentInput,'captured current input');
 assert.equal(r.value.digest,domain('orkworks.research-source-recovery-decision.v1',d),'original recovery digest');
 assert.equal(l.value.digest,domain('orkworks.research-lifecycle-decision.v1',lb),'original completion digest');
 same(lb.request.trigger,{kind:'recovery',task:d.task,recoveryId:d.recoveryId,decisionDigest:r.value.digest},'original recovery trigger');
 same(lb.readSet.expectedCommit,d.readSet.expectedCommit,'shared original H');same(lb.readSet.run,d.readSet.run,'shared original run read');
 const C=r.value.lastCommit;for(const c of [r.value.receipt.commit,l.value.lastCommit,l.value.receipt.commit])same(c,C,'same original compound C');
 assert.equal(r.value.receipt.decisionDigest,r.value.digest);assert.equal(l.value.receipt.decisionDigest,l.value.digest);
 assert.equal(r.value.receipt.runVersion,d.expectedRunVersion+1);assert.equal(l.value.receipt.runVersion,r.value.receipt.runVersion);
 assert.equal(r.value.receipt.taskVersion,d.expectedTaskVersion+1,'selected launch repair advances task once');assert.equal(l.value.receipt.planVersion,lb.request.expectedPlanVersion+1);
 return {recovery:clone(r.value.receipt),completion:clone(l.value.receipt)};
}
export function validatePairedRecoveryWidths(r,l,b,jobBytes){
 // Individual wrappers may fit while their combined job does not.
 validateContinuationWidths(r,1048576);validateCompletionWidths(l,1048576);
 assert(b.bindings.length>=1&&b.bindings.length<=16,'1..16 binding limit');
 for(const x of b.bindings)for(const [field,max] of [['interruptionRun',1048576],['interruptionTask',16384],['interruptionLaunch',32768]]){
  const s=x[field];assert(size({key:s.key,value:s.value})<=max,'binding snapshot wrapper limit');
 }
 const floor=size({recovery:r,completion:l,bindings:b});assert(floor<=jobBytes,'supplied local component floor exceeds job');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent combined job limit');
}
