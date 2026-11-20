"""Offline research clarification components: accepted #1911 shapes only; supplied facts never executed."""
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
    return dict(storageEpoch='f'*64, commitSequence=str(n), changeDigest=sha('UNQUALIFIED clarification label: '+label))
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
    return dict(kind='parent',**pick(prefix['runtimeHistory'][0]['key'],'workspaceId runId sessionId sidecarGeneration launchGeneration'))

def run_body(prefix,version,state,**changes):
    inp=pick(prefix['inputs'][1]['value']['body'],'workspaceId runId preparationRevision inputRevision inputDigest')
    chain=prefix['chain']
    b=dict(schemaVersion=1,profile='single-task-retained-file-research-clarification-v1',
        **pick(chain['startup']['context']['run'],'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
        runVersion=version,state=state,currentInput=inp,planReferences=[],currentPlan=None,pendingPlan=None,
        parentRuntime=parent_runtime(prefix),blockers=[],stageContinueRequired=False,pendingClarification=None,
        pauseReason=None,resumeTarget=None)
    b.update(changes);return b

def packet():
    prefix_raw=(ROOT/'research-prefix-components.json').read_bytes()
    prefix=json.loads(prefix_raw)
    chain=prefix['chain']
    run=pick(chain['startup']['context']['run'],'workspaceId runId')
    researcher_task=clone(chain['startup']['scope']['task'])
    config_ref=pick(chain['plan']['task']['configuration'],'configurationId configurationDigest')
    parent=parent_runtime(prefix)
    user_origin=dict(policy='research-clarification-answer-v1',
        reviewContextId=sha('Unqualified shown answer context.'),sidecarGeneration=parent['sidecarGeneration'])
    question=dict(questionId='goal-scope',questionText='May existing records be replaced during the import?',
        whyItMatters='Determines whether the proposed approach preserves or replaces rows.',
        suggestions=['Replace only duplicated rows.','Ask before each replacement.'])
    # assessment manifest: fresh clarification decision at revision 2/2
    decision=dict(schemaVersion=1,inputRevision=2,clarificationNeeded=True,researchNeeded=False,
        reasons=['The goal does not state the replacement policy.'],question=clone(question))
    manifest=dict(schemaVersion=1,**pick(run,'workspaceId runId'),preparationRevision=2,inputRevision=2,
        goalDigest=prefix['inputs'][1]['value']['body']['goalDigest'],
        bootstrapConfigurationDigest=prefix['inputs'][1]['value']['body']['bootstrapConfigurationDigest'],
        decision=decision,
        records=[dict(kind='configuration',reference=clone(config_ref))])
    manifest['inputDigest']=digest('orkworks.preparation-input.v1',manifest)
    manifest_row=row('objects',dict(kind='research-control-input',workspaceId=run['workspaceId'],runId=run['runId'],
        preparationRevision=2,inputRevision=2),manifest,'clarification-assessment',2)
    manifest_row['value']['producer']=clone(parent)
    # request row, pending binding, awaiting_clarification state
    request_body=dict(schemaVersion=1,run=run,inputRevision=2,question=clone(question),producer=clone(parent))
    request_body['requestDigest']=digest('orkworks.research-clarification-request.v1',request_body)
    request_row=dict(table='research_clarification_requests',key=dict(run=clone(run),questionId='goal-scope',questionVersion=1),
        value=dict(schemaVersion=1,body=request_body,digest=request_body['requestDigest'],lastCommit=origin('clarification-assessment',3)))
    pending=dict(questionId='goal-scope',questionVersion=1,requestDigest=request_body['requestDigest'])
    run_awaiting=run_body(prefix,3,'awaiting_clarification',currentInput=dict(workspaceId=run['workspaceId'],runId=run['runId'],
        preparationRevision=2,inputRevision=2,inputDigest=manifest['inputDigest']),pendingClarification=clone(pending))
    run_row=row('research_run_controls',run,run_awaiting,'clarification-assessment',2)
    # answer admission: request, body, receipt, family row
    answer_request=dict(schemaVersion=1,action='orkworks.research-clarification.answer.v1',run=run,
        questionId='goal-scope',questionVersion=1,answerId='answer-1',
        answerText='Replace only duplicated rows.',reviewContextId=user_origin['reviewContextId'],
        sidecarGeneration=user_origin['sidecarGeneration'],expectedInputRevision=2)
    answer=dict(schemaVersion=1,run=run,questionId='goal-scope',questionVersion=1,answerId='answer-1',
        answerText='Replace only duplicated rows.',requestDigest=request_body['requestDigest'],origin=user_origin)
    answer['answerDigest']=digest('orkworks.research-clarification-answer.v1',answer)
    receipt=dict(schemaVersion=1,run=run,questionId='goal-scope',questionVersion=1,answerId='answer-1',
        answerDigest=answer['answerDigest'],runVersion=4,inputRevision=3,commit=origin('answer-admission',2))
    answer_ref=dict(run=run,questionId='goal-scope',questionVersion=1,answerId='answer-1',answerDigest=answer['answerDigest'])
    answer_row=dict(table='research_clarification_answers',key=dict(run=clone(run),questionId='goal-scope',questionVersion=1,answerId='answer-1'),
        value=dict(schemaVersion=1,request=dict(run=run,questionId='goal-scope',questionVersion=1,requestDigest=request_body['requestDigest']),
            answer=answer,receipt=receipt,lastCommit=origin('answer-admission',3)))
    # answer outcome: copied decision, clarification-answer record, user-origin producer, pending cleared
    answer_manifest=dict(manifest,preparationRevision=3,inputRevision=3,
        records=sorted(manifest['records']+[dict(kind='clarification-answer',reference=clone(answer_ref))],
            key=lambda r:(r['kind'],canonical(r))))
    answer_manifest.pop('inputDigest',None)
    answer_manifest['inputDigest']=digest('orkworks.preparation-input.v1',answer_manifest)
    answer_manifest_row=row('objects',dict(kind='research-control-input',workspaceId=run['workspaceId'],runId=run['runId'],
        preparationRevision=3,inputRevision=3),answer_manifest,'answer-outcome',2)
    answer_manifest_row['value']['producer']=clone(user_origin)
    run_after=row('research_run_controls',run,run_body(prefix,4,'assessing',currentInput=dict(workspaceId=run['workspaceId'],
        runId=run['runId'],preparationRevision=3,inputRevision=3,inputDigest=answer_manifest['inputDigest']),
        pendingClarification=None),'answer-outcome',2)
    # retained-clarification-answer entries rule: fresh assessment at revision 4
    fresh=dict(answer_manifest,preparationRevision=4,inputRevision=4,
        decision=dict(schemaVersion=1,inputRevision=4,clarificationNeeded=False,researchNeeded=True,
            reasons=['The replacement policy is now explicit.'],
            scope=prefix['inputs'][1]['value']['body']['decision']['scope']),
        records=sorted([dict(kind='configuration',reference=clone(config_ref)),
                 dict(kind='clarification-answer',reference=clone(answer_ref))] +
                [clone(r) for r in prefix['inputs'][1]['value']['body']['records'] if r['kind']=='research-capture'] +
                [dict(kind='research-scope',reference=dict(task=researcher_task,scopeDigest=prefix['inputs'][1]['value']['body']['decision']['scope']['scopeDigest']))],
                key=lambda r:(r['kind'],canonical(r))))

    fresh.pop('inputDigest',None)
    fresh['inputDigest']=digest('orkworks.preparation-input.v1',fresh)
    fresh_row=row('objects',dict(kind='research-control-input',workspaceId=run['workspaceId'],runId=run['runId'],
        preparationRevision=4,inputRevision=4),fresh,'fresh-assessment',2)
    fresh_row['value']['producer']=clone(parent)
    # gate-respecting state alternatives after the answer
    blocker=dict(id='retained-attempt',kind='launch-interrupted',task=researcher_task,
        detail='Supplied unknown original native disposition; no capacity release.')
    states=[
        dict(id='assessing-when-eligible',state='assessing',expected='runtime-active/no-higher-gate'),
        dict(id='blocked-retained',state='blocked',blockers=[blocker],expected='retained-blocker-keeps-run-blocked'),
        dict(id='paused-retained',state='paused',pauseReason='parent_ended',resumeTarget='assessing',
            expected='paused-retained/resumeTarget-assessing/exact-identity-resume-first'),
        dict(id='awaiting-resume-retained',state='awaiting_resume_approval',expected='resume-approval-gate-retained')]
    # changed-answer re-ask at questionVersion+1
    reask_request=dict(schemaVersion=1,run=run,inputRevision=4,question=clone(question),
        producer=clone(parent))
    reask_request['requestDigest']=digest('orkworks.research-clarification-request.v1',reask_request)
    reask_row=dict(table='research_clarification_requests',key=dict(run=clone(run),questionId='goal-scope',questionVersion=2),
        value=dict(schemaVersion=1,body=reask_request,digest=reask_request['requestDigest'],lastCommit=origin('re-ask',3)))
    reask=dict(request=reask_row,previous=dict(questionId='goal-scope',questionVersion=1,requestDigest=request_body['requestDigest']),
        expected='changed-answer-re-ask-at-questionVersion-plus-one/original-retained')
    rows=[manifest_row,request_row,run_row,answer_manifest_row,run_after,fresh_row,reask_row]
    return dict(prefixSha256=sha(prefix_raw),assumptions=[
        'separate decoded examples, not a store or legal event history and independent of the prefix packet revision numbering',
        'current Main authority, epoch, owner, lease, health, rate and shown context supplied, never authenticated here',
        'sidecar-derived user-origin provenance, runtime eligibility and complete publication facts supplied and unqualified',
        'complete source/consumer/index memberships, original H/C/proofs, ledgers and actual effects unqualified'],
        assessment=dict(manifest=manifest_row,decision=decision,request=request_row,
            runAfter=run_row,pending=pending,
            gated=dict(blocked=run_body(prefix,3,'blocked',currentInput=dict(workspaceId=run['workspaceId'],
                runId=run['runId'],preparationRevision=2,inputRevision=2,inputDigest=manifest['inputDigest']),
                blockers=[blocker],pendingClarification=clone(pending)),
                expected='question-asked/higher-gate-governs',
                stageFence=dict(stageContinueRequired=True,expected='clarification-mutations-rejected-under-stage-fence'))),
        answer=dict(request=answer_request,body=answer,row=answer_row,receipt=receipt,
            manifest=answer_manifest_row,runAfter=run_after,
            admission=dict(exactRetry=dict(expected='original-receipt-before-CAS/even-after-terminal'),
                conflict=dict(sameAnswerIdDifferentBytes=sha('conflicting answer bytes'),expected='conflict-no-mutation'),
                staleRequest=dict(questionVersion=2,requestDigest=sha('superseded request'),expected='reject-with-current-request/no-version'),
                staleInput=dict(expectedInputRevision=2,currentInputRevision=3,expected='reject-stale-input'),
                rendererProvenance=dict(suppliedField=True,expected='never-trusted/derive-from-authorized-operation'),
                bearerAsUser=dict(parentRuntime=parent,expected='never-user-provenance'),
                runtimeCast=dict(kind='child',expected='never-cast-into-user-origin'),
                duplicateQuestionId=dict(reuse=True,expected='questionId-never-recycled')),
            stateAlternatives=states),
        freshAssessment=dict(manifest=fresh_row,retainedAnswers=[dict(kind='clarification-answer',reference=clone(answer_ref))],
            expected='retained-applicable-clarification-answer-entries/fresh-decision-at-own-revision/parent-producer'),
        changedAnswer=reask,
        exhaustion=dict(pairs=32,revisions=64,expected='atomic-denial/no-wrap-reset-pruning'),
        resume=dict(restoredBinding=clone(pending),expected='same-request-identity-version/no-duplicate-no-replacement'),
        effects=[dict(id=k,expected=v) for k,v in [
            ('assessment','fresh-parent-decision/request-row/pending-binding/awaiting-clarification-gated'),
            ('answer-admission','main-authority-shown-context-before-lookup/identical-retry-before-CAS'),
            ('answer-outcome','answer-row/next-input-revision-copied-decision-user-origin/pending-cleared/runVersion-once'),
            ('consumer-invalidation','recognized-pending-proposals-invalidated/history-retained'),
            ('fresh-assessment','separate-outcome/retained-answer-entries/per-decision-records'),
            ('changed-answer','new-request-version/no-overwrite'),
            ('exhaustion','atomic-denial'),
            ('resume','exact-restoration')]],
        measurements=[dict(table=r['table'],key=r['key'],**measure(r)) for r in rows])

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:args.output.write_bytes(wire)
    else:sys.stdout.buffer.write(wire)
