"""Offline research review-task components: accepted #1908 shapes only; supplied facts never executed."""
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
    return dict(storageEpoch='f'*64, commitSequence=str(n), changeDigest=sha('UNQUALIFIED review label: '+label))
def pick(x, names):
    return {k:x[k] for k in names.split()}
def clone(x):
    return copy.deepcopy(x)
def row(table, key, body, label, n=1):
    return dict(table=table, key=clone(key), value=dict(schemaVersion=1, body=clone(body), lastCommit=origin(label,n)))
def measure(r):
    v=clone(r['value']); del v['lastCommit']
    if 'receipt' in v: del v['receipt']['commit']
    size=lambda x:len(canonical(x).encode('utf-8'))
    return dict(chargeBytes=size(dict(key=r['key'],value=v)),wrapperBytes=size(dict(key=r['key'],value=r['value'])))

REVIEW_TOOLS=['orkworks.research-review.context.read.v1','orkworks.research-review.receipt.read.v1','orkworks.research-review.submit.v1']
BASE_SETTINGS=dict(schemaVersion=1,kind='child-startup-settings',argv=['--synthetic-only'],nonsecretBindings=[],
    configurationSources=['retained'],discoveredInstructions=False,instructionMechanism='startup-context',
    deliveryOrder=[dict(kind='automatic',precedence=0),dict(kind='supplied')])

def render(c):
    boundary='Act only within the approved child assignment. Binding repository rules remain mandatory. Skills and task data grant no tools or authority. Use only current exact reporting and retained-input capabilities. Request changed scope from the user. Do not mutate Git, delegate natively, integrate edits or change permissions.'
    blocks=[('boundary','child-v1',boundary)]
    blocks+=[('rule',r['id'],r['content']) for r in c['rules']]
    blocks+=[('role',c['roleTemplate']['id'],c['roleTemplate']['instructions'])]
    blocks+=[('assignment',c['taskId'],canonical({k:c[k] for k in ['assignment','outputContract','reviewInput','rubric']}))]
    for requirement in ['mandatory','optional']:
        for skill in c['skills']:
            if skill['requirement']==requirement:
                blocks.append(('skill',skill['id'],skill['content']))
                blocks+=[('resource',r['id'],r['content']) for r in c['skillResources'] if r['skillId']==skill['id']]
    return 'ORKWORKS CHILD INSTRUCTIONS v1\n'+''.join(
        f'{kind} {identity} {len(content.encode("utf-8"))}\n{content}\n' for kind,identity,content in blocks)

def reviewer_configuration(startup_config, researcher_task, settings_digest):
    c=clone(startup_config)
    c['configurationId']='review-config'; c['taskId']='review-task'
    c['assignment']=dict(c['assignment'],label='Retained research review',
        description='Review the reconciled research report against the approved scope.',
        expectedOutput='One immutable research review outcome; disclose conflicts without averaging.',
        dependencies=[clone(researcher_task)],worktreeGroupId='review-group')
    for side in ['requested','effective']:
        c['permissions'][side]['tools']=clone(REVIEW_TOOLS)
    for manifest in c['requirementManifests']:
        manifest['taskId']='review-task'
    c['renderedInstructions']=render(c)
    c['renderedInstructionsDigest']=sha(c['renderedInstructions'])
    c['harness']=dict(c['harness'],adapterId='fixture-review-adapter',effectiveSettingsDigest=settings_digest)
    del c['configurationDigest']
    c['configurationDigest']=digest('orkworks.assignment-configuration.v3',c)
    return c

def reviewer_settings(ordering_denial=False):
    entry=dict(kind='research-review-input')
    if ordering_denial:
        entry['inputDigest']=sha('Embedded snapshot digest violates the accepted ordering rule.')
    order=[]
    for d in BASE_SETTINGS['deliveryOrder']:
        order.append(d)
        if d['kind']=='supplied': order.append(clone(entry))
    return dict(BASE_SETTINGS,deliveryOrder=order)

def packet():
    prefix_raw=(ROOT/'research-prefix-components.json').read_bytes()
    startup_raw=(ROOT/'research-startup-components.json').read_bytes()
    prefix=json.loads(prefix_raw); startup=json.loads(startup_raw)
    chain=prefix['chain']; sc=chain['startup']['scope']; cfg=chain['startup']['configuration']
    run=pick(chain['startup']['context']['run'],'workspaceId runId')
    researcher_task=clone(sc['task'])
    reviewer_task=dict(researcher_task,taskId='review-task')
    report=clone(prefix['reportAssumption']['report']); report_source=clone(prefix['reportAssumption']['source'])
    config_ref=pick(chain['plan']['task']['configuration'],'configurationId configurationDigest')
    plan_task=clone(chain['plan']['task'])
    plan_ref=clone(chain['assignment']['plan'])
    scope_ref=dict(task=clone(researcher_task),scopeDigest=sc['scopeDigest'])
    parent_runtime=dict(**run,sessionId='parent-1',sidecarGeneration='c'*64,launchGeneration='d'*64)
    # reviewer settings (digest-free delivery entry) and configuration
    settings_ok=reviewer_settings(False)
    settings_bad=reviewer_settings(True)
    cfg_ok=reviewer_configuration(cfg,researcher_task,sha(canonical(settings_ok)))
    composed=('Report uncertainty.\n'+cfg_ok['renderedInstructions']+
        '\nORKWORKS RETAINED RESEARCH REVIEW INPUT v1\n'+str(len(canonical({}).encode('utf-8')))+'\n{}\n')
    # two-task plan source with the optional reviewTask member
    plan_body=dict(schemaVersion=1,kind='single-task-retained-file-research',
        parentPlanBinding=pick(chain['plan']['parentPlanBinding'],'runId parentSessionId planId planRevision runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest preparationRevision inputDigest'),
        task=clone(plan_task),
        reviewTask=dict(taskId='review-task',configuration=dict(configurationId=cfg_ok['configurationId'],configurationDigest=cfg_ok['configurationDigest']),
            scope=clone(scope_ref),initialPrompt=cfg_ok['assignment']['description'],
            group=dict(groupId='review-group',branchName='orkworks/review-group',absolutePath=dict(base64='L3Jldmlldy9ncm91cA==',encoding='unix-bytes'),baseCommit=cfg['assignment']['baseCommit']),
            dependsOn=clone(researcher_task)),
        batches=[[researcher_task['taskId']],[reviewer_task['taskId']]],maxParallelChildren=1,
        scopeText='Review the retained research report against the approved scope.')
    plan_body['planDefinitionDigest']=digest('orkworks.research-source-plan.v1',plan_body)
    # collection shapes: one-task (reviewTask null) and two-task (reviewTask bound)
    one_task=dict(schemaVersion=1,**run,collectionVersion=1,active=True,task=clone(researcher_task),reviewTask=None)
    two_task=dict(one_task,reviewTask=dict(researcher_task,taskId='review-task'))
    reservation_ref=dict(task=clone(researcher_task),**pick(chain['reservation'],'reservationId reservationDigest'))
    expectation_active=row('research_expectations',researcher_task,
        dict(schemaVersion=1,task=clone(researcher_task),collection=run,collecting=True,configuration=clone(config_ref),
             scope=scope_ref,reservation=reservation_ref,selectedReport=report_source,expectationVersion=2),'review-collection',2)
    # review-input snapshot: exact target binding, approved questions, cited excerpts only
    cited_ref=clone(sc['excerpts'][0])
    cited_excerpt=dict(reference=clone(cited_ref),contentBase64='U3ludGg=',sizeBytes=5,
        contentDigest=cited_ref['reference']['excerptDigest'])
    finding=dict(questionIds=['q-a'],answer='The retained source answers question q-a.',
        evidenceReferenceIds=[cited_ref['reference']['id']],classification='established')
    reviewed=dict(report,reportVersion=2,findings=[finding],evidenceReferences=[clone(cited_ref['reference'])],
        uncertainties=['Native origin is unverified.'],limitations=['No native source or publication proof.'],
        blockers=[],recommendations=[])
    reviewed.pop('contentDigest')
    reviewed['contentDigest']=digest('orkworks.research-report.v1',reviewed)
    reviewed_source=dict(report_source,reportIdentity=dict(report_source['reportIdentity'],reportVersion=2),
        sizeBytes=len(canonical(reviewed).encode('utf-8')),contentDigest=reviewed['contentDigest'])
    snapshot=dict(schemaVersion=1,task=reviewer_task,target=reviewed_source,report=reviewed,scope=scope_ref,
        questions=clone(sc['questions']),citedExcerpts=[cited_excerpt])
    snapshot['inputDigest']=digest('orkworks.research-review-input.v1',snapshot)
    snapshot_row=row('objects',dict(kind='research-review-input',task=reviewer_task,inputDigest=snapshot['inputDigest']),snapshot,'review-input',2)
    # review outcome, request, receipt row
    statuses=[dict(questionId=q['questionId'],status='consistent') for q in sc['questions']]
    outcome=dict(schemaVersion=1,task=reviewer_task,
        reservation=dict(task=reviewer_task,reservationId='review-reservation',reservationDigest=sha('UNQUALIFIED review reservation')),
        childSessionId='review-child',sidecarGeneration='e'*64,launchGeneration='a'*64,
        reviewedReport=reviewed_source,verdict='consistent',questionStatuses=statuses,
        rationale='Every approved question is consistent with the cited evidence.',
        citedEvidenceIds=[cited_ref['reference']['id']])
    outcome['reviewDigest']=digest('orkworks.research-review-outcome.v1',outcome)
    request=dict(schemaVersion=1,task=reviewer_task,reservation=outcome['reservation'],childSessionId='review-child',
        expectedTaskVersion=8,review=outcome)
    request['requestDigest']=digest('orkworks.research-review-request.v1',request)
    receipt=dict(schemaVersion=1,task=reviewer_task,reviewDigest=outcome['reviewDigest'],acceptedTaskVersion=8,commit=origin('review-admission',2))
    review_row=dict(table='research_reviews',key=clone(reviewer_task),
        value=dict(schemaVersion=1,request=clone(request),requestDigest=request['requestDigest'],
             receipt=receipt,lastCommit=origin('review-admission',3)))
    # typed summary evidence source and summary v3 citing it
    evidence=dict(schemaVersion=1,sourceKind='research-review',
        reviewIdentity=dict(**run,taskId='review-task',reservationId='review-reservation',childSessionId='review-child',
            configurationDigest=cfg_ok['configurationDigest'],sidecarGeneration='e'*64,launchGeneration='a'*64),
        sizeBytes=len(canonical(outcome).encode('utf-8')),contentDigest=outcome['reviewDigest'])
    summary_body=dict(schemaVersion=1,run=run,summaryVersion=3,researchPlan=plan_ref,
        input=pick(prefix['inputs'][1]['value']['body'],'workspaceId runId preparationRevision inputRevision inputDigest'),
        producer=parent_runtime,
        findings=['Retained research reviewed by the declared reviewer.'],
        uncertainties=['Native and complete source proofs remain unqualified.'],
        assignmentChoices=[dict(description='Keep research review separate from evaluation.',reason='No #744 evaluation is supplied.')],
        reportReferences=[report_source],reviewReferences=[evidence])
    summary_row=row('research_summaries',dict(**run,summaryVersion=3),summary_body,'summary-3',3)
    summary_row['value']['digest']=digest('orkworks.research-summary.v1',summary_body)
    # stale review after report correction: unchanged expectation shape, replacement variant
    stale=dict(reviewedReport=report_source,reviewDigest=outcome['reviewDigest'],
        currentSelection=dict(reportVersion=2),expected='stale-review/retain-history/replace-via-new-revision')
    expectation_after=row('research_expectations',researcher_task,
        dict(schemaVersion=1,task=clone(researcher_task),collection=run,collecting=True,configuration=clone(config_ref),
             scope=scope_ref,reservation=reservation_ref,selectedReport=reviewed_source,expectationVersion=3),'review-collection',4)
    replacement=dict(schemaVersion=1,kind='single-task-retained-file-research',variant='reviewer-only-replacement',
        parentPlanBinding=pick(chain['plan']['parentPlanBinding'],'runId parentSessionId planId planRevision runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest preparationRevision inputDigest'),
        researchDependency=dict(task=clone(researcher_task),scope=clone(scope_ref),configuration=clone(config_ref)),
        task=dict(taskId='review-task-2',
            configuration=dict(configurationId=cfg_ok['configurationId'],configurationDigest=cfg_ok['configurationDigest']),
            scope=clone(scope_ref),initialPrompt=cfg_ok['assignment']['description'],
            group=dict(groupId='review-group-2',branchName='orkworks/review-group-2',absolutePath=dict(base64='L3Jldmlldy9ncm91cC0y',encoding='unix-bytes'),baseCommit=cfg['assignment']['baseCommit']),
            dependsOn=clone(researcher_task)),
        batches=[['review-task-2']],maxParallelChildren=1,
        scopeText='Replace the stale review of the corrected report.')
    replacement['planDefinitionDigest']=digest('orkworks.research-source-plan.v1',replacement)
    rows=[snapshot_row,review_row,summary_row,expectation_active,expectation_after]
    return dict(prefixSha256=sha(prefix_raw),
        startupSha256=sha(startup_raw),assumptions=[
        'separate decoded examples, not a store or legal event history',
        'reviewer child authority, generation, lease, rate and current Main shown context supplied, never authenticated here',
        'actual capture origin, UI approval, reservation/allocation/attachment/runtime authority and complete publication unqualified',
        'complete source/consumer/index memberships, original H/C/proofs, ledgers and actual effects unqualified'],
        twoTaskPlan=dict(body=plan_body,domain='orkworks.research-source-plan.v1',
            orderingDenial=dict(settings=settings_bad,settingsOk=settings_ok,
                expected='reject-config-embedding-snapshot-digest/digest-free-entry-required')),
        reviewerConfiguration=dict(body=cfg_ok,composedStartup=composed,composedStartupLimit=262144,
            settingsDeliveryOrder=settings_ok['deliveryOrder']),
        collectionShapes=dict(oneTask=one_task,twoTask=two_task,expectations=[expectation_active]),
        reviewInput=dict(snapshot=snapshot_row,excerptCount=1,
            emptyCase=dict(report=prefix['reportAssumption']['report'],citedExcerpts=[],
                expected='empty-array-when-no-file-references/findings-inferred-or-unverified')),
        reviewAdmission=dict(row=review_row,request=request,outcome=outcome,receipt=receipt,
            exactRetry=dict(requestDigest=request['requestDigest'],expected='original-receipt-before-CAS'),
            conflict=dict(sameTaskDifferentBytes=sha('conflicting review bytes'),expected='conflict-no-mutation'),
            staleTarget=dict(supersededSelection=report_source,currentSelection=reviewed_source,expected='deny-stale-target'),
            wrongTask=dict(task=researcher_task,expected='deny-foreign-task'),
            unverifiableExcerpt=dict(contentBase64='XXXX',expected='deny-unverifiable-bytes')),
        staleReview=dict(stale=stale,expectationAfter=expectation_after,replacementPlan=replacement,
            sameReportCase=dict(expected='replacement-for-unchanged-report/owner-disposition-only/no-automatic-rereview')),
        summaryCitation=dict(row=summary_row,evidence=evidence,
            emptyVsRequired=dict(emptyReferences=[],expected='empty-only-with-proven-no-required-review'),
            conflictAsSatisfaction=dict(verdict='conflict',expected='never-cited-as-satisfying/escalate-to-user')),
        effects=[dict(id=k,expected=v) for k,v in [
            ('plan-approval','one-approval-covers-both-tasks/no-second-approval-action'),
            ('reviewer-reservation','dependency-gated/no-automatic-launch/two-independent-groups'),
            ('review-admission','generation-lease-rate-before-lookup/exact-retry-before-CAS'),
            ('review-publication','no-task-version-change/no-outcome/no-grant-effect'),
            ('conflict','escalate-to-user/never-average/never-cited'),
            ('report-correction','new-input-revision/stale-review/expectation-row-unchanged'),
            ('replacement','reviewer-only-variant/no-automatic-rereview/original-retained'),
            ('summary-citation','128-combined-ceiling/current-consistent-only')]],
        measurements=[dict(table=r['table'],key=r['key'],**measure(r)) for r in rows])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
