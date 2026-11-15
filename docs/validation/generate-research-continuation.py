"""Independent offline continuation components; no events, store or authority."""
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
def domain(name,x):
    return sha(name+'\n'+canonical(x))
def origin(label,n):
    return dict(storageEpoch='f'*64,commitSequence=str(n),changeDigest=sha('UNQUALIFIED continuation label: '+label))
def packet():
    raw=(ROOT/'research-prefix-components.json').read_bytes()
    prefix=json.loads(raw)
    chain=prefix['chain'];a=chain['assignment'];P=a['assignmentIdentity']
    task=chain['startup']['scope']['task'];run=chain['startup']['context']['run']
    parent,child=[dict(**r['key'],attachmentDigest=r['value']['digest']) for r in prefix['runtimeHistory']]
    fields=lambda b,keys:{k:b[k] for k in keys.split()}
    input_ref=fields(prefix['inputs'][0]['value']['body'],'workspaceId runId preparationRevision inputRevision inputDigest')
    run_key=fields(run,'workspaceId runId')
    w=chain['allocation']['allocatedWorktree']
    observed=[dict(kind='private-git',path=w['canonicalPrivateGitDirectory'],object=w['privateGitDirectoryObject']),dict(kind='root',path=w['canonicalRoot'],object=w['rootObject'])]
    def row(table,key,body,label,n=1):
        return dict(table=table,key=copy.deepcopy(key),value=dict(schemaVersion=1,body=copy.deepcopy(body),lastCommit=origin(label,n)))
    def task_body(version,status,result=None):
        return dict(schemaVersion=1,task=task,taskVersion=version,status=status,reservation=a['reservation'],coordinationResult=result)
    def session(label,state):
        return row('research_session_states',task,dict(schemaVersion=1,task=task,reservation=a['reservation'],assignmentIdentity=P,
            attachmentId=a['attachmentId'],sessionId=P['childSessionId'],producerRevision='1' if state=='prepared' else '3',state=state,runtime=None if state=='prepared' else child),label)
    turns=[]
    for kind in ['codex_stop_receipt','ui_action','child_terminal_without_receipt']:
        label='turn-'+kind
        evidence=dict(kind=kind,session=session(label+'-terminal','terminal')) if kind=='child_terminal_without_receipt' else dict(kind=kind,runtime=child)
        if kind=='ui_action':evidence.update(reviewContextId=sha('Unqualified shown original turn context'),sidecarGeneration=child['sidecarGeneration'])
        body=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=P['childSessionId'],expectedTaskVersion=7,acceptedTaskVersion=8,evidence=evidence)
        gate=row('research_turn_completions',task,body,label,3)
        gate['value']['digest']=domain('orkworks.research-source-turn-completion.v1',body)
        turns.append(dict(id=kind,gate=gate,taskBefore=row('research_task_controls',task,task_body(7,'running'),label+'-before'),
                          taskAfter=row('research_task_controls',task,task_body(8,'needs_parent_result'),label,3),runVersions=dict(before=10,after=11)))
    coordinations=[]
    for outcome in ['completed','blocked','failed']:
        label='coordination-'+outcome
        request=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=P['childSessionId'],expectedTaskVersion=8,outcome=outcome,summary='Synthetic parent outcome — not quality or native proof.')
        d=domain('orkworks.research-source-coordination-request.v1',request);C=origin(label,4)
        receipt=dict(schemaVersion=1,task=task,reservation=a['reservation'],childSessionId=P['childSessionId'],acceptedTaskVersion=9,commit=C)
        metadata=dict(table='research_coordination_receipts',key=task,value=dict(schemaVersion=1,request=request,requestDigest=d,receipt=receipt,lastCommit=C))
        coordinations.append(dict(id=outcome,gate=copy.deepcopy(turns[1]['gate']),row=metadata,taskBefore=copy.deepcopy(turns[1]['taskAfter']),
            taskAfter=row('research_task_controls',task,task_body(9,{'completed':'reported_complete','blocked':'reported_blocked','failed':'reported_failed'}[outcome],dict(requestDigest=d)),label,4),runVersions=dict(before=11,after=12)))
    recoveries=[]
    for kind in ['settle-no-child','confirm-original-allocation','confirm-original-attachment','observe-terminal']:
        label='recovery-'+kind
        attached=kind in ['confirm-original-attachment','observe-terminal'];terminal=kind=='observe-terminal'
        version=8 if terminal else 5
        blockers=[dict(id='a-disposition',kind='launch-interrupted',task=task,detail='Synthetic original disposition needs proof.'),
                  dict(id='b-approval',kind='recovery-approval-required',task=task,detail='Retry still needs a separately approved task/revision.')]
        run_body=dict(schemaVersion=1,profile='single-task-retained-file-research-prefix-v1',
            **fields(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
            runVersion=20,state='blocked',currentInput=input_ref,planReferences=[a['plan']],currentPlan=a['plan'],pendingPlan=None,parentRuntime=parent,
            blockers=blockers,stageContinueRequired=False,pendingClarification=None,pauseReason=None,resumeTarget=None)
        launch=dict(schemaVersion=1,task=task,reservation=a['reservation'],admission=dict(approval=a['approval'],parentRuntime=parent,
            expectedTaskVersion=chain['reservation']['expectedTaskVersion'],assignmentTaskVersion=P['taskVersion']),phase='ended' if terminal else 'interrupted',
            allocation=a['allocation'] if attached else None,attachmentId=a['attachmentId'] if attached else None,
            spawnRuntime=fields(child,'sidecarGeneration launchGeneration') if terminal else None,observedDirectories=observed if attached else [],
            interruptedAt=None if terminal else 'spawn' if attached else 'allocation',reasonCode=None if terminal else 'disposition-unknown')
        reads=dict(expectedCommit=origin(label+'-H',2),run=row('research_run_controls',run_key,run_body,label+'-run'),
            task=row('research_task_controls',task,task_body(version,'needs_parent_result' if terminal else 'launch_interrupted'),label+'-task'),
            launch=row('research_launch_controls',task,launch,label+'-launch'),session=session(label+'-session','terminal' if terminal else 'prepared') if attached else None)
        body=dict(schemaVersion=1,recoveryId=label,task=task,reservation=a['reservation'],expectedRunVersion=20,expectedTaskVersion=version,
            reviewContextId=sha('Unqualified shown recovery: '+kind),sidecarGeneration=parent['sidecarGeneration'],kind=kind,
            allocation=None if kind=='settle-no-child' else a['allocation'],attachmentId=a['attachmentId'] if attached else None,readSet=reads,
            observedDirectories=observed if kind!='settle-no-child' else [],blockers=copy.deepcopy(blockers),reason='Synthetic original-attempt decision; no actual native proof.')
        d=domain('orkworks.research-source-recovery-decision.v1',body);C=origin(label,3)
        receipt=dict(schemaVersion=1,recoveryId=label,decisionDigest=d,runVersion=21,taskVersion=version if terminal else version+1,commit=C)
        metadata=dict(table='research_recovery_decisions',key=dict(task=task,recoveryId=label),value=dict(schemaVersion=1,body=body,digest=d,receipt=receipt,lastCommit=C))
        recoveries.append(dict(id=kind,row=metadata))
    def measure(r):
        value=copy.deepcopy(r['value']);del value['lastCommit']
        if 'receipt' in value:del value['receipt']['commit']
        size=lambda x:len(canonical(x).encode('utf-8'))
        return dict(chargeBytes=size(dict(key=r['key'],value=value)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))
    return dict(inputSha256=sha(raw),assumptions=[
        'alternative examples, not one store or legal history',
        'fixture-tool is not Codex Stop capability evidence; actual authenticated handler supplied',
        'current parent/UI authority, rate, report/review eligibility and original native facts supplied',
        'complete memberships, currentness, symbolic C/commands/proofs and actual outcomes unqualified',
        'resource balances and full command/job/physical fit unqualified'],
        turns=turns,coordinations=coordinations,recoveries=recoveries,
        policies=[dict(id=k,expected=v) for k,v in [('late-Stop-after-UI','original-UI-gate/no-write'),('exact-parent-retry','original-receipt/no-write'),
            ('changed-parent-retry','conflict'),('revoked-parent-retry','deny-before-lookup'),('unknown-recovery-outcome','retain/no-retry'),
            ('new-recovery-at-32','deny'),('completed-task-live-child','capacity-held'),('recovery-with-stage-gate','stage-gate-preserved')]],
        measurements=dict(turns=[measure(x['gate']) for x in turns],coordinations=[measure(x['row']) for x in coordinations],recoveries=[measure(x['row']) for x in recoveries]))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
