"""Independent finite offline prospective examples; no store or event history."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
clone=copy.deepcopy
def canonical(x):return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def sha(x):return hashlib.sha256(x if isinstance(x,bytes) else x.encode()).hexdigest()
def domain(n,x):return sha(n+'\n'+canonical(x))
def fields(x,ks):return {k:x[k] for k in ks.split()}
def origin(n):return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED prospective membership label '+str(n)))
def row(table,key,body,n,**extra):return dict(table=table,key=clone(key),value=dict(schemaVersion=1,body=clone(body),**extra,lastCommit=origin(n)))
def ref(b):return fields(b,'workspaceId runId preparationRevision inputRevision inputDigest')
def measure(r):
    v=clone(r['value']);v.pop('lastCommit');v.get('receipt',{}).pop('commit',None)
    return dict(chargeBytes=len(canonical(dict(key=r['key'],value=v)).encode()),wrapperBytes=len(canonical(dict(key=r['key'],value=r['value'])).encode()))
def members(a,b,pending):return dict(all=[a,b],collecting=[a],prospective=[b] if pending else [],retired=[] if pending else [b])
def view_rows(v):return [v[k] for k in ['run','currentPlan','pendingPlan','currentTask','proposedTask','selector','input','collection']]+v['expectations']
def packet():
    names=['research-prefix-components.json','research-scope-rejection-components.json'];raw=[(ROOT/n).read_bytes() for n in names];prefix,scope=map(json.loads,raw)
    q=scope['proposal'];a=prefix['chain']['assignment'];ta=a['reservation']['task'];tb=q['task'];rk=fields(ta,'workspaceId runId');base=scope['cases'][1]
    original=row('research_control_inputs',fields(q['input'],'workspaceId runId preparationRevision inputRevision'),q['input'],9,producer=clone(prefix['inputs'][0]['value']['producer']))
    fresh=clone(q['input']);fresh.update(preparationRevision=4,inputRevision=4)
    fresh['records'].append(dict(kind='research-report',source=clone(prefix['reportAssumption']['source'])))
    fresh['records'].sort(key=lambda x:(x['kind'].encode(),canonical(x).encode()));fresh.pop('inputDigest');fresh['inputDigest']=domain('orkworks.preparation-input.v1',fresh)
    selected=row('research_control_inputs',fields(fresh,'workspaceId runId preparationRevision inputRevision'),fresh,11,producer=clone(prefix['inputs'][1]['value']['producer']))
    ac=clone(next(r for r in prefix['views'][2]['rows'] if r['table']=='research_collections')['value']['body']);ac['collectionVersion']=3
    ae=clone(next(r for r in prefix['views'][2]['rows'] if r['table']=='research_expectations'))
    be=row('research_expectations',tb,dict(schemaVersion=1,task=tb,collection=rk,collecting=False,configuration=dict(task=tb,**fields(q['startup']['configuration'],'configurationId configurationDigest')),scope=dict(task=tb,scopeDigest=q['startup']['scope']['scopeDigest']),reservation=None,selectedReport=None,expectationVersion=1),9)
    submitted=dict(id='submitted',run=clone(base['rejection']['value']['body']['readSet']['run']),currentPlan=clone(base['rejection']['value']['body']['readSet']['currentPlan']),pendingPlan=clone(base['rejection']['value']['body']['readSet']['pendingPlan']),currentTask=clone(base['source']['currentTask']),proposedTask=clone(base['after']['proposedTask']),selector=clone(base['rejection']['value']['body']['readSet']['lifecycle']),input=clone(original),collection=row('research_collections',rk,ac,9),expectations=[ae,be],memberships=members(ta,tb,True))
    invalid=clone(submitted);invalid['id']='invalidated';invalid['run']['value']['body'].update(runVersion=21,currentInput=ref(fresh));invalid['run']['value']['lastCommit']=origin(11);invalid['input']=clone(selected)
    invalid['collection']['value']['body']['collectionVersion']=4;invalid['collection']['value']['lastCommit']=origin(11)
    invalid['expectations'][0]['value']['body'].update(expectationVersion=3,selectedReport=clone(prefix['reportAssumption']['source']));invalid['expectations'][0]['value']['lastCommit']=origin(11)
    before=dict(run=clone(submitted['run']),collection=clone(submitted['collection']),expectation=clone(ae),input=clone(prefix['inputs'][0]))
    before['run']['value']['body'].update(runVersion=19,currentInput=ref(prefix['inputs'][0]['value']['body']),pendingPlan=None,planReferences=[a['plan']]);before['collection']['value']['body']['collectionVersion']=2
    cases=[]
    for old in scope['cases']:
        complete=old['completion'] is not None;label=old['id'];v=clone(invalid);v['currentTask']=clone(old['source']['currentTask']);reads=clone(old['rejection']['value']['body']['readSet']);reads.update(expectedCommit=origin(12),run=clone(v['run']))
        req=clone(old['rejection']['value']['body']['request']);req.update(expectedRunVersion=21,input=ref(fresh),decisionId='prospective-reject-'+label,reviewContextId=sha('UNQUALIFIED prospective shown '+label))
        def lifecycle(request,key,pv):
            body=dict(schemaVersion=1,key=key,request=clone(request),readSet=clone(reads));digest=domain('orkworks.research-lifecycle-decision.v1',body)
            return row('research_lifecycle_records',key,body,13,digest=digest,receipt=dict(schemaVersion=1,key=key,decisionDigest=digest,runVersion=22,planVersion=pv,commit=origin(13)))
        rejection=lifecycle(req,dict(kind='ui-decision',run=rk,decisionId=req['decisionId']),None)
        completion=lifecycle(dict(schemaVersion=1,kind='plan-completion',run=rk,expectedRunVersion=21,input=ref(fresh),plan=a['plan'],expectedPlanVersion=2,trigger=dict(kind='scope-rejection',rejection=dict(key=rejection['key'],digest=rejection['value']['digest']))),dict(kind='plan-completion',plan=a['plan']),3) if complete else None
        after=clone(v);after['id']='rejected-'+label;after['run']['value']['body'].update(runVersion=22,pendingPlan=None,currentPlan=None if complete else a['plan']);after['run']['value']['lastCommit']=origin(13)
        after['pendingPlan']['value']['body'].update(status='cancelled',planVersion=2);after['pendingPlan']['value']['lastCommit']=origin(13)
        after['collection']['value']['body']['collectionVersion']=5;after['collection']['value']['lastCommit']=origin(13);after['memberships']=members(ta,tb,False)
        if complete:
            after['currentPlan']['value']['body'].update(status='complete',planVersion=3);after['currentPlan']['value']['lastCommit']=origin(13)
            after['selector']['value']['body']['completedResearchPlan']=clone(a['plan']);after['selector']['value']['lastCommit']=origin(13)
        cases.append(dict(id=label,before=v,rejection=rejection,completion=completion,after=after))
    stream=fields(a['assignmentIdentity'],'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest')
    report_request=dict(schemaVersion=1,operation='submitResearchReport',stream=stream,expectedTaskVersion=base['source']['currentTask']['value']['body']['taskVersion'],expectedRunVersion=None,expectedReportHead=None,report=clone(prefix['reportAssumption']['report']))
    report_receipt=dict(table='report_receipts',key=dict(stream=stream,reportVersion=1),value=dict(schemaVersion=1,operation='submitResearchReport',expectedTaskVersion=report_request['expectedTaskVersion'],expectedRunVersion=None,expectedReportHead=None,requestDigest=domain('orkworks.research-report-request.v1',report_request),response=dict(schemaVersion=1,acceptedHead=clone(prefix['reportAssumption']['head'])),lastCommit=origin(11)))
    report_original=dict(request=report_request,headRow=dict(table='report_heads',key=stream,value=dict(schemaVersion=1,head=clone(prefix['reportAssumption']['head']),lastCommit=origin(11))),receiptRow=report_receipt)
    report_retry=dict(request=clone(report_request),receipt=clone(report_receipt['value']),retained=dict(input=clone(selected),run=clone(invalid['run']),collection=clone(invalid['collection']),expectations=clone(invalid['expectations'])))
    facts=clone(scope['facts']);facts['originalOutcomes']={c['rejection']['value']['digest']:dict(commit=origin(13),completionDigest=c['completion']['value']['digest'] if c['completion'] else None) for c in cases}
    comparisons=[dict(id='submission',before=[before['run'],before['collection']],after=[submitted['run'],submitted['collection']]),dict(id='report',before=[submitted['run'],submitted['collection'],submitted['expectations'][0]],after=[invalid['run'],invalid['collection'],invalid['expectations'][0]])]
    for c in cases:comparisons.append(dict(id='rejection-'+c['id'],before=[c['before']['run'],c['before']['collection']],after=[c['after']['run'],c['after']['collection']]))
    measurements=dict(views=[[measure(r) for r in view_rows(v)] for v in [submitted,invalid]],rejections=[[measure(r) for r in [c['rejection']]+([c['completion']] if c['completion'] else [])] for c in cases],localDeltas=[sum(measure(r)['chargeBytes'] for r in x['after'])-sum(measure(r)['chargeBytes'] for r in x['before']) for x in comparisons])
    return dict(inputHashes=dict(zip(names,map(sha,raw))),assumptions=[
        'finite decoded alternatives, not a legal pending prestate/store/history or publisher',
        'auth/native/source/currentness/complete owner-index membership/original H-C proofs/outcomes supplied and unqualified',
        'post-rejection/completion report writers, approval handover and complete accounting/physical profile remain unqualified'],originalAssessment=original,reportOriginal=report_original,reportRetry=report_retry,submissionBefore=before,views=[submitted,invalid],cases=cases,facts=facts,measurements=measurements)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.write_bytes((json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode())
