// Documentation-only source-codec and captured-transition assay.
// Never import into production: supplied context is not approval, custody,
// authentication/rate admission, atomic publication or runtime eligibility.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {isIP} from 'node:net';
import {decodeSummary,validateResponse} from './hierarchy-summary-components.mjs';
const order=(a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b));
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort(order).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const hash=x=>createHash('sha256').update(x).digest('hex');
const domain=(d,x)=>hash(d+'\n'+canonical(x));
const same=(a,b)=>assert.deepEqual(a,b,'exact captured/source join');
const exact=(x,fields)=>{assert(x!==null&&typeof x==='object'&&!Array.isArray(x),'closed object');same(Object.keys(x).sort(),fields.split(' ').sort());};
const integer=(x,min,max)=>assert(Number.isSafeInteger(x)&&!Object.is(x,-0)&&x>=min&&x<=max,'bounded integer');
const id=x=>assert(typeof x==='string'&&/^[A-Za-z0-9._-]{1,128}$/.test(x),'ASCII ID');
const digest=x=>assert(typeof x==='string'&&/^[a-f0-9]{64}$/.test(x),'SHA-256');
const text=(x,max=2048)=>assert(typeof x==='string'&&x.isWellFormed()&&Buffer.byteLength(x)>0&&Buffer.byteLength(x)<=max,'bounded UTF-8 text');
const list=(xs,max)=>assert(Array.isArray(xs)&&xs.length<=max,'bounded array');
const unique=xs=>same(new Set(xs).size,xs.length);
const ids=(xs,max,min=0)=>{list(xs,max);assert(xs.length>=min);xs.forEach(id);unique(xs);same(xs,[...xs].sort(order));};
const size=(x,max)=>assert(Buffer.byteLength(canonical(x))<=max,'serialized byte ceiling');
const one=xs=>{same(xs.length,1);return xs[0];};
const streamFields=['workspaceId','runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest'];
const reportStream=(r,workspaceId)=>({workspaceId,...Object.fromEntries(streamFields.filter(k=>k!=='workspaceId').map(k=>[k,r[k]]))});
const head=r=>({reportId:r.reportId,reportVersion:r.reportVersion,contentDigest:r.contentDigest});
const savedKey=(stream,rid,version)=>canonical([stream,rid,version]);
function validateStream(s){exact(s,streamFields.join(' '));for(const k of streamFields){if(k==='planRevision')integer(s[k],1,64);else if(k==='configurationDigest')digest(s[k]);else id(s[k]);}}
function validateHead(h){exact(h,'reportId reportVersion contentDigest');id(h.reportId);integer(h.reportVersion,1,8);digest(h.contentDigest);}

function uri(value){
  text(value);assert(/^[\x21-\x7e]+$/.test(value)&&!value.includes('\\'),'ASCII URI, no space/control/backslash');
  const match=/^([A-Za-z][A-Za-z0-9+.-]*):\/\/([^/?#]*)(.*)$/.exec(value);assert(match,'absolute URI authority');
  assert(['http','https'].includes(match[1].toLowerCase()));const authority=match[2];assert(authority&&!authority.includes('@'),'nonempty authority, no userinfo');
  let host,port;
  if(authority.startsWith('[')){
    const end=authority.indexOf(']');assert(end>1);host=authority.slice(1,end);
    assert(!host.includes('%'),'IP-literal excludes zone identifiers');
    assert(isIP(host)===6 || /^[vV][0-9a-fA-F]+\.[A-Za-z0-9._~!$&'()*+,;=:]+$/.test(host),'IP-literal');
    const rest=authority.slice(end+1);assert(rest===''||rest.startsWith(':'));if(rest)port=rest.slice(1);
  }else{
    const index=authority.indexOf(':');assert(index===authority.lastIndexOf(':'),'bracket IPv6');
    host=index<0?authority:authority.slice(0,index);if(index>=0)port=authority.slice(index+1);
    assert(/^(?:[A-Za-z0-9._~!$&'()*+,;=-]|%[0-9a-fA-F]{2})+$/.test(host),'URI registered host');
  }
  if(port!==undefined){assert(/^[0-9]+$/.test(port),'decimal nonempty port');assert(BigInt(port)<=65535n,'port range');}
  let tail=match[3],fragment='',query='';const fragmentIndex=tail.indexOf('#');
  if(fragmentIndex>=0){fragment=tail.slice(fragmentIndex+1);tail=tail.slice(0,fragmentIndex);}
  const queryIndex=tail.indexOf('?');if(queryIndex>=0){query=tail.slice(queryIndex+1);tail=tail.slice(0,queryIndex);}
  assert(tail===''||tail.startsWith('/'),'authority path');
  assert(/^(?:[A-Za-z0-9._~!$&'()*+,;=:@/]|%[0-9a-fA-F]{2})*$/.test(tail),'URI path encoding');
  for(const part of [query,fragment])assert(/^(?:[A-Za-z0-9._~!$&'()*+,;=:@/?]|%[0-9a-fA-F]{2})*$/.test(part),'URI query/fragment encoding');
}
// Positive leap dates derived from IERS Leap_Second.dat, Bulletin 72 (July
// 2026), expiry 2027-06-28: https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat
// Unknown/future :60 dates require an explicit evidence refresh; no network I/O.
const knownLeapDates=new Set(('1972-06-30 1972-12-31 1973-12-31 1974-12-31 1975-12-31 1976-12-31 1977-12-31 1978-12-31 1979-12-31 '+
  '1981-06-30 1982-06-30 1983-06-30 1985-06-30 1987-12-31 1989-12-31 1990-12-31 1992-06-30 1993-06-30 1994-06-30 1995-12-31 '+
  '1997-06-30 1998-12-31 2005-12-31 2008-12-31 2012-06-30 2015-06-30 2016-12-31').split(' '));
function time(value){
  assert(typeof value==='string'&&value.length<=35);const m=/^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?(Z|\+00:00)$/.exec(value);assert(m,'UTC timestamp');
  const [y,month,day,hour,minute,second]=m.slice(1,7).map(Number);integer(month,1,12);
  const leap=y%4===0&&(y%100!==0||y%400===0),days=[31,leap?29:28,31,30,31,30,31,31,30,31,30,31][month-1];
  integer(day,1,days);integer(hour,0,23);integer(minute,0,59);integer(second,0,60);
  if(second===60)assert(hour===23&&minute===59&&knownLeapDates.has(value.slice(0,10)),'known UTC leap-second occurrence');
}
function reference(r,profile){
  assert(['file','url'].includes(r?.kind));exact(r,r.kind==='file'?'kind id path revision location excerptDigest':'kind id url retrievedAt excerptDigest');
  id(r.id);digest(r.excerptDigest);
  if(r.kind==='file'){
    text(r.path);text(r.revision);text(r.location);
    assert(!r.path.includes('\\')&&!r.path.includes('\0')&&!/^[A-Za-z]:/.test(r.path)&&r.path.split('/').every(x=>x&&x!=='.'&&x!=='..'),'root-relative path');
  }else{uri(r.url);time(r.retrievedAt);}
  size(r,1024);
  const capture=one(profile.captures.filter(c=>canonical(c.reference)===canonical(r)));
  assert(typeof capture.bytesBase64==='string'&&capture.bytesBase64.length<=2732,'bounded captured excerpt encoding');
  const bytes=Buffer.from(capture.bytesBase64,'base64');same(bytes.toString('base64'),capture.bytesBase64);
  assert(bytes.length>0&&bytes.length<=2048,'component excerpt 1..2048 bytes');same(hash(bytes),r.excerptDigest);
}
export function validateReport(r,profile){
  exact(r,'schemaVersion reportId reportVersion runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration questionIds findings uncertainties blockers evidenceReferences recommendations limitations author contentDigest');
  same(r.schemaVersion,1);id(r.reportId);integer(r.reportVersion,1,8);validateStream(profile.stream);same(reportStream(r,profile.stream.workspaceId),profile.stream);
  exact(profile.lineage,'sidecarGeneration launchGeneration');digest(r.sidecarGeneration);digest(r.launchGeneration);same({sidecarGeneration:r.sidecarGeneration,launchGeneration:r.launchGeneration},profile.lineage);
  ids(r.questionIds,32,1);same(r.questionIds,profile.questionIds);
  exact(r.author,'kind');assert(['child','user'].includes(r.author.kind));digest(r.contentDigest);
  list(r.evidenceReferences,64);unique(r.evidenceReferences.map(x=>x.id));same(r.evidenceReferences.map(x=>x.id),r.evidenceReferences.map(x=>x.id).sort(order));
  r.evidenceReferences.forEach(ref=>reference(ref,profile));
  list(r.findings,32);
  for(const f of r.findings){
    exact(f,'questionIds answer evidenceReferenceIds classification');ids(f.questionIds,32,1);assert(f.questionIds.every(q=>r.questionIds.includes(q)));
    text(f.answer);ids(f.evidenceReferenceIds,64);assert(f.evidenceReferenceIds.every(id=>r.evidenceReferences.some(ref=>ref.id===id)));
    assert(['established','inferred','unverified'].includes(f.classification));if(f.classification==='established')assert(f.evidenceReferenceIds.length>0);
    size(f,2048);
  }
  for(const key of ['uncertainties','blockers','recommendations','limitations']){list(r[key],16);r[key].forEach(x=>text(x));}
  assert(r.uncertainties.length+r.blockers.length<=16);assert(r.recommendations.length+r.limitations.length<=16);
  const {contentDigest,...body}=r;same(domain('orkworks.research-report.v1',body),contentDigest);size(r,32768);return contentDigest;
}
export function validateReportEvidenceSource(source,report,profile,sourceKey){
  validateReport(report,profile);
  const reportIdentity={workspaceId:profile.stream.workspaceId,...Object.fromEntries(['runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest','sidecarGeneration','launchGeneration','reportId','reportVersion'].map(k=>[k,report[k]]))};
  same(source,{schemaVersion:1,sourceKind:'research-report',reportIdentity,sizeBytes:Buffer.byteLength(canonical(report)),contentDigest:report.contentDigest});
  same(domain('orkworks.assignment-evidence-source.v1',source),sourceKey);return sourceKey;
}
export function emptyCapturedState(profiles){
  return {runVersion:1,heads:[],saved:[],taskVersions:profiles.map(p=>({stream:structuredClone(p.stream),version:1}))};
}
// Precondition is supplied captured context, not a live endpoint: real sender,
// generation capability, rate admission and durable accounting are unqualified.
export function applyCaptured(state,r,profile){
  exact(r,'schemaVersion operation stream expectedTaskVersion expectedRunVersion expectedReportHead report');same(r.schemaVersion,1);
  assert(['submitResearchReport','correctResearchReport'].includes(r.operation));validateStream(r.stream);same(r.stream,profile.stream);
  validateReport(r.report,profile);size(r,32768);
  const user=r.operation==='correctResearchReport';same(r.report.author,{kind:user?'user':'child'});
  if(user){same(r.expectedTaskVersion,null);integer(r.expectedRunVersion,1,2147483647);assert(r.expectedReportHead!==null);}
  else{integer(r.expectedTaskVersion,1,2147483647);same(r.expectedRunVersion,null);}
  if(r.expectedReportHead!==null){
    validateHead(r.expectedReportHead);
    const stored=one(state.saved.filter(s=>s.key===savedKey(r.stream,r.expectedReportHead.reportId,r.expectedReportHead.reportVersion)));
    const predecessor=JSON.parse(stored.canonicalRequest).report;same(head(predecessor),r.expectedReportHead);
    if(user){
      same(r.report.sidecarGeneration,predecessor.sidecarGeneration);same(r.report.launchGeneration,predecessor.launchGeneration);
      same(r.report.questionIds,predecessor.questionIds);
    }
  }
  const normalized=canonical(r),requestDigest=domain('orkworks.research-report-request.v1',r),key=savedKey(r.stream,r.report.reportId,r.report.reportVersion);
  const prior=state.saved.filter(s=>s.key===key);assert(prior.length<=1);
  if(prior.length){same(prior[0].canonicalRequest,normalized);same(prior[0].requestDigest,requestDigest);return {state,kind:'retry',acceptedHead:head(r.report),requestDigest};}
  const current=state.heads.filter(h=>canonical(h.stream)===canonical(r.stream));assert(current.length<=1);
  const currentHead=current[0]?.head??null;same(r.expectedReportHead,currentHead);
  if(user)same(r.expectedRunVersion,state.runVersion);
  else same(r.expectedTaskVersion,one(state.taskVersions.filter(t=>canonical(t.stream)===canonical(r.stream))).version);
  if(currentHead){same(r.report.reportId,currentHead.reportId);same(r.report.reportVersion,currentHead.reportVersion+1);}
  else{assert(!user);same(r.report.reportVersion,1);}
  integer(state.runVersion+1,1,2147483647);
  const next=structuredClone(state),index=next.heads.findIndex(h=>canonical(h.stream)===canonical(r.stream));
  if(index<0)next.heads.push({stream:structuredClone(r.stream),head:head(r.report)});else next.heads[index].head=head(r.report);
  next.saved.push({key,canonicalRequest:normalized,requestDigest});next.runVersion++;
  return {state:next,kind:'accepted',acceptedHead:head(r.report),requestDigest};
}
function read(entry,limit=524288){
  assert(typeof entry.canonicalUtf8==='string'&&entry.canonicalUtf8.isWellFormed());const bytes=Buffer.from(entry.canonicalUtf8);
  same(bytes.length,entry.utf8Bytes);same(hash(bytes),entry.sha256);const value=decodeSummary(bytes,limit);same(canonical(value),entry.canonicalUtf8);return value;
}
function validateEvaluation(e,a,resultRef,criteria,rubric,sources){
  exact(e,'schemaVersion assignmentIdentity resultRevision resultDigest reviewerIdentity reporterSource evaluationRevision predecessor criteriaSnapshotDigest rubricSnapshotDigest criterionOutcomes qualityDimensionOutcomes rating findings findingDisputes correctionReason reviewedEvaluationRef');
  same(e.schemaVersion,1);same(e.assignmentIdentity,a.assignmentIdentity);same(e.resultRevision,resultRef.resultRevision);same(e.resultDigest,resultRef.resultDigest);
  same(e.reviewerIdentity,{kind:'user',userReviewId:a.userReviewId});same(e.reporterSource,{kind:'user'});same(e.evaluationRevision,1);same(e.predecessor,{kind:'no-head'});
  same(e.criteriaSnapshotDigest,domain('orkworks.criteria-snapshot.v1',criteria));same(e.rubricSnapshotDigest,domain('orkworks.rubric-snapshot.v1',rubric));
  same(criteria.map(c=>[c.id,c.requirement]),[['c1','required'],['c2','required'],['o','optional']]);for(const c of criteria){exact(c,'id requirement description');text(c.description);}
  exact(rubric,'id version dimensions evaluatorRole');id(rubric.id);integer(rubric.version,1,2147483647);same(rubric.evaluatorRole,'review');
  same(rubric.dimensions.length,1);exact(rubric.dimensions[0],'id description');same(rubric.dimensions[0].id,'d');text(rubric.dimensions[0].description);
  same(e.criterionOutcomes.length,3);
  for(let i=0;i<criteria.length;i++){
    const c=criteria[i],o=e.criterionOutcomes[i];exact(o,'criterionId outcome rationale evidenceReferences');same(o.criterionId,c.id);text(o.rationale);
    same(o.outcome,c.id==='o'?'satisfied':'unsatisfied');same(o.evidenceReferences,[sources.get(c.id+'-v1').sourceKey]);
  }
  same(e.qualityDimensionOutcomes.length,1);const q=e.qualityDimensionOutcomes[0];exact(q,'dimensionId outcome rationale evidenceReferences');
  same(q.dimensionId,'d');same(q.outcome,'unassessed');text(q.rationale);same(q.evidenceReferences,[]);
  exact(e.rating,'rating rationale evidenceReferences');same(e.rating.rating,'unassessed');text(e.rating.rationale);same(e.rating.evidenceReferences,[]);
  same(e.findings.length,2);
  for(let i=0;i<2;i++){
    const f=e.findings[i],key='c'+(i+1);exact(f,'id description location severity evidenceReference requiresRework requiredImpactTargets');
    same(f.id,'f-'+key);text(f.description);text(f.location);assert(Buffer.byteLength(f.description)+Buffer.byteLength(f.location)<=2048);id(f.severity);
    same(f.evidenceReference,sources.get(key+'-v1').sourceKey);same(f.requiresRework,true);same(f.requiredImpactTargets,[{kind:'criterion',id:key}]);
  }
  same(e.findingDisputes,[]);same(e.correctionReason,null);same(e.reviewedEvaluationRef,null);
}
export function verifyR2(packet){
  same(packet.schemaVersion,1);same(packet.evidenceKind,'canonical-source-components-and-logical-R2-not-admission-or-runtime');
  const a=packet.approvedContext,criteria=read(a.criteria),rubric=read(a.rubric),candidate=read(packet.candidate),result=read(packet.result),e=read(packet.evaluation);
  exact(candidate,'schemaVersion assignmentIdentity predecessor authorityGeneration outputs');same(candidate.schemaVersion,1);same(candidate.assignmentIdentity,a.assignmentIdentity);
  same(candidate.predecessor,{kind:'no-head'});same(candidate.outputs,[]);exact(candidate.authorityGeneration,'kind sidecarGeneration generation');
  same(candidate.authorityGeneration.kind,'parent-runtime');digest(candidate.authorityGeneration.sidecarGeneration);digest(candidate.authorityGeneration.generation);
  const candidateDigest=domain('orkworks.assignment-result-candidate.v1',candidate);same(candidateDigest,packet.candidate.candidateDigest);
  same(result,{...candidate,resultRevision:1,candidateDigest,timeCostEvidence:[]});const resultDigest=domain('orkworks.assignment-result.v1',result);same(resultDigest,packet.result.resultDigest);
  const resultRef={assignmentIdentity:a.assignmentIdentity,resultRevision:1,resultDigest};same(read(packet.acceptedHead),resultRef);
  same(packet.profiles.map(p=>p.id),['c1','c2','o']);const profiles=new Map(packet.profiles.map(p=>[p.id,p]));
  for(const p of packet.profiles){same(p.stream.workspaceId,a.assignmentIdentity.workspaceId);same(p.stream.runId,a.assignmentIdentity.runId);}
  const reports=new Map();same(packet.reports.length,5);
  for(const entry of packet.reports){assert(!reports.has(entry.id));const r=read(entry,32768);validateReport(r,profiles.get(entry.profileId));reports.set(entry.id,{entry,report:r,profile:profiles.get(entry.profileId)});}
  const urlReport=read(packet.urlComponent,32768);validateReport(urlReport,profiles.get(packet.urlComponent.profileId));
  const sources=new Map();same(packet.evidenceSources.length,5);
  for(const entry of packet.evidenceSources){
    assert(!sources.has(entry.reportEntryId));const pair=reports.get(entry.reportEntryId);assert(pair);const r=pair.report;
    const source=read(entry);validateReportEvidenceSource(source,r,pair.profile,entry.sourceKey);
    sources.set(entry.reportEntryId,{...entry,source,profile:pair.profile});
  }
  validateEvaluation(e,a,resultRef,criteria,rubric,sources);const evaluationDigest=domain('orkworks.assignment-evaluation.v1',e);same(evaluationDigest,packet.evaluation.evaluationDigest);
  const evaluationRef={assignmentIdentity:a.assignmentIdentity,resultRevision:1,resultDigest,reviewerIdentity:e.reviewerIdentity,evaluationRevision:1,evaluationDigest};same(read(packet.evaluationHead),evaluationRef);
  const requests=new Map();same(packet.requests.length,5);
  for(const entry of packet.requests){assert(!requests.has(entry.id));const r=read(entry,32768);same(r.report,reports.get(entry.id).report);same(domain('orkworks.research-report-request.v1',r),entry.requestDigest);requests.set(entry.id,{value:r,profile:profiles.get(entry.profileId)});}
  let initial=emptyCapturedState(packet.profiles);
  for(const key of ['c1-v1','c2-v1','o-v1']){const r=requests.get(key);initial=applyCaptured(initial,r.value,r.profile).state;}
  same(initial.runVersion,4);same(packet.histories.length,2);const histories=[];
  const byKey=new Map([...sources.values()].map(s=>[s.sourceKey,s]));
  const status=(sourceKey,state)=>{
    const s=byKey.get(sourceKey);assert(s,'exact source key');const h=one(state.heads.filter(h=>canonical(h.stream)===canonical(s.profile.stream))).head;
    const current=one([...reports.values()].filter(r=>canonical(r.profile.stream)===canonical(s.profile.stream)&&canonical(head(r.report))===canonical(h)));
    return current.report.reportVersion===s.source.reportIdentity.reportVersion&&current.report.contentDigest===s.source.contentDigest?'current':'stale';
  };
  const project=state=>{
    const required=e.criterionOutcomes.filter(o=>criteria.find(c=>c.id===o.criterionId).requirement==='required');
    const outcomes=required.map(o=>{const current=o.evidenceReferences.every(k=>status(k,state)==='current');return {target:{kind:'criterion',id:o.criterionId},fullyAssessed:current,
      assessments:[{evaluationDigest,claimed:o.outcome,effective:current?o.outcome:'unassessed',participates:true,sourceState:current?'current':'stale',reasons:current?[]:['stale-evidence']}]};});
    for(const target of [{kind:'quality-dimension',id:'d'},{kind:'rating'}])outcomes.push({target,fullyAssessed:false,assessments:[{evaluationDigest,claimed:'unassessed',effective:'unassessed',participates:true,sourceState:'current',reasons:['declared-unassessed']}]});
    outcomes.sort((a,b)=>order(canonical(a.target),canonical(b.target)));
    const currentFindings=e.findings.filter(f=>status(f.evidenceReference,state)==='current');
    const findings=currentFindings.map(({id,description,location,severity,requiredImpactTargets})=>({reviewerIdentity:e.reviewerIdentity,findingId:id,description,location,severity,requiredImpactTargets,disposition:'current'}));
    assert(outcomes.some(o=>o.target.kind==='criterion'&&o.fullyAssessed&&o.assessments[0].effective==='unsatisfied'),'R2 retains independent C2 failure');
    const material={schemaVersion:1,assignmentIdentity:a.assignmentIdentity,accepted:{kind:'result',reference:resultRef},criteriaSnapshotDigest:e.criteriaSnapshotDigest,
      rubricSnapshotDigest:e.rubricSnapshotDigest,overall:'needs-rework',requiredOutcomes:outcomes.map(o=>({target:o.target,disposition:o.fullyAssessed?'needs-rework':'unassessed'})),requiredFindings:findings};
    return {outcomes,material,findingSources:findings.map(f=>({reviewerIdentity:e.reviewerIdentity,findingId:f.findingId,originEvaluationDigest:evaluationDigest,disputeEvaluationDigests:[]}))};
  };
  const baseline=project(initial),prepared=structuredClone(baseline.material);
  prepared.accepted={kind:'none'};prepared.overall='unassessed';prepared.requiredFindings=[];prepared.requiredOutcomes.forEach(o=>{o.disposition='unassessed';});
  const accepted={...structuredClone(prepared),accepted:{kind:'result',reference:resultRef}};
  same(packet.materialLineage.map(r=>r.stage),['prepared','accepted','evaluated']);
  for(const [i,material] of [prepared,accepted,baseline.material].entries()){
    const row=read(packet.materialLineage[i],2097152);
    same(row,{schemaVersion:1,assignmentIdentity:a.assignmentIdentity,summaryRevision:String(i+1),state:{kind:'current',material}});
  }
  same(packet.histories.map(h=>h.correctionRequestId),['c1-v2','o-v2']);
  const cited=[...e.criterionOutcomes.flatMap(o=>o.evidenceReferences),...e.qualityDimensionOutcomes.flatMap(o=>o.evidenceReferences),...e.rating.evidenceReferences,...e.findings.map(f=>f.evidenceReference)];
  for(const h of packet.histories){
    const correction=requests.get(h.correctionRequestId);assert(correction);const state=applyCaptured(initial,correction.value,correction.profile).state;
    same({runVersion:state.runVersion,heads:state.heads},h.expectedState);
    const sourceStates=Object.fromEntries(['c1','c2','o'].map(k=>[k,status(sources.get(k+'-v1').sourceKey,state)]));same(sourceStates,h.expectedSourceStates);
    const freshness=cited.every(k=>status(k,state)==='current')?'current':'historical';same(freshness,h.expectedRecordFreshness);
    const response=read(h.response,4194304),request=read(h.rendererRequest,4096);validateResponse(response,request,h.context);
    const projection=project(state),d=response.detail;same(d.criteria,criteria);same(d.rubric,rubric);same(d.evaluations,[evaluationRef]);
    same(d.outcomes,projection.outcomes);same(d.material,projection.material);same(d.findingSources,projection.findingSources);
    const token=canonical(projection.material)===canonical(baseline.material)?'3':'4';same(response.selection.summaryRevision,token);
    histories.push({id:h.id,sourceStates,freshness,requiredFindingIds:projection.material.requiredFindings.map(f=>f.findingId)});
  }
  return {evaluationDigest,histories};
}
