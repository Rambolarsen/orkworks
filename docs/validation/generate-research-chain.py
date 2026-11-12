"""Independent synthetic research source chain; no actual approval or launch."""
import argparse
import base64
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(value):
    return hashlib.sha256(value.encode('utf-8')).hexdigest()


def domain(name, value):
    return sha(name + '\n' + canonical(value))


def seal(value, field, name):
    value[field] = domain(name, value)
    return value


def path(value):
    return dict(encoding='unix-bytes', base64=base64.b64encode(value.encode('utf-8')).decode('ascii'))


def packet():
    startup = json.loads((ROOT / 'research-startup-components.json').read_bytes())
    c, run, task = startup['configuration'], startup['context']['run'], startup['scope']['task']
    prepared = dict(preparationRevision=1, inputDigest=sha('Unqualified research prepared input; no codec or history supplied.'))
    commits = {name: dict(storageEpoch='f' * 64, commitSequence=str(i), changeDigest=sha('No commit proof: ' + name))
               for i, name in enumerate(['approval', 'reservation', 'allocation', 'attachment'], 2)}
    scope_ref = dict(task=task, scopeDigest=startup['scope']['scopeDigest'])
    configuration = {k: c[k] for k in ['configurationId', 'configurationDigest']}
    binding = {k: run[k] for k in ['runId', 'parentSessionId', 'runDefinitionDigest', 'bootstrapConfigurationDigest',
                                 'repositoryBindingDigest', 'sourceWorktreeBindingDigest']}
    binding.update(planId=c['planId'], planRevision=c['planRevision'], **prepared)
    group = dict(groupId=startup['context']['groupId'], branchName='fixture/research',
                 absolutePath=path('/fixture/research'), baseCommit=startup['context']['baseCommit'])
    plan = seal(dict(schemaVersion=1, kind='single-task-retained-file-research', parentPlanBinding=binding,
        task=dict(taskId=c['taskId'], configuration=configuration, scope=scope_ref, initialPrompt=c['assignment']['description'], group=group),
        batches=[[c['taskId']]], maxParallelChildren=1, scopeText='Synthetic retained-file research source chain only.'),
        'planDefinitionDigest', 'orkworks.research-source-plan.v1')
    plan_ref = {k: task[k] for k in ['workspaceId', 'runId', 'planId', 'planRevision']} | dict(planDefinitionDigest=plan['planDefinitionDigest'])
    request = dict(schemaVersion=1, action='orkworks.research-source-plan.approve.v1', plan=plan_ref,
                   approvalId='research-approval', reviewContextId=sha('Synthetic shown plan bytes: ' + canonical(plan)))
    request_digest = domain('orkworks.research-source-plan-request.v1', request)
    approval = dict(schemaVersion=1, plan=plan_ref, approvalId=request['approvalId'], reviewContextId=request['reviewContextId'],
                    sidecarGeneration='c' * 64, requestDigest=request_digest)
    approval_digest = domain('orkworks.research-source-plan-approval.v1', approval)
    approval_ref = dict(plan=plan_ref, approvalId=approval['approvalId'], approvalDigest=approval_digest)
    receipt = dict(schemaVersion=1, plan=plan_ref, approvalId=approval['approvalId'], approvalDigest=approval_digest,
                   requestDigest=request_digest, commit=commits['approval'])
    reservation = seal(dict(schemaVersion=1, task=task, scope=scope_ref, reservationId='research-reservation', childSessionId='research-session',
        **configuration, approval=approval_ref, expectedTaskVersion=1, assignmentTaskVersion=2),
        'reservationDigest', 'orkworks.research-source-reservation.v1')
    reservation_ref = dict(task=task, reservationId=reservation['reservationId'], reservationDigest=reservation['reservationDigest'])
    source = c['sourceWorktreeBinding']
    allocated = seal(dict(repositoryBinding=c['repositoryBinding'], canonicalRoot=group['absolutePath'],
        rootObject={**source['rootObject'], 'fileId': '901'}, canonicalPrivateGitDirectory=path('/fixture/main/.git/worktrees/research'),
        privateGitDirectoryObject={**source['privateGitDirectoryObject'], 'fileId': '902'}, kind='linked'),
        'worktreeBindingDigest', 'orkworks.worktree-binding.v1')
    allocation = seal(dict(schemaVersion=1, reservation=reservation_ref, allocationId='research-allocation', group=group, allocatedWorktree=allocated),
                      'allocationDigest', 'orkworks.research-source-allocation.v1')
    allocation_ref = dict(reservation=reservation_ref, allocationId=allocation['allocationId'], allocationDigest=allocation['allocationDigest'])
    identity = dict(kind='child', **task, taskVersion=reservation['assignmentTaskVersion'], reservationId=reservation['reservationId'],
                    parentSessionId=c['parentSessionId'], childSessionId=reservation['childSessionId'], **configuration)
    attachment = seal(dict(schemaVersion=1, attachmentId='research-attachment', reservation=reservation_ref,
                           allocation=allocation_ref, assignmentIdentity=identity),
                      'attachmentDigest', 'orkworks.research-source-attachment.v1')
    assignment = dict(schemaVersion=1, assignmentIdentity=identity, plan=plan_ref, configuration=dict(task=task, **configuration),
        scope=scope_ref, approval=approval_ref, reservation=reservation_ref, allocation=allocation_ref, attachmentId=attachment['attachmentId'],
        criteria=sorted(copy.deepcopy(c['assignment']['criteria']), key=lambda x: x['id']), rubric=c['rubric'])
    fingerprints = dict(requestDigest=request_digest, approvalDigest=approval_digest,
        assignmentSourceDigest=domain('orkworks.research-assignment-source.v1', assignment),
        evidenceSourceDigests=[dict(referenceId=e['referenceId'], sourceDigest=domain('orkworks.research-source-evidence.v1', e)) for e in startup['evidenceObjects']])
    metadata = [dict(table='research_plan_approvals', key=dict(plan=plan_ref, approvalId=approval['approvalId']),
                     value=dict(schemaVersion=1, body=approval, digest=approval_digest, receipt=receipt, lastCommit=commits['approval'])),
        dict(table='research_reservations', key=task, value=dict(schemaVersion=1, body=reservation, lastCommit=commits['reservation'])),
        dict(table='research_allocations', key=reservation_ref, value=dict(schemaVersion=1, body=allocation, lastCommit=commits['allocation'])),
        dict(table='research_attachments', key=reservation['childSessionId'], value=dict(schemaVersion=1, body=attachment, lastCommit=commits['attachment']))]
    return dict(startup=startup, unqualified=dict(preparedInputBinding=prepared, metadataCommits=commits), plan=plan, request=request,
                approval=approval, receipt=receipt, reservation=reservation, allocation=allocation, attachment=attachment,
                assignment=assignment, fingerprints=fingerprints, metadata=metadata)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
