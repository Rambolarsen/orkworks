// Documentation-only display component checker. Never import into production.
// Synthetic producer decisions are inputs: this does not prove aggregation,
// source currentness, authentication, storage publication or browser rendering.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const order = (a,b) => Buffer.compare(Buffer.from(a),Buffer.from(b));
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const exact = (x,fields) => {
  assert(object(x),'record object');
  assert.deepEqual(Object.keys(x).sort(),fields.split(' ').sort(),'closed fields');
};
const oneOf = (x,values) => assert(values.includes(x),'supported variant');
const integer = (x,min,max) => assert(Number.isSafeInteger(x) && !Object.is(x,-0) && x>=min && x<=max,'integer bound');
const id = x => assert(typeof x==='string' && /^[A-Za-z0-9._-]{1,128}$/.test(x),'ASCII ID');
const digest = x => assert(typeof x==='string' && /^[a-f0-9]{64}$/.test(x),'lowercase digest');
const text = (x,max) => assert(typeof x==='string' && x.isWellFormed() && Buffer.byteLength(x)>0 && Buffer.byteLength(x)<=max,'bounded UTF-8 text');
const list = (xs,max) => assert(Array.isArray(xs) && xs.length<=max,'bounded array');
const unique = xs => assert.equal(new Set(xs).size,xs.length,'unique members');
const bool = x => assert.equal(typeof x,'boolean');
const canonical = x => Array.isArray(x) ? '['+x.map(canonical).join(',')+']' : object(x) ? '{'+Object.keys(x).sort(order).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}' : JSON.stringify(x);
const hash = (domain,x) => createHash('sha256').update(domain+'\n'+canonical(x)).digest('hex');
const same = (a,b) => assert.deepEqual(a,b,'exact binding');
const sorted = (xs,key=canonical) => {
  const keys=xs.map(key);unique(keys);same(keys,[...keys].sort(order));
};
const size = (x,max) => assert(Buffer.byteLength(canonical(x))<=max,'serialized byte bound');

// A small bounded JSON reader preserves lexical checks before number conversion
// and detects duplicate decoded keys even when escaped differently. Whitespace
// and object order are permitted on transport; only retained goldens are canonical.
export function decodeSummary(bytes,limit) {
  integer(limit,1,4194304);assert(bytes.byteLength<=limit,'wire byte bound');
  const wire=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);
  let p=0;
  const ws=()=>{while(/[\x20\t\r\n]/.test(wire[p]??'!'))p++;};
  const string=()=>{
    const start=p;assert.equal(wire[p++],'"');
    while(p<wire.length){
      const c=wire[p++];
      if(c==='"'){
        const value=JSON.parse(wire.slice(start,p));assert(value.isWellFormed(),'well-formed Unicode');return value;
      }
      if(c==='\\')p++;
    }
    assert.fail('unterminated JSON string');
  };
  const value=depth=>{
    ws();const c=wire[p];
    if(c==='"')return string();
    if(c==='{' || c==='['){
      assert(depth<32,'JSON depth bound');p++;ws();
      const array=c==='[', close=array?']':'}', result=array?[]:{}, keys=new Set();
      if(wire[p]===close){p++;return result;}
      while(true){
        ws();
        if(array)result.push(value(depth+1));
        else{
          const key=string();assert(!keys.has(key),'duplicate JSON key');keys.add(key);
          ws();assert.equal(wire[p++],':');
          Object.defineProperty(result,key,{value:value(depth+1),enumerable:true,writable:true,configurable:true});
        }
        ws();if(wire[p]===close){p++;return result;}
        assert.equal(wire[p++],',');
      }
    }
    for(const [token,result] of [['true',true],['false',false],['null',null]]){
      if(wire.startsWith(token,p)){p+=token.length;return result;}
    }
    const match=/^-?(?:0|[1-9][0-9]*)/.exec(wire.slice(p));assert(match,'JSON integer token');
    p+=match[0].length;assert(!/[.eE0-9]/.test(wire[p]??'!'),'plain decimal integer');
    const result=Number(match[0]);assert(Number.isSafeInteger(result) && !Object.is(result,-0),'lossless integer');return result;
  };
  const result=value(0);ws();assert.equal(p,wire.length,'no trailing data');return result;
}

function assignment(x) {
  oneOf(x?.kind,['root','child']);
  const common='kind workspaceId runId parentSessionId';
  exact(x,common+(x.kind==='root'?' rootAssignmentId bootstrapId bootstrapConfigurationDigest':' planId planRevision taskId taskVersion reservationId childSessionId configurationId configurationDigest'));
  for(const [k,v] of Object.entries(x)){
    if(k==='planRevision')integer(v,1,64);
    else if(k==='taskVersion')integer(v,1,2147483647);
    else if(k.endsWith('Digest'))digest(v);
    else id(v);
  }
}
function reviewer(x) {
  oneOf(x?.kind,['user','child']);exact(x,x.kind==='user'?'kind userReviewId':'kind assignmentIdentity');
  if(x.kind==='user')id(x.userReviewId);
  else {assignment(x.assignmentIdentity);assert.equal(x.assignmentIdentity.kind,'child');}
}
function resultRef(x,subject) {
  exact(x,'assignmentIdentity resultRevision resultDigest');assignment(x.assignmentIdentity);same(x.assignmentIdentity,subject);
  integer(x.resultRevision,1,32);digest(x.resultDigest);
}
function selection(x,context) {
  exact(x,'assignmentIdentity accepted summaryRevision');assignment(x.assignmentIdentity);
  same(x.assignmentIdentity.workspaceId,context.workspaceId);same(x.assignmentIdentity.runId,context.runId);
  assert(typeof x.summaryRevision==='string' && /^[1-9][0-9]{0,9}$/.test(x.summaryRevision),'canonical summary token');
  assert(BigInt(x.summaryRevision)<=2147483647n,'summary token bound');
  oneOf(x.accepted?.kind,['none','result']);exact(x.accepted,x.accepted.kind==='none'?'kind':'kind reference');
  if(x.accepted.kind==='result')resultRef(x.accepted.reference,x.assignmentIdentity);
}
export function validateRequest(request,context,hop='renderer') {
  oneOf(hop,['renderer','main']);
  exact(request,'schemaVersion runId expectedSnapshotVersion selection'+(hop==='main'?' workspaceId sidecarGeneration':''));
  assert.equal(request.schemaVersion,1);id(request.runId);integer(request.expectedSnapshotVersion,1,2147483647);
  id(context.workspaceId);id(context.runId);digest(context.sidecarGeneration);integer(context.snapshotVersion,1,2147483647);
  assert.equal(context.fresh,true,'fresh complete overview witness');
  same(request.runId,context.runId);same(request.expectedSnapshotVersion,context.snapshotVersion);
  selection(request.selection,context);same(request.selection,context.selection);
  if(hop==='main'){same(request.workspaceId,context.workspaceId);same(request.sidecarGeneration,context.sidecarGeneration);}
  size(request,4096);return request;
}
function evaluation(x,chosen) {
  exact(x,'assignmentIdentity resultRevision resultDigest reviewerIdentity evaluationRevision evaluationDigest');
  assert.equal(chosen.accepted.kind,'result','evaluations require an accepted result');
  resultRef({assignmentIdentity:x.assignmentIdentity,resultRevision:x.resultRevision,resultDigest:x.resultDigest},chosen.assignmentIdentity);
  same(x.resultRevision,chosen.accepted.reference.resultRevision);same(x.resultDigest,chosen.accepted.reference.resultDigest);
  reviewer(x.reviewerIdentity);integer(x.evaluationRevision,1,16);digest(x.evaluationDigest);
}
function target(x,criteria,rubric) {
  oneOf(x?.kind,['criterion','quality-dimension','rating']);exact(x,x.kind==='rating'?'kind':'kind id');
  if(x.kind==='criterion')assert(criteria.some(c=>c.id===x.id && c.requirement==='required'),'approved required criterion');
  if(x.kind==='quality-dimension')assert(rubric.dimensions.some(d=>d.id===x.id),'approved dimension');
}
function snapshots(detail) {
  list(detail.criteria,32);
  for(const c of detail.criteria){exact(c,'id requirement description');id(c.id);oneOf(c.requirement,['required','optional']);text(c.description,2048);}
  sorted(detail.criteria,c=>c.id);
  const r=detail.rubric;exact(r,'id version dimensions evaluatorRole');id(r.id);integer(r.version,1,2147483647);
  oneOf(r.evaluatorRole,['orchestrator','research','implementation','review','verification','remediation']);
  list(r.dimensions,16);assert(r.dimensions.length>=1);
  for(const d of r.dimensions){exact(d,'id description');id(d.id);text(d.description,2048);}unique(r.dimensions.map(d=>d.id));
  same(hash('orkworks.criteria-snapshot.v1',detail.criteria),detail.material.criteriaSnapshotDigest);
  same(hash('orkworks.rubric-snapshot.v1',r),detail.material.rubricSnapshotDigest);
}
const reasonCodes=['declared-unassessed','missing-evidence','stale-evidence','evidence-unavailable','declared-conflict','invalidated','superseded','reviewer-ineligible'];
function outcomes(detail) {
  const m=detail.material;
  const expected=[...detail.criteria.filter(c=>c.requirement==='required').map(c=>({kind:'criterion',id:c.id})),
    ...detail.rubric.dimensions.map(d=>({kind:'quality-dimension',id:d.id})),{kind:'rating'}].sort((a,b)=>order(canonical(a),canonical(b)));
  list(m.requiredOutcomes,49);list(detail.outcomes,49);
  same(m.requiredOutcomes.map(o=>o.target),expected);same(detail.outcomes.map(o=>o.target),expected);
  const evaluationDigests=detail.evaluations.map(e=>e.evaluationDigest).sort(order);
  for(let i=0;i<expected.length;i++){
    const material=m.requiredOutcomes[i], o=detail.outcomes[i];
    exact(material,'target disposition');target(material.target,detail.criteria,detail.rubric);
    oneOf(material.disposition,['meets','needs-rework','unassessed','conflicted']);
    exact(o,'target fullyAssessed assessments');bool(o.fullyAssessed);list(o.assessments,16);
    same(o.assessments.map(a=>a.evaluationDigest),evaluationDigests);
    const domain=o.target.kind==='criterion'?['satisfied','unsatisfied','unassessed']:o.target.kind==='quality-dimension'?['meets','below-standard','unassessed']:[0,1,2,3,'unassessed'];
    for(const a of o.assessments){
      exact(a,'evaluationDigest claimed effective participates sourceState reasons');digest(a.evaluationDigest);
      oneOf(a.claimed,domain);oneOf(a.effective,domain);bool(a.participates);oneOf(a.sourceState,['current','stale','unavailable']);
      list(a.reasons,8);unique(a.reasons);a.reasons.forEach(r=>oneOf(r,reasonCodes));
      if(a.claimed==='unassessed' || a.sourceState!=='current' || !a.participates || a.reasons.some(r=>['invalidated','superseded','reviewer-ineligible','missing-evidence','stale-evidence','evidence-unavailable','declared-conflict'].includes(r)))
        same(a.effective,'unassessed');
      else assert(a.effective===a.claimed || a.effective==='unassessed','no invented assessed claim');
      if(a.reasons.some(r=>['invalidated','superseded','reviewer-ineligible'].includes(r)))assert.equal(a.participates,false);
    }
    if(material.disposition==='meets')assert.equal(o.fullyAssessed,true,'meets requires full assessment');
    if(o.fullyAssessed){
      const p=o.assessments.filter(a=>a.participates);assert(p.length>0,'fully assessed needs participants');
      assert(p.every(a=>a.sourceState==='current' && a.effective!=='unassessed' && !a.reasons.includes('declared-conflict')),'fully assessed support');
      const values=[...new Set(p.map(a=>a.effective))];
      assert(values.length===1 || (o.target.kind==='rating' && values.every(v=>Number.isInteger(v)&&v<3)),'no unresolved target conflict');
      const meets=o.target.kind==='criterion'?values[0]==='satisfied':o.target.kind==='quality-dimension'?values[0]==='meets':values[0]===3;
      same(material.disposition,meets?'meets':'needs-rework');
    }
  }
}
function findingSources(detail) {
  const findings=detail.material.requiredFindings,sources=detail.findingSources, refs=detail.evaluations;
  list(findings,512);list(sources,512);
  const key=x=>canonical([x.reviewerIdentity,x.findingId]);sorted(findings,key);same(sources.map(key),findings.map(key));
  for(let i=0;i<findings.length;i++){
    const f=findings[i],s=sources[i];
    exact(f,'reviewerIdentity findingId description location severity requiredImpactTargets disposition');
    reviewer(f.reviewerIdentity);id(f.findingId);text(f.description,2048);text(f.location,2048);id(f.severity);
    assert(Buffer.byteLength(f.description)+Buffer.byteLength(f.location)<=2048,'finding text total');
    oneOf(f.disposition,['current','disputed']);list(f.requiredImpactTargets,49);assert(f.requiredImpactTargets.length>0);
    for(const t of f.requiredImpactTargets)target(t,detail.criteria,detail.rubric);
    sorted(f.requiredImpactTargets,t=>t.kind+'\0'+(t.id??''));
    exact(s,'reviewerIdentity findingId originEvaluationDigest disputeEvaluationDigests');digest(s.originEvaluationDigest);
    const origin=refs.find(e=>e.evaluationDigest===s.originEvaluationDigest);assert(origin,'finding origin reference');same(origin.reviewerIdentity,f.reviewerIdentity);
    list(s.disputeEvaluationDigests,16);sorted(s.disputeEvaluationDigests,x=>x);
    for(const d of s.disputeEvaluationDigests){digest(d);assert(refs.some(e=>e.evaluationDigest===d),'dispute reference');}
    same(f.disposition==='disputed',s.disputeEvaluationDigests.length>0);
  }
}
function metrics(detail) {
  const c=detail.completeness;exact(c,'required satisfied unsatisfied unassessed percentage');
  for(const field of ['required','satisfied','unsatisfied','unassessed'])integer(c[field],0,32);
  const required=detail.outcomes.filter(o=>o.target.kind==='criterion');
  const counts={required:required.length,satisfied:0,unsatisfied:0,unassessed:0};
  for(const o of required){
    const m=detail.material.requiredOutcomes.find(m=>canonical(m.target)===canonical(o.target));
    counts[o.fullyAssessed && m.disposition==='meets'?'satisfied':o.fullyAssessed && m.disposition==='needs-rework'?'unsatisfied':'unassessed']++;
  }
  for(const k of Object.keys(counts))same(c[k],counts[k]);
  const percentage=counts.required>0 && counts.unassessed===0 ? {kind:'assessed',numerator:counts.satisfied,denominator:counts.required} : {kind:'unassessed'};
  same(c.percentage,percentage);
  const dimensions=detail.outcomes.filter(o=>o.target.kind==='quality-dimension'),rating=detail.outcomes.find(o=>o.target.kind==='rating');
  let quality={kind:'unassessed'};
  if(dimensions.length>0 && dimensions.every(o=>o.fullyAssessed) && rating.fullyAssessed){
    const values=[...new Set(rating.assessments.filter(a=>a.participates).map(a=>a.effective))].sort((a,b)=>a-b);
    if(values.length===1 && (values[0]!==3 || detail.material.requiredOutcomes.filter(o=>o.target.kind==='quality-dimension').every(o=>o.disposition==='meets')))
      quality={kind:'rating',value:values[0]};
    else if(values.length>1 && values.every(v=>Number.isInteger(v)&&v>=0&&v<3))quality={kind:'disagreed',values};
  }
  same(detail.quality,quality);
  if(detail.material.overall==='meets-requirements'){
    assert(counts.required>0 && counts.satisfied===counts.required && quality.kind==='rating' && quality.value===3,'no pass over missing required support');
  }
}
export function validateResponse(response,request,context,hop='renderer') {
  validateRequest(request,context,hop);
  exact(response,'schemaVersion workspaceId runId sidecarGeneration snapshotVersion selection detail');assert.equal(response.schemaVersion,1);
  same(response.workspaceId,context.workspaceId);same(response.runId,context.runId);same(response.sidecarGeneration,context.sidecarGeneration);
  same(response.snapshotVersion,request.expectedSnapshotVersion);same(response.selection,request.selection);
  const d=response.detail;exact(d,'material criteria rubric evaluations outcomes findingSources completeness quality');
  const m=d.material;exact(m,'schemaVersion assignmentIdentity accepted criteriaSnapshotDigest rubricSnapshotDigest overall requiredOutcomes requiredFindings');
  assert.equal(m.schemaVersion,1);same(m.assignmentIdentity,request.selection.assignmentIdentity);same(m.accepted,request.selection.accepted);
  digest(m.criteriaSnapshotDigest);digest(m.rubricSnapshotDigest);oneOf(m.overall,['meets-requirements','needs-rework','unassessed']);
  if(m.accepted.kind==='none')assert.notEqual(m.overall,'meets-requirements');
  snapshots(d);list(d.evaluations,16);for(const e of d.evaluations)evaluation(e,request.selection);
  sorted(d.evaluations);unique(d.evaluations.map(e=>e.evaluationDigest));same(d.evaluations,context.evaluations);
  outcomes(d);findingSources(d);metrics(d);size(m,2097152);size(response,4194304);
  return response;
}

// Presentation flags are a synthetic local witness, not a new wire schema or
// proof of paint/focus. A browser implementation must establish these facts.
export function viewedIdentity(response,request,context,presentation) {
  try {validateResponse(response,request,context);} catch {return null;}
  if(!['explicitOpen','visible','completeMaterial','painted','sameRequest'].every(k=>presentation?.[k]===true))return null;
  if(response.selection.accepted.kind!=='result')return null;
  const {assignmentIdentity,accepted,summaryRevision}=response.selection;
  return {workspaceId:response.workspaceId,runId:response.runId,assignmentIdentity,
    resultRevision:accepted.reference.resultRevision,resultDigest:accepted.reference.resultDigest,summaryRevision};
}
