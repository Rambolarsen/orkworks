"""Independent canonical report/evaluation bytes and two logical R2 histories."""
import argparse
import base64
import copy
import hashlib
import json
from pathlib import Path


def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def digest(domain, x):
    return sha((domain+'\n'+canonical(x)).encode())


def wire(x, **metadata):
    raw=canonical(x).encode()
    return dict(canonicalUtf8=raw.decode(),utf8Bytes=len(raw),sha256=sha(raw),**metadata)


def seal_report(body):
    return dict(body,contentDigest=digest('orkworks.research-report.v1',body))


run_id=sha(b'synthetic-r2-epoch')+'-1'
workspace='r2-workspace'
source_generation=sha(b'synthetic-report-sidecar')
subject=dict(kind='root',workspaceId=workspace,runId=run_id,rootAssignmentId='r2-root',
             parentSessionId='r2-parent',bootstrapId='r2-bootstrap',
             bootstrapConfigurationDigest=sha(b'UNVERIFIED-r2-bootstrap-reference'))
criteria=[dict(id='c1',requirement='required',description='Required C1.'),
          dict(id='c2',requirement='required',description='Required C2.'),
          dict(id='o',requirement='optional',description='Optional O.')]
rubric=dict(id='r2-rubric',version=1,dimensions=[dict(id='d',description='Declared quality dimension.')],evaluatorRole='review')
criteria_digest=digest('orkworks.criteria-snapshot.v1',criteria)
rubric_digest=digest('orkworks.rubric-snapshot.v1',rubric)
candidate=dict(schemaVersion=1,assignmentIdentity=subject,predecessor=dict(kind='no-head'),
               authorityGeneration=dict(kind='parent-runtime',sidecarGeneration=source_generation,generation=sha(b'synthetic-parent-runtime')),outputs=[])
candidate_digest=digest('orkworks.assignment-result-candidate.v1',candidate)
result=dict(candidate,resultRevision=1,candidateDigest=candidate_digest,timeCostEvidence=[])
result_digest=digest('orkworks.assignment-result.v1',result)
result_ref=dict(assignmentIdentity=subject,resultRevision=1,resultDigest=result_digest)
profiles=[]
reports={}
for key in ['c1','c2','o']:
    stream=dict(workspaceId=workspace,runId=run_id,planId='r2-research-plan',planRevision=1,taskId='research-'+key,
                reservationId='res-'+key,childSessionId='researcher-'+key,configurationDigest=sha(('UNVERIFIED-research-config-'+key).encode()))
    lineage=dict(sidecarGeneration=source_generation,launchGeneration=sha(('synthetic-launch-'+key).encode()))
    captures=[]
    for version in ([1] if key=='c2' else [1,2]):
        raw=(key+(': original observation — æ\n' if version==1 else ': corrected observation\n')).encode()
        reference=dict(kind='file',id='e-'+key,path='research/'+key+'.txt',revision='fixture-v'+str(version),location='L1',excerptDigest=sha(raw))
        captures.append(dict(reference=reference,bytesBase64=base64.b64encode(raw).decode()))
        body=dict(schemaVersion=1,reportId='report-'+key,reportVersion=version,
                  **{k:v for k,v in stream.items() if k!='workspaceId'},**lineage,questionIds=['q-'+key],
                  findings=[dict(questionIds=['q-'+key],answer=key+(' original observation.' if version==1 else ' corrected observation.'),
                  evidenceReferenceIds=['e-'+key],classification='established')],
                  uncertainties=[],blockers=[],evidenceReferences=[reference],recommendations=[],limitations=[],
                  author=dict(kind='child' if version==1 else 'user'))
        reports[key+'-v'+str(version)]=seal_report(body)
    profiles.append(dict(id=key,stream=stream,questionIds=['q-'+key],lineage=lineage,captures=captures))
# A separate component exercises the URL leaf without performing retrieval.
url_raw=b'captured URL excerpt, synthetic only\n'
url_ref=dict(kind='url',id='e-c1',url='HTTPS://example.test:0443/research%2Fc1?q=%C3%A6#L1',
             retrievedAt='2026-10-28T12:00:00.123456789+00:00',excerptDigest=sha(url_raw))
profiles[0]['captures'].append(dict(reference=url_ref,bytesBase64=base64.b64encode(url_raw).decode()))
url_body=copy.deepcopy(reports['c1-v1']);del url_body['contentDigest'];url_body['evidenceReferences']=[url_ref]
url_report=seal_report(url_body)


def evidence(report, profile):
    identity=dict(workspaceId=workspace,**{k:report[k] for k in ['runId','planId','planRevision','taskId','reservationId','childSessionId','configurationDigest','sidecarGeneration','launchGeneration','reportId','reportVersion']})
    value=dict(schemaVersion=1,sourceKind='research-report',reportIdentity=identity,sizeBytes=size_report(report),contentDigest=report['contentDigest'])
    return value,digest('orkworks.assignment-evidence-source.v1',value)


def size_report(report):
    return len(canonical(report).encode())


sources={key:evidence(report,next(p for p in profiles if p['id']==key.split('-')[0])) for key,report in reports.items()}
keys={k:v[1] for k,v in sources.items()}
reviewer=dict(kind='user',userReviewId='r2-user-review')
criterion_outcomes=[dict(criterionId=k,outcome='satisfied' if k=='o' else 'unsatisfied',rationale=k+' cites its original report.',evidenceReferences=[keys[k+'-v1']]) for k in ['c1','c2','o']]
findings=[dict(id='f-'+k,description=k+' requires rework.',location='criterion:'+k,severity='major',evidenceReference=keys[k+'-v1'],requiresRework=True,
               requiredImpactTargets=[dict(kind='criterion',id=k)]) for k in ['c1','c2']]
evaluation=dict(schemaVersion=1,assignmentIdentity=subject,resultRevision=1,resultDigest=result_digest,reviewerIdentity=reviewer,
                reporterSource=dict(kind='user'),evaluationRevision=1,predecessor=dict(kind='no-head'),criteriaSnapshotDigest=criteria_digest,
                rubricSnapshotDigest=rubric_digest,criterionOutcomes=criterion_outcomes,
                qualityDimensionOutcomes=[dict(dimensionId='d',outcome='unassessed',rationale='Quality not assessed in this fixture.',evidenceReferences=[])],
                rating=dict(rating='unassessed',rationale='Quality not assessed.',evidenceReferences=[]),findings=findings,findingDisputes=[],
                correctionReason=None,reviewedEvaluationRef=None)
evaluation_digest=digest('orkworks.assignment-evaluation.v1',evaluation)
evaluation_ref=dict(assignmentIdentity=subject,resultRevision=1,resultDigest=result_digest,reviewerIdentity=reviewer,evaluationRevision=1,evaluationDigest=evaluation_digest)


def head(report):
    return {k:report[k] for k in ['reportId','reportVersion','contentDigest']}


def request(key,version):
    profile=next(p for p in profiles if p['id']==key)
    return dict(schemaVersion=1,operation='submitResearchReport' if version==1 else 'correctResearchReport',stream=profile['stream'],
                expectedTaskVersion=1 if version==1 else None,expectedRunVersion=None if version==1 else 4,
                expectedReportHead=None if version==1 else head(reports[key+'-v1']),report=reports[key+'-v'+str(version)])


requests={k+'-v1':request(k,1) for k in ['c1','c2','o']}
requests.update({k+'-v2':request(k,2) for k in ['c1','o']})


def state_after(correction):
    heads=[dict(stream=p['stream'],head=head(reports[p['id']+('-v2' if p['id']==correction else '-v1')])) for p in profiles]
    return dict(runVersion=5,heads=heads)


def display(history,correction):
    current={k:k!=correction for k in ['c1','c2','o']}
    outcomes=[]
    for kind,key in [('criterion','c1'),('criterion','c2'),('quality-dimension','d'),('rating',None)]:
        target=dict(kind=kind,**({'id':key} if key else {}))
        claimed='unsatisfied' if kind=='criterion' else 'unassessed'
        state='current' if kind!='criterion' or current[key] else 'stale'
        effective=claimed if state=='current' else 'unassessed'
        annotation=dict(evaluationDigest=evaluation_digest,claimed=claimed,effective=effective,participates=True,sourceState=state,
                        reasons=['stale-evidence'] if state=='stale' else ['declared-unassessed'] if claimed=='unassessed' else [])
        outcomes.append(dict(target=target,fullyAssessed=kind=='criterion' and current[key],assessments=[annotation]))
    outcomes.sort(key=lambda x:canonical(x['target']))
    material_findings=[{k:v for k,v in f.items() if k not in ['id','evidenceReference','requiresRework']} | dict(reviewerIdentity=reviewer,findingId=f['id'],disposition='current')
                       for f in findings if current[f['requiredImpactTargets'][0]['id']]]
    material=dict(schemaVersion=1,assignmentIdentity=subject,accepted=dict(kind='result',reference=result_ref),criteriaSnapshotDigest=criteria_digest,
                  rubricSnapshotDigest=rubric_digest,overall='needs-rework',requiredOutcomes=[dict(target=o['target'],disposition='needs-rework' if o['fullyAssessed'] else 'unassessed') for o in outcomes],requiredFindings=material_findings)
    selection=dict(assignmentIdentity=subject,accepted=material['accepted'],summaryRevision='4' if correction=='c1' else '3')
    context=dict(workspaceId=workspace,runId=run_id,sidecarGeneration=sha(('synthetic-UI-lifetime-'+history).encode()),snapshotVersion=2,
                 selection=selection,evaluations=[evaluation_ref],fresh=True)
    req=dict(schemaVersion=1,runId=run_id,expectedSnapshotVersion=2,selection=selection)
    response=dict(schemaVersion=1,workspaceId=workspace,runId=run_id,sidecarGeneration=context['sidecarGeneration'],snapshotVersion=2,selection=selection,
                  detail=dict(material=material,criteria=criteria,rubric=rubric,evaluations=[evaluation_ref],outcomes=outcomes,
                  findingSources=[dict(reviewerIdentity=reviewer,findingId=f['findingId'],originEvaluationDigest=evaluation_digest,disputeEvaluationDigests=[]) for f in material_findings],
                  completeness=dict(required=2,satisfied=0,unsatisfied=1 if correction=='c1' else 2,unassessed=1 if correction=='c1' else 0,
                  percentage=dict(kind='unassessed') if correction=='c1' else dict(kind='assessed',numerator=0,denominator=2)),quality=dict(kind='unassessed')))
    return dict(id=history,correctionRequestId=correction+'-v2',expectedState=state_after(correction),
                expectedSourceStates={k:'current' if v else 'stale' for k,v in current.items()},expectedRecordFreshness='historical',
                context=context,rendererRequest=wire(req),response=wire(response))


evaluated_material=json.loads(display('optional-O-correction','o')['response']['canonicalUtf8'])['detail']['material']
prepared_material=copy.deepcopy(evaluated_material)
prepared_material['accepted']=dict(kind='none')
prepared_material['overall']='unassessed'
prepared_material['requiredFindings']=[]
for outcome in prepared_material['requiredOutcomes']:
    outcome['disposition']='unassessed'
accepted_material=copy.deepcopy(prepared_material)
accepted_material['accepted']=dict(kind='result',reference=result_ref)
lineage=[dict(stage=stage,**wire(dict(schemaVersion=1,assignmentIdentity=subject,summaryRevision=str(i+1),state=dict(kind='current',material=material))))
         for i,(stage,material) in enumerate([('prepared',prepared_material),('accepted',accepted_material),('evaluated',evaluated_material)])]

packet=dict(schemaVersion=1,evidenceKind='canonical-source-components-and-logical-R2-not-admission-or-runtime',
            unverified=['approved run/configuration/admission authority','actual source custody and retrieval','native report publication and dependency fencing','runtime eligibility and browser presentation'],
            approvedContext=dict(assignmentIdentity=subject,userReviewId='r2-user-review',criteria=wire(criteria),rubric=wire(rubric)),
            candidate=wire(candidate,candidateDigest=candidate_digest),result=wire(result,resultDigest=result_digest),acceptedHead=wire(result_ref),
            evaluation=wire(evaluation,evaluationDigest=evaluation_digest),evaluationHead=wire(evaluation_ref),
            materialLineage=lineage,profiles=profiles,reports=[dict(id=k,profileId=k.split('-')[0],**wire(v)) for k,v in reports.items()],
            urlComponent=dict(profileId='c1',**wire(url_report)),
            evidenceSources=[dict(reportEntryId=k,sourceKey=v[1],**wire(v[0])) for k,v in sources.items()],
            requests=[dict(id=k,profileId=k.split('-')[0],requestDigest=digest('orkworks.research-report-request.v1',v),**wire(v)) for k,v in requests.items()],
            histories=[display('required-C1-correction','c1'),display('optional-O-correction','o')])
parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args();args.output.write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n')
