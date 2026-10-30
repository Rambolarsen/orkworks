// Fixed conditional documentation fixture, not a generic codec or publisher.
// Supplied identities/input/output bindings are not approval or capability proof.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSummary} from './hierarchy-summary-components.mjs';
import {verifyR2} from './r2-report-components.mjs';
const order=(a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b));
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort(order).map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const hash=(domain,x)=>sha(domain+'\n'+canonical(x));
const same=(a,b,message)=>assert.deepEqual(a,b,message);
const id=x=>assert(typeof x==='string'&&/^[A-Za-z0-9._-]{1,128}$/.test(x),'ID');
const digest=x=>assert(typeof x==='string'&&/^[a-f0-9]{64}$/.test(x),'digest');
const text=x=>assert(typeof x==='string'&&x.isWellFormed()&&Buffer.byteLength(x)>0&&Buffer.byteLength(x)<=2048,'text bound');
function child(A){
  assert.equal(A.kind,'child','child target/reviewer required');
  same(Object.keys(A).sort(),'kind workspaceId runId planId planRevision taskId taskVersion reservationId parentSessionId childSessionId configurationId configurationDigest'.split(' ').sort(),'child identity fields');
  for(const [key,value] of Object.entries(A)){
    if(key==='planRevision'||key==='taskVersion')assert(Number.isSafeInteger(value)&&value>=1&&value<=(key==='planRevision'?64:2147483647),'identity revision');
    else if(key==='configurationDigest')digest(value);
    else id(value);
  }
}
function read(entry,domain){
  const fields=domain?'canonicalUtf8 utf8Bytes sha256 domainDigest':'canonicalUtf8 utf8Bytes sha256';
  same(Object.keys(entry).sort(),fields.split(' ').sort(),'closed component entry');
  assert(typeof entry.canonicalUtf8==='string'&&entry.canonicalUtf8.isWellFormed(),'canonical UTF-8');
  const bytes=Buffer.from(entry.canonicalUtf8);same(bytes.length,entry.utf8Bytes,'exact source size');same(sha(bytes),entry.sha256,'exact source SHA');
  const value=decodeSummary(bytes,524288);same(canonical(value),entry.canonicalUtf8,'canonical source bytes');
  if(domain)same(hash(domain,value),entry.domainDigest,'source domain digest');
  return value;
}
export function verifyReviewHistory(packet,r2){
  verifyR2(r2);same(packet.schemaVersion,1);same(packet.evidenceKind,'conditional-coherent-child-review-source-history-not-publication');
  const b=packet.bindings,A=b.targetAssignment,reviewer=b.reviewerAssignment;child(A);child(reviewer);
  same(A.workspaceId,reviewer.workspaceId,'workspace-local review');same(A.workspaceId,r2.approvedContext.assignmentIdentity.workspaceId,'workspace-local evidence');
  assert(A.runId!==reviewer.runId,'fixture cross-run independence');
  for(const key of ['taskId','reservationId','childSessionId','configurationId','configurationDigest'])assert(A[key]!==reviewer[key],'necessary reviewer independence');
  same(b.role,'review','eligible fixture role');
  same(Object.keys(b.authorityGeneration).sort(),['generation','kind','sidecarGeneration']);same(b.authorityGeneration.kind,'child-launch');digest(b.authorityGeneration.generation);digest(b.authorityGeneration.sidecarGeneration);
  const candidate=read(packet.candidate,'orkworks.assignment-result-candidate.v1');
  same(Object.keys(candidate).sort(),'schemaVersion assignmentIdentity authorityGeneration predecessor outputs'.split(' ').sort());
  same(candidate.schemaVersion,1);same(candidate.assignmentIdentity,A,'candidate assignment');same(candidate.predecessor,{kind:'no-head'});same(candidate.outputs,[]);
  same(candidate.authorityGeneration.kind,'child-launch');digest(candidate.authorityGeneration.sidecarGeneration);digest(candidate.authorityGeneration.generation);
  const result=read(packet.result,'orkworks.assignment-result.v1');same(result,{...candidate,resultRevision:1,candidateDigest:packet.candidate.domainDigest,timeCostEvidence:[]},'exact candidate/result source join');
  const reference={assignmentIdentity:A,resultRevision:1,resultDigest:packet.result.domainDigest};
  const criteria=read(packet.criteria,'orkworks.criteria-snapshot.v1'),rubric=read(packet.rubric,'orkworks.rubric-snapshot.v1');
  same(criteria,JSON.parse(r2.approvedContext.criteria.canonicalUtf8),'criteria snapshot');same(rubric,JSON.parse(r2.approvedContext.rubric.canonicalUtf8),'rubric snapshot');same(rubric.evaluatorRole,b.role);
  const evidence=r2.evidenceSources.filter(e=>['c1-v1','c2-v1','o-v1'].includes(e.reportEntryId)).sort((a,b)=>order(a.sourceKey,b.sourceKey));
  const sources=evidence.map(e=>JSON.parse(e.canonicalUtf8));
  same(b.reviewInput.evidenceSources,sources,'exact evidence selection');
  const readBytes=packet.result.utf8Bytes+packet.criteria.utf8Bytes+packet.rubric.utf8Bytes+sources.reduce((n,s)=>n+s.sizeBytes,0);
  assert(Number.isSafeInteger(b.reviewInput.maxReadBytes)&&b.reviewInput.maxReadBytes>=readBytes,'approved read budget');
  same(b.reviewInput,{schemaVersion:1,kind:'accepted-child-result',target:reference,outputDeclarationIds:[],evidenceSources:sources,evaluationOutputDeclarationId:'evaluation',maxReadBytes:b.reviewInput.maxReadBytes},'exact review input');
  same(b.outputContract,{schemaVersion:1,declarations:[{id:'evaluation',kind:'assignment_evaluation',maxBytes:524288,source:{kind:'server-artifact'}}],maxSnapshotBytes:524288,maxRehashBytes:1048576},'fixture output contract');
  const base=JSON.parse(r2.evaluation.canonicalUtf8);
  Object.assign(base,{assignmentIdentity:A,resultDigest:reference.resultDigest,reviewerIdentity:{kind:'child',assignmentIdentity:reviewer},reporterSource:{kind:'child',authorityGeneration:b.authorityGeneration}});
  let head=null,initial=null,frozen=false;
  let retained=BigInt(packet.candidate.utf8Bytes+packet.result.utf8Bytes),artifactBytes=0n;
  const baseTargetRecordBytes=String(retained),phases=[],slots=[];
  const seenRequests=new Set();
  for(const [i,item] of packet.history.entries()){
    assert(!frozen,'invalidated review stream is frozen');
    same(item.id,['initial','correction','invalidation'][i],'initial/correction/invalidation sequence');
    same(item.kind,i<2?'evaluation':'disposition','source history kind');
    const source=read(item.record,i<2?'orkworks.assignment-evaluation.v1':'orkworks.assignment-disposition.v1');
    if(i<2){
      same(source.assignmentIdentity,A,'result join assignment');same(source.resultRevision,reference.resultRevision,'result join revision');same(source.resultDigest,reference.resultDigest,'result join digest');
      same(source.reviewerIdentity,base.reviewerIdentity,'reviewer stream');same(source.reporterSource,base.reporterSource,'reporter generation/provenance');
      same(source.evaluationRevision,i+1,'evaluation revision');
      same(source.criteriaSnapshotDigest,packet.criteria.domainDigest,'criteria snapshot binding');same(source.rubricSnapshotDigest,packet.rubric.domainDigest,'rubric snapshot binding');
      if(i===0){same(source.predecessor,{kind:'no-head'},'initial predecessor');same(source,base,'fixed initial source semantics');initial=source;}
      else{
        same(source.predecessor,{kind:'revision',evaluationRevision:head.evaluationRevision,evaluationDigest:head.evaluationDigest},'exact predecessor chain');
        text(source.correctionReason);text(source.criterionOutcomes[0].rationale);assert.notEqual(source.criterionOutcomes[0].rationale,initial.criterionOutcomes[0].rationale,'nontrivial rationale correction');
        const expected=structuredClone(initial);Object.assign(expected,{evaluationRevision:2,predecessor:source.predecessor,correctionReason:source.correctionReason});expected.criterionOutcomes[0].rationale=source.criterionOutcomes[0].rationale;
        same(source,expected,'fixed correction preserves target/evidence/outcomes');
      }
      head={...reference,reviewerIdentity:base.reviewerIdentity,evaluationRevision:i+1,evaluationDigest:item.record.domainDigest};
    }else{
      same(source.actor,'user','invalidation actor');
      same(source.evaluationRevision,head.evaluationRevision,'invalidation head revision');same(source.evaluationDigest,head.evaluationDigest,'invalidation head digest');
      same(source,{schemaVersion:1,assignmentIdentity:A,resultRevision:1,reviewerIdentity:base.reviewerIdentity,evaluationRevision:2,evaluationDigest:head.evaluationDigest,dispositionRevision:1,action:'invalidated',actor:'user'},'exact invalidation source');
      frozen=true;
    }
    const request=read(item.request,'orkworks.assignment-report-request.v1');
    const action=i<2?'orkworks.evaluation.submit.v1':'orkworks.user-evaluation.invalidate.v1';
    const expectedRequest={schemaVersion:1,action,idempotencyKey:'coherent-'+item.id,...(i<2?{evaluation:source}:{disposition:source,resultDigest:reference.resultDigest})};
    same(request,expectedRequest,'exact reporting request');
    const requestKey=action+'\n'+request.idempotencyKey;assert(!seenRequests.has(requestKey));seenRequests.add(requestKey);
    const artifact=i<2?{artifactId:'coherent-evaluation-artifact',version:source.evaluationRevision,sizeBytes:item.record.utf8Bytes,contentDigest:item.record.sha256}:null;
    const receipt=read(item.receipt);
    same(receipt.artifact,artifact,'artifact raw byte digest and version');
    same(receipt,{schemaVersion:1,action,idempotencyKey:request.idempotencyKey,requestDigest:item.request.domainDigest,recordKind:item.kind,recordDigest:item.record.domainDigest,reviewerIdentity:base.reviewerIdentity,recordRevision:i<2?source.evaluationRevision:1,artifact},'exact reporting receipt');
    if(i<2){same(read(item.artifactRead),{schemaVersion:1,artifact,evaluation:source},'exact artifact read');artifactBytes+=BigInt(item.record.utf8Bytes);}
    else assert(!Object.hasOwn(item,'artifactRead'),'no invalidation artifact');
    retained+=BigInt(item.record.utf8Bytes);
    if(i<2){
      const ordinal=320+2*i;
      slots.push({slotId:ordinal,purpose:'child-evaluation',state:'consumed'},{slotId:ordinal+1,purpose:'child-invalidation',state:'reserved'});
    }else slots[1].state='consumed';
    const used=BigInt(slots.filter(s=>s.state==='consumed').length)*16384n;
    const reserved=BigInt(slots.filter(s=>s.state==='reserved').length)*16384n;
    phases.push({stage:item.id,targetRecordBytes:String(retained),reviewerArtifactBytes:String(artifactBytes),resultPayloadBytes:'0',childReceiptUsedBytes:String(used),childReceiptReservedBytes:String(reserved),childSlots:structuredClone(slots)});
  }
  assert(frozen&&packet.history.length===3,'complete conditional source sequence');
  return {readBytes,baseTargetRecordBytes,phases};
}
