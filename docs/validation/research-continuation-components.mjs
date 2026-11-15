// Finite decoded documentation examples. No publisher, wire codec or authority.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
const raw=readFileSync(new URL('research-prefix-components.json',import.meta.url)),prefix=JSON.parse(raw);
const same=(a,b,label)=>assert.deepEqual(a,b,label);
const bytes=x=>Buffer.byteLength(canonical(x));
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const origin=(label,n)=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED continuation label: '+label)});
const assumptions=['alternative examples, not one store or legal history',
  'fixture-tool is not Codex Stop capability evidence; actual authenticated handler supplied',
  'current parent/UI authority, rate, report/review eligibility and original native facts supplied',
  'complete memberships, currentness, symbolic C/commands/proofs and actual outcomes unqualified',
  'resource balances and full command/job/physical fit unqualified'];
const a=prefix.chain.assignment,P=a.assignmentIdentity,t=prefix.chain.startup.scope.task,run=prefix.chain.startup.context.run;
const [parent,child]=prefix.runtimeHistory.map(r=>({...r.key,attachmentDigest:r.value.digest}));
function snapshot(table,key,body,label,n=1){return {table,key,value:{schemaVersion:1,body,lastCommit:origin(label,n)}};}
const taskBody=(version,status,result=null)=>({schemaVersion:1,task:t,taskVersion:version,status,reservation:a.reservation,coordinationResult:result});
const taskRow=(version,status,label,n=1,result=null)=>snapshot('research_task_controls',t,taskBody(version,status,result),label,n);
function sessionRow(label,state){
  return snapshot('research_session_states',t,{schemaVersion:1,task:t,reservation:a.reservation,assignmentIdentity:P,
    attachmentId:a.attachmentId,sessionId:P.childSessionId,producerRevision:state==='prepared'?'1':'3',state,runtime:state==='prepared'?null:child},label);
}
const worktree=prefix.chain.allocation.allocatedWorktree;
const observed=[{kind:'private-git',path:worktree.canonicalPrivateGitDirectory,object:worktree.privateGitDirectoryObject},
  {kind:'root',path:worktree.canonicalRoot,object:worktree.rootObject}];
const caps={research_turn_completions:65536,research_coordination_receipts:16384,research_recovery_decisions:2097152};
function snapshotWidth(r,max){assert(bytes({key:r.key,value:r.value})<=max,'snapshot wrapper limit');}
export function validateContinuationWidths(r,jobBytes){
  if(r.table==='research_recovery_decisions'){
    const s=r.value.body.readSet;snapshotWidth(s.run,1048576);snapshotWidth(s.task,16384);snapshotWidth(s.launch,32768);
    if(s.session!==null)snapshotWidth(s.session,16384);
  }
  if(r.table==='research_turn_completions'&&r.value.body.evidence.kind==='child_terminal_without_receipt')snapshotWidth(r.value.body.evidence.session,16384);
  assert(Object.hasOwn(caps,r.table),'supported component family');
  assert(bytes({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
  if(r.value.receipt)assert(bytes(r.value.receipt)<=4096,'receipt limit');
  assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');
}
// Pure supplied-fact lookup oracle, not a credential/rate validator or handler.
export function checkCoordinationRetry(r,request,facts){
  assert.equal(facts.authority,'supplied-current','current authority before lookup');
  assert.equal(facts.rate,'supplied-allowed','rate before lookup');
  same(request,r.value.request,'original request conflict');
  return structuredClone(r.value.receipt); // liveTaskVersion deliberately not CASed on exact retry.
}
// Keys are supplied membership components, not a complete inventory proof.
export function checkRecoveryInventory(keys,next){
  const encoded=keys.map(canonical);assert.equal(new Set(encoded).size,encoded.length,'duplicate recovery key');
  assert(!encoded.includes(canonical(next)),'existing identity needs original retry binding');
  assert(keys.length<32,'32 per run recovery limit');
}
function measure(r){
  const value=structuredClone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
  return {chargeBytes:bytes({key:r.key,value}),wrapperBytes:bytes({key:r.key,value:r.value})};
}
function checkValue(r,table,key,fields){
  exact(r,'table key value');assert.equal(r.table,table,'fixed research family');same(r.key,key,'complete original key');
  exact(r.value,fields);assert.equal(r.value.schemaVersion,1);
}
function turnExpected(kind){
  const label='turn-'+kind;
  let evidence=kind==='child_terminal_without_receipt'?{kind,session:sessionRow(label+'-terminal','terminal')}:{kind,runtime:child};
  if(kind==='ui_action')evidence={...evidence,reviewContextId:hash('Unqualified shown original turn context'),sidecarGeneration:child.sidecarGeneration};
  return {schemaVersion:1,task:t,reservation:a.reservation,childSessionId:P.childSessionId,expectedTaskVersion:7,acceptedTaskVersion:8,evidence};
}
function checkTurnExample(x,kind){
  exact(x,'id gate taskBefore taskAfter runVersions');assert.equal(x.id,kind);
  const r=x.gate,label='turn-'+kind;
  checkValue(r,'research_turn_completions',t,'schemaVersion body digest lastCommit');
  same(r.value.body,turnExpected(kind),'complete original turn body/evidence');
  assert.equal(r.value.digest,domain('orkworks.research-source-turn-completion.v1',r.value.body),'research turn source digest');
  same(r.value.lastCommit,origin(label,3),'original turn origin label');
  same(x.taskBefore,taskRow(7,'running',label+'-before'),'running source task');
  same(x.taskAfter,taskRow(8,'needs_parent_result',label,3),'readiness is no outcome');
  same(x.runVersions,{before:10,after:11},'coalesced turn run version sample');
  validateContinuationWidths(r,bytes(r));
}
function recoveryExpected(kind){
  const label='recovery-'+kind,attached=['confirm-original-attachment','observe-terminal'].includes(kind),terminal=kind==='observe-terminal';
  const version=terminal?8:5;
  const blockers=[{id:'a-disposition',kind:'launch-interrupted',task:t,detail:'Synthetic original disposition needs proof.'},
    {id:'b-approval',kind:'recovery-approval-required',task:t,detail:'Retry still needs a separately approved task/revision.'}];
  // Each field is constructed from original sources, never a later control view.
  const runBody={schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',
    ...pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
    runVersion:20,state:'blocked',currentInput:pick(prefix.inputs[0].value.body,'workspaceId runId preparationRevision inputRevision inputDigest'),
    planReferences:[a.plan],currentPlan:a.plan,pendingPlan:null,parentRuntime:parent,blockers,stageContinueRequired:false,
    pendingClarification:null,pauseReason:null,resumeTarget:null};
  const launch={schemaVersion:1,task:t,reservation:a.reservation,admission:{approval:a.approval,parentRuntime:parent,
    expectedTaskVersion:prefix.chain.reservation.expectedTaskVersion,assignmentTaskVersion:P.taskVersion},
    phase:terminal?'ended':'interrupted',allocation:attached?a.allocation:null,attachmentId:attached?a.attachmentId:null,
    spawnRuntime:terminal?pick(child,'sidecarGeneration launchGeneration'):null,observedDirectories:attached?observed:[],
    interruptedAt:terminal?null:attached?'spawn':'allocation',reasonCode:terminal?null:'disposition-unknown'};
  const readSet={expectedCommit:origin(label+'-H',2),
    run:snapshot('research_run_controls',pick(run,'workspaceId runId'),runBody,label+'-run'),
    task:taskRow(version,terminal?'needs_parent_result':'launch_interrupted',label+'-task'),
    launch:snapshot('research_launch_controls',t,launch,label+'-launch'),
    session:attached?sessionRow(label+'-session',terminal?'terminal':'prepared'):null};
  return {schemaVersion:1,recoveryId:label,task:t,reservation:a.reservation,expectedRunVersion:20,expectedTaskVersion:version,
    reviewContextId:hash('Unqualified shown recovery: '+kind),sidecarGeneration:parent.sidecarGeneration,kind,
    allocation:kind==='settle-no-child'?null:a.allocation,attachmentId:attached?a.attachmentId:null,readSet,
    observedDirectories:kind==='settle-no-child'?[]:observed,blockers,reason:'Synthetic original-attempt decision; no actual native proof.'};
}
export function validateResearchContinuation(p){
  exact(p,'inputSha256 assumptions turns coordinations recoveries policies measurements');
  assert.equal(p.inputSha256,hash(raw),'unchanged original documentation input');validateResearchPrefix(prefix);
  same(p.assumptions,assumptions,'explicit unqualified source boundary');
  assert(Array.isArray(p.turns)&&p.turns.length===3,'three alternative turns');
  ['codex_stop_receipt','ui_action','child_terminal_without_receipt'].forEach((kind,n)=>checkTurnExample(p.turns[n],kind));
  assert(Array.isArray(p.coordinations)&&p.coordinations.length===3,'three alternative outcomes');
  for(const [n,outcome] of ['completed','blocked','failed'].entries()){
    const x=p.coordinations[n],label='coordination-'+outcome;exact(x,'id gate row taskBefore taskAfter runVersions');assert.equal(x.id,outcome);
    same(x.gate,p.turns[1].gate,'original UI turn gate before outcome');
    const r=x.row;checkValue(r,'research_coordination_receipts',t,'schemaVersion request requestDigest receipt lastCommit');
    const request={schemaVersion:1,task:t,reservation:a.reservation,childSessionId:P.childSessionId,expectedTaskVersion:8,outcome,summary:'Synthetic parent outcome — not quality or native proof.'};
    same(r.value.request,request,'original parent coordination request');
    const d=domain('orkworks.research-source-coordination-request.v1',request),C=origin(label,4);
    assert.equal(r.value.requestDigest,d,'research coordination request digest');
    same(r.value.receipt,{schemaVersion:1,task:t,reservation:a.reservation,childSessionId:P.childSessionId,acceptedTaskVersion:9,commit:C},'original coordination receipt');
    same(r.value.lastCommit,C,'original coordination origin');
    same(x.taskBefore,p.turns[1].taskAfter,'outcome needs original readiness');
    same(x.taskAfter,taskRow(9,({completed:'reported_complete',blocked:'reported_blocked',failed:'reported_failed'})[outcome],label,4,{requestDigest:d}),'original task outcome digest/version');
    same(x.runVersions,{before:11,after:12},'coalesced result run version sample');
    validateContinuationWidths(r,bytes(r));
  }
  assert(Array.isArray(p.recoveries)&&p.recoveries.length===4,'four alternative original recovery kinds');
  const kinds=['settle-no-child','confirm-original-allocation','confirm-original-attachment','observe-terminal'];
  const keys=[];
  for(const [n,kind] of kinds.entries()){
    const x=p.recoveries[n];exact(x,'id row');assert.equal(x.id,kind);
    const r=x.row,b=r.value.body,expected=recoveryExpected(kind),key={task:t,recoveryId:'recovery-'+kind};
    checkValue(r,'research_recovery_decisions',key,'schemaVersion body digest receipt lastCommit');
    // These finite semantic comparisons include all original snapshot key/body/
    // origin, expected-version, frozen assignment, blocker and kind/null joins.
    same(b,expected,'original recovery body/snapshots/blockers');
    const d=domain('orkworks.research-source-recovery-decision.v1',b),C=origin('recovery-'+kind,3);
    assert.equal(r.value.digest,d,'research recovery decision digest');
    same(r.value.receipt,{schemaVersion:1,recoveryId:b.recoveryId,decisionDigest:d,runVersion:21,
      taskVersion:kind==='observe-terminal'?8:6,commit:C},'original recovery receipt/version sample');
    same(r.value.lastCommit,C,'original recovery origin');
    checkRecoveryInventory(keys,key);keys.push(key);validateContinuationWidths(r,bytes(r));
  }
  const policy=[['late-Stop-after-UI','original-UI-gate/no-write'],['exact-parent-retry','original-receipt/no-write'],
    ['changed-parent-retry','conflict'],['revoked-parent-retry','deny-before-lookup'],['unknown-recovery-outcome','retain/no-retry'],
    ['new-recovery-at-32','deny'],['completed-task-live-child','capacity-held'],['recovery-with-stage-gate','stage-gate-preserved']];
  same(p.policies,policy.map(([id,expected])=>({id,expected})),'policy labels, not executed lifecycle schedules');
  const measured={turns:p.turns.map(x=>measure(x.gate)),coordinations:p.coordinations.map(x=>measure(x.row)),recoveries:p.recoveries.map(x=>measure(x.row))};
  same(p.measurements,measured,'independent key plus committed projection measurements');
  return measured;
}

// Existing exact recovery retries bypass new-decision count and live CAS.
export function checkRecoveryRetry(r,body,facts){
  assert.equal(facts.authority,'supplied-current-UI','current UI authority before lookup');
  same(body,r.value.body,'original recovery decision conflict');
  return structuredClone(r.value.receipt);
}
