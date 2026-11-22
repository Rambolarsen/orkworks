"""Offline research terminal-completion components: accepted #1920 shapes only; supplied facts never executed."""
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
    return dict(storageEpoch='f'*64, commitSequence=str(n), changeDigest=sha('UNQUALIFIED terminal label: '+label))
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
    continuation_raw=(ROOT/'research-continuation-components.json').read_bytes()
    completion_raw=(ROOT/'research-completion-components.json').read_bytes()
    execution_raw=(ROOT/'research-execution-components.json').read_bytes()
    prefix=json.loads(prefix_raw); continuation=json.loads(continuation_raw)
    completion=json.loads(completion_raw); execution=json.loads(execution_raw)
    chain=prefix['chain']; sc=chain['startup']['scope']
    run=pick(chain['startup']['context']['run'],'workspaceId runId')
    researcher_task=clone(sc['task'])
    execution_task=dict(researcher_task,planId='execution-plan',planRevision=1,taskId='execution-task')
    execution_plan_ref=dict(workspaceId=run['workspaceId'],runId=run['runId'],planId='execution-plan',planRevision=1,
        planDefinitionDigest=sha('UNQUALIFIED supplied execution plan digest'))
    parent=parent_runtime(prefix)
    # synthetic execution-proposal prestate (labeled against that contract's accepted choices)
    prestate=dict(labeledAgainst='research-execution-proposal contract accepted choices',
        executionPlan=execution_plan_ref,
        executionTaskControl=row('research_task_controls',execution_task,
            dict(schemaVersion=1,task=execution_task,taskVersion=7,status='running',reservation=None),'terminal-prestate',2),
        sessionState=row('research_session_states',execution_task,
            dict(schemaVersion=1,task=execution_task,attachmentId='terminal-attachment',session=dict(kind='child',
                workspaceId=run['workspaceId'],runId=run['runId'],sessionId='execution-child',
                sidecarGeneration='e'*64,launchGeneration='a'*64),producerRevision='3',state='running',
                runtime=dict(kind='child',**run,sessionId='execution-child',sidecarGeneration='e'*64,
                    launchGeneration='a'*64)),'terminal-prestate',2),
        grantLabels=dict(grantExisted='supplied',grantRevokedAtCompletion='supplied',
            expected='label-assertions-only/never-serialized/never-restored'),
        postLaunchState='awaiting_execution_approval',
        postLaunchStateLabel='supplied-launch-chain-state-writer-remains-separately-gated')
    # execution task turn gate: child_terminal_without_receipt variant (synthetic runtime, supplied origin)
    turn_body=dict(schemaVersion=1,task=execution_task,reservation=dict(task=execution_task,
        reservationId='terminal-reservation',reservationDigest=sha('UNQUALIFIED terminal reservation')),
        childSessionId='execution-child',expectedTaskVersion=7,acceptedTaskVersion=8,
        evidence=dict(kind='child_terminal_without_receipt',
            session=dict(table='research_session_states',key=clone(execution_task),
                value=dict(schemaVersion=1,body=dict(clone(prestate['sessionState']['value']['body']),
                    state='terminal'),lastCommit=origin('terminal-prestate',2)))))
    turn_row=dict(table='research_turn_completions',key=clone(execution_task),
        value=dict(schemaVersion=1,body=turn_body,digest=digest('orkworks.research-source-turn-completion.v1',turn_body),
            lastCommit=origin('terminal-turn',2)))
    # coordination outcome (completed) over the execution task
    coordination_request=dict(schemaVersion=1,task=execution_task,reservation=turn_body['reservation'],
        childSessionId='execution-child',expectedTaskVersion=8,outcome='completed',
        summary='The execution task completed under its accepted result contract.')
    coordination_receipt=dict(schemaVersion=1,task=execution_task,reservation=turn_body['reservation'],
        childSessionId='execution-child',acceptedTaskVersion=9,commit=origin('terminal-coordination',2))
    coordination_row=dict(table='research_coordination_receipts',key=execution_task,
        value=dict(schemaVersion=1,request=coordination_request,
            requestDigest=digest('orkworks.research-source-coordination-request.v1',coordination_request),
            receipt=coordination_receipt,lastCommit=origin('terminal-coordination',3)))
    # completion trigger + lifecycle record (plan-completion under the execution plan)
    trigger=dict(kind='coordination',task=execution_task,acceptedTaskVersion=9,
        requestDigest=digest('orkworks.research-source-coordination-request.v1',coordination_request))
    completion_request=dict(schemaVersion=1,kind='plan-completion',run=run,expectedRunVersion=52,
        input=dict(**run,preparationRevision=3,inputRevision=3,inputDigest=sha('UNQUALIFIED current input digest')),
        plan=execution_plan_ref,expectedPlanVersion=1,trigger=trigger)
    lifecycle_key=dict(kind='plan-completion',plan=execution_plan_ref)
    run_read=row('research_run_controls',run,dict(schemaVersion=1,profile='single-task-retained-file-research-execution-v1',
        **pick(chain['startup']['context']['run'],'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
        runVersion=52,state='awaiting_execution_approval',currentInput=dict(**run,preparationRevision=3,inputRevision=3,
            inputDigest=sha('UNQUALIFIED current input digest')),planReferences=[dict(researcher_task|{},planId='prefix-plan',planRevision=1),execution_plan_ref],
        currentPlan=None,pendingPlan=execution_plan_ref,parentRuntime=parent,blockers=[],stageContinueRequired=False,
        pendingClarification=None,pauseReason=None,resumeTarget=None),'terminal-run-read',2)
    selector_read=row('research_lifecycle_state',run,dict(schemaVersion=1,run=run,summaryVersion=2,
        completedResearchPlan=dict(researcher_task|{},planId='prefix-plan',planRevision=1,
            planDefinitionDigest=sha('UNQUALIFIED research plan digest')),
        currentSummary=dict(**run,summaryVersion=2,summaryDigest=sha('UNQUALIFIED summary digest')),
        pendingStageContinue=None,finishDisposition=None),'terminal-selector-read',2)
    pending_plan_read=row('research_plan_controls',execution_plan_ref,
        dict(schemaVersion=1,plan=execution_plan_ref,planVersion=1,status='active',latestApproval=None,
            task=execution_task,replaces=None),'terminal-pending-read',2)
    readSet=dict(expectedCommit=origin('terminal-completion-H',2),run=run_read,lifecycle=selector_read,
        currentPlan=None,pendingPlan=pending_plan_read)
    lifecycle_body=dict(schemaVersion=1,key=lifecycle_key,request=completion_request,readSet=readSet)
    lifecycle_digest=digest('orkworks.research-lifecycle-decision.v1',lifecycle_body)
    lifecycle_receipt=dict(schemaVersion=1,key=lifecycle_key,decisionDigest=lifecycle_digest,runVersion=53,
        planVersion=2,commit=origin('terminal-completion',2))
    lifecycle_row=dict(table='research_lifecycle_records',key=lifecycle_key,
        value=dict(schemaVersion=1,body=lifecycle_body,digest=lifecycle_digest,receipt=lifecycle_receipt,
            lastCommit=origin('terminal-completion',3)))
    # absorbing terminal transition
    terminal_run=dict(schemaVersion=1,profile='single-task-retained-file-research-execution-v1',
        **pick(chain['startup']['context']['run'],'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
        runVersion=53,state='complete',currentInput=dict(**run,preparationRevision=3,inputRevision=3,
            inputDigest=sha('UNQUALIFIED current input digest')),
        planReferences=[dict(researcher_task|{},planId='prefix-plan',planRevision=1),execution_plan_ref],
        currentPlan=None,pendingPlan=None,parentRuntime=None,blockers=[],stageContinueRequired=False,
        pendingClarification=None,pauseReason=None,resumeTarget=None)
    terminal_run_row=row('research_run_controls',run,terminal_run,'terminal-transition',2)
    terminal_selector=row('research_lifecycle_state',run,dict(schemaVersion=1,run=run,summaryVersion=2,
        completedResearchPlan=dict(researcher_task|{},planId='prefix-plan',planRevision=1,
            planDefinitionDigest=sha('UNQUALIFIED research plan digest')),
        currentSummary=dict(**run,summaryVersion=2,summaryDigest=sha('UNQUALIFIED summary digest')),
        pendingStageContinue=None,finishDisposition=None),'terminal-transition',2)
    terminal_plan=row('research_plan_controls',execution_plan_ref,
        dict(schemaVersion=1,plan=execution_plan_ref,planVersion=2,status='complete',latestApproval=None,
            task=execution_task,replaces=None),'terminal-transition',2)
    # collection/expectation retirement (research task's rows; execution task has none in this slice)
    retired_collection=row('research_collections',run,
        dict(schemaVersion=1,workspaceId=run['workspaceId'],runId=run['runId'],collectionVersion=3,active=False,
            task=researcher_task,reviewTask=None),'collection-retirement',2)
    retired_expectation=row('research_expectations',researcher_task,
        dict(schemaVersion=1,task=researcher_task,collection=run,collecting=False,
            configuration=pick(chain['plan']['task']['configuration'],'configurationId configurationDigest'),
            scope=dict(task=researcher_task,scopeDigest=sc['scopeDigest']),
            reservation=dict(task=researcher_task,**pick(chain['reservation'],'reservationId reservationDigest')),
            selectedReport=prefix['reportAssumption']['source'],expectationVersion=3),'collection-retirement',2)
    # live child still consuming capacity (label + supplied session row)
    live_child=dict(sessionState=row('research_session_states',execution_task,
        dict(schemaVersion=1,task=execution_task,attachmentId='terminal-attachment',session=dict(kind='child',
            workspaceId=run['workspaceId'],runId=run['runId'],sessionId='execution-child',
            sidecarGeneration='e'*64,launchGeneration='a'*64),producerRevision='3',state='running',
            runtime=dict(kind='child',**run,sessionId='execution-child',sidecarGeneration='e'*64,
                launchGeneration='a'*64)),'live-child',2),
        label='live-child-capacity-held/derived-from-session-launch-controls',
        expected='no-capacity-release-for-nonterminal-children')
    # failed/blocked execution variant: run stays nonterminal
    blocked_run=dict(schemaVersion=1,profile='single-task-retained-file-research-execution-v1',
            **pick(chain['startup']['context']['run'],'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
            runVersion=53,state='blocked',currentInput=dict(**run,preparationRevision=3,inputRevision=3,
                inputDigest=sha('UNQUALIFIED current input digest')),
            planReferences=[dict(researcher_task|{},planId='prefix-plan',planRevision=1),execution_plan_ref],
            currentPlan=execution_plan_ref,pendingPlan=None,parentRuntime=parent,
            blockers=[dict(id='failed-execution',kind='launch-interrupted',task=execution_task,
                detail='Supplied failed execution outcome; recovery revision remains unclosed.')],
            stageContinueRequired=False,pendingClarification=None,pauseReason=None,resumeTarget=None)
    blocked=dict(trigger=dict(kind='coordination',task=execution_task,acceptedTaskVersion=8,
        requestDigest=sha('UNQUALIFIED blocked coordination digest')),runAfter=blocked_run,
        expected='failed-execution-never-completes/recovery-revision-unclosed')
    rows=[turn_row,coordination_row,lifecycle_row,terminal_run_row,terminal_selector,terminal_plan,
        retired_collection,retired_expectation,prestate['executionTaskControl'],prestate['sessionState'],
        live_child['sessionState']]
    return dict(prefixSha256=sha(prefix_raw),continuationSha256=sha(continuation_raw),
        completionSha256=sha(completion_raw),executionSha256=sha(execution_raw),assumptions=[
        'separate decoded examples, not a store or legal event history',
        'reviewer/execution child authority, turn-authentication, launch chain and current Main shown context supplied, never authenticated here',
        'grants are volatile server-held facts represented as label assertions only, never persisted or serialized',
        'capability/profile qualification, complete memberships, original H/C/proofs, ledgers and actual effects unqualified'],
        prestate=prestate,
        turn=dict(row=turn_row,body=turn_body,
            variant='child_terminal_without_receipt/synthetic-runtime/supplied-handler-labeling'),
        coordination=dict(row=coordination_row,request=coordination_request,receipt=coordination_receipt),
        completion=dict(trigger=trigger,request=completion_request,row=lifecycle_row,
            preconditions=dict(stageContinueRequired=False,pendingClarification=None,pendingProposal=None,
                scopeRevision='none',allocation='settled'),
            exactRetry=dict(expected='original-outcome-returned/no-second-completion'),
            parentRetryDenied=dict(afterTerminal=True,expected='parent-level-post-completion-retry-denied/source-level-idempotence-retained')),
        terminal=dict(runBefore=run_read,runAfter=terminal_run_row,selectorBefore=selector_read,
            selectorAfter=terminal_selector,planAfter=terminal_plan,
            pauseFieldsNull=True,rawGatesPreserved=True,
            grantRevocation=dict(labels=prestate['grantLabels'],
                retainedApprovalFacts=dict(approvalId='supplied',approvalDigest='supplied',
                    planDigest=sha('UNQUALIFIED supplied execution plan digest'),commitRef='supplied'),
                expected='historical-binding-facts/never-live-authority'),
            waits=dict(runClosed=True,expected='effect-label-not-a-record/no-reconnect'),
            noTerminalReportRunV1=dict(runStatesRewritten=False,secondCounter=False,
                expected='terminal-state-only-in-research_run_controls/display-label-not-qualification')),
        retirement=dict(collection=retired_collection,expectation=retired_expectation,
            expected='accepted-finish-pattern/selected-reports-retained/no-ledger-fabrication'),
        liveChild=live_child,
        failedExecution=blocked,
        accounting=dict(terminalRecordPool='child-control-retained-bytes/8MiB-run-64MiB-workspace',
            planSourcePool='child-source-retained-bytes/charged-at-creation/unchanged-here',
            expected='actual-changed-row-deltas-only/no-refund-no-new-allowance'),
        catalog=dict(candidate=63,expected='no-new-families-or-kinds/new-rows-not-names'),
        effects=[dict(id=k,expected=v) for k,v in [
            ('turn-gate','accepted-variants/synthetic-runtime-supplied-origin'),
            ('coordination','accepted-result-contract/no-research-report-check'),
            ('plan-completion','baseline-predicate/typed-trigger/same-C'),
            ('terminal-transition','absorbing-complete/authority-revoked/waits-closed/collections-retired'),
            ('selector','completedResearchPlan-unchanged/execution-plan-via-own-record'),
            ('terminal-mapping','research_run_controls-only/no-TerminalReportRunV1-cast'),
            ('live-child','capacity-held/no-release'),
            ('failed-execution','never-completes/recovery-unclosed')]],
        measurements=[dict(table=r['table'],key=r['key'],**measure(r)) for r in rows])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
