// Decoded finite documentation samples and supplied-fact oracles only.
// No wire decoder, publisher, credential validator, runtime reducer or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateResearchContinuation} from './research-continuation-components.mjs';
const prefixRaw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const continuationRaw=readFileSync(new URL('research-continuation-components.json',import.meta.url));
const prefix=JSON.parse(prefixRaw),continuation=JSON.parse(continuationRaw);
const same=(a,b,label)=>assert.deepEqual(a,b,label),clone=structuredClone;
const size=x=>Buffer.byteLength(canonical(x)),digest=(n,x)=>hash(n+'\n'+canonical(x));
const fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const a=prefix.chain.assignment,task=prefix.chain.startup.scope.task,plan=a.plan,sourceRun=prefix.chain.startup.context.run;
const runKey=fields(sourceRun,'workspaceId runId');
const parent={...prefix.runtimeHistory[0].key,attachmentDigest:prefix.runtimeHistory[0].value.digest};
const input=fields(prefix.inputs[1].value.body,'workspaceId runId preparationRevision inputRevision inputDigest');
const report=prefix.inputs[1].value.body.records.find(x=>x.kind==='research-report').source;
const origin=(label,n=1)=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED completion label: '+label)});
function row(table,key,body,label,n=1){return {table,key:clone(key),value:{schemaVersion:1,body:clone(body),lastCommit:origin(label,n)}};}
function run(version,state,changes={}){return {schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',
 ...fields(sourceRun,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
 runVersion:version,state,currentInput:input,planReferences:[plan],currentPlan:plan,pendingPlan:null,parentRuntime:parent,
 blockers:[],stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null,...changes};}
function selector(changes={}){return {schemaVersion:1,run:runKey,summaryVersion:0,completedResearchPlan:null,currentSummary:null,pendingStageContinue:null,finishDisposition:null,...changes};}
function planRow(label,status='active',version=2,n=1){return row('research_plan_controls',plan,{schemaVersion:1,plan,planVersion:version,status,latestApproval:a.approval,task,replaces:null},label,n);}
function request(kind,version,changes={}){return {schemaVersion:1,kind,run:runKey,expectedRunVersion:version,input,
 ...(kind==='plan-completion'?{}:{decisionId:kind,reviewContextId:hash('Unqualified shown action: '+kind),sidecarGeneration:parent.sidecarGeneration}),...changes};}
function decision(label,req,rb,sb,currentPlan=null){
 const key=req.kind==='plan-completion'?{kind:'plan-completion',plan:req.plan}:{kind:'ui-decision',run:runKey,decisionId:req.decisionId};
 const readSet={expectedCommit:origin(label+'-H',2),run:row('research_run_controls',runKey,rb,label+'-run'),
  lifecycle:row('research_lifecycle_state',runKey,sb,label+'-selector'),currentPlan,pendingPlan:null};
 const body={schemaVersion:1,key,request:req,readSet},d=digest('orkworks.research-lifecycle-decision.v1',body),C=origin(label,3);
 const receipt={schemaVersion:1,key,decisionDigest:d,runVersion:rb.runVersion+1,
  planVersion:['plan-completion','cancel-stage'].includes(req.kind)?currentPlan.value.body.planVersion+1:null,commit:C};
 return {table:'research_lifecycle_records',key,value:{schemaVersion:1,body,digest:d,receipt,lastCommit:C}};
}
const caps={research_lifecycle_state:16384,research_summaries:16384,research_lifecycle_records:2097152};
export function measureCompletionRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function validateCompletionWidths(r,jobBytes){
 assert(Object.hasOwn(caps,r.table),'supported family');
 if(r.table==='research_lifecycle_records'){
  for(const [name,max] of [['run',1048576],['lifecycle',16384],['currentPlan',16384],['pendingPlan',16384]]){
   const s=r.value.body.readSet[name];if(s!==null)assert(size({key:s.key,value:s.value})<=max,'snapshot wrapper limit');
  }
  assert(size(r.value.receipt)<=4096,'receipt limit');
 }
 if(r.table==='research_summaries')assert(size(r.value.body)<=8192,'summary body limit');
 assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper limit');
 assert(Number.isSafeInteger(jobBytes)&&jobBytes>=0&&jobBytes<=1048576,'independent job limit');
}
export function validateResearchCompletion(p){
 exact(p,'inputSha256 continuationSha256 assumptions initial completions summaries ui invalidation effects measurements');
 assert.equal(p.inputSha256,hash(prefixRaw));assert.equal(p.continuationSha256,hash(continuationRaw));
 validateResearchPrefix(prefix);validateResearchContinuation(continuation);
 same(p.assumptions,[
  'separate decoded examples, not a store or legal event history',
  'current Main/parent authority, rate, runtime and source eligibility supplied, never authenticated here',
  'complete source/consumer/review/native memberships, original H/C/proofs and outcomes unqualified',
  'resource ledgers, actual effects, job/command/index/physical fit and nonblocking authority fences unqualified'],'qualification boundary');
 same(p.initial,row('research_lifecycle_state',runKey,selector(),'root-admission'),'original admission selector');
 assert.equal(p.completions.length,2);assert.equal(p.summaries.length,2);assert.equal(p.ui.length,6);
 const original=continuation.coordinations[0];
 const trigger={kind:'coordination',task,acceptedTaskVersion:9,requestDigest:original.row.value.requestDigest};
 const completed=decision('automatic-completion',request('plan-completion',11,{plan,expectedPlanVersion:2,trigger}),run(11,'researching'),selector(),planRow('completion-plan'));
 same(p.completions[0],{id:'coordination',source:original,row:completed,
  runAfter:row('research_run_controls',runKey,run(12,'synthesizing',{currentPlan:null}),'automatic-completion',3),
  planAfter:planRow('automatic-completion','complete',3,3),selectorAfter:row('research_lifecycle_state',runKey,selector({completedResearchPlan:plan}),'automatic-completion',3)},'automatic completion and coalesced versions');
 const recovery=continuation.recoveries[3].row;
 same(p.completions[1],{id:'recovery-not-complete',source:recovery,
  trigger:{kind:'recovery',task,recoveryId:recovery.key.recoveryId,decisionDigest:recovery.value.digest},candidateTask:recovery.value.body.readSet.task,row:null,expected:'deny-incomplete-task'},'recovery digest is not a completed task');
 const expectedSummaries=[];
 for(const version of [1,2]){
  const body={schemaVersion:1,run:runKey,summaryVersion:version,researchPlan:plan,input,producer:parent,
   findings:['Retained research source — offline example '+version+'.'],uncertainties:['Native and complete source proofs remain unqualified.'],
   assignmentChoices:[{description:'Keep research separate from execution.',reason:'No execution approval is supplied.'}],reportReferences:[report],reviewReferences:[]};
  const r=row('research_summaries',{...runKey,summaryVersion:version},body,'summary-'+version,3);r.value.digest=digest('orkworks.research-summary.v1',body);
  const ref={...r.key,summaryDigest:r.value.digest};
  expectedSummaries.push({id:'summary-'+version,publication:{schemaVersion:1,expectedRunVersion:30+version,summary:body},row:r,
   selectorAfter:row('research_lifecycle_state',runKey,selector({summaryVersion:version,completedResearchPlan:plan,currentSummary:ref}),'summary-'+version,3)});
 }
 same(p.summaries,expectedSummaries,'complete immutable summary source and selection');
 const summary=expectedSummaries[1].selectorAfter.value.body.currentSummary;
 const summarySelector=selector({summaryVersion:2,completedResearchPlan:plan,currentSummary:summary});
 for(const [n,disposition] of ['research-only','no-go','declined-execution'].entries()){
  const label='finish-'+disposition,req=request('finish-research',40,{researchPlan:plan,summary,disposition,decisionId:label});
  same(p.ui[n],{id:label,row:decision(label,req,run(40,'synthesizing',{currentPlan:null}),summarySelector),
   runAfter:row('research_run_controls',runKey,run(41,'complete',{currentPlan:null,parentRuntime:null}),label,3),
   selectorAfter:row('research_lifecycle_state',runKey,{...summarySelector,finishDisposition:disposition},label,3)},'exact finish disposition and terminal source');
 }
 const blocker={id:'retained-attempt',kind:'launch-interrupted',task,detail:'Supplied unknown original native disposition; no capacity release.'};
 const stage=decision('cancel-stage',request('cancel-stage',50,{plan,expectedPlanVersion:2,reason:'Stop this research stage.'}),run(50,'blocked',{blockers:[blocker]}),selector(),planRow('cancel-stage-plan'));
 const cancelRef={key:stage.key,digest:stage.value.digest},stageSelector=selector({pendingStageContinue:cancelRef});
 const paused=run(51,'paused',{currentPlan:null,blockers:[blocker],stageContinueRequired:true,pauseReason:'stage_cancelled',resumeTarget:'researching'});
 same(p.ui[3],{id:'cancel-stage',row:stage,runAfter:row('research_run_controls',runKey,paused,'cancel-stage',3),planAfter:planRow('cancel-stage','cancelled',3,3),
  selectorAfter:row('research_lifecycle_state',runKey,stageSelector,'cancel-stage',3)},'stage cancellation retains blocker and parent');
 same(p.ui[4],{id:'continue-stage',row:decision('continue-stage',request('continue-stage',51,{cancellation:cancelRef}),paused,stageSelector),
  runAfter:row('research_run_controls',runKey,run(52,'blocked',{currentPlan:null,blockers:[blocker]}),'continue-stage',3),
  selectorAfter:row('research_lifecycle_state',runKey,selector(),'continue-stage',3)},'continue clears only stage gate');
 const terminalSelector={...summarySelector,pendingStageContinue:cancelRef};
 const cancelledBefore=run(60,'paused',{currentPlan:null,blockers:[blocker],stageContinueRequired:true,pauseReason:'stage_cancelled',resumeTarget:'synthesizing'});
 same(p.ui[5],{id:'cancel-run',row:decision('cancel-run',request('cancel-run',60,{reason:'End automation; leave the ordinary child alone.'}),cancelledBefore,terminalSelector),
  runAfter:row('research_run_controls',runKey,{...cancelledBefore,runVersion:61,state:'cancelled',parentRuntime:null,pauseReason:null,resumeTarget:null},'cancel-run',3),
  selectorAfter:row('research_lifecycle_state',runKey,terminalSelector,'cancel-run',3)},'terminal cancellation preserves raw gates/history');
 same(p.invalidation,{before:expectedSummaries[1].selectorAfter,after:row('research_lifecycle_state',runKey,{...summarySelector,currentSummary:null},'report-invalidation',3),
  retainedSummaries:expectedSummaries.map(x=>x.row),expected:'clear-current-selection/retain-history'},'invalidation preserves high-water and history');
 same(p.effects,[
  ['plan-completion','plan-grant-revoked/parent-planning/live-child-held/collection-active'],
  ['summary','derived-output/no-current-input-or-task-change/no-execution-approval'],
  ['stage-cancel','result-only-parent/child-report-held/collection-active/continue-required'],
  ['runtime-resume','stage-gate-preserved/no-grant-restore'],
  ['continue','gate-only-cleared/blocker-kept/no-task-reservation-reuse'],
  ['terminal','all-authority-revoked/waits-closed/collection-retired/live-child-held'],
  ['unknown-native','retain-original-owner/capacity-held/no-rollback-or-respawn'],
  ['unknown-store','unavailable/no-success-receipt/no-replacement-worker'],
  ['terminal-late-event','absorbing/no-authority-restore'],
  ['unsupported-trigger','deny-whole-transition/no-manual-completion-delay']].map(([id,expected])=>({id,expected})),'effect labels, not executed events');
 const rows=[p.initial,...p.summaries.map(x=>x.row),p.completions[0].row,...p.ui.map(x=>x.row)];
 const measurements=rows.map(r=>{validateCompletionWidths(r,size(r));return {table:r.table,key:r.key,...measureCompletionRow(r)};});
 same(p.measurements,measurements,'exact key plus committed control occurrence projection');return measurements;
}
// All facts below are supplied assertions. These functions do not establish
// credentials, complete inventory, actual outcome, currentness or source proof.
export function checkSummaryAdmission(publication,retained,facts){
 assert.equal(facts.authority,'supplied-current-parent','current parent authority before lookup');
 assert.equal(facts.rate,'supplied-allowed','rate before lookup');assert.equal(facts.stageContinueRequired,false,'stage gate before lookup');
 exact(publication,'schemaVersion expectedRunVersion summary');assert.equal(publication.schemaVersion,1);
 const b=publication.summary,k={...b.run,summaryVersion:b.summaryVersion};
 if(retained!==null){same(k,retained.key,'exact original key');same(b,retained.value.body,'original summary conflict');
  return {kind:'original',ref:{...retained.key,summaryDigest:retained.value.digest},commit:clone(retained.value.lastCommit)};}
 assert.equal(publication.expectedRunVersion,facts.runVersion,'current run version');assert(facts.summaryVersion<64,'summary version exhaustion');
 assert.equal(b.summaryVersion,facts.summaryVersion+1,'exact next summary version');return {kind:'new',summaryVersion:b.summaryVersion};
}
export function checkLifecycleRetry(r,req,facts){
 assert.equal(facts.authority,'supplied-current-Main','current Main authority before lookup');assert.equal(facts.context,'supplied-exact','exact shown context before lookup');
 same(req,r.value.body.request,'original UI request conflict');return clone(r.value.receipt);
}
export function checkCompletionPredicate(f){
 assert(['coordination','recovery'].includes(f.trigger),'unsupported completion trigger');
 return f.taskStatus==='reported_complete'&&f.reservations==='supplied-attached-settled'&&f.allocation==='supplied-no-interruption'&&f.scope==='supplied-no-pending-revision'?'eligible':'incomplete';
}
export function checkContinueTarget(f,action){
 if(['complete','cancelled'].includes(f.state))return f.state;
 if(action==='runtime-resume'&&f.stageContinueRequired)return 'paused';
 assert.equal(action,'ui-continue','explicit UI continue');assert.equal(f.stageContinueRequired,true,'stage gate');assert.equal(f.parent,'supplied-live','resume actual parent first');
 if(f.blockers>0||f.allocation!=='settled')return 'blocked';assert.equal(f.clarification,null,'unsupported clarification source');return 'assessing';
}
