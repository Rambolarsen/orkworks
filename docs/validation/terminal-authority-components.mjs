// Offline documentation model only. Captured predicates are NOT tokens, leases, rates or SQL proof.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {canonical} from './report-publication-components.mjs';
import {validateReport} from './r2-report-components.mjs';
const clone=structuredClone;
const MAX=18446744073709551615n;
const bytes=x=>Buffer.byteLength(canonical(x));
const same=(a,b)=>assert.deepEqual(a,b);
const exact=(x,keys)=>{assert(x&&typeof x==='object'&&!Array.isArray(x));same(Object.keys(x).sort(),keys.split(' ').sort());};
const digest=x=>assert(typeof x==='string'&&/^[a-f0-9]{64}$/.test(x));
const hash=x=>createHash('sha256').update(x).digest('hex');
const domain=(d,x)=>hash(d+'\n'+canonical(x));
const id=x=>assert(typeof x==='string'&&/^[A-Za-z0-9._-]{1,128}$/.test(x));
const integer=(x,max=2147483647)=>assert(Number.isSafeInteger(x)&&x>0&&x<=max);
const counter=(x,positive=false)=>{assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(x));const n=BigInt(x);assert(n<=MAX&&(!positive||n>0n));return n;};
function scopeShape(s){exact(s,'workspaceId sidecarGeneration scopeSequence');id(s.workspaceId);digest(s.sidecarGeneration);counter(s.scopeSequence,true);}
function decode(raw,limit){assert(typeof raw==='string'&&Buffer.byteLength(raw)<=limit);const x=JSON.parse(raw);same(raw,canonical(x));return x;}
function snapshot(s,scope){return {schemaVersion:1,scope:clone(scope),closedThrough:s.closedThrough,openScopeSequence:s.openScopeSequence,unsettledScopeSequence:s.activeDecision?.scope.scopeSequence??null};}
function result(state,status,scope){return {state:clone(state),status,...(scope?{snapshot:snapshot(state,scope)}:{})};}
const sameOwner=(s,scope)=>s.workspaceId===scope.workspaceId&&s.sidecarGeneration===scope.sidecarGeneration;
export function initialOwner(binding){exact(binding,'workspaceId sidecarGeneration');id(binding.workspaceId);digest(binding.sidecarGeneration);return {...clone(binding),closedThrough:'0',openScopeSequence:null,lastAdmissionSequence:'0',activeDecision:null};}
export function control(state,raw,captured){
 if(captured.authenticated!==true||captured.originalOwner!==true)return result(state,'unauthorized');
 let q;try{q=decode(raw,4096);exact(q,'schemaVersion operation scope');same(q.schemaVersion,1);assert(['open','close','inspect'].includes(q.operation));scopeShape(q.scope);}catch{return result(state,'bad-request');}
 if(!sameOwner(state,q.scope))return result(state,'stale');
 if(q.operation==='close'?captured.closeSlotAvailable!==true:captured.controlSlotAvailable!==true)return result(state,'unavailable');
 if(q.operation!=='close'&&captured.readRateAdmitted!==true)return result(state,'limit-exceeded');
 const s=clone(state),n=counter(q.scope.scopeSequence,true),water=counter(s.closedThrough);
 if(q.operation==='inspect')return result(s,'ok',q.scope);
 if(q.operation==='close'){
  if(n>water+1n)return result(state,'conflict');
  if(n>water){s.closedThrough=String(n);if(s.openScopeSequence===String(n))s.openScopeSequence=null;}
  return result(s,'ok',q.scope);
 }
 if(n<=water)return result(state,'closed');
 if(n!==water+1n||s.openScopeSequence!==null&&s.openScopeSequence!==String(n))return result(state,'conflict');
 if(s.activeDecision&&s.activeDecision.scope.scopeSequence!==s.openScopeSequence)return result(state,'unavailable');
 if(captured.leaseCurrent!==true||captured.ownerHealthy!==true)return result(state,'unavailable');
 s.openScopeSequence=String(n);return result(s,'ok',q.scope);
}
function requestShape(raw,profile){
 const q=decode(raw,32768);exact(q,'schemaVersion operation stream expectedTaskVersion expectedRunVersion expectedReportHead report');
 same(q.schemaVersion,1);same(q.operation,'correctResearchReport');same(q.expectedTaskVersion,null);integer(q.expectedRunVersion);
 exact(q.stream,'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest');
 for(const k of ['workspaceId','runId','planId','taskId','reservationId','childSessionId'])id(q.stream[k]);
 integer(q.stream.planRevision,64);digest(q.stream.configurationDigest);
 exact(q.expectedReportHead,'reportId reportVersion contentDigest');id(q.expectedReportHead.reportId);integer(q.expectedReportHead.reportVersion,7);digest(q.expectedReportHead.contentDigest);
 same(q.stream,profile.stream);validateReport(q.report,profile);same(q.report.author,{kind:'user'});integer(q.report.reportVersion,8);
 same(q.report.reportVersion,q.expectedReportHead.reportVersion+1);same(q.report.reportId,q.expectedReportHead.reportId);digest(q.report.contentDigest);
 for(const k of ['runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest'])same(q.report[k],q.stream[k]);
 const body=clone(q.report);delete body.contentDigest;same(domain('orkworks.research-report.v1',body),q.report.contentDigest);
 return q;
}
export function admit(state,scope,requestUtf8,captured){
 if(captured.authenticated!==true||captured.originalOwner!==true)return result(state,'unauthorized');
 let request;try{scopeShape(scope);request=requestShape(requestUtf8,captured.reportProfile);}catch{return result(state,'bad-request');}
 if(!sameOwner(state,scope)||request.stream.workspaceId!==state.workspaceId)return result(state,'stale');
 if(state.openScopeSequence!==scope.scopeSequence||counter(scope.scopeSequence)!==counter(state.closedThrough)+1n)return result(state,'closed');
 if(state.activeDecision)return result(state,'busy');
 if(captured.rateAdmitted!==true)return result(state,'limit-exceeded');
 if(captured.sourceEligible!==true||captured.leaseCurrent!==true||captured.ownerHealthy!==true)return result(state,'unavailable');
 const last=counter(state.lastAdmissionSequence);if(last===MAX)return result(state,'exhausted');
 const decision={scope:clone(scope),admissionSequence:String(last+1n),requestUtf8,requestDigest:domain('orkworks.research-report-request.v1',request)};
 const s={...clone(state),lastAdmissionSequence:decision.admissionSequence,activeDecision:clone(decision)};
 return {...result(s,'admitted'),decision};
}
export function settle(state,decision,outcome,captured){
 if(!state.activeDecision||canonical(state.activeDecision)!==canonical(decision))return result(state,'conflict');
 if(outcome==='uncertain')return result(state,'unavailable');
 if(captured.originalOwner!==true||captured.leaseCurrent!==true)return result(state,'unavailable');
 if(outcome==='committed'?captured.publicationSettled!==true:outcome!=='rolled-back'||captured.rollbackProven!==true)return result(state,'unavailable');
 return result({...clone(state),activeDecision:null},'settled');
}
export function reconcilePending(pending,ack){
 const no={released:false};
 try{
  exact(ack,'schemaVersion scope closedThrough openScopeSequence unsettledScopeSequence');same(ack.schemaVersion,1);scopeShape(pending.scope);scopeShape(ack.scope);
  same(ack.scope,pending.scope);counter(ack.closedThrough);
  if(ack.openScopeSequence!==null)counter(ack.openScopeSequence,true);
  if(ack.unsettledScopeSequence!==null)counter(ack.unsettledScopeSequence,true);
  if(ack.openScopeSequence!==null&&counter(ack.openScopeSequence)!==counter(ack.closedThrough)+1n)return no;
  if(counter(ack.closedThrough)<counter(pending.scope.scopeSequence)||ack.unsettledScopeSequence!==null||ack.openScopeSequence===pending.scope.scopeSequence)return no;
  return {released:true,durableDisposition:'unknown'};
 }catch{return no;}
}
export function canDeliver(captured,current){
 try{
  exact(captured,'scope frameId requestId');exact(current,'scope frameId requestId open');
  scopeShape(captured.scope);scopeShape(current.scope);id(captured.frameId);id(current.frameId);id(captured.requestId);id(current.requestId);
  return current.open===true&&current.frameId===captured.frameId&&current.requestId===captured.requestId&&canonical(current.scope)===canonical(captured.scope);
 }catch{return false;}
}
function link(c){exact(c,'storageEpoch commitSequence changeDigest');digest(c.storageEpoch);digest(c.changeDigest);counter(c.commitSequence,true);}
function receiptJoin(row,requestUtf8,version,profile){
 exact(row,'table key body');same(row.table,'report_receipts');exact(row.key,'stream reportVersion');
 const b=row.body;exact(b,'schemaVersion operation expectedTaskVersion expectedRunVersion expectedReportHead requestDigest response lastCommit'+(version===2?' authorization':''));same(b.schemaVersion,version);link(b.lastCommit);
 const q=requestShape(requestUtf8,profile);same(row.key.stream,q.stream);same(row.key.reportVersion,q.report.reportVersion);
 for(const k of ['operation','expectedTaskVersion','expectedRunVersion','expectedReportHead'])same(b[k],q[k]);
 same(b.requestDigest,domain('orkworks.research-report-request.v1',q));
 same(b.response,{schemaVersion:1,acceptedHead:{reportId:q.report.reportId,reportVersion:q.report.reportVersion,contentDigest:q.report.contentDigest}});
 receiptUsage(row);return q;
}
export function rowCommitment(row){const body=clone(row.body);delete body.lastCommit;return domain('orkworks.hierarchy-storage-row.v3',{table:row.table,key:row.key,body});}
export function makeReceiptV2(original,requestUtf8,decision,profile){
 receiptJoin(original,requestUtf8,1,profile);exact(decision,'scope admissionSequence requestUtf8 requestDigest');scopeShape(decision.scope);counter(decision.admissionSequence,true);
 same(decision.requestUtf8,requestUtf8);same(decision.requestDigest,original.body.requestDigest);same(decision.scope.workspaceId,original.key.stream.workspaceId);
 const row=clone(original);row.body.schemaVersion=2;
 row.body.authorization={policy:'terminal-ui-correction-v1',sidecarGeneration:decision.scope.sidecarGeneration,scopeSequence:decision.scope.scopeSequence,admissionSequence:decision.admissionSequence};
 receiptJoin(row,requestUtf8,2,profile);return row;
}
export function validateReceiptV2(row,captured){
 for(const k of ['currentScopeAuthorized','rateAdmitted','sourceAccessQualified','originalProducerQualified'])same(captured[k],true);
 receiptJoin(row,captured.requestUtf8,2,captured.reportProfile);
 const a=row.body.authorization;exact(a,'policy sidecarGeneration scopeSequence admissionSequence');same(a.policy,'terminal-ui-correction-v1');digest(a.sidecarGeneration);counter(a.scopeSequence,true);counter(a.admissionSequence,true);
 same(rowCommitment(row),captured.originalRowCommitment);same(row.body.lastCommit,captured.originalCommit);
 const m=captured.originalManifest;exact(m,'schemaVersion workspaceId storageEpoch commitSequence predecessor commandKind changes');
 same(m.schemaVersion,3);same(m.workspaceId,row.key.stream.workspaceId);same(m.storageEpoch,row.body.lastCommit.storageEpoch);same(m.commitSequence,row.body.lastCommit.commitSequence);same(m.commandKind,'report-publication');
 link(m.predecessor);same(m.predecessor.storageEpoch,m.storageEpoch);same(counter(m.predecessor.commitSequence)+1n,counter(m.commitSequence));
 same(domain('orkworks.hierarchy-storage-change.v3',m),row.body.lastCommit.changeDigest);
 assert(Array.isArray(m.changes)&&m.changes.length>0&&m.changes.length<=4096);assert(bytes(m)<=16777216);
 const keys=m.changes.map(x=>{exact(x,'table key beforeDigest afterDigest');assert(typeof x.table==='string');if(x.beforeDigest!==null)digest(x.beforeDigest);if(x.afterDigest!==null)digest(x.afterDigest);assert(x.beforeDigest!==x.afterDigest);return x.table+'\n'+canonical(x.key);});
 same(keys,[...keys].sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))));same(new Set(keys).size,keys.length);
 const found=m.changes.filter(x=>x.table==='report_receipts'&&canonical(x.key)===canonical(row.key));same(found.length,1);same(found[0].afterDigest,rowCommitment(row));
 // Historical metadata never needs a current/live decision or current lifetime equality.
 return {responseUtf8:canonical(row.body.response),...receiptUsage(row)};
}

export function beginPending(existing,request){
 if(existing!==null)return {status:'busy',pending:clone(existing)};
 exact(request,'scope frameId requestId');scopeShape(request.scope);id(request.frameId);id(request.requestId);
 return {status:'pending',pending:clone(request)};
}

export function receiptUsage(row){
 const metadataBytes=bytes(row.key)+bytes(row.body);assert(metadataBytes<=2048,'receipt metadata cap');
 const body=clone(row.body);delete body.lastCommit;
 return {metadataBytes,retainedBytes:bytes(row.key)+bytes(body)};
}
