// Finite decoded comparisons only. No store, event dispatcher or publisher.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact,integer} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
import {validateScopeRejection,checkScopeRetry,checkScopeWidths} from './research-scope-rejection-components.mjs';
const clone=structuredClone,same=(a,b,label)=>assert.deepEqual(a,b,label),size=x=>Buffer.byteLength(canonical(x));
const names=['research-prefix-components.json','research-scope-rejection-components.json'],raw=names.map(n=>readFileSync(new URL(n,import.meta.url)));
const [prefix,scope]=raw.map(x=>JSON.parse(x)),a=prefix.chain.assignment,q=scope.proposal;
const fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const ta=a.reservation.task,tb=q.task,rk=fields(ta,'workspaceId runId'),inputRef=b=>fields(b,'workspaceId runId preparationRevision inputRevision inputDigest');
const domain=(n,b)=>hash(n+'\n'+canonical(b));
const origin=n=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED prospective membership label '+n)});
const row=(table,key,body,n,extra={})=>({table,key:clone(key),value:{schemaVersion:1,body:clone(body),...extra,lastCommit:origin(n)}});
const viewRows=v=>['run','currentPlan','pendingPlan','currentTask','proposedTask','selector','input','collection'].map(k=>v[k]).concat(v.expectations);
export function measureProspectiveRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function checkProspectiveWidths(rows){
 const caps={research_control_inputs:65536,research_run_controls:1048576,research_plan_controls:16384,research_task_controls:16384,research_lifecycle_state:16384,research_collections:16384,research_expectations:16384};
 for(const r of rows){assert(Object.hasOwn(caps,r.table),'known finite family');assert(size({key:r.key,value:r.value})<=caps[r.table],'complete wrapper cap');
  if(r.table==='research_control_inputs'){const b=r.value.body;assert(size(b)<=32768,'input body cap');assert(b.records.length<=256,'input record cap');integer(b.inputRevision,2,64);assert.equal(b.preparationRevision,b.inputRevision);}}
 return rows.map(measureProspectiveRow);
}
function memberships(v){
 const [x,y]=v.expectations;assert.equal(v.expectations.length,2,'complete supplied local A/B owners');
 same(v.collection.key,rk);same(v.collection.value.body.task,ta,'never retarget collecting reservation');assert.equal(v.collection.value.body.active,true);
 same(x.key,ta);same(y.key,tb);same(x.value.body.task,ta);same(y.value.body.task,tb);same(x.value.body.collection,rk);same(y.value.body.collection,rk);
 assert.equal(x.value.body.collecting,true);same(x.value.body.reservation,a.reservation);assert.equal(y.value.body.collecting,false);assert.equal(y.value.body.reservation,null);assert.equal(y.value.body.selectedReport,null);
 same(v.currentPlan.key,a.plan);same(v.pendingPlan.key,q.ref);same(v.pendingPlan.value.body.task,tb);same(v.pendingPlan.value.body.replaces,a.plan);assert.equal(v.pendingPlan.value.body.latestApproval,null);
 const pending=v.pendingPlan.value.body.status==='proposed';
 if(pending)same(v.run.value.body.pendingPlan,q.ref,'exact original prospective pointer');
 else{assert.equal(v.pendingPlan.value.body.status,'cancelled','retained rejected control');assert.equal(v.run.value.body.pendingPlan,null);}
 return {all:[ta,tb],collecting:[ta],prospective:pending?[tb]:[],retired:pending?[]:[tb]};
}
function finiteSources(){
 const base=scope.cases[1],original=row('research_control_inputs',fields(q.input,'workspaceId runId preparationRevision inputRevision'),q.input,9,{producer:clone(prefix.inputs[0].value.producer)});
 const body=clone(q.input);body.preparationRevision=body.inputRevision=4;body.records.push({kind:'research-report',source:clone(prefix.reportAssumption.source)});
 body.records.sort((x,y)=>Buffer.compare(Buffer.from(x.kind),Buffer.from(y.kind))||Buffer.compare(Buffer.from(canonical(x)),Buffer.from(canonical(y))));
 delete body.inputDigest;body.inputDigest=domain('orkworks.preparation-input.v1',body);
 const selected=row('research_control_inputs',fields(body,'workspaceId runId preparationRevision inputRevision'),body,11,{producer:clone(prefix.inputs[1].value.producer)});
 const oldCollection=prefix.views[2].rows.find(x=>x.table==='research_collections'),ae=clone(prefix.views[2].rows.find(x=>x.table==='research_expectations'));
 const be=row('research_expectations',tb,{schemaVersion:1,task:tb,collection:rk,collecting:false,configuration:{task:tb,...fields(q.startup.configuration,'configurationId configurationDigest')},scope:{task:tb,scopeDigest:q.startup.scope.scopeDigest},reservation:null,selectedReport:null,expectationVersion:1},9);
 const reads=base.rejection.value.body.readSet;
 const submitted={id:'submitted',run:clone(reads.run),currentPlan:clone(reads.currentPlan),pendingPlan:clone(reads.pendingPlan),currentTask:clone(base.source.currentTask),proposedTask:clone(base.after.proposedTask),selector:clone(reads.lifecycle),input:original,
  collection:row('research_collections',rk,{...oldCollection.value.body,collectionVersion:3},9),expectations:[ae,be]};submitted.memberships=memberships(submitted);
 const invalid=clone(submitted);invalid.id='invalidated';invalid.run=row('research_run_controls',rk,{...reads.run.value.body,runVersion:21,currentInput:inputRef(body)},11);invalid.input=selected;
 invalid.collection=row('research_collections',rk,{...oldCollection.value.body,collectionVersion:4},11);
 invalid.expectations[0]=row('research_expectations',ta,{...ae.value.body,expectationVersion:3,selectedReport:clone(prefix.reportAssumption.source)},11);
 const before={run:clone(submitted.run),collection:clone(submitted.collection),expectation:ae,input:clone(prefix.inputs[0])};
 Object.assign(before.run.value.body,{runVersion:19,currentInput:inputRef(prefix.inputs[0].value.body),pendingPlan:null,planReferences:[a.plan]});before.collection.value.body.collectionVersion=2;
 return {original,selected,submitted,invalid,before};
}
function rejectionExamples(invalid){return scope.cases.map(old=>{
 const complete=old.completion!==null,label=old.id,before=clone(invalid);before.currentTask=clone(old.source.currentTask);
 const reads={...clone(old.rejection.value.body.readSet),expectedCommit:origin(12),run:clone(before.run)};
 const req={...clone(old.rejection.value.body.request),expectedRunVersion:21,input:inputRef(before.input.value.body),decisionId:'prospective-reject-'+label,reviewContextId:hash('UNQUALIFIED prospective shown '+label)};
 function lifecycle(request,key,pv){const body={schemaVersion:1,key,request:clone(request),readSet:clone(reads)},digest=domain('orkworks.research-lifecycle-decision.v1',body);
  return row('research_lifecycle_records',key,body,13,{digest,receipt:{schemaVersion:1,key,decisionDigest:digest,runVersion:22,planVersion:pv,commit:origin(13)}});}
 const rejection=lifecycle(req,{kind:'ui-decision',run:rk,decisionId:req.decisionId},null);
 const completion=complete?lifecycle({schemaVersion:1,kind:'plan-completion',run:rk,expectedRunVersion:21,input:req.input,plan:a.plan,expectedPlanVersion:2,trigger:{kind:'scope-rejection',rejection:{key:rejection.key,digest:rejection.value.digest}}},{kind:'plan-completion',plan:a.plan},3):null;
 const after=clone(before);after.id='rejected-'+label;after.run=row('research_run_controls',rk,{...before.run.value.body,runVersion:22,pendingPlan:null,currentPlan:complete?null:a.plan},13);
 after.pendingPlan=row('research_plan_controls',q.ref,{...before.pendingPlan.value.body,status:'cancelled',planVersion:2},13);
 after.collection=row('research_collections',rk,{...before.collection.value.body,collectionVersion:5},13);
 if(complete){after.currentPlan=row('research_plan_controls',a.plan,{...before.currentPlan.value.body,status:'complete',planVersion:3},13);after.selector=row('research_lifecycle_state',rk,{...before.selector.value.body,completedResearchPlan:a.plan},13);}
 after.memberships=memberships(after);return {id:label,before,rejection,completion,after};
 });}
export function validateProspective(p){
 exact(p,'inputHashes assumptions originalAssessment reportOriginal reportRetry submissionBefore views cases facts measurements');
 same(p.inputHashes,Object.fromEntries(names.map((n,i)=>[n,hash(raw[i])])),'unchanged source bytes');validateResearchPrefix(prefix);validateScopeRejection(scope);
 same(p.assumptions,[
  'finite decoded alternatives, not a legal pending prestate/store/history or publisher',
  'auth/native/source/currentness/complete owner-index membership/original H-C proofs/outcomes supplied and unqualified',
  'post-rejection/completion report writers, approval handover and complete accounting/physical profile remain unqualified'],'explicit qualification limits');
 const {original,selected,submitted,invalid,before}=finiteSources(),cases=rejectionExamples(invalid);
 same(p.originalAssessment,original,'original B assessment with parent producer, distinct from child event');same(p.submissionBefore,before,'supplied earlier comparison only, no legal prestate');
 assert.equal(p.views.length,2);for(const v of [...p.views,...p.cases.flatMap(c=>[c.before,c.after])]){same(v.memberships,memberships(v),'source-derived role relation');checkProspectiveWidths(viewRows(v));}
 same(p.views,[submitted,invalid],'complete finite source-derived selection/input/version comparison');same(p.cases,cases,'exact original Main rejection and source-preserving outcome alternatives');
 const stream=fields(a.assignmentIdentity,'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest');
 const request={schemaVersion:1,operation:'submitResearchReport',stream,expectedTaskVersion:scope.cases[1].source.currentTask.value.body.taskVersion,expectedRunVersion:null,expectedReportHead:null,report:clone(prefix.reportAssumption.report)};
 const receiptRow={table:'report_receipts',key:{stream,reportVersion:1},value:{schemaVersion:1,operation:'submitResearchReport',expectedTaskVersion:request.expectedTaskVersion,expectedRunVersion:null,expectedReportHead:null,requestDigest:domain('orkworks.research-report-request.v1',request),response:{schemaVersion:1,acceptedHead:clone(prefix.reportAssumption.head)},lastCommit:origin(11)}};
 const originalReport={request,headRow:{table:'report_heads',key:stream,value:{schemaVersion:1,head:clone(prefix.reportAssumption.head),lastCommit:origin(11)}},receiptRow};
 same(p.reportOriginal,originalReport,'closed report original request/registry/response at live A CAS');assert(size(request)<=32768);assert(size({key:receiptRow.key,value:receiptRow.value})<=2048);assert(size(originalReport.headRow.value)<=512);
 same(p.reportRetry,{request,receipt:receiptRow.value,retained:{input:selected,run:invalid.run,collection:invalid.collection,expectations:invalid.expectations}},'exact report retry retains original response and creates no input/version');
 const facts={...clone(scope.facts),originalOutcomes:Object.fromEntries(cases.map(c=>[c.rejection.value.digest,{commit:origin(13),completionDigest:c.completion?.value.digest??null}]))};same(p.facts,facts,'stipulated original outcome correspondence, not current-state election');
 for(const c of p.cases)checkScopeRetry(c,c.rejection.value.body.request,p.facts);
 const delta=(old,next)=>next.reduce((n,r)=>n+measureProspectiveRow(r).chargeBytes,0)-old.reduce((n,r)=>n+measureProspectiveRow(r).chargeBytes,0);
 const m={views:p.views.map(v=>viewRows(v).map(measureProspectiveRow)),rejections:p.cases.map(c=>checkScopeWidths([c.rejection,...(c.completion?[c.completion]:[])])),localDeltas:[delta([before.run,before.collection],[submitted.run,submitted.collection]),delta([submitted.run,submitted.collection,submitted.expectations[0]],[invalid.run,invalid.collection,invalid.expectations[0]]),...p.cases.map(c=>delta([c.before.run,c.before.collection],[c.after.run,c.after.collection]))]};
 same(p.measurements,m,'independent occurrence bytes and limited original-final deltas');return m;
}
