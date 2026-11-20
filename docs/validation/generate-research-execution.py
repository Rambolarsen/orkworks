"""Offline research execution-proposal components: accepted #1914 shapes only; supplied facts never executed."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parent

def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
def sha(x):
    return hashlib.sha256(x if isinstance(x, bytes) else x.encode('utf-8')).hexdigest()
def digest(name, x):
    return sha(name + '\n' + canonical(x))
def origin(label, n=1):
    return dict(storageEpoch='f'*64, commitSequence=str(n), changeDigest=sha('UNQUALIFIED execution label: '+label))
def pick(x, names):
    return {k:x[k] for k in names.split()}
def clone(x):
    return copy.deepcopy(x)
def row(table, key, body, label, n=1):
    return dict(table=table, key=clone(key), value=dict(schemaVersion=1, body=clone(body), lastCommit=origin(label,n)))
def measure(r):
    v=clone(r['value']); v.pop('lastCommit',None)
    if 'receipt' in v: v=clone(v); del v['receipt']['commit']
    size=lambda x:len(canonical(x).encode('utf-8'))
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))

def parent_runtime(prefix):
    return dict(kind='parent',**pick(prefix['runtimeHistory'][0]['key'],'workspaceId runId sessionId sidecarGeneration launchGeneration'),
        attachmentDigest=prefix['runtimeHistory'][0]['value']['digest'])

def packet():
    prefix_raw=(ROOT/'research-prefix-components.json').read_bytes()
    completion_raw=(ROOT/'research-completion-components.json').read_bytes()
    prefix=json.loads(prefix_raw); completion=json.loads(completion_raw)
    chain=prefix['chain']; cfg=chain['startup']['configuration']
    run=pick(chain['startup']['context']['run'],'workspaceId runId')
    researcher_task=clone(chain['startup']['scope']['task'])
    execution_task=dict(researcher_task,planId='execution-plan',planRevision=1,taskId='execution-task')
    plan=completion['completions'][0]['row']['value']['body']['request']['plan']
    completed_selector=completion['summaries'][1]['selectorAfter']['value']['body']
    summary_ref=completed_selector['currentSummary']
    current_input=pick(completion['summaries'][1]['row']['value']['body']['input'],'workspaceId runId preparationRevision inputRevision inputDigest')
    parent=parent_runtime(prefix)
    group=dict(groupId='execution-group',branchName='orkworks/execution-group',
        absolutePath=dict(base64='L2V4ZWN1dGlvbi9ncm91cA==',encoding='unix-bytes'),baseCommit=cfg['assignment']['baseCommit'])
    # execution proposal source binding the exact summary/completed plan/input
    proposal=dict(schemaVersion=1,kind='single-task-execution-source',
        parentPlanBinding=dict(pick(chain['plan']['parentPlanBinding'],'runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
            planId='execution-plan',planRevision=1,
            preparationRevision=current_input['preparationRevision'],inputDigest=current_input['inputDigest']),
        researchBinding=dict(summary=clone(summary_ref),completedPlan=clone(plan),input=clone(current_input)),
        task=dict(taskId='execution-task',configuration=dict(configurationId='execution-configuration',
            configurationDigest=sha('UNQUALIFIED supplied execution configuration bytes')),
            initialPrompt='Implement the reviewed approach.',group=clone(group)),
        reasons=['The reviewed research supports the proposed implementation.'],
        batches=[['execution-task']],maxParallelChildren=1,
        scopeText='Implement the reviewed approach in an independent worktree.')
    proposal['planDefinitionDigest']=digest('orkworks.research-execution-source-plan.v1',proposal)
    publication=dict(schemaVersion=1,expectedRunVersion=50,plan=proposal)
    proposal_row=dict(table='research_execution_plan_objects',key=pick(plan,'workspaceId runId')|{'planId':'execution-plan'},
        value=dict(schemaVersion=1,body=proposal,lastCommit=origin('proposal-publication',2)))
    # run control before/after: awaiting_execution_approval with pending binding
    def run_body(version,state,**changes):
        b=dict(schemaVersion=1,profile='single-task-retained-file-research-execution-v1',
            **pick(chain['startup']['context']['run'],'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
            runVersion=version,state=state,currentInput=clone(current_input),planReferences=[plan],currentPlan=None,pendingPlan=None,
            parentRuntime=parent,blockers=[],stageContinueRequired=False,pendingClarification=None,
            pauseReason=None,resumeTarget=None)
        b.update(changes);return b
    execution_plan_ref=dict(pick(plan,'workspaceId runId'),planId='execution-plan',planRevision=1,
        planDefinitionDigest=proposal['planDefinitionDigest'])
    run_after=run_body(51,'awaiting_execution_approval',pendingPlan=execution_plan_ref,
        planReferences=[plan,execution_plan_ref],
        resumeTarget='awaiting_execution_approval')
    run_row=row('research_run_controls',run,run_after,'proposal-publication',2)
    plan_control=row('research_plan_controls',execution_plan_ref,
        dict(schemaVersion=1,plan=execution_plan_ref,planVersion=1,status='proposed',latestApproval=None,
            task=execution_task,replaces=None),'proposal-publication',2)
    task_control=row('research_task_controls',execution_task,
        dict(schemaVersion=1,task=execution_task,taskVersion=1,status='pending',reservation=None),'proposal-publication',2)
    # exact approval: request/source/receipt under label-assertion grants
    approval_request=dict(schemaVersion=1,action='orkworks.research-execution-plan.approve.v1',
        plan=execution_plan_ref,approvalId='execution-approval-1',
        reviewContextId=sha('Unqualified shown approval context.'))
    approval_source=dict(schemaVersion=1,plan=execution_plan_ref,approvalId='execution-approval-1',
        reviewContextId=approval_request['reviewContextId'],sidecarGeneration=parent['sidecarGeneration'])
    approval_source['requestDigest']=digest('orkworks.research-execution-plan-request.v1',approval_request)
    approval_digest=digest('orkworks.research-execution-plan-approval.v1',approval_source)
    approval_receipt=dict(schemaVersion=1,plan=execution_plan_ref,approvalId='execution-approval-1',
        approvalDigest=approval_digest,requestDigest=approval_source['requestDigest'],commit=origin('execution-approval',2))
    approval_row=dict(table='research_execution_plan_approvals',key=dict(plan=execution_plan_ref,approvalId='execution-approval-1'),
        value=dict(schemaVersion=1,body=approval_source,digest=approval_digest,receipt=approval_receipt,
            lastCommit=origin('execution-approval',3)))
    plan_after=row('research_plan_controls',execution_plan_ref,
        dict(schemaVersion=1,plan=execution_plan_ref,planVersion=2,status='active',latestApproval=dict(plan=execution_plan_ref,
            approvalId='execution-approval-1',approvalDigest=approval_digest),
            task=execution_task,replaces=None),'execution-approval',2)
    approval_run_after=row('research_run_controls',run,run_body(52,'awaiting_execution_approval',
        currentPlan=execution_plan_ref,pendingPlan=None,planReferences=[plan,execution_plan_ref],
        resumeTarget='awaiting_execution_approval'),'execution-approval',2)
    # rejection: lifecycle decision with cancelled proposed plan
    reject_request=dict(schemaVersion=1,kind='reject-execution-proposal',run=run,expectedRunVersion=51,
        input=clone(current_input),decisionId='reject-execution-1',proposal=execution_plan_ref,
        reason='The proposed approach needs a different group.',reviewContextId=approval_request['reviewContextId'],
        sidecarGeneration=parent['sidecarGeneration'])
    def lifecycle(request,read_run,read_selector,currentPlan,pendingPlan,label,planVersion=None):
        key=dict(kind='ui-decision',run=run,decisionId=request['decisionId'])
        readSet=dict(expectedCommit=origin(label+'-H',2),run=row('research_run_controls',run,read_run,label+'-run'),
            lifecycle=row('research_lifecycle_state',run,read_selector,label+'-selector'),currentPlan=currentPlan,pendingPlan=pendingPlan)
        body=dict(schemaVersion=1,key=key,request=request,readSet=readSet)
        d=digest('orkworks.research-lifecycle-decision.v1',body)
        C=origin(label,3)
        receipt=dict(schemaVersion=1,key=key,decisionDigest=d,runVersion=read_run['runVersion']+1,planVersion=planVersion,commit=C)
        return dict(table='research_lifecycle_records',key=key,value=dict(schemaVersion=1,body=body,digest=d,receipt=receipt,lastCommit=C))
    pending_read=row('research_plan_controls',execution_plan_ref,clone(plan_control['value']['body']),'reject-read',3)
    paused_read=run_body(51,'paused',pendingPlan=execution_plan_ref,planReferences=[plan,execution_plan_ref],
        pauseReason='parent_ended',resumeTarget='awaiting_execution_approval')
    reject=lifecycle(reject_request,paused_read,
        completed_selector,None,pending_read,'reject-execution')
    reject_run_after=row('research_run_controls',run,run_body(52,'paused',pendingPlan=None,
        planReferences=[plan,execution_plan_ref],pauseReason='parent_ended',resumeTarget='awaiting_execution_approval'),
        'reject-execution',2)
    reject_plan_after=row('research_plan_controls',execution_plan_ref,
        dict(schemaVersion=1,plan=execution_plan_ref,planVersion=2,status='cancelled',latestApproval=None,
            task=execution_task,replaces=None),'reject-execution',2)
    reject_paused_case=dict(expected='captured-paused-prestate-preserved-through-rejection/same-reason-and-target')
    rows=[proposal_row,run_row,plan_control,task_control,approval_row,plan_after,approval_run_after,reject,reject_run_after,reject_plan_after]
    return dict(prefixSha256=sha(prefix_raw),completionSha256=sha(completion_raw),assumptions=[
        'separate decoded examples, not a store or legal event history',
        'current Main/parent authority, epoch, owner, lease, health, rate and shown context supplied, never authenticated here',
        'grants are volatile server-held facts represented as label assertions only, never persisted or serialized',
        'capability/profile qualification, launch-chain admission, complete memberships, original H/C/proofs, ledgers and actual effects unqualified'],
        proposal=dict(source=proposal,domain='orkworks.research-execution-source-plan.v1',
            publication=publication,objectRow=proposal_row,
            binding=dict(summary=clone(summary_ref),completedPlan=clone(plan),input=clone(current_input),
                expected='exact-current-context/no-new-input-revision/no-decision-union-sibling'),
            staleBinding=dict(currentSummary=None,expected='deny-with-current-context/admission-time-backstop'),
            denialCases=dict(multiTask=dict(batches=[['execution-task'],['execution-task-2']],expected='unavailable'),
                reusedGroup=dict(groupId='review-group',expected='no-research-or-cross-plan-reuse'),
                bearerAsApprover=dict(runBearer=True,expected='bearer-never-approves'),
                researchGrantCarryover=dict(carryover=True,expected='research-grant-revoked-at-its-own-completion'),
                unqualifiedProfile=dict(capabilityEvidence='none',expected='approval-unavailable-no-fallback'),
                staleCompletedPlan=dict(completedPlan=dict(planId='other-plan'),expected='deny-with-current-context'))),
        publicationEffects=dict(runBefore=run_body(50,'synthesizing',parentRuntime=parent),
            runAfter=run_row,planControl=plan_control,taskControl=task_control,
            gated=dict(blocked=run_body(51,'blocked',pendingPlan=execution_plan_ref,
                planReferences=[plan,execution_plan_ref],blockers=[dict(id='b',kind='launch-interrupted',
                task=execution_task,detail='Supplied unknown original native disposition.')]),
                expected='gated-state-retained-with-pending-recorded'),
            paused=dict(state='paused',pauseReason='sidecar_replaced',resumeTarget='assessing',
                expected='resumeTarget-assessing-while-paused-before-this-state')),
        approval=dict(request=approval_request,source=approval_source,row=approval_row,receipt=approval_receipt,
            planAfter=plan_after,runAfter=approval_run_after,
            grantLabels=dict(grantExisted='supplied',grantRevokedAtCompletion='supplied',grantFencing='supplied',
                expected='label-assertions-only/never-serialized/never-restored'),
            preconditions=dict(resumedParent='supplied-eligible',settledReservations='supplied-settled',
                stageContinueRequired=False,declaredTaskUnreserved=True)),
        rejection=dict(request=reject_request,row=reject,runAfter=reject_run_after,
            planAfter=reject_plan_after,pausedCase=reject_paused_case,
            replacement=dict(newPublication=True,expected='new-proposal-bound-to-current-context/never-a-mutation'),
            invalidation=dict(finish='finish-research/cancel-stage/cancel-run invalidate the recognized pending proposal',
                expected='accepted-effects-unchanged/stage-fence-rejects-publication-approval-rejection')),
        effects=[dict(id=k,expected=v) for k,v in [
            ('publication','plan/task-controls-proposed-v1/planReferences-appended/pendingPlan-bound/awaiting-execution-approval'),
            ('gated-state','blocked-or-paused-retained/resumeTarget-rules'),
            ('approval','exact-shown-bytes/resumed-eligible-parent/grant-label-assertions/no-launch'),
            ('rejection','cancelled-proposed-plan/pendingPlan-cleared/pause-retained/receipt-planVersion-null'),
            ('invalidation','finish-cancel-invalidate-recognized-pending-proposal'),
            ('denials','no-second-active-grant/stale-binding/no-research-grant-carryover')]],
        measurements=[dict(table=r['table'],key=r['key'],**measure(r)) for r in rows])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
