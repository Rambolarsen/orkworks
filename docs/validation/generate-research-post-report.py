"""Independent finite offline post-rejection examples, never a legal history."""
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
def origin(n):return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED post-report label '+str(n)))
def row(table,key,body,n,**extra):return dict(table=table,key=clone(key),value=dict(schemaVersion=1,body=clone(body),**extra,lastCommit=origin(n)))
def ref(b):return fields(b,'workspaceId runId preparationRevision inputRevision inputDigest')
def measure(r):
    v=clone(r['value']);v.pop('lastCommit');v.get('receipt',{}).pop('commit',None)
    size=lambda x:len(canonical(x).encode())
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))
def view(v):
    return {new:clone(v[old]) for new,old in [('run','run'),('aPlan','currentPlan'),('bPlan','pendingPlan'),('aTask','currentTask'),('bTask','proposedTask'),('selector','selector'),('input','input'),('collection','collection')]} | dict(aExpectation=clone(v['expectations'][0]),bExpectation=clone(v['expectations'][1]),memberships=clone(v['memberships']),summary=None)
def packet():
    names=['research-prefix-components.json','research-prospective-components.json'];raw=[(ROOT/n).read_bytes() for n in names];prefix,p=map(json.loads,raw)
    a=prefix['chain']['assignment'];ta=a['reservation']['task'];rk=fields(ta,'workspaceId runId');stream=fields(a['assignmentIdentity'],'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest')
    bridgebody=clone(next(r for r in prefix['views'][2]['rows'] if r['table']=='research_session_states')['value']['body'])
    cases=[];mainfacts=clone(p['facts']);childoutcomes={}
    for i,label in enumerate(['incomplete-first','incomplete-correction','complete-null','complete-summary']):
        complete=i>=2;original=clone(p['cases'][0 if complete else 1]);before=view(original['after'])
        if i==0:
            # New hypothetical rejection BEFORE first report, not relabelled old
            # input4/report history. Rebuild request/read/body/receipt coherently.
            submitted=p['views'][0];body=clone(original['rejection']['value']['body']);body['readSet'].update(expectedCommit=origin(29),run=clone(submitted['run']))
            body['request'].update(expectedRunVersion=20,input=ref(submitted['input']['value']['body']),decisionId='post-reject-before-first',reviewContextId=sha('UNQUALIFIED shown rejection before first report'))
            body['key']=dict(kind='ui-decision',run=rk,decisionId=body['request']['decisionId']);d=domain('orkworks.research-lifecycle-decision.v1',body)
            original=dict(rejection=row('research_lifecycle_records',body['key'],body,30,digest=d,receipt=dict(schemaVersion=1,key=body['key'],decisionDigest=d,runVersion=21,planVersion=None,commit=origin(30))),completion=None)
            before['input']=clone(p['originalAssessment']);rb=clone(submitted['run']['value']['body']);rb.update(runVersion=21,pendingPlan=None);before['run']=row('research_run_controls',rk,rb,30)
            before['collection']=row('research_collections',rk,dict(submitted['collection']['value']['body'],collectionVersion=4),30)
            before['aExpectation']=clone(submitted['expectations'][0]);before['bPlan']['value']['lastCommit']=origin(30)
        else:original=dict(rejection=original['rejection'],completion=original['completion'])
        before['bridge']=row('research_session_states',ta,bridgebody,31)
        if i==3:
            sb=dict(schemaVersion=1,run=rk,summaryVersion=1,researchPlan=a['plan'],input=ref(before['input']['value']['body']),producer=clone(prefix['inputs'][0]['value']['producer']),
                    findings=['Synthetic original summary before post-rejection correction.'],uncertainties=['Native and complete source facts remain unqualified.'],assignmentChoices=[],reportReferences=[clone(before['aExpectation']['value']['body']['selectedReport'])],reviewReferences=[])
            sd=domain('orkworks.research-summary.v1',sb);before['summary']=row('research_summaries',dict(**rk,summaryVersion=1),sb,31,digest=sd)
            b=clone(before['selector']['value']['body']);b.update(summaryVersion=1,currentSummary=dict(**rk,summaryVersion=1,summaryDigest=sd));before['selector']=row('research_lifecycle_state',rk,b,31)
            before['run']=row('research_run_controls',rk,dict(before['run']['value']['body'],runVersion=23),31)
        report=clone(prefix['reportAssumption']['report']);report.update(reportVersion=1 if i==0 else 2,uncertainties=['Synthetic post-rejection report — '+label+'.']);report.pop('contentDigest');report['contentDigest']=domain('orkworks.research-report.v1',report)
        source=dict(schemaVersion=1,sourceKind='research-report',reportIdentity=dict(workspaceId=rk['workspaceId'],**fields(report,'runId planId planRevision taskId reservationId childSessionId configurationDigest sidecarGeneration launchGeneration reportId reportVersion')),sizeBytes=len(canonical(report).encode()),contentDigest=report['contentDigest'])
        head=fields(report,'reportId reportVersion contentDigest');previous=None if i==0 else clone(prefix['reportAssumption']['head'])
        request=dict(schemaVersion=1,operation='submitResearchReport' if i==0 else 'correctResearchReport',stream=stream,expectedTaskVersion=before['aTask']['value']['body']['taskVersion'],expectedRunVersion=None,expectedReportHead=previous,report=report)
        C=origin(50+i);d=domain('orkworks.research-report-request.v1',request)
        publication=dict(report=report,source=source,request=request,headRow=dict(table='report_heads',key=stream,value=dict(schemaVersion=1,head=head,lastCommit=C)),receiptRow=dict(table='report_receipts',key=dict(stream=stream,reportVersion=report['reportVersion']),value=dict(schemaVersion=1,**fields(request,'operation expectedTaskVersion expectedRunVersion expectedReportHead'),requestDigest=d,response=dict(schemaVersion=1,acceptedHead=head),lastCommit=C)))
        after=clone(before);b=clone(before['input']['value']['body']);b['preparationRevision']=b['inputRevision']=b['inputRevision']+1
        b['records']=[x for x in b['records'] if x['kind']!='research-report']+[dict(kind='research-report',source=source)];b['records'].sort(key=lambda x:(x['kind'].encode(),canonical(x).encode()));b.pop('inputDigest');b['inputDigest']=domain('orkworks.preparation-input.v1',b)
        after['input']=row('research_control_inputs',fields(b,'workspaceId runId preparationRevision inputRevision'),b,50+i,producer=clone(prefix['inputs'][1]['value']['producer']))
        after['run']=row('research_run_controls',rk,dict(before['run']['value']['body'],currentInput=ref(b),runVersion=before['run']['value']['body']['runVersion']+1),50+i)
        after['collection']=row('research_collections',rk,dict(before['collection']['value']['body'],collectionVersion=before['collection']['value']['body']['collectionVersion']+1),50+i)
        after['aExpectation']=row('research_expectations',ta,dict(before['aExpectation']['value']['body'],selectedReport=source,expectationVersion=before['aExpectation']['value']['body']['expectationVersion']+1),50+i)
        if i==3:after['selector']=row('research_lifecycle_state',rk,dict(before['selector']['value']['body'],currentSummary=None),50+i)
        writes=['head','receipt','input','run','collection','aExpectation']+(['selector'] if i==3 else [])
        rows=[publication['headRow'] if k=='head' else publication['receiptRow'] if k=='receipt' else after[k] for k in writes]
        childoutcomes[d]=dict(commit=C)
        mainfacts['originalOutcomes'][original['rejection']['value']['digest']]=dict(commit=original['rejection']['value']['lastCommit'],completionDigest=original['completion']['value']['digest'] if original['completion'] else None)
        cases.append(dict(id=label,original=original,before=before,publication=publication,after=after,writes=writes,measurements=[measure(r) for r in rows]))
    return dict(inputHashes=dict(zip(names,map(sha,raw))),assumptions=[
        'four independent decoded alternatives, not a legal post-rejection store/history or publisher',
        'original disposition/source/auth/native/currentness/complete membership/H-C/proofs/outcomes supplied and unqualified',
        'nonterminal bridge and summary are hypothetical source values, not native observation or summary creation proof',
        'complete resources/discovery/index/physical/job/command fit and activation remain unqualified'],originalAssessment=clone(p['originalAssessment']),cases=cases,
        facts=dict(main=mainfacts,child=dict(authority='supplied-live-child',runtime='supplied-nonterminal',rate='supplied-allowed',currentness='supplied-current-owner',membership='supplied-complete',sourceProofs='supplied-original',disposition='supplied-committed',runState='blocked',outcomes=childoutcomes)))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.write_bytes((json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode())
