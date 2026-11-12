"""Independent finite prefix components, not a store, event history or reducer."""
import argparse
import base64
import copy
import json
import runpy
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
# Reuse unchanged Python byte/framing helpers; never invoke JavaScript validators.
startup_helpers = runpy.run_path(str(ROOT / 'generate-research-startup.py'))
chain_helpers = runpy.run_path(str(ROOT / 'generate-research-chain.py'))
canonical, sha, seal = (startup_helpers[k] for k in ['canonical', 'sha', 'seal'])
domain = chain_helpers['domain']


def native(value):
    return dict(encoding='unix-bytes', base64=base64.b64encode(value.encode()).decode())


def fresh_startup(old):
    s = copy.deepcopy(old)
    c, run = s['configuration'], s['context']['run']
    c.update(configurationId='prefix-configuration', planId='prefix-plan', taskId='prefix-task')
    c['assignment']['worktreeGroupId'] = s['context']['groupId'] = 'prefix-group'
    task = dict(workspaceId=c['workspaceId'], runId=run['runId'], planId=c['planId'], planRevision=1, taskId=c['taskId'])
    for source in [s['scope'], *s['captures']]:
        source['task'] = copy.deepcopy(task)
    for capture in s['captures']:
        seal(capture, 'captureDigest', 'orkworks.research-source-capture.v1')
    s['scope']['captures'] = [dict(task=task, sourceId=x['sourceId'], captureDigest=x['captureDigest']) for x in s['captures']]
    seal(s['scope'], 'scopeDigest', 'orkworks.research-task-scope.v1')
    for manifest in c['requirementManifests']:
        manifest['taskId'] = c['taskId']
    settings = json.loads(base64.b64decode(s['evidenceObjects'][0]['contentBase64']))
    settings['deliveryOrder'][-1]['scopeDigest'] = s['scope']['scopeDigest']
    wire = canonical(settings).encode()
    e = s['evidenceObjects'][0]
    e.update(contentBase64=base64.b64encode(wire).decode(), sizeBytes=len(wire), contentDigest=sha(wire))
    for evidence in s['evidenceObjects']:
        evidence['task'] = copy.deepcopy(task)
    capability = c['capabilityEvidence']
    capability['references'][0]['contentDigest'] = capability['effectiveSettingsDigest'] = e['contentDigest']
    capability['evidenceId'] = 'prefix-evidence'
    seal(capability, 'evidenceDigest', 'orkworks.capability-evidence.v1')
    c['harness'].update(evidenceId=capability['evidenceId'], evidenceDigest=capability['evidenceDigest'], effectiveSettingsDigest=e['contentDigest'])
    c['renderedInstructions'] = startup_helpers['render'](c)
    c['renderedInstructionsDigest'] = sha(c['renderedInstructions'])
    seal(c, 'configurationDigest', 'orkworks.assignment-configuration.v3')
    payload = canonical(dict(scope=s['scope'], captures=s['captures']))
    s['declaredStartup'] = ''.join(x['content'] for x in c['rules']) + c['renderedInstructions'] + '\nORKWORKS RETAINED RESEARCH INPUT v1\n' + str(len(payload.encode())) + '\n' + payload + '\n'
    return s


def rebind_chain(p, s, input2):
    # Rebuild a separate synthetic source chain, never rewrite/import old bytes.
    p['startup'] = s
    c, t = s['configuration'], s['scope']['task']
    config = {k: c[k] for k in ['configurationId', 'configurationDigest']}
    scope = dict(task=t, scopeDigest=s['scope']['scopeDigest'])
    prepared = dict(preparationRevision=2, inputDigest=input2['inputDigest'])
    p['unqualified']['preparedInputBinding'] = prepared
    plan = p['plan']
    plan['parentPlanBinding'].update(planId=c['planId'], **prepared)
    plan['task'].update(taskId=c['taskId'], configuration=config, scope=scope,
                        group=dict(groupId='prefix-group', branchName='fixture/prefix-research', absolutePath=native('/fixture/prefix-research'), baseCommit=c['assignment']['baseCommit']))
    plan['batches'] = [[c['taskId']]]
    seal(plan, 'planDefinitionDigest', 'orkworks.research-source-plan.v1')
    plan_ref = {k: t[k] for k in ['workspaceId', 'runId', 'planId', 'planRevision']} | dict(planDefinitionDigest=plan['planDefinitionDigest'])
    request, approval = p['request'], p['approval']
    request.update(plan=plan_ref, approvalId='prefix-approval', reviewContextId=sha('Unqualified prefix shown bytes: ' + canonical(plan)))
    approval.update(plan=plan_ref, approvalId=request['approvalId'], reviewContextId=request['reviewContextId'], requestDigest=domain('orkworks.research-source-plan-request.v1', request))
    approval_digest = domain('orkworks.research-source-plan-approval.v1', approval)
    approval_ref = dict(plan=plan_ref, approvalId=approval['approvalId'], approvalDigest=approval_digest)
    p['receipt'].update(plan=plan_ref, approvalId=approval['approvalId'], approvalDigest=approval_digest, requestDigest=approval['requestDigest'])
    r = p['reservation']
    r.update(task=t, scope=scope, reservationId='prefix-reservation', childSessionId='prefix-child', **config, approval=approval_ref)
    seal(r, 'reservationDigest', 'orkworks.research-source-reservation.v1')
    reservation_ref = dict(task=t, reservationId=r['reservationId'], reservationDigest=r['reservationDigest'])
    al = p['allocation']
    al.update(reservation=reservation_ref, allocationId='prefix-allocation', group=plan['task']['group'])
    al['allocatedWorktree']['canonicalRoot'] = plan['task']['group']['absolutePath']
    al['allocatedWorktree']['canonicalPrivateGitDirectory'] = native('/fixture/main/.git/worktrees/prefix-research')
    al['allocatedWorktree']['rootObject']['fileId'] = '1901'
    al['allocatedWorktree']['privateGitDirectoryObject']['fileId'] = '1902'
    seal(al['allocatedWorktree'], 'worktreeBindingDigest', 'orkworks.worktree-binding.v1')
    seal(al, 'allocationDigest', 'orkworks.research-source-allocation.v1')
    allocation_ref = dict(reservation=reservation_ref, allocationId=al['allocationId'], allocationDigest=al['allocationDigest'])
    identity = dict(kind='child', **t, taskVersion=2, reservationId=r['reservationId'], parentSessionId=c['parentSessionId'], childSessionId=r['childSessionId'], **config)
    at = p['attachment']
    at.update(attachmentId='prefix-attachment', reservation=reservation_ref, allocation=allocation_ref, assignmentIdentity=identity)
    seal(at, 'attachmentDigest', 'orkworks.research-source-attachment.v1')
    p['assignment'].update(assignmentIdentity=identity, plan=plan_ref, configuration=dict(task=t, **config), scope=scope,
                           approval=approval_ref, reservation=reservation_ref, allocation=allocation_ref, attachmentId=at['attachmentId'])
    p['fingerprints'] = dict(requestDigest=approval['requestDigest'], approvalDigest=approval_digest,
        assignmentSourceDigest=domain('orkworks.research-assignment-source.v1', p['assignment']),
        evidenceSourceDigests=[dict(referenceId=e['referenceId'], sourceDigest=domain('orkworks.research-source-evidence.v1', e)) for e in s['evidenceObjects']])
    commits = p['unqualified']['metadataCommits']
    p['metadata'] = [dict(table='research_plan_approvals', key=dict(plan=plan_ref, approvalId=approval['approvalId']), value=dict(schemaVersion=1, body=approval, digest=approval_digest, receipt=p['receipt'], lastCommit=commits['approval'])),
        dict(table='research_reservations', key=t, value=dict(schemaVersion=1, body=r, lastCommit=commits['reservation'])),
        dict(table='research_allocations', key=reservation_ref, value=dict(schemaVersion=1, body=al, lastCommit=commits['allocation'])),
        dict(table='research_attachments', key=r['childSessionId'], value=dict(schemaVersion=1, body=at, lastCommit=commits['attachment']))]
    return p


def packet():
    chain = chain_helpers['packet']()
    s = fresh_startup(chain['startup'])
    c, run, t = s['configuration'], s['context']['run'], s['scope']['task']
    scope = dict(task=t, scopeDigest=s['scope']['scopeDigest'])
    configuration = dict(task=t, configurationId=c['configurationId'], configurationDigest=c['configurationDigest'])
    records = [dict(kind='configuration', reference=configuration), dict(kind='research-scope', reference=scope)]
    records += [dict(kind='research-capture', reference=x) for x in s['scope']['captures']]
    records.sort(key=lambda x: (x['kind'].encode(), canonical(x).encode()))
    decision = dict(schemaVersion=1, inputRevision=2, clarificationNeeded=False, researchNeeded=True,
                    reasons=['Synthetic retained-file research assessment only.'], scope=scope)
    input2 = seal(dict(schemaVersion=1, **{k: run[k] for k in ['workspaceId', 'runId', 'goalDigest', 'bootstrapConfigurationDigest']},
                       preparationRevision=2, inputRevision=2, decision=decision, records=records), 'inputDigest', 'orkworks.preparation-input.v1')
    chain = rebind_chain(chain, s, input2)
    original = chain['assignment']
    initial = dict(schemaVersion=1, **{k: run[k] for k in ['workspaceId', 'runId', 'goalDigest', 'bootstrapConfigurationDigest']},
                   preparationRevision=1, inputRevision=1, decision=None, records=[])
    context = dict(rootAssignmentId='root-1', initialInput=initial,
                   unqualifiedCommit=dict(storageEpoch='f' * 64, commitSequence='1', changeDigest=sha('No prefix control proof or transaction.')))

    def row(table, key, body, **extra):
        return dict(table=table, key=copy.deepcopy(key), value=dict(schemaVersion=1, body=copy.deepcopy(body), **copy.deepcopy(extra), lastCommit=context['unqualifiedCommit']))

    parent = dict(schemaVersion=1, kind='parent', parent=dict(schemaVersion=1, kind='attached', **{k: run[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'bootstrapConfigurationDigest']},
                  rootAssignmentId=context['rootAssignmentId'], sidecarGeneration='c' * 64, launchGeneration='a' * 64))
    child = dict(schemaVersion=1, kind='child', assignmentIdentity=original['assignmentIdentity'], attachmentId=original['attachmentId'], sidecarGeneration='c' * 64, launchGeneration='d' * 64)
    runtime_rows, runtime_refs = [], []
    for body in [parent, child]:
        fields = body['parent'] if body['kind'] == 'parent' else body
        key = dict(kind=body['kind'], workspaceId=run['workspaceId'], runId=run['runId'], sessionId=run['parentSessionId'] if body['kind'] == 'parent' else chain['reservation']['childSessionId'],
                   sidecarGeneration=fields['sidecarGeneration'], launchGeneration=fields['launchGeneration'])
        d = domain('orkworks.research-source-runtime-attachment.v1', body)
        runtime_rows.append(row('research_runtime_attachments', key, body, digest=d))
        runtime_refs.append(dict(**key, attachmentDigest=d))
    parent_ref, child_ref = runtime_refs
    report = seal(dict(schemaVersion=1, reportId='prefix-report', reportVersion=1,
        **{k: original['assignmentIdentity'][k] for k in ['runId', 'planId', 'planRevision', 'taskId', 'reservationId', 'childSessionId', 'configurationDigest']},
        sidecarGeneration=child_ref['sidecarGeneration'], launchGeneration=child_ref['launchGeneration'], questionIds=[q['questionId'] for q in s['scope']['questions']],
        findings=[], uncertainties=['Synthetic selection only; no report acceptance or truth proved.'], blockers=[], evidenceReferences=[], recommendations=[], limitations=['No native source or publication proof.'], author=dict(kind='child')),
        'contentDigest', 'orkworks.research-report.v1')
    identity = dict(workspaceId=run['workspaceId'], **{k: report[k] for k in ['runId', 'planId', 'planRevision', 'taskId', 'reservationId', 'childSessionId', 'configurationDigest', 'sidecarGeneration', 'launchGeneration', 'reportId', 'reportVersion']})
    source = dict(schemaVersion=1, sourceKind='research-report', reportIdentity=identity, sizeBytes=len(canonical(report).encode()), contentDigest=report['contentDigest'])
    assumption = dict(report=report, source=source, sourceKey=domain('orkworks.assignment-evidence-source.v1', source),
                      head={k: report[k] for k in ['reportId', 'reportVersion', 'contentDigest']})
    input3 = copy.deepcopy(input2)
    input3.update(preparationRevision=3, inputRevision=3)
    input3['records'].append(dict(kind='research-report', source=source))
    input3['records'].sort(key=lambda x: (x['kind'].encode(), canonical(x).encode()))
    seal(input3, 'inputDigest', 'orkworks.preparation-input.v1')
    inputs = [row('research_control_inputs', {k: b[k] for k in ['workspaceId', 'runId', 'preparationRevision', 'inputRevision']}, b, producer=producer) for b, producer in [(input2, parent_ref), (input3, child_ref)]]
    run_key = {k: run[k] for k in ['workspaceId', 'runId']}
    ref = lambda b: {k: b[k] for k in ['workspaceId', 'runId', 'preparationRevision', 'inputRevision']} | dict(inputDigest=b.get('inputDigest', domain('orkworks.preparation-input.v1', b)))
    views = []
    for name, stage in [('initial-control', 0), ('proposed', 1), ('before-selection', 2), ('selected-source', 3)]:
        run_body = dict(schemaVersion=1, profile='single-task-retained-file-research-prefix-v1',
            **{k: run[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'runDefinitionDigest', 'bootstrapConfigurationDigest', 'repositoryBindingDigest', 'sourceWorktreeBindingDigest']},
            runVersion=[1, 3, 10, 11][stage], state=['assessing', 'awaiting_research_approval', 'researching', 'researching'][stage],
            currentInput=ref(initial if stage == 0 else input3 if stage == 3 else input2), planReferences=[] if stage == 0 else [original['plan']],
            currentPlan=original['plan'] if stage >= 2 else None, pendingPlan=original['plan'] if stage == 1 else None,
            parentRuntime=None if stage == 0 else parent_ref, blockers=[], stageContinueRequired=False, pendingClarification=None, pauseReason=None, resumeTarget=None)
        rows = [row('research_run_controls', run_key, run_body)]
        if stage:
            rows += [row('research_plan_controls', original['plan'], dict(schemaVersion=1, plan=original['plan'], planVersion=1 if stage == 1 else 2, status='proposed' if stage == 1 else 'active', latestApproval=None if stage == 1 else original['approval'], task=t, replaces=None)),
                row('research_task_controls', t, dict(schemaVersion=1, task=t, taskVersion=1 if stage == 1 else 7, status='pending' if stage == 1 else 'running', reservation=None if stage == 1 else original['reservation'], coordinationResult=None))]
            if stage >= 2:
                w = chain['allocation']['allocatedWorktree']
                observed = sorted([dict(kind='root', path=w['canonicalRoot'], object=w['rootObject']), dict(kind='private-git', path=w['canonicalPrivateGitDirectory'], object=w['privateGitDirectoryObject'])], key=lambda x: x['kind'])
                rows += [row('research_launch_controls', t, dict(schemaVersion=1, task=t, reservation=original['reservation'], admission=dict(approval=original['approval'], parentRuntime=parent_ref, expectedTaskVersion=1, assignmentTaskVersion=2),
                    phase='running', allocation=original['allocation'], attachmentId=original['attachmentId'], spawnRuntime={k: child_ref[k] for k in ['sidecarGeneration', 'launchGeneration']}, observedDirectories=observed, interruptedAt=None, reasonCode=None)),
                    row('research_session_states', t, dict(schemaVersion=1, task=t, reservation=original['reservation'], assignmentIdentity=original['assignmentIdentity'], attachmentId=original['attachmentId'], sessionId=chain['reservation']['childSessionId'], producerRevision='2', state='nonterminal', runtime=child_ref))]
            version = stage
            rows += [row('research_collections', run_key, dict(schemaVersion=1, **run_key, collectionVersion=version, active=True, task=t)),
                row('research_expectations', t, dict(schemaVersion=1, task=t, collection=run_key, collecting=True, configuration=configuration, scope=scope, reservation=None if stage == 1 else original['reservation'], selectedReport=source if stage == 3 else None, expectationVersion=version))]
        views.append(dict(id=name, rows=rows))
    return dict(context=context, chain=chain, reportAssumption=assumption, inputs=inputs, runtimeHistory=runtime_rows, views=views)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
