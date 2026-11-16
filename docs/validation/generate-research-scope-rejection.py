"""Independent finite offline source examples, not a legal pending history."""
import argparse
import base64
import copy
import hashlib
import json
import runpy
from pathlib import Path
ROOT=Path(__file__).resolve().parent
helpers=runpy.run_path(str(ROOT/'generate-research-startup.py'))
def clone(x):return copy.deepcopy(x)
def canonical(x):return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def sha(x):return hashlib.sha256(x if isinstance(x,bytes) else x.encode()).hexdigest()
def domain(n,b):return sha(n+'\n'+canonical(b))
def seal(b,k,n):
    b.pop(k,None);b[k]=domain(n,b);return b
def origin(n):return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED scope rejection label '+str(n)))
def row(table,key,body,n):return dict(table=table,key=clone(key),value=dict(schemaVersion=1,body=clone(body),lastCommit=origin(n)))
def fields(x,ks):return {k:x[k] for k in ks.split()}
def measure(r):
    v=clone(r['value']);del v['lastCommit'];del v['receipt']['commit']
    size=lambda x:len(canonical(x).encode())
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))
def packet():
    names=['research-prefix-components.json','research-continuation-components.json']
    raw=[(ROOT/n).read_bytes() for n in names];p,c=[json.loads(x) for x in raw]
    a=p['chain']['assignment'];old=a['plan'];runkey=fields(old,'workspaceId runId')
    startup=clone(p['chain']['startup']);cfg=startup['configuration'];task=dict(**fields(old,'workspaceId runId planId'),planRevision=2,taskId='scope-revision-task-2')
    cfg.update(planRevision=2,taskId=task['taskId'],configurationId='scope-revision-configuration-2')
    cfg['assignment']['worktreeGroupId']=startup['context']['groupId']='scope-revision-group-2'
    for b in [startup['scope'],*startup['captures']]:b['task']=clone(task)
    for b in startup['captures']:seal(b,'captureDigest','orkworks.research-source-capture.v1')
    startup['scope']['captures']=[dict(task=task,**fields(x,'sourceId captureDigest')) for x in startup['captures']]
    seal(startup['scope'],'scopeDigest','orkworks.research-task-scope.v1');scope=dict(task=task,scopeDigest=startup['scope']['scopeDigest'])
    for manifest in cfg['requirementManifests']:manifest['taskId']=task['taskId']
    e=startup['evidenceObjects'][0];settings=json.loads(base64.b64decode(e['contentBase64']));settings['deliveryOrder'][-1]['scopeDigest']=scope['scopeDigest'];wire=canonical(settings).encode()
    e.update(contentBase64=base64.b64encode(wire).decode(),contentDigest=sha(wire),sizeBytes=len(wire))
    for b in startup['evidenceObjects']:b['task']=clone(task)
    cap=cfg['capabilityEvidence'];cap['references'][0]['contentDigest']=cap['effectiveSettingsDigest']=e['contentDigest'];cap['evidenceId']='scope-revision-evidence-2';seal(cap,'evidenceDigest','orkworks.capability-evidence.v1')
    cfg['harness'].update(evidenceId=cap['evidenceId'],evidenceDigest=cap['evidenceDigest'],effectiveSettingsDigest=e['contentDigest'])
    cfg['renderedInstructions']=helpers['render'](cfg);cfg['renderedInstructionsDigest']=sha(cfg['renderedInstructions']);seal(cfg,'configurationDigest','orkworks.assignment-configuration.v3')
    payload=canonical(dict(scope=startup['scope'],captures=startup['captures']))
    startup['declaredStartup']=''.join(x['content'] for x in cfg['rules'])+cfg['renderedInstructions']+'\nORKWORKS RETAINED RESEARCH INPUT v1\n'+str(len(payload.encode()))+'\n'+payload+'\n'
    inp=clone(p['inputs'][0]['value']['body']);inp.update(preparationRevision=3,inputRevision=3)
    inp['decision'].update(inputRevision=3,scope=scope,reasons=['Synthetic changed scope proposal; no approval or producer proof.'])
    records=[dict(kind='configuration',reference=dict(task=task,**fields(cfg,'configurationId configurationDigest'))),dict(kind='research-scope',reference=scope),dict(kind='prior-plan',plan=old)]
    records += [dict(kind='research-capture',reference=x) for x in startup['scope']['captures']]
    inp['records']=sorted(records,key=lambda x:(x['kind'].encode(),canonical(x).encode()));seal(inp,'inputDigest','orkworks.preparation-input.v1')
    plan=clone(p['chain']['plan']);plan['parentPlanBinding'].update(planRevision=2,preparationRevision=3,inputDigest=inp['inputDigest'])
    plan['task'].update(taskId=task['taskId'],configuration=fields(cfg,'configurationId configurationDigest'),scope=scope,group=dict(groupId=startup['context']['groupId'],branchName='fixture/scope-revision-2',absolutePath=dict(encoding='unix-bytes',base64=base64.b64encode(b'/fixture/scope-revision-2').decode()),baseCommit=cfg['assignment']['baseCommit']))
    plan['batches']=[[task['taskId']]];seal(plan,'planDefinitionDigest','orkworks.research-source-plan.v1')
    ref=dict(**fields(task,'workspaceId runId planId planRevision'),planDefinitionDigest=plan['planDefinitionDigest']);inputref=fields(inp,'workspaceId runId preparationRevision inputRevision inputDigest')
    cases=[]
    for complete in [True,False]:
        label='complete' if complete else 'incomplete';source=dict(turn=clone(c['turns'][1]['gate']),coordination=clone(c['coordinations'][0]['row']) if complete else None,
            currentTask=clone(c['coordinations'][0]['taskAfter'] if complete else c['turns'][1]['taskAfter']))
        rb=clone(c['recoveries'][3]['row']['value']['body']['readSet']['run']['value']['body'])
        blocker=dict(id='independent-approval',kind='recovery-approval-required',task=a['reservation']['task'],detail='Independent retained gate; rejection cannot clear it.')
        rb.update(runVersion=20,state='blocked',currentInput=inputref,planReferences=[old,ref],currentPlan=old,pendingPlan=ref,blockers=[blocker])
        current=dict(schemaVersion=1,plan=old,planVersion=2,status='active',latestApproval=a['approval'],task=a['reservation']['task'],replaces=None)
        pending=dict(schemaVersion=1,plan=ref,planVersion=1,status='proposed',latestApproval=None,task=task,replaces=old)
        proposedtask=row('research_task_controls',task,dict(schemaVersion=1,task=task,taskVersion=1,status='pending',reservation=None,coordinationResult=None),9)
        selector=dict(schemaVersion=1,run=runkey,summaryVersion=0,completedResearchPlan=None,currentSummary=None,pendingStageContinue=None,finishDisposition=None)
        reads=dict(expectedCommit=origin(10),run=row('research_run_controls',runkey,rb,9),lifecycle=row('research_lifecycle_state',runkey,selector,1),currentPlan=row('research_plan_controls',old,current,2),pendingPlan=row('research_plan_controls',ref,pending,9))
        req=dict(schemaVersion=1,kind='reject-scope-revision',run=runkey,expectedRunVersion=20,input=inputref,decisionId='reject-'+label,plan=old,expectedPlanVersion=2,proposedPlan=ref,expectedProposedPlanVersion=1,reason='Reject this exact changed scope proposal.',reviewContextId=sha('UNQUALIFIED shown rejection '+label),sidecarGeneration='c'*64)
        def record(request,key,pv):
            body=dict(schemaVersion=1,key=key,request=request,readSet=clone(reads));d=domain('orkworks.research-lifecycle-decision.v1',body)
            return dict(table='research_lifecycle_records',key=key,value=dict(schemaVersion=1,body=body,digest=d,receipt=dict(schemaVersion=1,key=key,decisionDigest=d,runVersion=21,planVersion=pv,commit=origin(11)),lastCommit=origin(11)))
        rejection=record(req,dict(kind='ui-decision',run=runkey,decisionId=req['decisionId']),None)
        completion=record(dict(schemaVersion=1,kind='plan-completion',run=runkey,expectedRunVersion=20,input=inputref,plan=old,expectedPlanVersion=2,trigger=dict(kind='scope-rejection',rejection=dict(key=rejection['key'],digest=rejection['value']['digest']))),dict(kind='plan-completion',plan=old),3) if complete else None
        afterrun=clone(rb);afterrun.update(runVersion=21,pendingPlan=None,currentPlan=None if complete else old)
        after=dict(run=row('research_run_controls',runkey,afterrun,11),currentPlan=row('research_plan_controls',old,dict(current,status='complete',planVersion=3),11) if complete else clone(reads['currentPlan']),pendingPlan=row('research_plan_controls',ref,dict(pending,status='cancelled',planVersion=2),11),currentTask=clone(source['currentTask']),proposedTask=proposedtask,selector=row('research_lifecycle_state',runkey,dict(selector,completedResearchPlan=old),11) if complete else clone(reads['lifecycle']))
        cases.append(dict(id=label,source=source,rejection=rejection,completion=completion,after=after,measurements=[measure(rejection)]+([measure(completion)] if complete else [])))
    return dict(inputHashes=dict(zip(names,map(sha,raw))),assumptions=[
        'separate finite decoded alternatives, not a store, legal pending prestate or history',
        'reserved-plan singleton collection membership/publisher remains unclosed; no second active task inferred',
        'Main/parent/native/source/currentness/membership/original H/C/proofs/outcome facts supplied and unqualified',
        'resource ledgers, full effects/job/command/physical fit and ordinary capacity remain unqualified'],proposal=dict(startup=startup,input=inp,plan=plan,ref=ref,task=task),cases=cases,submissions=[old,ref],
        facts=dict(authority='supplied-current-Main',context='supplied-exact',rate='supplied-allowed',currentness='supplied-current-owner',membership='supplied-complete',sourceProofs='supplied-original',disposition='supplied-committed',originalOutcome='supplied-exact',
                   originalOutcomes={c['rejection']['value']['digest']:dict(commit=origin(11),completionDigest=c['completion']['value']['digest'] if c['completion'] else None) for c in cases}))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.write_bytes((json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode())
