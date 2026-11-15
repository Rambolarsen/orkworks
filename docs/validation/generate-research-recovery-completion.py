"""Independent offline source examples; no native events or physical history."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
def clone(x): return copy.deepcopy(x)
def canonical(x): return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def sha(x): return hashlib.sha256(x if isinstance(x,bytes) else x.encode()).hexdigest()
def digest(domain,x): return sha(domain+'\n'+canonical(x))
def origin(n): return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED positive recovery label '+str(n)))
def row(table,key,body,n): return dict(table=table,key=clone(key),value=dict(schemaVersion=1,body=clone(body),lastCommit=origin(n)))
def measure(r):
    v=clone(r['value']);del v['lastCommit'];del v['receipt']['commit']
    size=lambda x:len(canonical(x).encode())
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))
def packet():
    inputs={n:(ROOT/n).read_bytes() for n in ['research-prefix-components.json','research-continuation-components.json','research-completion-components.json']}
    p=json.loads(inputs['research-prefix-components.json'])
    a=p['chain']['assignment'];task=p['chain']['startup']['scope']['task'];plan=a['plan']
    runkey={k:task[k] for k in ['workspaceId','runId']}
    parent,child=[dict(**r['key'],attachmentDigest=r['value']['digest']) for r in p['runtimeHistory']]
    inp={k:p['inputs'][0]['value']['body'][k] for k in ['workspaceId','runId','preparationRevision','inputRevision','inputDigest']}
    source=p['chain']['startup']['context']['run']
    base={k:source[k] for k in ['workspaceId','runId','parentSessionId','runDefinitionDigest','bootstrapConfigurationDigest','repositoryBindingDigest','sourceWorktreeBindingDigest']}
    blockers=[dict(id='a-disposition',kind='launch-interrupted',task=task,detail='Supplied late disposition interruption of the proven original spawn.'),
              dict(id='b-approval',kind='recovery-approval-required',task=task,detail='Independent approval gate remains unresolved.')]
    def runbody(v):
        return dict(schemaVersion=1,profile='single-task-retained-file-research-prefix-v1',**base,runVersion=v,state='blocked',currentInput=inp,
                    planReferences=[plan],currentPlan=plan,pendingPlan=None,parentRuntime=parent,blockers=clone(blockers),
                    stageContinueRequired=False,pendingClarification=None,pauseReason=None,resumeTarget=None)
    def taskbody(v,status,result=None): return dict(schemaVersion=1,task=task,taskVersion=v,status=status,reservation=a['reservation'],coordinationResult=result)
    w=p['chain']['allocation']['allocatedWorktree']
    observed=[dict(kind='private-git',path=w['canonicalPrivateGitDirectory'],object=w['privateGitDirectoryObject']),dict(kind='root',path=w['canonicalRoot'],object=w['rootObject'])]
    launch=dict(schemaVersion=1,task=task,reservation=a['reservation'],admission=dict(approval=a['approval'],parentRuntime=parent,
        expectedTaskVersion=p['chain']['reservation']['expectedTaskVersion'],assignmentTaskVersion=a['assignmentIdentity']['taskVersion']),
        phase='interrupted',allocation=a['allocation'],attachmentId=a['attachmentId'],spawnRuntime={k:child[k] for k in ['sidecarGeneration','launchGeneration']},
        observedDirectories=observed,interruptedAt='spawn',reasonCode='disposition-unknown')
    # New source-derived snapshots, not edits to or adoption of the old negative
    # recovery. Sequence numbers are label ordering, never a physical history.
    turnbody=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=a['assignmentIdentity']['childSessionId'],
                  expectedTaskVersion=7,acceptedTaskVersion=8,evidence=dict(kind='ui_action',runtime=child,
                  reviewContextId=sha('UNQUALIFIED positive turn context'),sidecarGeneration=child['sidecarGeneration']))
    turn=row('research_turn_completions',task,turnbody,3);turn['value']['digest']=digest('orkworks.research-source-turn-completion.v1',turnbody)
    interruption=dict(run=row('research_run_controls',runkey,runbody(12),4),task=row('research_task_controls',task,taskbody(9,'needs_parent_result'),4),
                      launch=row('research_launch_controls',task,launch,4))
    request=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=a['assignmentIdentity']['childSessionId'],expectedTaskVersion=9,
                 outcome='completed',summary='Explicit synthetic parent outcome after the disposition interruption.')
    rd=digest('orkworks.research-source-coordination-request.v1',request)
    coordination=dict(table='research_coordination_receipts',key=task,value=dict(schemaVersion=1,request=request,requestDigest=rd,
        receipt=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=request['childSessionId'],acceptedTaskVersion=10,commit=origin(5)),lastCommit=origin(5)))
    terminal=dict(session=row('research_session_states',task,dict(schemaVersion=1,task=task,reservation=a['reservation'],assignmentIdentity=a['assignmentIdentity'],
        attachmentId=a['attachmentId'],sessionId=request['childSessionId'],producerRevision='4',state='terminal',runtime=child),6),
        task=row('research_task_controls',task,taskbody(11,'reported_complete',dict(requestDigest=rd)),6),run=row('research_run_controls',runkey,runbody(14),6))
    recoverybody=dict(schemaVersion=1,recoveryId='late-disposition-terminal',task=task,reservation=a['reservation'],expectedRunVersion=14,expectedTaskVersion=11,
        reviewContextId=sha('UNQUALIFIED positive recovery context'),sidecarGeneration=parent['sidecarGeneration'],kind='observe-terminal',allocation=a['allocation'],attachmentId=a['attachmentId'],
        readSet=dict(expectedCommit=origin(7),run=terminal['run'],task=terminal['task'],launch=interruption['launch'],session=terminal['session']),
        observedDirectories=observed,blockers=blockers,reason='Settle the retained original disposition only; no launch or result rewrite.')
    d=digest('orkworks.research-source-recovery-decision.v1',recoverybody)
    recovery=dict(table='research_recovery_decisions',key=dict(task=task,recoveryId=recoverybody['recoveryId']),value=dict(schemaVersion=1,body=recoverybody,digest=d,
        receipt=dict(schemaVersion=1,recoveryId=recoverybody['recoveryId'],decisionDigest=d,runVersion=15,taskVersion=12,commit=origin(8)),lastCommit=origin(8)))
    selector=dict(schemaVersion=1,run=runkey,summaryVersion=0,completedResearchPlan=None,currentSummary=None,pendingStageContinue=None,finishDisposition=None)
    planbody=dict(schemaVersion=1,plan=plan,planVersion=2,status='active',latestApproval=a['approval'],task=task,replaces=None)
    key=dict(kind='plan-completion',plan=plan)
    req=dict(schemaVersion=1,kind='plan-completion',run=runkey,expectedRunVersion=14,input=inp,plan=plan,expectedPlanVersion=2,
             trigger=dict(kind='recovery',task=task,recoveryId=recoverybody['recoveryId'],decisionDigest=d))
    lifecyclebody=dict(schemaVersion=1,key=key,request=req,readSet=dict(expectedCommit=origin(7),run=terminal['run'],
                      lifecycle=row('research_lifecycle_state',runkey,selector,1),currentPlan=row('research_plan_controls',plan,planbody,2),pendingPlan=None))
    ld=digest('orkworks.research-lifecycle-decision.v1',lifecyclebody)
    lifecycle=dict(table='research_lifecycle_records',key=key,value=dict(schemaVersion=1,body=lifecyclebody,digest=ld,
        receipt=dict(schemaVersion=1,key=key,decisionDigest=ld,runVersion=15,planVersion=3,commit=origin(8)),lastCommit=origin(8)))
    afterrun=runbody(15);afterrun.update(currentPlan=None,blockers=[blockers[1]])
    afterlaunch=clone(launch);afterlaunch.update(phase='ended',interruptedAt=None,reasonCode=None)
    after=dict(run=row('research_run_controls',runkey,afterrun,8),task=row('research_task_controls',task,taskbody(12,'reported_complete',dict(requestDigest=rd)),8),
               launch=row('research_launch_controls',task,afterlaunch,8),session=clone(terminal['session']),
               plan=row('research_plan_controls',plan,dict(planbody,planVersion=3,status='complete'),8),
               selector=row('research_lifecycle_state',runkey,dict(selector,completedResearchPlan=plan),8))
    bindings=dict(schemaVersion=1,expectedCommit=origin(7),bindings=[dict(blocker=blockers[0],reservation=a['reservation'],interruptionRun=interruption['run'],
                  interruptionTask=interruption['task'],interruptionLaunch=interruption['launch'],disposition='supplied-resolved')])
    facts={k:'supplied-qualified' for k in ['originalSpawn','nativeSettlement','sourceProofs','membership','currentness','planEligibility','grantFencing','capacity']}
    facts.update(authority='supplied-current-Main',rate='supplied-allowed',context='supplied-exact',disposition='supplied-committed',runState='blocked')
    return dict(inputHashes={k:sha(v) for k,v in inputs.items()},assumptions=[
        'decoded hypothetical source snapshots, not a legal history or current store',
        'UI/native/authority/source/proof/currentness/membership/outcome facts supplied and unqualified',
        'symbolic CommitRefs hash labels only; no header, manifest, row/change hash or storage profile',
        'full effects, ledgers, grant fences, capacity and physical/job/command fit remain unqualified'],
        turn=turn,interruption=interruption,coordination=coordination,terminal=terminal,recovery=recovery,lifecycle=lifecycle,bindings=bindings,after=after,facts=facts,
        measurements=[measure(recovery),measure(lifecycle)])
if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    args.output.write_bytes((json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
