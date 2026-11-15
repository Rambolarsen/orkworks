"""Independent offline positive recovery components; no legal history or publisher."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def canonical(x):
    return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def sha(x):
    return hashlib.sha256(x if isinstance(x,bytes) else x.encode('utf-8')).hexdigest()
def digest(name,x):
    return sha(name+'\n'+canonical(x))
def clone(x):
    return copy.deepcopy(x)
def pick(x,names):
    return {k:x[k] for k in names.split()}
def origin(label,n):
    return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED positive recovery label: '+label))
def row(table,key,body,C):
    return dict(table=table,key=clone(key),value=dict(schemaVersion=1,body=clone(body),lastCommit=clone(C)))
def measure(r):
    v=clone(r['value']);del v['lastCommit']
    if 'receipt' in v:del v['receipt']['commit']
    size=lambda x:len(canonical(x).encode('utf-8'))
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))

def packet():
    raw=(ROOT/'research-prefix-components.json').read_bytes()
    old=(ROOT/'research-completion-components.json').read_bytes()
    prefix=json.loads(raw);negative=json.loads(old)['completions'][1]
    a=prefix['chain']['assignment'];task=prefix['chain']['startup']['scope']['task'];plan=a['plan'];run=prefix['chain']['startup']['context']['run']
    key=pick(run,'workspaceId runId');inp=pick(prefix['inputs'][1]['value']['body'],'workspaceId runId preparationRevision inputRevision inputDigest')
    parent,child=[dict(**x['key'],attachmentDigest=x['value']['digest']) for x in prefix['runtimeHistory']]
    w=prefix['chain']['allocation']['allocatedWorktree']
    directories=[dict(kind='private-git',path=w['canonicalPrivateGitDirectory'],object=w['privateGitDirectoryObject']),dict(kind='root',path=w['canonicalRoot'],object=w['rootObject'])]
    cases=[]
    for name in ['disposition-only','independent-approval-retained']:
        retained=name=='independent-approval-retained';T=origin(name+'-turn',3);I=origin(name+'-interruption',4);Q=origin(name+'-coordination',5);O=origin(name+'-ordinary-terminal',6);H=origin(name+'-H',7);C=origin(name+'-compound',8)
        blockers=[dict(id='a-disposition',kind='launch-interrupted',task=task,detail='Supplied original late disposition uncertainty.')]
        if retained:blockers.extend([dict(id='b-disposition',kind='launch-interrupted',task=task,detail='Supplied second original disposition predicate.'),dict(id='c-approval',kind='recovery-approval-required',task=task,detail='Independent approval blocker must remain.')])
        def run_body(version,state,**changes):
            b=dict(schemaVersion=1,profile='single-task-retained-file-research-prefix-v1',**pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
                   runVersion=version,state=state,currentInput=inp,planReferences=[plan],currentPlan=plan,pendingPlan=None,parentRuntime=parent,blockers=clone(blockers),stageContinueRequired=False,pendingClarification=None,pauseReason=None,resumeTarget=None)
            b.update(changes);return b
        def task_body(version,status,result=None):
            return dict(schemaVersion=1,task=task,taskVersion=version,status=status,reservation=a['reservation'],coordinationResult=result)
        launch=dict(schemaVersion=1,task=task,reservation=a['reservation'],admission=dict(approval=a['approval'],parentRuntime=parent,expectedTaskVersion=prefix['chain']['reservation']['expectedTaskVersion'],assignmentTaskVersion=a['assignmentIdentity']['taskVersion']),
                    phase='interrupted',allocation=a['allocation'],attachmentId=a['attachmentId'],spawnRuntime=pick(child,'sidecarGeneration launchGeneration'),observedDirectories=directories,interruptedAt='spawn',reasonCode='disposition-unknown')
        interruption=dict(run=row('research_run_controls',key,run_body(12,'blocked'),I),task=row('research_task_controls',task,task_body(9,'needs_parent_result'),I),launch=row('research_launch_controls',task,launch,I))
        turn_body=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=a['assignmentIdentity']['childSessionId'],expectedTaskVersion=7,acceptedTaskVersion=8,
                       evidence=dict(kind='ui_action',runtime=child,reviewContextId=sha('Unqualified original turn: '+name),sidecarGeneration=child['sidecarGeneration']))
        turn=row('research_turn_completions',task,turn_body,T);turn['value']['digest']=digest('orkworks.research-source-turn-completion.v1',turn_body)
        request=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=a['assignmentIdentity']['childSessionId'],expectedTaskVersion=9,outcome='completed',summary='Explicit hypothetical parent outcome; no native or quality proof.')
        D=digest('orkworks.research-source-coordination-request.v1',request)
        coordination=dict(table='research_coordination_receipts',key=task,value=dict(schemaVersion=1,request=request,requestDigest=D,receipt=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=a['assignmentIdentity']['childSessionId'],acceptedTaskVersion=10,commit=Q),lastCommit=Q))
        current_run=row('research_run_controls',key,run_body(14,'blocked'),O)
        current_task=row('research_task_controls',task,task_body(11,'reported_complete',dict(requestDigest=D)),O)
        session=row('research_session_states',task,dict(schemaVersion=1,task=task,reservation=a['reservation'],assignmentIdentity=a['assignmentIdentity'],attachmentId=a['attachmentId'],sessionId=a['assignmentIdentity']['childSessionId'],producerRevision='4',state='terminal',runtime=child),O)
        state=dict(schemaVersion=1,run=key,summaryVersion=0,completedResearchPlan=None,currentSummary=None,pendingStageContinue=None,finishDisposition=None)
        selector=row('research_lifecycle_state',key,state,origin(name+'-selector',1))
        plan_body=dict(schemaVersion=1,plan=plan,planVersion=2,status='active',latestApproval=a['approval'],task=task,replaces=None)
        plan_before=row('research_plan_controls',plan,plan_body,origin(name+'-plan',1))
        recovery_id='positive-'+name
        body=dict(schemaVersion=1,recoveryId=recovery_id,task=task,reservation=a['reservation'],expectedRunVersion=14,expectedTaskVersion=11,
                  reviewContextId=sha('Unqualified shown settlement: '+name),sidecarGeneration=parent['sidecarGeneration'],kind='observe-terminal',allocation=a['allocation'],attachmentId=a['attachmentId'],
                  readSet=dict(expectedCommit=H,run=current_run,task=current_task,launch=interruption['launch'],session=session),observedDirectories=directories,blockers=clone(blockers),reason='Supplied proven original ownership and current terminal/native settlement.')
        RD=digest('orkworks.research-source-recovery-decision.v1',body)
        recovery=dict(table='research_recovery_decisions',key=dict(task=task,recoveryId=recovery_id),value=dict(schemaVersion=1,body=body,digest=RD,receipt=dict(schemaVersion=1,recoveryId=recovery_id,decisionDigest=RD,runVersion=15,taskVersion=12,commit=C),lastCommit=C))
        lifecycle_key=dict(kind='plan-completion',plan=plan)
        lifecycle_request=dict(schemaVersion=1,kind='plan-completion',run=key,expectedRunVersion=14,input=inp,plan=plan,expectedPlanVersion=2,trigger=dict(kind='recovery',task=task,recoveryId=recovery_id,decisionDigest=RD))
        lifecycle_body=dict(schemaVersion=1,key=lifecycle_key,request=lifecycle_request,readSet=dict(expectedCommit=H,run=clone(current_run),lifecycle=selector,currentPlan=plan_before,pendingPlan=None))
        LD=digest('orkworks.research-lifecycle-decision.v1',lifecycle_body)
        completion=dict(table='research_lifecycle_records',key=lifecycle_key,value=dict(schemaVersion=1,body=lifecycle_body,digest=LD,receipt=dict(schemaVersion=1,key=lifecycle_key,decisionDigest=LD,runVersion=15,planVersion=3,commit=C),lastCommit=C))
        bindings=dict(schemaVersion=1,expectedCommit=H,bindings=[dict(blocker=clone(b),reservation=a['reservation'],interruptionRun=clone(interruption['run']),interruptionTask=clone(interruption['task']),interruptionLaunch=clone(interruption['launch']),disposition='supplied-resolved') for b in blockers if b['kind']=='launch-interrupted'])
        remaining=[b for b in blockers if b['kind']!='launch-interrupted']
        after=dict(run=row('research_run_controls',key,run_body(15,'blocked' if retained else 'synthesizing',currentPlan=None,blockers=remaining),C),
                   task=row('research_task_controls',task,task_body(12,'reported_complete',dict(requestDigest=D)),C),
                   launch=row('research_launch_controls',task,dict(launch,phase='ended',interruptedAt=None,reasonCode=None),C),
                   plan=row('research_plan_controls',plan,dict(plan_body,planVersion=3,status='complete'),C),
                   lifecycle=row('research_lifecycle_state',key,dict(state,completedResearchPlan=plan),C),sessionRetained=clone(session))
        sources=dict(turn=turn,interruption=interruption,coordination=coordination,coordinationTask=row('research_task_controls',task,task_body(10,'reported_complete',dict(requestDigest=D)),Q),coordinationRun=row('research_run_controls',key,run_body(13,'blocked'),Q))
        cases.append(dict(id=name,sources=sources,recovery=recovery,completion=completion,bindings=bindings,after=after))
    measurements=[dict(id=x['id'],recovery=measure(x['recovery']),completion=measure(x['completion']),bindingsBytes=len(canonical(x['bindings']).encode('utf-8')),
                       suppliedComponentBytes=len(canonical(dict(recovery=x['recovery'],completion=x['completion'],bindings=x['bindings'])).encode('utf-8'))) for x in cases]
    return dict(prefixSha256=sha(raw),priorCompletionSha256=sha(old),assumptions=[
        'independent decoded components, not a store, legal history or publisher',
        'original spawn/turn/result and late interruption occurrence are stipulated source facts',
        'Main authority/context, native ownership/quiescence, terminal bridge and complete membership/proofs supplied, not qualified',
        'symbolic H/C, actual atomic effects, capacity, ledger balances and complete physical/job fit unqualified'],
        negative=negative,cases=cases,measurements=measurements)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
