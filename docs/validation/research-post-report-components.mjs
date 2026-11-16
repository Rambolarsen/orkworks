// Finite decoded offline comparisons only; no publisher, wire codec or store.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact,integer} from './root-bootstrap-components.mjs';
import {validateProspective} from './research-prospective-components.mjs';
import {checkScopeRetry} from './research-scope-rejection-components.mjs';
const clone=structuredClone,same=(a,b,label)=>assert.deepEqual(a,b,label),size=x=>Buffer.byteLength(canonical(x));
const names=['research-prefix-components.json','research-prospective-components.json'],raw=names.map(n=>readFileSync(new URL(n,import.meta.url))),[prefix,prospective]=raw.map(x=>JSON.parse(x));
const fields=(x,names)=>Object.fromEntries(names.split(' ').map(k=>[k,x[k]]));
const a=prefix.chain.assignment,ta=a.reservation.task,rk=fields(ta,'workspaceId runId'),stream=fields(a.assignmentIdentity,'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest');
const domain=(n,b)=>hash(n+'\n'+canonical(b)),ref=b=>fields(b,'workspaceId runId preparationRevision inputRevision inputDigest');
const origin=n=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED post-report label '+n)});
const row=(table,key,body,n,extra={})=>({table,key:clone(key),value:{schemaVersion:1,body:clone(body),...extra,lastCommit:origin(n)}});
function view(v){return {...Object.fromEntries([['run','run'],['aPlan','currentPlan'],['bPlan','pendingPlan'],['aTask','currentTask'],['bTask','proposedTask'],['selector','selector'],['input','input'],['collection','collection']].map(([n,k])=>[n,clone(v[k])])),aExpectation:clone(v.expectations[0]),bExpectation:clone(v.expectations[1]),memberships:clone(v.memberships),summary:null};}
export function measurePostReportRow(r){const value=clone(r.value);delete value.lastCommit;if(value.receipt)delete value.receipt.commit;
 return {chargeBytes:size({key:r.key,value}),wrapperBytes:size({key:r.key,value:r.value})};}
export function checkPostReportWidths(rows){
 const caps={research_control_inputs:65536,research_run_controls:1048576,research_plan_controls:16384,research_task_controls:16384,research_session_states:16384,research_lifecycle_state:16384,research_collections:16384,research_expectations:16384,research_summaries:16384,report_heads:2097152,report_receipts:2048};
 for(const r of rows){assert(Object.hasOwn(caps,r.table),'known finite family');assert(size({key:r.key,value:r.value})<=caps[r.table],'wrapper ceiling');
  if(r.table==='research_control_inputs'){const b=r.value.body;assert(size(b)<=32768,'input body');assert(b.records.length<=256,'input record count');integer(b.inputRevision,2,64);assert.equal(b.preparationRevision,b.inputRevision);}
  if(r.table==='research_summaries')assert(size(r.value.body)<=8192,'summary body');if(r.table==='report_heads')assert(size(r.value)<=512,'head value ceiling');}
 const m=rows.map(measurePostReportRow);assert(m.reduce((n,x)=>n+x.wrapperBytes,0)<=1048576,'necessary lower job ceiling');return m;
}
export function checkSummaryDrop(before,after){
 assert.equal(before.table,'research_lifecycle_state');same(after.key,before.key);integer(before.value.body.summaryVersion,0,64);
 if(before.value.body.currentSummary===null)same(after,before,'null selector is not rewritten');
 else same(after,{...before,value:{...before.value,body:{...before.value.body,currentSummary:null},lastCommit:after.value.lastCommit}},'only currentSummary and outer origin may change');
 return true;
}
function sources(i){
 const source=prospective.cases[i>=2?0:1];let original={rejection:clone(source.rejection),completion:clone(source.completion)},before=view(source.after);
 if(i===0){
  const submitted=prospective.views[0],b=clone(source.rejection.value.body);
  Object.assign(b.readSet,{expectedCommit:origin(29),run:clone(submitted.run)});
  Object.assign(b.request,{expectedRunVersion:20,input:ref(submitted.input.value.body),decisionId:'post-reject-before-first',reviewContextId:hash('UNQUALIFIED shown rejection before first report')});
  b.key={kind:'ui-decision',run:rk,decisionId:b.request.decisionId};const d=domain('orkworks.research-lifecycle-decision.v1',b);
  original={rejection:row('research_lifecycle_records',b.key,b,30,{digest:d,receipt:{schemaVersion:1,key:b.key,decisionDigest:d,runVersion:21,planVersion:null,commit:origin(30)}}),completion:null};
  before.input=clone(prospective.originalAssessment);before.run=row('research_run_controls',rk,{...submitted.run.value.body,runVersion:21,pendingPlan:null},30);
  before.collection=row('research_collections',rk,{...submitted.collection.value.body,collectionVersion:4},30);before.aExpectation=clone(submitted.expectations[0]);before.bPlan.value.lastCommit=origin(30);
 }
 before.bridge=row('research_session_states',ta,clone(prefix.views[2].rows.find(x=>x.table==='research_session_states').value.body),31);
 if(i===3){
  const b={schemaVersion:1,run:rk,summaryVersion:1,researchPlan:a.plan,input:ref(before.input.value.body),producer:clone(prefix.inputs[0].value.producer),findings:['Synthetic original summary before post-rejection correction.'],uncertainties:['Native and complete source facts remain unqualified.'],assignmentChoices:[],reportReferences:[clone(before.aExpectation.value.body.selectedReport)],reviewReferences:[]};
  const digest=domain('orkworks.research-summary.v1',b);before.summary=row('research_summaries',{...rk,summaryVersion:1},b,31,{digest});
  before.selector=row('research_lifecycle_state',rk,{...before.selector.value.body,summaryVersion:1,currentSummary:{...rk,summaryVersion:1,summaryDigest:digest}},31);
  before.run=row('research_run_controls',rk,{...before.run.value.body,runVersion:23},31);
 }
 return {original,before};
}
function evidence(report){return {schemaVersion:1,sourceKind:'research-report',reportIdentity:{workspaceId:rk.workspaceId,...fields(report,'runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration reportId reportVersion')},sizeBytes:size(report),contentDigest:report.contentDigest};}
export function validatePostReport(p){
 exact(p,'inputHashes assumptions originalAssessment cases facts');same(p.inputHashes,Object.fromEntries(names.map((n,i)=>[n,hash(raw[i])])),'unchanged input fingerprints');validateProspective(prospective);
 same(p.assumptions,[
  'four independent decoded alternatives, not a legal post-rejection store/history or publisher',
  'original disposition/source/auth/native/currentness/complete membership/H-C/proofs/outcomes supplied and unqualified',
  'nonterminal bridge and summary are hypothetical source values, not native observation or summary creation proof',
  'complete resources/discovery/index/physical/job/command fit and activation remain unqualified'],'qualification boundary');
 same(p.originalAssessment,prospective.originalAssessment,'original parent assessment remains retained');
 assert.equal(p.cases.length,4);const measurements=[],outcomes={},main=clone(prospective.facts);
 for(const [i,c] of p.cases.entries()){
  exact(c,'id original before publication after writes measurements');const id=['incomplete-first','incomplete-correction','complete-null','complete-summary'][i];assert.equal(c.id,id);
  const {original,before}=sources(i);same(c.original,original,'original rejection and paired disposition, never a live election');same(c.before,before,'source-derived post-rejection role snapshot');
  main.originalOutcomes[original.rejection.value.digest]={commit:original.rejection.value.lastCommit,completionDigest:original.completion?.value.digest??null};
  const report=clone(prefix.reportAssumption.report);Object.assign(report,{reportVersion:i===0?1:2,uncertainties:['Synthetic post-rejection report — '+id+'.']});delete report.contentDigest;report.contentDigest=domain('orkworks.research-report.v1',report);
  const source=evidence(report),head=fields(report,'reportId reportVersion contentDigest'),C=origin(50+i);
  const request={schemaVersion:1,operation:i===0?'submitResearchReport':'correctResearchReport',stream,expectedTaskVersion:before.aTask.value.body.taskVersion,expectedRunVersion:null,expectedReportHead:i===0?null:clone(prefix.reportAssumption.head),report};
  const d=domain('orkworks.research-report-request.v1',request);
  const publication={report,source,request,headRow:{table:'report_heads',key:stream,value:{schemaVersion:1,head,lastCommit:C}},receiptRow:{table:'report_receipts',key:{stream,reportVersion:report.reportVersion},value:{schemaVersion:1,...fields(request,'operation expectedTaskVersion expectedRunVersion expectedReportHead'),requestDigest:d,response:{schemaVersion:1,acceptedHead:head},lastCommit:C}}};
  same(c.publication,publication,'exact original child descriptor/source/registry/request/closed response');assert(size(report)<=32768,'descriptor cap');assert(size(request)<=32768,'request cap');
  const input=clone(before.input.value.body);input.preparationRevision=input.inputRevision=input.inputRevision+1;
  input.records=input.records.filter(x=>x.kind!=='research-report').concat({kind:'research-report',source});input.records.sort((x,y)=>Buffer.compare(Buffer.from(x.kind),Buffer.from(y.kind))||Buffer.compare(Buffer.from(canonical(x)),Buffer.from(canonical(y))));
  delete input.inputDigest;input.inputDigest=domain('orkworks.preparation-input.v1',input);
  const after=clone(before);Object.assign(after,{
   input:row('research_control_inputs',fields(input,'workspaceId runId preparationRevision inputRevision'),input,50+i,{producer:clone(prefix.inputs[1].value.producer)}),
   run:row('research_run_controls',rk,{...before.run.value.body,currentInput:ref(input),runVersion:before.run.value.body.runVersion+1},50+i),
   collection:row('research_collections',rk,{...before.collection.value.body,collectionVersion:before.collection.value.body.collectionVersion+1},50+i),
   aExpectation:row('research_expectations',ta,{...before.aExpectation.value.body,selectedReport:source,expectationVersion:before.aExpectation.value.body.expectationVersion+1},50+i)});
  if(i===3)after.selector=row('research_lifecycle_state',rk,{...before.selector.value.body,currentSummary:null},50+i);
  same(c.after,after,'complete finite after-values preserve rejected source/task/plan/ordinary/summary history');checkSummaryDrop(c.before.selector,c.after.selector);
  const writes=['head','receipt','input','run','collection','aExpectation'].concat(i===3?['selector']:[]);same(c.writes,writes,'represented rows only, not complete command effects');
  const rows=writes.map(k=>k==='head'?c.publication.headRow:k==='receipt'?c.publication.receiptRow:c.after[k]);const m=checkPostReportWidths(rows);same(c.measurements,m,'independently measured occurrence bytes');measurements.push(m);
  checkPostReportWidths(Object.values(c.before).filter(x=>x?.table));checkPostReportWidths(Object.values(c.after).filter(x=>x?.table));outcomes[d]={commit:C};
 }
 same(p.facts,{main,child:{authority:'supplied-live-child',runtime:'supplied-nonterminal',rate:'supplied-allowed',currentness:'supplied-current-owner',membership:'supplied-complete',sourceProofs:'supplied-original',disposition:'supplied-committed',runState:'blocked',outcomes}},'stipulated authority/proof/outcome premises, never produced qualification');
 for(const c of p.cases){checkScopeRetry(c.original,c.original.rejection.value.body.request,p.facts.main);same(checkPostReportRetry(c.publication,c.publication.request,p.facts.child),c.publication.receiptRow.value.response);}
 return measurements;
}
export function checkPostReportRetry(original,request,facts){
 assert.equal(facts.authority,'supplied-live-child','current child authority before lookup');assert.equal(facts.runtime,'supplied-nonterminal','runtime closure before lookup');assert.equal(facts.rate,'supplied-allowed','rate before lookup');
 assert.equal(facts.currentness,'supplied-current-owner','current scope/epoch/lease/health owner');assert.equal(facts.membership,'supplied-complete','complete original membership');assert.equal(facts.sourceProofs,'supplied-original','original full source/proofs');assert.equal(facts.disposition,'supplied-committed','known committed original outcome');assert(['researching','synthesizing','blocked','paused'].includes(facts.runState),'run closure before lookup');
 assert(original,'original report required');same(request,original.request,'original request conflict');exact(request,'schemaVersion operation stream expectedTaskVersion expectedRunVersion expectedReportHead report');assert.equal(request.schemaVersion,1);same(request.stream,stream,'original exact stream');assert.equal(request.expectedRunVersion,null);integer(request.expectedTaskVersion,1,2147483647);
 same(original.report,request.report);const report=clone(request.report),saved=report.contentDigest;delete report.contentDigest;assert.equal(saved,domain('orkworks.research-report.v1',report),'original descriptor digest');integer(report.reportVersion,1,8);same(original.source,evidence(request.report));
 assert.equal(request.operation,report.reportVersion===1?'submitResearchReport':'correctResearchReport');if(report.reportVersion===1)assert.equal(request.expectedReportHead,null);else{assert(request.expectedReportHead);assert.equal(request.expectedReportHead.reportVersion,report.reportVersion-1);assert.equal(request.expectedReportHead.reportId,report.reportId);}
 const d=domain('orkworks.research-report-request.v1',request),outcome=facts.outcomes?.[d];assert(outcome,'original exact outcome proof');exact(outcome,'commit');const C=outcome.commit,head=fields(request.report,'reportId reportVersion contentDigest');
 same(original.headRow,{table:'report_heads',key:stream,value:{schemaVersion:1,head,lastCommit:C}},'original head ownership');
 same(original.receiptRow,{table:'report_receipts',key:{stream,reportVersion:report.reportVersion},value:{schemaVersion:1,...fields(request,'operation expectedTaskVersion expectedRunVersion expectedReportHead'),requestDigest:d,response:{schemaVersion:1,acceptedHead:head},lastCommit:C}},'exact original receipt key/CAS/closed response/C');
 assert(size(request)<=32768);checkPostReportWidths([original.headRow,original.receiptRow]);return clone(original.receiptRow.value.response);
}
