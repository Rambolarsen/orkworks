// Finite decoded offline examples only; never a product writer or wire codec.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact,integer} from './root-bootstrap-components.mjs';
import {validateResearchStartup} from './research-startup-components.mjs';
const clone=structuredClone,same=(a,b,label)=>assert.deepEqual(a,b,label),size=x=>Buffer.byteLength(canonical(x));
const names=['research-prefix-components.json','research-continuation-components.json'],raw=names.map(n=>readFileSync(new URL(n,import.meta.url)));
const [prefix,continuation]=raw.map(x=>JSON.parse(x)),a=prefix.chain.assignment,old=a.plan;
const fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const runKey=fields(old,'workspaceId runId'),domain=(n,b)=>hash(n+'\n'+canonical(b));
const origin=n=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED scope rejection label '+n)});
const row=(table,key,body,n)=>({table,key:clone(key),value:{schemaVersion:1,body:clone(body),lastCommit:origin(n)}});
function lifecycle(req,key,readSet,pv){const body={schemaVersion:1,key,request:req,readSet:clone(readSet)},d=domain('orkworks.research-lifecycle-decision.v1',body);
 return {table:'research_lifecycle_records',key,value:{schemaVersion:1,body,digest:d,receipt:{schemaVersion:1,key,decisionDigest:d,runVersion:21,planVersion:pv,commit:origin(11)},lastCommit:origin(11)}};}
export function measureScopeRow(r){const v=clone(r.value);delete v.lastCommit;delete v.receipt.commit;
 return {chargeBytes:size({key:r.key,value:v}),wrapperBytes:size({key:r.key,value:r.value})};}
export function checkScopeWidths(rows){for(const r of rows){assert(size({key:r.key,value:r.value})<=2097152,'decision wrapper');assert(size(r.value.receipt)<=4096,'receipt');}
 const m=rows.map(measureScopeRow);assert(m.reduce((n,x)=>n+x.wrapperBytes,0)<=1048576,'paired job ceiling');return m;}
function snapshotWidth(r){const cap=r.table==='research_run_controls'?1048576:16384;assert(size(r)<=cap,'snapshot ceiling');}
export function validateScopeRejection(p){
 exact(p,'inputHashes assumptions proposal cases submissions facts');same(p.inputHashes,Object.fromEntries(names.map((n,i)=>[n,hash(raw[i])])),'unchanged source bytes');
 same(p.assumptions,[
  'separate finite decoded alternatives, not a store, legal pending prestate or history',
  'reserved-plan singleton collection membership/publisher remains unclosed; no second active task inferred',
  'Main/parent/native/source/currentness/membership/original H/C/proofs/outcome facts supplied and unqualified',
  'resource ledgers, full effects/job/command/physical fit and ordinary capacity remain unqualified'],'qualification boundary');
 const q=p.proposal;exact(q,'startup input plan ref task');validateResearchStartup(q.startup);
 same(q.startup.context.run,prefix.chain.startup.context.run,'original immutable bootstrap/run');
 const task={...fields(old,'workspaceId runId planId'),planRevision:2,taskId:'scope-revision-task-2'},cfg=q.startup.configuration;
 same(q.task,task);same(fields(cfg,'planId planRevision taskId'),fields(task,'planId planRevision taskId'));
 assert.equal(cfg.configurationId,'scope-revision-configuration-2');assert.equal(cfg.assignment.worktreeGroupId,'scope-revision-group-2');
 const scope={task,scopeDigest:q.startup.scope.scopeDigest};
 const records=[{kind:'configuration',reference:{task,...fields(cfg,'configurationId configurationDigest')}},{kind:'research-scope',reference:scope},{kind:'prior-plan',plan:old},
  ...q.startup.scope.captures.map(reference=>({kind:'research-capture',reference}))];
 records.sort((x,y)=>Buffer.compare(Buffer.from(x.kind),Buffer.from(y.kind))||Buffer.compare(Buffer.from(canonical(x)),Buffer.from(canonical(y))));
 const input=clone(prefix.inputs[0].value.body);Object.assign(input,{preparationRevision:3,inputRevision:3,records});
 Object.assign(input.decision,{inputRevision:3,scope,reasons:['Synthetic changed scope proposal; no approval or producer proof.']});
 delete input.inputDigest;input.inputDigest=domain('orkworks.preparation-input.v1',input);same(q.input,input,'complete source-derived input including exact prior plan');assert(size(input)<=32768);
 const plan=clone(prefix.chain.plan);Object.assign(plan.parentPlanBinding,{planRevision:2,preparationRevision:3,inputDigest:input.inputDigest});
 Object.assign(plan.task,{taskId:task.taskId,configuration:fields(cfg,'configurationId configurationDigest'),scope,
  group:{groupId:'scope-revision-group-2',branchName:'fixture/scope-revision-2',absolutePath:{encoding:'unix-bytes',base64:Buffer.from('/fixture/scope-revision-2').toString('base64')},baseCommit:cfg.assignment.baseCommit}});
 plan.batches=[[task.taskId]];delete plan.planDefinitionDigest;plan.planDefinitionDigest=domain('orkworks.research-source-plan.v1',plan);
 same(q.plan,plan,'new exact proposal source without approval');assert(size(plan)<=65536);
 const ref={...fields(task,'workspaceId runId planId planRevision'),planDefinitionDigest:plan.planDefinitionDigest};same(q.ref,ref);
 // Necessary source occurrences, not a complete approval/job packing witness.
 assert(size(plan)+size(cfg)+size(q.startup.scope)+q.startup.captures.reduce((n,x)=>n+size(x),0)+q.startup.evidenceObjects.reduce((n,x)=>n+size(x),0)<=2097152,'approved source occurrence ceiling');
 same(p.submissions,[old,ref],'original retained submission order');assert.equal(nextScopeRevision(p.submissions,p.facts),3);
 const inputRef=fields(input,'workspaceId runId preparationRevision inputRevision inputDigest');
 assert.equal(p.cases.length,2);const measurements=[];
 for(const [i,c] of p.cases.entries()){
  exact(c,'id source rejection completion after measurements');const complete=i===0,label=complete?'complete':'incomplete';assert.equal(c.id,label);
  const source={turn:continuation.turns[1].gate,coordination:complete?continuation.coordinations[0].row:null,currentTask:complete?continuation.coordinations[0].taskAfter:continuation.turns[1].taskAfter};same(c.source,source,'unchanged original turn/result/task evidence');
  const rb=clone(continuation.recoveries[3].row.value.body.readSet.run.value.body);
  const blocker={id:'independent-approval',kind:'recovery-approval-required',task:a.reservation.task,detail:'Independent retained gate; rejection cannot clear it.'};
  Object.assign(rb,{runVersion:20,state:'blocked',currentInput:inputRef,planReferences:[old,ref],currentPlan:old,pendingPlan:ref,blockers:[blocker]});
  const current={schemaVersion:1,plan:old,planVersion:2,status:'active',latestApproval:a.approval,task:a.reservation.task,replaces:null};
  const pending={schemaVersion:1,plan:ref,planVersion:1,status:'proposed',latestApproval:null,task,replaces:old};
  const proposedTask=row('research_task_controls',task,{schemaVersion:1,task,taskVersion:1,status:'pending',reservation:null,coordinationResult:null},9);
  const selector={schemaVersion:1,run:runKey,summaryVersion:0,completedResearchPlan:null,currentSummary:null,pendingStageContinue:null,finishDisposition:null};
  const reads={expectedCommit:origin(10),run:row('research_run_controls',runKey,rb,9),lifecycle:row('research_lifecycle_state',runKey,selector,1),currentPlan:row('research_plan_controls',old,current,2),pendingPlan:row('research_plan_controls',ref,pending,9)};
  const req={schemaVersion:1,kind:'reject-scope-revision',run:runKey,expectedRunVersion:20,input:inputRef,decisionId:'reject-'+label,plan:old,expectedPlanVersion:2,proposedPlan:ref,expectedProposedPlanVersion:1,reason:'Reject this exact changed scope proposal.',reviewContextId:hash('UNQUALIFIED shown rejection '+label),sidecarGeneration:'c'.repeat(64)};
  const rejection=lifecycle(req,{kind:'ui-decision',run:runKey,decisionId:req.decisionId},reads,null);same(c.rejection,rejection,'complete original rejection snapshots/request/domain/receipt');
  const completion=complete?lifecycle({schemaVersion:1,kind:'plan-completion',run:runKey,expectedRunVersion:20,input:inputRef,plan:old,expectedPlanVersion:2,
   trigger:{kind:'scope-rejection',rejection:{key:rejection.key,digest:rejection.value.digest}}},{kind:'plan-completion',plan:old},reads,3):null;
  same(c.completion,completion,'typed exact original companion, or no completion for incomplete work');
  const afterRun={...rb,runVersion:21,pendingPlan:null,currentPlan:complete?null:old};
  same(c.after,{run:row('research_run_controls',runKey,afterRun,11),currentPlan:complete?row('research_plan_controls',old,{...current,status:'complete',planVersion:3},11):reads.currentPlan,
   pendingPlan:row('research_plan_controls',ref,{...pending,status:'cancelled',planVersion:2},11),currentTask:source.currentTask,proposedTask,
   selector:complete?row('research_lifecycle_state',runKey,{...selector,completedResearchPlan:old},11):reads.lifecycle},'coalesced source-derived after-values retain history/gates/tasks');
  Object.values(reads).filter(x=>x?.table).forEach(snapshotWidth);Object.values(c.after).forEach(snapshotWidth);
  const m=checkScopeWidths([c.rejection,...(c.completion?[c.completion]:[])]);same(c.measurements,m,'independent key plus projection occurrence bytes');measurements.push(m);
 }
 same(p.facts.originalOutcomes,Object.fromEntries(p.cases.map(c=>[c.rejection.value.digest,{commit:origin(11),completionDigest:c.completion?.value.digest??null}])),'stipulated original outcome correspondence, not proof');
 return measurements;
}
function authorized(f){assert.equal(f.authority,'supplied-current-Main','current authority before lookup');assert.equal(f.context,'supplied-exact','exact current shown context');assert.equal(f.rate,'supplied-allowed','current rate before lookup');assert.equal(f.currentness,'supplied-current-owner','current scope/workspace/epoch/owner/lease/health before lookup');assert.equal(f.membership,'supplied-complete','complete original membership');assert.equal(f.sourceProofs,'supplied-original','original source proofs');assert.equal(f.disposition,'supplied-committed','known original disposition');assert.equal(f.originalOutcome,'supplied-exact','original complete outcome proof');}
export function nextScopeRevision(refs,facts){
 assert.equal(facts.membership,'supplied-complete','complete submitted history');assert.equal(facts.sourceProofs,'supplied-original','original submission proofs');
 assert(refs.length>0&&refs.length<=64,'retained revision bound');const first=refs[0];
 refs.forEach((r,i)=>{same(fields(r,'workspaceId runId planId'),fields(first,'workspaceId runId planId'));assert.equal(r.planRevision,i+1,'exact retained submission order, no sorting/max election');assert(/^[0-9a-f]{64}$/.test(r.planDefinitionDigest));});
 assert(refs.length<64,'revision exhaustion');return refs.length+1;
}
export function checkScopeRetry(c,request,facts){
 authorized(facts);assert(c?.rejection,'original rejection required');same(request,c.rejection.value.body.request,'original request conflict');
 const r=c.rejection,b=r.value.body;assert.equal(r.table,'research_lifecycle_records');exact(b,'schemaVersion key request readSet');assert.equal(b.schemaVersion,1);assert.equal(request.kind,'reject-scope-revision');
 const key={kind:'ui-decision',run:request.run,decisionId:request.decisionId};same(r.key,key,'original indexed rejection key');same(b.key,key,'original hashed rejection key');
 assert.equal(r.value.digest,domain('orkworks.research-lifecycle-decision.v1',b));
 const rv=b.readSet.run.value.body.runVersion,pv=b.readSet.currentPlan.value.body.planVersion;
 integer(rv,1,2147483646);integer(pv,1,2147483647);assert.equal(request.expectedRunVersion,rv);assert.equal(request.expectedPlanVersion,pv);
 same(r.value.receipt,{schemaVersion:1,key,decisionDigest:r.value.digest,runVersion:rv+1,planVersion:null,commit:r.value.lastCommit},'exact original rejection receipt');
 // This fixture-only correspondence is stipulated original-proof input. The
 // case label, current rows and missing companion cannot elect its branch.
 const outcome=facts.originalOutcomes?.[r.value.digest];assert(outcome,'original exact outcome correspondence');exact(outcome,'commit completionDigest');same(outcome.commit,r.value.lastCommit);
 if(outcome.completionDigest!==null){
  integer(pv+1,2,2147483647);
  const x=c.completion;assert(x,'original completion companion required');assert.equal(x.table,'research_lifecycle_records');assert.equal(x.value.digest,domain('orkworks.research-lifecycle-decision.v1',x.value.body));
  assert.equal(x.value.digest,outcome.completionDigest,'original companion digest');const ck={kind:'plan-completion',plan:request.plan};same(x.key,ck,'original indexed completion key');
  same(x.value.body,{schemaVersion:1,key:ck,request:{schemaVersion:1,kind:'plan-completion',run:request.run,expectedRunVersion:rv,input:request.input,plan:request.plan,expectedPlanVersion:pv,
   trigger:{kind:'scope-rejection',rejection:{key,digest:r.value.digest}}},readSet:b.readSet},'original exact compound completion body');
  same(x.value.lastCommit,r.value.lastCommit);same(x.value.receipt,{schemaVersion:1,key:ck,decisionDigest:x.value.digest,runVersion:rv+1,planVersion:pv+1,commit:r.value.lastCommit},'exact original completion receipt');
 }else{assert.equal(c.completion,null,'original non-completing outcome cannot borrow a later completion');}
 return {rejection:clone(r.value.receipt),completion:c.completion?clone(c.completion.value.receipt):null};
}
