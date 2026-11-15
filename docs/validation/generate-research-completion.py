"""Independent decoded offline examples; no event history, publisher or authority."""
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
    return dict(storageEpoch='f'*64, commitSequence=str(n), changeDigest=sha('UNQUALIFIED completion label: '+label))
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

def packet():
    raw=(ROOT/'research-prefix-components.json').read_bytes()
    old=(ROOT/'research-continuation-components.json').read_bytes()
    p=json.loads(raw); continuation=json.loads(old)
    a=p['chain']['assignment']; task=p['chain']['startup']['scope']['task']; plan=a['plan']
    run=p['chain']['startup']['context']['run']; key=pick(run,'workspaceId runId')
    parent=dict(**p['runtimeHistory'][0]['key'],attachmentDigest=p['runtimeHistory'][0]['value']['digest'])
    inp=pick(p['inputs'][1]['value']['body'],'workspaceId runId preparationRevision inputRevision inputDigest')
    report=next(x['source'] for x in p['inputs'][1]['value']['body']['records'] if x['kind']=='research-report')
    def run_body(version,state,**changes):
        b=dict(schemaVersion=1,profile='single-task-retained-file-research-prefix-v1',
               **pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
               runVersion=version,state=state,currentInput=inp,planReferences=[plan],currentPlan=plan,pendingPlan=None,
               parentRuntime=parent,blockers=[],stageContinueRequired=False,pendingClarification=None,pauseReason=None,resumeTarget=None)
        b.update(changes);return b
    def selector(**changes):
        b=dict(schemaVersion=1,run=key,summaryVersion=0,completedResearchPlan=None,currentSummary=None,pendingStageContinue=None,finishDisposition=None)
        b.update(changes);return b
    def plan_row(label,status='active',version=2,n=1):
        return row('research_plan_controls',plan,dict(schemaVersion=1,plan=plan,planVersion=version,status=status,latestApproval=a['approval'],task=task,replaces=None),label,n)
    initial=row('research_lifecycle_state',key,selector(),'root-admission')
    def decision(label,request,rb,sb,cp=None):
        k=dict(kind='plan-completion',plan=request['plan']) if request['kind']=='plan-completion' else dict(kind='ui-decision',run=key,decisionId=request['decisionId'])
        reads=dict(expectedCommit=origin(label+'-H',2),run=row('research_run_controls',key,rb,label+'-run'),
                   lifecycle=row('research_lifecycle_state',key,sb,label+'-selector'),currentPlan=cp,pendingPlan=None)
        body=dict(schemaVersion=1,key=k,request=request,readSet=reads);d=digest('orkworks.research-lifecycle-decision.v1',body);C=origin(label,3)
        receipt=dict(schemaVersion=1,key=k,decisionDigest=d,runVersion=rb['runVersion']+1,
                     planVersion=cp['value']['body']['planVersion']+1 if request['kind'] in ['plan-completion','cancel-stage'] else None,commit=C)
        return dict(table='research_lifecycle_records',key=k,value=dict(schemaVersion=1,body=body,digest=d,receipt=receipt,lastCommit=C))
    def request(kind,version,**changes):
        b=dict(schemaVersion=1,kind=kind,run=key,expectedRunVersion=version,input=inp)
        if kind!='plan-completion':b.update(decisionId=kind,reviewContextId=sha('Unqualified shown action: '+kind),sidecarGeneration=parent['sidecarGeneration'])
        b.update(changes);return b
    coordination=clone(continuation['coordinations'][0])
    trigger=dict(kind='coordination',task=task,acceptedTaskVersion=9,requestDigest=coordination['row']['value']['requestDigest'])
    completion=decision('automatic-completion',request('plan-completion',11,plan=plan,expectedPlanVersion=2,trigger=trigger),run_body(11,'researching'),selector(),plan_row('completion-plan'))
    completed_selector=selector(completedResearchPlan=plan)
    completions=[dict(id='coordination',source=coordination,row=completion,
        runAfter=row('research_run_controls',key,run_body(12,'synthesizing',currentPlan=None),'automatic-completion',3),
        planAfter=plan_row('automatic-completion','complete',3,3),selectorAfter=row('research_lifecycle_state',key,completed_selector,'automatic-completion',3))]
    # This retained recovery's candidate task is not reported_complete. Never
    # fabricate a successful completion record from its digest alone.
    recovery=clone(continuation['recoveries'][3]['row'])
    completions.append(dict(id='recovery-not-complete',source=recovery,
        trigger=dict(kind='recovery',task=task,recoveryId=recovery['key']['recoveryId'],decisionDigest=recovery['value']['digest']),
        candidateTask=recovery['value']['body']['readSet']['task'],row=None,expected='deny-incomplete-task'))
    summaries=[]
    for version in [1,2]:
        body=dict(schemaVersion=1,run=key,summaryVersion=version,researchPlan=plan,input=inp,producer=parent,
                  findings=['Retained research source — offline example '+str(version)+'.'],uncertainties=['Native and complete source proofs remain unqualified.'],
                  assignmentChoices=[dict(description='Keep research separate from execution.',reason='No execution approval is supplied.')],
                  reportReferences=[report],reviewReferences=[])
        r=row('research_summaries',dict(**key,summaryVersion=version),body,'summary-'+str(version),3)
        r['value']['digest']=digest('orkworks.research-summary.v1',body)
        ref=dict(**r['key'],summaryDigest=r['value']['digest'])
        summaries.append(dict(id='summary-'+str(version),publication=dict(schemaVersion=1,expectedRunVersion=30+version,summary=body),row=r,
            selectorAfter=row('research_lifecycle_state',key,selector(summaryVersion=version,completedResearchPlan=plan,currentSummary=ref),'summary-'+str(version),3)))
    summary_ref=summaries[1]['selectorAfter']['value']['body']['currentSummary']
    summary_selector=selector(summaryVersion=2,completedResearchPlan=plan,currentSummary=summary_ref)
    ui=[]
    for disposition in ['research-only','no-go','declined-execution']:
        req=request('finish-research',40,researchPlan=plan,summary=summary_ref,disposition=disposition,decisionId='finish-'+disposition)
        rec=decision('finish-'+disposition,req,run_body(40,'synthesizing',currentPlan=None),summary_selector)
        ui.append(dict(id='finish-'+disposition,row=rec,
            runAfter=row('research_run_controls',key,run_body(41,'complete',currentPlan=None,parentRuntime=None),'finish-'+disposition,3),
            selectorAfter=row('research_lifecycle_state',key,dict(summary_selector,finishDisposition=disposition),'finish-'+disposition,3)))
    blocker=dict(id='retained-attempt',kind='launch-interrupted',task=task,detail='Supplied unknown original native disposition; no capacity release.')
    stage=decision('cancel-stage',request('cancel-stage',50,plan=plan,expectedPlanVersion=2,reason='Stop this research stage.'),
                   run_body(50,'blocked',blockers=[blocker]),selector(),plan_row('cancel-stage-plan'))
    stage_ref=dict(key=stage['key'],digest=stage['value']['digest'])
    stage_selector=selector(pendingStageContinue=stage_ref)
    paused=run_body(51,'paused',currentPlan=None,blockers=[blocker],stageContinueRequired=True,pauseReason='stage_cancelled',resumeTarget='researching')
    ui.append(dict(id='cancel-stage',row=stage,runAfter=row('research_run_controls',key,paused,'cancel-stage',3),
                   planAfter=plan_row('cancel-stage','cancelled',3,3),selectorAfter=row('research_lifecycle_state',key,stage_selector,'cancel-stage',3)))
    cont=decision('continue-stage',request('continue-stage',51,cancellation=stage_ref),paused,stage_selector)
    ui.append(dict(id='continue-stage',row=cont,
        runAfter=row('research_run_controls',key,run_body(52,'blocked',currentPlan=None,blockers=[blocker]),'continue-stage',3),
        selectorAfter=row('research_lifecycle_state',key,selector(),'continue-stage',3)))
    terminal_selector=dict(summary_selector,pendingStageContinue=stage_ref)
    cancel_before=run_body(60,'paused',currentPlan=None,blockers=[blocker],stageContinueRequired=True,pauseReason='stage_cancelled',resumeTarget='synthesizing')
    cancel=decision('cancel-run',request('cancel-run',60,reason='End automation; leave the ordinary child alone.'),cancel_before,terminal_selector)
    ui.append(dict(id='cancel-run',row=cancel,
        runAfter=row('research_run_controls',key,dict(cancel_before,runVersion=61,state='cancelled',parentRuntime=None,pauseReason=None,resumeTarget=None),'cancel-run',3),
        selectorAfter=row('research_lifecycle_state',key,terminal_selector,'cancel-run',3)))
    invalidation=dict(before=summaries[1]['selectorAfter'],after=row('research_lifecycle_state',key,dict(summary_selector,currentSummary=None),'report-invalidation',3),
                      retainedSummaries=[x['row'] for x in summaries],expected='clear-current-selection/retain-history')
    effects=[dict(id=k,expected=v) for k,v in [
        ('plan-completion','plan-grant-revoked/parent-planning/live-child-held/collection-active'),
        ('summary','derived-output/no-current-input-or-task-change/no-execution-approval'),
        ('stage-cancel','result-only-parent/child-report-held/collection-active/continue-required'),
        ('runtime-resume','stage-gate-preserved/no-grant-restore'),
        ('continue','gate-only-cleared/blocker-kept/no-task-reservation-reuse'),
        ('terminal','all-authority-revoked/waits-closed/collection-retired/live-child-held'),
        ('unknown-native','retain-original-owner/capacity-held/no-rollback-or-respawn'),
        ('unknown-store','unavailable/no-success-receipt/no-replacement-worker'),
        ('terminal-late-event','absorbing/no-authority-restore'),
        ('unsupported-trigger','deny-whole-transition/no-manual-completion-delay')]]
    rows=[initial]+[x['row'] for x in summaries]+[completion]+[x['row'] for x in ui]
    return dict(inputSha256=sha(raw),continuationSha256=sha(old),assumptions=[
        'separate decoded examples, not a store or legal event history',
        'current Main/parent authority, rate, runtime and source eligibility supplied, never authenticated here',
        'complete source/consumer/review/native memberships, original H/C/proofs and outcomes unqualified',
        'resource ledgers, actual effects, job/command/index/physical fit and nonblocking authority fences unqualified'],
        initial=initial,completions=completions,summaries=summaries,ui=ui,invalidation=invalidation,effects=effects,
        measurements=[dict(table=r['table'],key=r['key'],**measure(r)) for r in rows])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
