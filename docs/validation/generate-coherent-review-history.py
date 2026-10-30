"""Conditional source/request/receipt history; no storage or capability producer."""
import copy
import hashlib
import json
from pathlib import Path
HERE=Path(__file__).resolve().parent

def canon(x):
    return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def sha(x):
    return hashlib.sha256(x if isinstance(x,bytes) else x.encode()).hexdigest()
def domain(d,x):
    return sha(d+'\n'+canon(x))
def entry(x,d=None):
    raw=canon(x)
    result=dict(canonicalUtf8=raw,utf8Bytes=len(raw.encode()),sha256=sha(raw))
    if d:result['domainDigest']=domain(d,x)
    return result

def load(name):
    data=(HERE/name).read_bytes()
    return json.loads(data),sha(data)
claims,claims_hash=load('hierarchy-claim-proof-vectors.json')
r2,r2_hash=load('r2-report-vectors.json')
byid={v['id']:v for v in claims['vectors']}
worker=copy.deepcopy(byid['child-candidate-initial']['source']['assignmentIdentity'])
worker.update(workspaceId=r2['approvedContext']['assignmentIdentity']['workspaceId'],runId=r2['approvedContext']['assignmentIdentity']['runId'])
reviewer=copy.deepcopy(byid['child-evaluation-initial']['source']['reviewerIdentity']['assignmentIdentity'])
reviewer['workspaceId']=worker['workspaceId']
authority=copy.deepcopy(byid['child-evaluation-initial']['source']['reporterSource']['authorityGeneration'])
authority['generation']=sha('coherent-reviewer-launch')
candidate=copy.deepcopy(byid['child-candidate-initial']['source']);candidate['assignmentIdentity']=worker
candidate_digest=domain('orkworks.assignment-result-candidate.v1',candidate)
result=dict(candidate,resultRevision=1,candidateDigest=candidate_digest,timeCostEvidence=[])
result_digest=domain('orkworks.assignment-result.v1',result)
reference=dict(assignmentIdentity=worker,resultRevision=1,resultDigest=result_digest)
criteria=json.loads(r2['approvedContext']['criteria']['canonicalUtf8']);rubric=json.loads(r2['approvedContext']['rubric']['canonicalUtf8'])
evidence=[v for v in r2['evidenceSources'] if v['reportEntryId'] in ('c1-v1','c2-v1','o-v1')]
evidence.sort(key=lambda v:v['sourceKey'])
read_bytes=len(canon(result).encode())+len(canon(criteria).encode())+len(canon(rubric).encode())+sum(json.loads(v['canonicalUtf8'])['sizeBytes'] for v in evidence)
review_input=dict(schemaVersion=1,kind='accepted-child-result',target=reference,outputDeclarationIds=[],evidenceSources=[json.loads(v['canonicalUtf8']) for v in evidence],evaluationOutputDeclarationId='evaluation',maxReadBytes=read_bytes)
output_contract=dict(schemaVersion=1,declarations=[dict(id='evaluation',kind='assignment_evaluation',maxBytes=524288,source=dict(kind='server-artifact'))],maxSnapshotBytes=524288,maxRehashBytes=1048576)
initial=json.loads(r2['evaluation']['canonicalUtf8'])
initial.update(assignmentIdentity=worker,resultDigest=result_digest,reviewerIdentity=dict(kind='child',assignmentIdentity=reviewer),reporterSource=dict(kind='child',authorityGeneration=authority))
initial_digest=domain('orkworks.assignment-evaluation.v1',initial)
correction=copy.deepcopy(initial)
correction.update(evaluationRevision=2,predecessor=dict(kind='revision',evaluationRevision=1,evaluationDigest=initial_digest),correctionReason='Clarify the existing C1 finding using the same captured evidence.')
correction['criterionOutcomes'][0]['rationale']='C1 still cites its exact original immutable report; this revision clarifies the rationale.'
correction_digest=domain('orkworks.assignment-evaluation.v1',correction)
disposition=dict(schemaVersion=1,assignmentIdentity=worker,resultRevision=1,reviewerIdentity=initial['reviewerIdentity'],evaluationRevision=2,evaluationDigest=correction_digest,dispositionRevision=1,action='invalidated',actor='user')
objects=[]
for name,kind,source in [('initial','evaluation',initial),('correction','evaluation',correction),('invalidation','disposition',disposition)]:
    d='orkworks.assignment-evaluation.v1' if kind=='evaluation' else 'orkworks.assignment-disposition.v1'
    source_digest=domain(d,source)
    action='orkworks.evaluation.submit.v1' if kind=='evaluation' else 'orkworks.user-evaluation.invalidate.v1'
    request=dict(schemaVersion=1,action=action,idempotencyKey='coherent-'+name)
    request['evaluation' if kind=='evaluation' else 'disposition']=source
    if kind=='disposition':request['resultDigest']=result_digest
    request_digest=domain('orkworks.assignment-report-request.v1',request)
    artifact=None if kind=='disposition' else dict(artifactId='coherent-evaluation-artifact',version=source['evaluationRevision'],sizeBytes=len(canon(source).encode()),contentDigest=sha(canon(source)))
    receipt=dict(schemaVersion=1,action=action,idempotencyKey=request['idempotencyKey'],requestDigest=request_digest,recordKind=kind,recordDigest=source_digest,reviewerIdentity=initial['reviewerIdentity'],recordRevision=1 if kind=='disposition' else source['evaluationRevision'],artifact=artifact)
    item=dict(id=name,kind=kind,record=entry(source,d),request=entry(request,'orkworks.assignment-report-request.v1'),receipt=entry(receipt))
    if artifact:item['artifactRead']=entry(dict(schemaVersion=1,artifact=artifact,evaluation=source))
    objects.append(item)
base=len(canon(candidate).encode())+len(canon(result).encode())
phases=[];retained=base;artifacts=0;slots=[]
for item in objects:
    retained+=item['record']['utf8Bytes']
    if item['kind']=='evaluation':artifacts+=item['record']['utf8Bytes']
    if item['kind']=='evaluation':
        ordinal=320+2*(len(slots)//2)
        slots.extend([dict(slotId=ordinal,purpose='child-evaluation',state='consumed'),dict(slotId=ordinal+1,purpose='child-invalidation',state='reserved')])
    else:slots[1]['state']='consumed'
    used=sum(s['state']=='consumed' for s in slots)*16384
    reserved=sum(s['state']=='reserved' for s in slots)*16384
    phases.append(dict(stage=item['id'],targetRecordBytes=str(retained),reviewerArtifactBytes=str(artifacts),resultPayloadBytes='0',childReceiptUsedBytes=str(used),childReceiptReservedBytes=str(reserved),childSlots=copy.deepcopy(slots)))
packet=dict(schemaVersion=1,evidenceKind='conditional-coherent-child-review-source-history-not-publication',inputs={'hierarchy-claim-proof-vectors.json':claims_hash,'r2-report-vectors.json':r2_hash},bindings=dict(targetAssignment=worker,reviewerAssignment=reviewer,authorityGeneration=authority,role='review',reviewInput=review_input,outputContract=output_contract),candidate=entry(candidate,'orkworks.assignment-result-candidate.v1'),result=entry(result,'orkworks.assignment-result.v1'),criteria=entry(criteria,'orkworks.criteria-snapshot.v1'),rubric=entry(rubric,'orkworks.rubric-snapshot.v1'),history=objects,measurements=dict(readBytes=read_bytes,baseTargetRecordBytes=str(base),phases=phases))
print(json.dumps(packet,indent=2,ensure_ascii=False))
