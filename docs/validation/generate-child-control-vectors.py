"""Independent synthetic control components, never complete states or native proofs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sources = json.loads((ROOT / 'child-source-vectors.json').read_text())
run = sources['run']
owner = sources['worker']
s = owner['source']
task = s['configuration']['task']
plan = s['plan']
commit = owner['receipt']['commit']


def canon(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(x):
    return hashlib.sha256(x.encode()).hexdigest()


def domain(name, x):
    return sha(name + '\n' + canon(x))


def seal(name, x, field):
    return {**x, field: domain(name, x)}


parent = {'schemaVersion': 1, 'kind': 'attached', **{k: run[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'bootstrapConfigurationDigest']},
          'rootAssignmentId': 'root-1', 'sidecarGeneration': 'c' * 64, 'launchGeneration': 'a' * 64}
parent_history = {'schemaVersion': 1, 'kind': 'parent', 'parent': parent}
child_history = {'schemaVersion': 1, 'kind': 'child', 'assignmentIdentity': s['assignmentIdentity'], 'attachmentId': s['attachmentId'],
                 'sidecarGeneration': 'c' * 64, 'launchGeneration': 'd' * 64}


def runtime(body):
    parent_kind = body['kind'] == 'parent'
    b = body['parent'] if parent_kind else body
    return {'kind': body['kind'], 'workspaceId': run['workspaceId'], 'runId': run['runId'],
            'sessionId': run['parentSessionId'] if parent_kind else s['assignmentIdentity']['childSessionId'],
            'sidecarGeneration': b['sidecarGeneration'], 'launchGeneration': b['launchGeneration'],
            'attachmentDigest': domain('orkworks.child-source-runtime-attachment.v1', body)}


parent_ref = runtime(parent_history)
child_ref = runtime(child_history)
observations = sorted([{'kind': 'root', 'path': owner['allocation']['allocatedWorktree']['canonicalRoot'],
                       'object': owner['allocation']['allocatedWorktree']['rootObject']},
                      {'kind': 'private-git', 'path': owner['allocation']['allocatedWorktree']['canonicalPrivateGitDirectory'],
                       'object': owner['allocation']['allocatedWorktree']['privateGitDirectoryObject']}], key=lambda x: x['kind'])
blocker = {'id': 'original-attempt', 'kind': 'launch-interrupted', 'task': task, 'detail': 'Synthetic original native disposition is unresolved.'}
run_control = {'schemaVersion': 1, 'profile': 'single-task-retained-data-execution-v1',
               **{k: run[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'runDefinitionDigest', 'bootstrapConfigurationDigest', 'repositoryBindingDigest', 'sourceWorktreeBindingDigest']},
               'runVersion': 8, 'state': 'blocked', 'currentInput': None, 'planReferences': [plan], 'currentPlan': plan, 'pendingPlan': None,
               'parentRuntime': parent_ref, 'blockers': [blocker], 'stageContinueRequired': True, 'pendingClarification': None, 'pauseReason': None, 'resumeTarget': None}
plan_control = {'schemaVersion': 1, 'plan': plan, 'planVersion': 3, 'status': 'active', 'latestApproval': s['approval'], 'task': task, 'replaces': None}
task_control = {'schemaVersion': 1, 'task': task, 'taskVersion': 5, 'status': 'launch_interrupted', 'reservation': s['reservation'], 'coordinationResult': None}
launch = {'schemaVersion': 1, 'task': task, 'reservation': s['reservation'],
          'admission': {'approval': s['approval'], 'parentRuntime': parent_ref, 'expectedTaskVersion': 1, 'assignmentTaskVersion': 2},
          'phase': 'interrupted', 'allocation': s['allocation'], 'attachmentId': None, 'spawnRuntime': None,
          'observedDirectories': observations, 'interruptedAt': 'attachment', 'reasonCode': 'disposition-unknown'}
session = {'schemaVersion': 1, 'task': task, 'reservation': s['reservation'], 'assignmentIdentity': s['assignmentIdentity'],
           'attachmentId': s['attachmentId'], 'sessionId': s['assignmentIdentity']['childSessionId'], 'producerRevision': '1', 'state': 'prepared', 'runtime': None}
rows = []


def add(name, table, key, body, **extra):
    value = {'schemaVersion': 1, 'body': copy.deepcopy(body), **copy.deepcopy(extra), 'lastCommit': copy.deepcopy(extra.get('receipt', {}).get('commit', commit))}
    row = {'table': table, 'key': copy.deepcopy(key), 'value': value}
    projection = copy.deepcopy(value)
    del projection['lastCommit']
    if 'receipt' in projection:
        del projection['receipt']['commit']
    row['rowDigest'] = domain('orkworks.hierarchy-storage-row.v4', {'table': table, 'key': key, 'body': projection})
    rows.append({'id': name, 'row': row})
    return row


for name in ['worker', 'reviewer']:
    o = sources[name]
    records = [{'kind': 'configuration', 'reference': o['source']['configuration']}]
    if name == 'reviewer':
        records += [{'kind': 'review-input', 'reference': o['source']['input']},
                    {'kind': 'result', 'reference': sources['snapshot']['target']},
                    {'kind': 'configuration', 'reference': s['configuration']},
                    {'kind': 'research-report', 'source': sources['snapshot']['evidence'][0]['source']}]
    # Prospective replacement input only: these independently typed components do
    # not rewrite the existing child source plan's original revision-one binding.
    records.append({'kind': 'prior-plan', 'plan': o['source']['plan']})
    records.sort(key=lambda x: (x['kind'].encode(), canon(x).encode()))
    inp = seal('orkworks.preparation-input.v1', {'schemaVersion': 1, 'workspaceId': run['workspaceId'], 'runId': run['runId'],
        'preparationRevision': 2, 'inputRevision': 2, 'goalDigest': run['goalDigest'], 'bootstrapConfigurationDigest': run['bootstrapConfigurationDigest'],
        'decision': {'schemaVersion': 1, 'inputRevision': 2, 'clarificationNeeded': False, 'researchNeeded': False,
                     'reasons': ['Synthetic direct-execution assessment only.'], 'unknowns': []}, 'records': records}, 'inputDigest')
    add(name + '-prepared-input', 'child_control_inputs', {k: inp[k] for k in ['workspaceId', 'runId', 'preparationRevision', 'inputRevision']}, inp, producer=parent_ref)
for name, body in [('parent', parent_history), ('child', child_history)]:
    ref = runtime(body)
    add(name + '-runtime', 'child_runtime_attachments', {k: v for k, v in ref.items() if k != 'attachmentDigest'}, body, digest=ref['attachmentDigest'])
for state in ['assessing', 'awaiting_execution_approval', 'executing', 'blocked', 'paused', 'awaiting_resume_approval', 'complete', 'cancelled']:
    b = {**run_control, 'state': state}
    if state == 'paused':
        b |= {'pauseReason': 'parent_ended', 'resumeTarget': 'executing'}
    add('run-' + state, 'child_run_controls', {k: run[k] for k in ['workspaceId', 'runId']}, b)
for status in ['proposed', 'active', 'paused', 'superseded', 'complete', 'cancelled']:
    add('plan-' + status, 'child_plan_controls', plan, {**plan_control, 'status': status})
for status in ['pending', 'launching', 'running', 'needs_parent_result', 'reported_complete', 'reported_blocked', 'reported_failed', 'launch_interrupted']:
    b = {**task_control, 'status': status}
    if status == 'pending':
        b |= {'taskVersion': 1, 'reservation': None}
    if status.startswith('reported_'):
        request = {'schemaVersion': 1, 'task': task, 'reservation': s['reservation'], 'childSessionId': s['assignmentIdentity']['childSessionId'],
                   'expectedTaskVersion': 4, 'outcome': {'reported_complete': 'completed', 'reported_blocked': 'blocked', 'reported_failed': 'failed'}[status], 'summary': 'Synthetic coordination outcome.'}
        request_digest = domain('orkworks.child-source-coordination-request.v1', request)
        b['coordinationResult'] = {'requestDigest': request_digest}
        receipt = {'schemaVersion': 1, 'task': task, 'reservation': s['reservation'], 'childSessionId': request['childSessionId'], 'acceptedTaskVersion': 5, 'commit': commit}
        value = {'schemaVersion': 1, 'request': request, 'requestDigest': request_digest, 'receipt': receipt, 'lastCommit': commit}
        projection = copy.deepcopy(value); del projection['lastCommit']; del projection['receipt']['commit']
        row = {'table': 'child_coordination_receipts', 'key': task, 'value': value}
        row['rowDigest'] = domain('orkworks.hierarchy-storage-row.v4', {'table': row['table'], 'key': task, 'body': projection})
        rows.append({'id': 'coordination-' + request['outcome'], 'row': row})
    add('task-' + status, 'child_task_controls', task, b)
for phase in ['reserved', 'allocating', 'allocated', 'prepared', 'spawn-pending', 'running', 'interrupted', 'ended', 'settled-no-child']:
    b = {**launch, 'phase': phase, 'interruptedAt': None, 'reasonCode': None}
    if phase in ['reserved', 'allocating']:
        b |= {'allocation': None, 'observedDirectories': []}
    if phase in ['prepared', 'spawn-pending', 'running', 'ended']:
        b['attachmentId'] = s['attachmentId']
    if phase in ['spawn-pending', 'running', 'ended']:
        b['spawnRuntime'] = {k: child_ref[k] for k in ['sidecarGeneration', 'launchGeneration']}
    if phase == 'interrupted':
        b = launch
    add('launch-' + phase, 'child_launch_controls', task, b)
for state in ['prepared', 'nonterminal', 'terminal']:
    b = {**session, 'state': state}
    if state != 'prepared':
        b |= {'producerRevision': '2', 'runtime': child_ref}
    add('session-' + state, 'child_session_states', task, b)
for kind in ['settle-no-child', 'confirm-original-allocation', 'confirm-original-attachment', 'observe-terminal']:
    attached = kind in ['confirm-original-attachment', 'observe-terminal']
    launch_body = {**launch}
    session_read = None
    if attached:
        launch_body['attachmentId'] = s['attachmentId']
        state = {**session}
        if kind == 'observe-terminal':
            state |= {'state': 'terminal', 'producerRevision': '3', 'runtime': child_ref}
            launch_body['spawnRuntime'] = {k: child_ref[k] for k in ['sidecarGeneration', 'launchGeneration']}
        session_read = {'table': 'child_session_states', 'key': task, 'value': {'schemaVersion': 1, 'body': state, 'lastCommit': commit}}
    read_set = {'expectedCommit': commit, 'session': session_read}
    for name, table, key, b in [('run', 'child_run_controls', {k: run[k] for k in ['workspaceId', 'runId']}, run_control),
                              ('task', 'child_task_controls', task, task_control), ('launch', 'child_launch_controls', task, launch_body)]:
        read_set[name] = {'table': table, 'key': key, 'value': {'schemaVersion': 1, 'body': b, 'lastCommit': commit}}
    body = {'schemaVersion': 1, 'recoveryId': 'recovery-' + kind, 'task': task, 'reservation': s['reservation'], 'expectedRunVersion': 8, 'expectedTaskVersion': 5,
            'reviewContextId': sha('Synthetic reviewed original attempt: ' + kind), 'sidecarGeneration': 'c' * 64, 'kind': kind,
            'allocation': s['allocation'], 'attachmentId': s['attachmentId'] if attached else None, 'readSet': read_set,
            'observedDirectories': observations, 'blockers': [blocker], 'reason': 'Synthetic captured decision basis; no actual proof or native observation.'}
    d = domain('orkworks.child-source-recovery-decision.v1', body)
    receipt = {'schemaVersion': 1, 'recoveryId': body['recoveryId'], 'decisionDigest': d, 'runVersion': 9, 'taskVersion': 6, 'commit': {**commit, 'commitSequence': '3', 'changeDigest': sha('Synthetic recovery successor; no manifest.')}}
    add('recovery-' + kind, 'child_recovery_decisions', {'task': task, 'recoveryId': body['recoveryId']}, body, digest=d, receipt=receipt)
for v in rows:
    wire = canon(v['row'])
    v |= {'canonical': wire, 'bytes': len(wire.encode()), 'sha256': sha(wire)}
packet = {'schemaVersion': 1, 'classification': 'synthetic-unverified-child-control-components-only', 'rows': rows}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT / 'child-control-vectors.json')
args = parser.parse_args()
args.output.write_bytes((json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode())
