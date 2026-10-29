// Conditional documentation assay only: no SQL, credentials, clocks, I/O or native atomicity.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {verifyR2,validateReport,validateReportEvidenceSource} from './r2-report-components.mjs';
export const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort(order).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const order=(a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b));
const clone=structuredClone, bytes=x=>Buffer.byteLength(canonical(x));
const hash=x=>createHash('sha256').update(x).digest('hex');
const domain=(d,x)=>hash(d+'\n'+canonical(x));
const same=(a,b)=>assert.deepEqual(a,b);
const exact=(x,fields)=>{assert(x&&typeof x==='object'&&!Array.isArray(x));same(Object.keys(x).sort(),fields.split(' ').sort());};
const id=x=>assert(typeof x==='string'&&/^[A-Za-z0-9._-]{1,128}$/.test(x));
const digest=x=>assert(typeof x==='string'&&/^[a-f0-9]{64}$/.test(x));
const integer=(x,max=2147483647)=>assert(Number.isSafeInteger(x)&&x>=1&&x<=max);
const counter=(x,max=18446744073709551615n)=>{assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(x)&&BigInt(x)<=max);return BigInt(x);};
const increment=(x,max)=>{const n=counter(x,max);need(n<max,'limit-exceeded');return String(n+1n);};
const key=r=>r.table+'\n'+canonical(r.key);
const sorted=rows=>[...rows].sort((a,b)=>order(a.table,b.table)||order(canonical(a.key),canonical(b.key)));
const get=(rows,table,predicate=()=>true)=>{const found=rows.filter(r=>r.table===table&&predicate(r));same(found.length,1);return found[0];};
const head=r=>({reportId:r.reportId,reportVersion:r.reportVersion,contentDigest:r.contentDigest});
const stream=(r,w)=>({workspaceId:w,...Object.fromEntries(['runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest'].map(k=>[k,r[k]]))});
const ref=(kind,identity,digest)=>({key:{kind,identity},digest});
function reportRef(r,s){return ref('research-report',{workspaceId:s.workspaceId,...Object.fromEntries(['runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest','sidecarGeneration','launchGeneration','reportId','reportVersion'].map(k=>[k,r[k]]))},r.contentDigest);}
const sourceRef=s=>ref('assignment-evidence-source',s,domain('orkworks.assignment-evidence-source.v1',s));
function projection(r){
 const b=clone(r.body);
 if(!['store_header','commit_proof'].includes(r.table))delete b.lastCommit;
 if(r.table==='summary_state'&&b.checkpoint)delete b.checkpoint.applied;
 if(r.table==='pending_material')delete b.origin;
 return b;
}
const rowDigest=r=>domain('orkworks.hierarchy-storage-row.v3',{table:r.table,key:r.key,body:projection(r)});
const inventoryDigest=rows=>hash(canonical(sorted(rows)));
const makeRow=(table,key,body)=>({table,key:clone(key),body:clone(body)});
function linkShape(c){exact(c,'storageEpoch commitSequence changeDigest');digest(c.storageEpoch);digest(c.changeDigest);assert(counter(c.commitSequence)>0n);}
function seedData(seed){
 verifyR2(seed);
 const A=seed.approvedContext.assignmentIdentity, result=JSON.parse(seed.result.canonicalUtf8), evaluation=JSON.parse(seed.evaluation.canonicalUtf8);
 const er=JSON.parse(seed.evaluationHead.canonicalUtf8), rr=JSON.parse(seed.acceptedHead.canonicalUtf8);
 return {A,result,evaluation,E:ref('assignment-evaluation',er,er.evaluationDigest),R:ref('assignment-result',rr,rr.resultDigest),
  M:JSON.parse(seed.materialLineage[2].canonicalUtf8).state.material,
  W:{kind:'workspace',workspaceId:A.workspaceId},U:{kind:'run',workspaceId:A.workspaceId,runId:A.runId}};
}
function usage(rows){
 const ss=get(rows,'summary_state').body,pending=rows.filter(r=>r.table==='pending_material');
 return {'report-retained-bytes':rows.filter(r=>['report_heads','report_receipts'].includes(r.table)||r.table==='objects'&&r.key.kind==='research-report').reduce((n,r)=>n+bytes(r.key)+bytes(projection(r)),0),
  'preparation-retained-bytes':rows.filter(r=>r.table==='run_states').reduce((n,r)=>n+bytes(r.key)+bytes(projection(r)),0),
  'summary-row-bytes':ss.row?bytes(ss.row):0,'summary-row-count':ss.row?1:0,
  'summary-lineage-bytes':(ss.checkpoint?bytes(ss.checkpoint.material):0)+pending.reduce((n,r)=>n+bytes(r.body.before)+bytes(r.body.after),0),
  'summary-pending-count':pending.length};
}
function projectedMaterial(rows,data){
 const sourceObjects=rows.filter(r=>r.table==='objects'&&r.key.kind==='assignment-evidence-source');
 const current=new Map(sourceObjects.map(r=>{
  const s=r.body.body,i=s.reportIdentity;
  const h=get(rows,'report_heads',h=>canonical(h.key)===canonical(stream(i,i.workspaceId))).body.head;
  return [r.body.digest,h.reportVersion===i.reportVersion&&h.reportId===i.reportId&&h.contentDigest===s.contentDigest];
 }));
 const material=clone(data.M);
 for(const o of material.requiredOutcomes)if(o.target.kind==='criterion'){
  const source=data.evaluation.criterionOutcomes.find(x=>x.criterionId===o.target.id);
  o.disposition=source.evidenceReferences.every(k=>current.get(k)===true)?'needs-rework':'unassessed';
 }
 material.requiredFindings=material.requiredFindings.filter(f=>current.get(data.evaluation.findings.find(x=>x.id===f.findingId).evidenceReference));
 assert(material.requiredOutcomes.some(o=>o.disposition==='needs-rework'),'retained C2 failure');
 return {material,current};
}
function validate(rows,seed,ctx,data){
 assert(Array.isArray(rows));same(rows,sorted(rows));same(new Set(rows.map(key)).size,rows.length);
 const {A,R,E,W,U}=data;
 const allowed=new Set(['objects','heads','report_heads','report_receipts','dependencies','run_states','summary_state','pending_material','quota_usage','store_header','commit_proof']);
 for(const r of rows){exact(r,'table key body');assert(allowed.has(r.table),'closed profile registry');}
 const header=get(rows,'store_header');same(header.key,1);exact(header.body,'schemaVersion workspaceId storageEpoch latest');same(header.body.schemaVersion,3);same(header.body.workspaceId,A.workspaceId);digest(header.body.storageEpoch);linkShape(header.body.latest);same(header.body.latest.storageEpoch,header.body.storageEpoch);
 const proofFor=c=>{
  linkShape(c);same(c.storageEpoch,header.body.storageEpoch);assert(counter(c.commitSequence)<=counter(header.body.latest.commitSequence));
  const p=get(rows,'commit_proof',r=>r.key===c.commitSequence);exact(p.body,'body changeDigest');
  same(p.body.changeDigest,c.changeDigest);same(domain('orkworks.hierarchy-storage-change.v3',p.body.body),c.changeDigest);
  return p.body.body;
 };
 proofFor(header.body.latest);
 for(const r of rows){
  if(r.table==='commit_proof'){
   const m=r.body.body;exact(m,'schemaVersion workspaceId storageEpoch commitSequence predecessor commandKind changes');same(m.schemaVersion,3);same(m.workspaceId,A.workspaceId);same(m.storageEpoch,header.body.storageEpoch);same(m.commitSequence,r.key);
   same(domain('orkworks.hierarchy-storage-change.v3',m),r.body.changeDigest);assert(counter(r.key)>0n&&counter(r.key)<=counter(header.body.latest.commitSequence));
   assert(['subject-preparation','report-publication'].includes(m.commandKind));
   const keys=m.changes.map(c=>{exact(c,'table key beforeDigest afterDigest');assert(c.beforeDigest!==c.afterDigest);if(c.beforeDigest!==null)digest(c.beforeDigest);if(c.afterDigest!==null)digest(c.afterDigest);return c.table+'\n'+canonical(c.key);});
   same(keys,[...keys].sort(order));same(keys.length,new Set(keys).size);
   if(m.predecessor){same(counter(m.predecessor.commitSequence)+1n,counter(m.commitSequence));proofFor(m.predecessor);}else same(m.commitSequence,'1');
  }else if(r.table!=='store_header'){
   const c=r.table==='pending_material'?r.body.origin:r.body.lastCommit;
   const m=proofFor(c),change=get(m.changes,r.table,x=>canonical(x.key)===canonical(r.key));same(change.afterDigest,rowDigest(r));
  }
 }
 const run=get(rows,'run_states');same(run.key,{workspaceId:A.workspaceId,runId:A.runId});exact(run.body,'schemaVersion terminal lastCommit');same(run.body.schemaVersion,1);
 const t=run.body.terminal;exact(t,'workspaceId runId parentSessionId admissionEpoch runSequence runDefinitionDigest bootstrapConfigurationDigest state runVersion inputRevision inputDigest plans currentPlan currentParentRuntimeGeneration blockers stageContinueRequired pendingClarification');
 same(t.workspaceId,A.workspaceId);same(t.runId,A.runId);same(t.parentSessionId,A.parentSessionId);id(t.workspaceId);id(t.runId);id(t.parentSessionId);digest(t.admissionEpoch);assert(counter(t.runSequence,9223372036854775807n)>0n);same(t.runId,t.admissionEpoch+'-'+t.runSequence);
 for(const k of ['runDefinitionDigest','bootstrapConfigurationDigest','inputDigest'])digest(t[k]);same(t.bootstrapConfigurationDigest,A.bootstrapConfigurationDigest);
 assert(['completed','cancelled'].includes(t.state));integer(t.runVersion);integer(t.inputRevision,64);assert(t.plans.length>=1&&t.plans.length<=16);same(new Set(t.plans.map(p=>p.planId)).size,t.plans.length);
 for(const p of t.plans){exact(p,'planId planRevision planDefinitionDigest');id(p.planId);integer(p.planRevision,64);digest(p.planDefinitionDigest);}
 same(t.currentPlan,null);same(t.currentParentRuntimeGeneration,null);same(t.blockers,[]);same(t.stageContinueRequired,false);same(t.pendingClarification,null);assert(bytes(run.key)+bytes(run.body)<=8192);
 const reportObjects=rows.filter(r=>r.table==='objects'&&r.key.kind==='research-report');
 for(const r of rows.filter(r=>r.table==='objects')){
  exact(r.key,'kind identity');exact(r.body,'body digest lastCommit');digest(r.body.digest);
  if(r.key.kind==='research-report'){
   const report=r.body.body,p=seed.profiles.find(p=>canonical(p.stream)===canonical(stream(report,A.workspaceId)));assert(p);validateReport(report,p);
   same(r.key,reportRef(report,p.stream).key);same(r.body.digest,report.contentDigest);
   assert(t.plans.some(p=>p.planId===report.planId&&p.planRevision===report.planRevision));
  }else if(r.key.kind==='assignment-evidence-source'){
   const s=r.body.body;assert(s.sourceKind==='research-report');const report=get(reportObjects,'objects',r=>canonical(r.key.identity)===canonical(s.reportIdentity)).body.body;
   const p=seed.profiles.find(p=>canonical(p.stream)===canonical(stream(report,A.workspaceId)));assert(p);validateReportEvidenceSource(s,report,p,r.body.digest);same(r.key.identity,s);
  }else if(r.key.kind==='assignment-result'){same(r.key,R.key);same(r.body.body,data.result);same(r.body.digest,R.digest);}
  else if(r.key.kind==='assignment-evaluation'){same(r.key,E.key);same(r.body.body,data.evaluation);same(r.body.digest,E.digest);}
  else assert.fail('unsupported consumer/source');
 }
 get(rows,'objects',r=>canonical(r.key)===canonical(R.key));get(rows,'objects',r=>canonical(r.key)===canonical(E.key));
 const expectedHeads=[{key:{kind:'assignment-result',identity:A},target:R},{key:{kind:'assignment-evaluation',identity:Object.fromEntries(['assignmentIdentity','resultRevision','resultDigest','reviewerIdentity'].map(k=>[k,E.key.identity[k]]))},target:E}];
 same(rows.filter(r=>r.table==='heads').map(r=>({key:r.key,target:r.body.target})).sort((a,b)=>order(canonical(a.key),canonical(b.key))),expectedHeads.sort((a,b)=>order(canonical(a.key),canonical(b.key))));
 for(const r of rows.filter(r=>r.table==='heads'))exact(r.body,'target lastCommit');
 same(rows.filter(r=>r.table==='report_heads').length,seed.profiles.length);
 const receiptRows=rows.filter(r=>r.table==='report_receipts');
 for(const p of seed.profiles){
  const h=get(rows,'report_heads',r=>canonical(r.key)===canonical(p.stream));exact(h.body,'schemaVersion head lastCommit');same(h.body.schemaVersion,1);exact(h.body.head,'reportId reportVersion contentDigest');integer(h.body.head.reportVersion,8);id(h.body.head.reportId);digest(h.body.head.contentDigest);assert(bytes(h.body)<=512);
  const rs=reportObjects.filter(r=>canonical(stream(r.body.body,A.workspaceId))===canonical(p.stream)).sort((a,b)=>a.body.body.reportVersion-b.body.body.reportVersion);
  same(rs.map(r=>r.body.body.reportVersion),Array.from({length:h.body.head.reportVersion},(_,i)=>i+1));same(head(rs.at(-1).body.body),h.body.head);
  for(const r of rs){
   const report=r.body.body;requestForReceipt(rows,p.stream,report);same(report.reportId,h.body.head.reportId);
  }
 }
 same(receiptRows.length,reportObjects.length);
 const expectedEdges=[];
 for(const r of rows.filter(r=>r.table==='objects'&&r.key.kind==='assignment-evidence-source')){
  const source=r.body.body,report=get(reportObjects,'objects',r=>canonical(r.key.identity)===canonical(source.reportIdentity));
  expectedEdges.push({consumer:sourceRef(source),target:{key:report.key,digest:report.body.digest},purpose:'evaluation-evidence'});
 }
 const cited=[...data.evaluation.criterionOutcomes,...data.evaluation.qualityDimensionOutcomes,data.evaluation.rating].flatMap(x=>x.evidenceReferences).concat(data.evaluation.findings.map(f=>f.evidenceReference));
 for(const k of new Set(cited)){
  const r=get(rows,'objects',r=>r.key.kind==='assignment-evidence-source'&&r.body.digest===k);expectedEdges.push({consumer:E,target:{key:r.key,digest:r.body.digest},purpose:'evaluation-evidence'});
 }
 const edges=rows.filter(r=>r.table==='dependencies');same(edges.map(r=>r.key).sort((a,b)=>order(canonical(a),canonical(b))),expectedEdges.sort((a,b)=>order(canonical(a),canonical(b))));
 for(const r of edges)exact(r.body,'lastCommit');
 const ss=get(rows,'summary_state');same(ss.key,A);exact(ss.body,'subjectRevision row checkpoint admission lastCommit');assert(counter(ss.body.subjectRevision)>0n);
 const pending=rows.filter(r=>r.table==='pending_material'),project=projectedMaterial(rows,data).material;
 if(ss.body.admission==='never-initialized'){same(ss.body.row,null);same(ss.body.checkpoint,null);same(pending,[]);}
 else{
  same(ss.body.admission,'initialized');const s=ss.body.row;exact(s,'schemaVersion assignmentIdentity summaryRevision state');same(s.schemaVersion,1);same(s.assignmentIdentity,A);assert(counter(s.summaryRevision,2147483647n)>0n);
  const cp=ss.body.checkpoint;exact(cp,'material summaryRevision applied');assert(counter(cp.summaryRevision,2147483647n)>0n);proofFor(cp.applied);
  if(s.state.kind==='current'){exact(s.state,'kind material');same(s.state.material,project);same(cp.material,project);same(cp.summaryRevision,s.summaryRevision);same(pending,[]);}
  else{
   exact(s.state,'kind reason');same(s.state.kind,'unavailable');assert(['reconciling','exhausted'].includes(s.state.reason));
   if(s.state.reason==='exhausted'){same(s.summaryRevision,'2147483647');same(cp.summaryRevision,'2147483647');same(pending,[]);}
   else{
    assert(pending.length>0);let material=cp.material,revision=counter(cp.summaryRevision);same(s.summaryRevision,cp.summaryRevision);
    for(const r of [...pending].sort((a,b)=>Number(counter(a.key.assignedRevision)-counter(b.key.assignedRevision)))){
     exact(r.key,'assignmentIdentity assignedRevision');same(r.key.assignmentIdentity,A);exact(r.body,'before after origin');same(counter(r.key.assignedRevision),revision+1n);same(r.body.before,material);assert(canonical(r.body.before)!==canonical(r.body.after));material=r.body.after;revision++;
    }
    same(material,project);
   }
  }
  assert(bytes(s)<=2097152&&bytes(cp.material)<=2097152);
 }
 const used=usage(rows),expectedQuotaKeys=[];
 for(const resource of ['report-retained-bytes','preparation-retained-bytes'])for(const owner of [U,W])expectedQuotaKeys.push({owner,resource});
 for(const resource of ['summary-row-bytes','summary-row-count','summary-lineage-bytes','summary-pending-count'])expectedQuotaKeys.push({owner:W,resource});
 const quota=rows.filter(r=>r.table==='quota_usage');same(quota.map(r=>r.key).sort((a,b)=>order(canonical(a),canonical(b))),expectedQuotaKeys.sort((a,b)=>order(canonical(a),canonical(b))));
 for(const r of quota){exact(r.body,'used reserved lastCommit');counter(r.body.used);same(r.body.used,String(used[r.key.resource]));same(r.body.reserved,'0');}
 assert(used['report-retained-bytes']<=268435456);assert(used['report-retained-bytes']+used['preparation-retained-bytes']+Number(counter(ctx.nonReportPreparationBytes))<=33554432);
 assert(used['summary-row-bytes']<=268435456&&used['summary-row-count']<=4096&&used['summary-lineage-bytes']<=67108864&&used['summary-pending-count']<=4096);
 return {header,run,ss,used,project};
}
function requestForReceipt(rows,s,report){
 const r=get(rows,'report_receipts',r=>canonical(r.key)===canonical({stream:s,reportVersion:report.reportVersion}));
 exact(r.body,'schemaVersion operation expectedTaskVersion expectedRunVersion expectedReportHead requestDigest response lastCommit');same(r.body.schemaVersion,1);assert(bytes(r.key)+bytes(r.body)<=2048);
 const b=r.body;exact(b.response,'schemaVersion acceptedHead');same(b.response,{schemaVersion:1,acceptedHead:head(report)});
 if(report.author.kind==='child'){same(b.operation,'submitResearchReport');integer(b.expectedTaskVersion);same(b.expectedRunVersion,null);}
 else{same(b.operation,'correctResearchReport');same(b.expectedTaskVersion,null);integer(b.expectedRunVersion);assert(report.reportVersion>1);}
 if(report.reportVersion===1)same(b.expectedReportHead,null);
 else{
  const prior=get(rows,'objects',r=>r.key.kind==='research-report'&&canonical(stream(r.body.body,s.workspaceId))===canonical(s)&&r.body.body.reportVersion===report.reportVersion-1).body.body;
  same(b.expectedReportHead,head(prior));
 }
 const req={schemaVersion:1,operation:b.operation,stream:s,expectedTaskVersion:b.expectedTaskVersion,expectedRunVersion:b.expectedRunVersion,expectedReportHead:b.expectedReportHead,report};
 same(domain('orkworks.research-report-request.v1',req),b.requestDigest);return req;
}
export function validatePublicationState(rows,seed,ctx){return validate(rows,seed,ctx,seedData(seed));}
class Fault extends Error{constructor(outcome){super(outcome);this.outcome=outcome;}}
function need(condition,outcome){if(!condition)throw new Fault(outcome);}
// Count every inventory-row read through the assay's query arrays. Repeated
// scans and final validation share this counter; this is not SQLite VM/I/O work.
function observeRows(rows,meter){
 return new Proxy(rows,{get(target,property,receiver){
  const value=Reflect.get(target,property,receiver);
  if(typeof property==='string'&&/^(0|[1-9][0-9]*)$/.test(property)&&value?.table){
   meter.count++;need(meter.count<=meter.limit,'limit-exceeded');
  }
  if(['filter','slice','concat'].includes(property))return (...args)=>observeRows(Array.prototype[property].apply(receiver,args),meter);
  return value;
 }});
}
function cap(ctx,name,maximum){const n=ctx[name]??maximum;assert(Number.isSafeInteger(n)&&n>=0&&n<=maximum);return n;}
export function executePublication(before,req,seed,ctx){
 const original=clone(before),oldDigest=inventoryDigest(before);
 const meter={count:0,limit:cap(ctx,'visitedLimit',100000)};
 const base={rows:original,invalidatedRuns:[],snapshotVersion:ctx.snapshotVersion};
 const failure=outcome=>({...base,oracle:{outcome,beforeStateDigest:oldDigest,afterStateDigest:oldDigest,commit:null,response:null}});
 try{
  need(ctx.authority===true&&ctx.epochCurrent===true,'denied');need(ctx.rateAdmitted===true,'limit-exceeded');
  need(ctx.sourceScope===true&&ctx.terminationSettled===true&&ctx.noUnsupportedConsumers===true,'unavailable');
  const data=seedData(seed);let checked;
  before=observeRows(before,meter);
  try{checked=validate(before,seed,ctx,data);}catch(e){if(e instanceof Fault)throw e;return failure('unavailable');}
  need(before.length<=cap(ctx,'visitedLimit',100000),'limit-exceeded');
  let p;
  try{
   exact(req,'schemaVersion operation stream expectedTaskVersion expectedRunVersion expectedReportHead report');same(req.schemaVersion,1);same(req.operation,'correctResearchReport');same(req.expectedTaskVersion,null);integer(req.expectedRunVersion);
   p=seed.profiles.find(p=>canonical(p.stream)===canonical(req.stream));assert(p);validateReport(req.report,p);same(req.report.author,{kind:'user'});assert(bytes(req)<=32768);
  }catch{return failure('denied');}
  const saved=before.find(r=>r.table==='report_receipts'&&canonical(r.key)===canonical({stream:req.stream,reportVersion:req.report.reportVersion}));
  if(saved){
   const report=get(before,'objects',r=>r.key.kind==='research-report'&&canonical(stream(r.body.body,data.A.workspaceId))===canonical(req.stream)&&r.body.body.reportVersion===req.report.reportVersion).body.body;
   need(canonical(requestForReceipt(before,req.stream,report))===canonical(req),'conflict');
   return {...base,oracle:{outcome:'retry',beforeStateDigest:oldDigest,afterStateDigest:oldDigest,commit:null,response:clone(saved.body.response)}};
  }
  const current=get(before,'report_heads',r=>canonical(r.key)===canonical(req.stream)).body.head;
  need(canonical(req.expectedReportHead)===canonical(current)&&req.expectedRunVersion===checked.run.body.terminal.runVersion,'conflict');
  need(req.report.reportId===current.reportId&&req.report.reportVersion===current.reportVersion+1,'conflict');
  const predecessor=get(before,'objects',r=>r.key.kind==='research-report'&&canonical(stream(r.body.body,data.A.workspaceId))===canonical(req.stream)&&r.body.body.reportVersion===current.reportVersion).body.body;
  need(req.report.sidecarGeneration===predecessor.sidecarGeneration&&req.report.launchGeneration===predecessor.launchGeneration&&canonical(req.report.questionIds)===canonical(predecessor.questionIds),'conflict');
  need(checked.run.body.terminal.runVersion<2147483647,'limit-exceeded');
  const next=observeRows(clone(original),meter),run=get(next,'run_states');run.body.terminal.runVersion++;
  get(next,'report_heads',r=>canonical(r.key)===canonical(req.stream)).body.head=head(req.report);
  const reference=reportRef(req.report,req.stream);
  next.push(makeRow('objects',reference.key,{body:req.report,digest:reference.digest,lastCommit:null}),makeRow('report_receipts',{stream:req.stream,reportVersion:req.report.reportVersion},{schemaVersion:1,operation:req.operation,expectedTaskVersion:null,expectedRunVersion:req.expectedRunVersion,expectedReportHead:req.expectedReportHead,requestDigest:domain('orkworks.research-report-request.v1',req),response:{schemaVersion:1,acceptedHead:head(req.report)},lastCommit:null}));
  const oldProjection=projectedMaterial(before,data),newProjection=projectedMaterial(next,data),ss=get(next,'summary_state').body;
  if(canonical([...oldProjection.current])!==canonical([...newProjection.current]))ss.subjectRevision=increment(ss.subjectRevision,18446744073709551615n);
  if(ss.row&&canonical(oldProjection.material)!==canonical(newProjection.material)){
   if(ss.row.state.kind==='unavailable'&&ss.row.state.reason==='exhausted'||ss.row.summaryRevision==='2147483647')ss.row.state={kind:'unavailable',reason:'exhausted'};
   else if(ctx.currentRowCapacity!==false&&ss.row.state.kind==='current'){
    const revision=increment(ss.row.summaryRevision,2147483647n);ss.row.summaryRevision=revision;ss.row.state={kind:'current',material:newProjection.material};ss.checkpoint={material:newProjection.material,summaryRevision:revision,applied:'NEW'};
   }else{
    const pending=next.filter(r=>r.table==='pending_material'),prior=pending.length?pending.at(-1).key.assignedRevision:ss.checkpoint.summaryRevision;
    ss.row.state={kind:'unavailable',reason:'reconciling'};
    next.push(makeRow('pending_material',{assignmentIdentity:data.A,assignedRevision:increment(prior,2147483647n)},{before:oldProjection.material,after:newProjection.material,origin:null}));
   }
  }
  const used=usage(next);
  need(used['report-retained-bytes']+used['preparation-retained-bytes']+Number(counter(ctx.nonReportPreparationBytes))<=cap(ctx,'maxRunBytes',33554432),'limit-exceeded');
  need(used['report-retained-bytes']<=cap(ctx,'maxWorkspaceReportBytes',268435456),'limit-exceeded');
  need(used['summary-lineage-bytes']<=cap(ctx,'maxLineageBytes',67108864)&&used['summary-pending-count']<=4096&&used['summary-row-bytes']<=268435456,'limit-exceeded');
  for(const r of next.filter(r=>r.table==='quota_usage'))r.body.used=String(used[r.key.resource]);
  const old=new Map(before.map(r=>[key(r),r])),changed=sorted(next.filter(r=>!old.has(key(r))||canonical(projection(r))!==canonical(projection(old.get(key(r))))));
  const sequence=increment(checked.header.body.latest.commitSequence,18446744073709551615n);
  const manifest={schemaVersion:3,workspaceId:data.A.workspaceId,storageEpoch:checked.header.body.storageEpoch,commitSequence:sequence,predecessor:checked.header.body.latest,commandKind:'report-publication',changes:changed.map(r=>({table:r.table,key:r.key,beforeDigest:old.has(key(r))?rowDigest(old.get(key(r))):null,afterDigest:rowDigest(r)}))};
  const commit={storageEpoch:manifest.storageEpoch,commitSequence:sequence,changeDigest:domain('orkworks.hierarchy-storage-change.v3',manifest)};
  for(const r of changed){
   if(r.table==='pending_material')r.body.origin=commit;else r.body.lastCommit=commit;
   if(r.table==='summary_state'&&r.body.checkpoint?.applied==='NEW')r.body.checkpoint.applied=commit;
  }
  const final=sorted(next.filter(r=>r.table!=='store_header').concat([makeRow('store_header',1,{schemaVersion:3,workspaceId:data.A.workspaceId,storageEpoch:manifest.storageEpoch,latest:commit}),makeRow('commit_proof',sequence,{body:manifest,changeDigest:commit.changeDigest})]));
  const actual=final.filter(r=>!old.has(key(r))||canonical(r.body)!==canonical(old.get(key(r)).body));
  const measurement={commit,changedRows:actual.length,commandBytes:actual.reduce((n,r)=>n+bytes(r.key)+bytes(r.body),0)};
  need(measurement.commandBytes<=cap(ctx,'maxCommandBytes',16777216)&&measurement.changedRows<=cap(ctx,'maxChangedRows',4096),'limit-exceeded');
  validate(observeRows(final,meter),seed,ctx,data);
  // This bounded row assay has one source/consumer run. No foreign owner is inferred.
  same(ctx.ownerRuns,[{workspaceId:data.A.workspaceId,runId:data.A.runId}]);integer(ctx.snapshotVersion);need(ctx.snapshotVersion<2147483647,'limit-exceeded');
  return {rows:final,measurement,logicalRowsVisited:meter.count,invalidatedRuns:clone(ctx.ownerRuns),snapshotVersion:ctx.snapshotVersion+1,oracle:{outcome:'accepted',beforeStateDigest:oldDigest,afterStateDigest:inventoryDigest(final),commit,response:{schemaVersion:1,acceptedHead:head(req.report)}}};
 }catch(e){return failure(e instanceof Fault?e.outcome:'unavailable');}
}
export function verifyPublicationCase(c,before,packet,seed){
 same(packet.schemaVersion,1);same(packet.evidenceKind,'conditional-terminal-logical-publication-not-admission-authority-or-native-durability');
 const result=executePublication(before,c.request,seed,{...clone(packet.capturedContext),currentRowCapacity:c.currentRowCapacity,snapshotVersion:c.snapshotVersion});
 same(result.oracle.outcome,'accepted');same(result.snapshotVersion,c.snapshotVersion+1);same(result.invalidatedRuns,packet.capturedContext.ownerRuns);same(result.rows,c.after);same(result.oracle,c.oracle);same(result.measurement,c.measurement);return true;
}
export function detailReadStatus(captured,live,proofs){return !proofs?'unavailable':captured!==live?'stale-reference':'current';}
