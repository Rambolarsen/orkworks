"""Independent synthetic membership components, never a complete retained store."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
S = json.loads((ROOT / 'child-source-vectors.json').read_text())
C = json.loads((ROOT / 'child-control-vectors.json').read_text())
R = S['run']


def canon(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(x):
    return hashlib.sha256(x.encode()).hexdigest()


def dom(d, x):
    return sha(d + '\n' + canon(x))


def pick(x, fields):
    return {k: x[k] for k in fields.split()}


def meta(table, key):
    return {'kind': 'metadata', 'table': table, 'key': copy.deepcopy(key)}


def obj(kind, identity, digest):
    return {'kind': 'object', 'reference': {'key': {'kind': kind, 'identity': copy.deepcopy(identity)}, 'digest': digest}}


def proof(commit):
    return {'kind': 'commit-proof', 'commit': copy.deepcopy(commit)}


def root():
    return meta('root_preparations', pick(R, 'workspaceId runId'))


def owner(task):
    return next(S[n] for n in ['worker', 'reviewer'] if S[n]['source']['configuration']['task'] == task)


def plan_ref(p):
    return obj('child-source-plan', p, p['planDefinitionDigest'])


def cfg_ref(c):
    return obj('child-source-configuration', c, c['configurationDigest'])


def input_ref(i):
    return obj('child-review-input', i, i['inputDigest'])


def assignment(a):
    o = owner(pick(a, 'workspaceId runId planId planRevision taskId'))
    assert a == o['source']['assignmentIdentity']
    return obj('child-assignment-source', a, dom('orkworks.child-assignment-source.v1', o['source']))


def result_ref(x):
    return obj('assignment-result', x, x['resultDigest'])


def report_refs(source):
    return [obj('assignment-evidence-source', source, dom('orkworks.assignment-evidence-source.v1', source)),
            obj('research-report', source['reportIdentity'], source['contentDigest'])]


def approval(a):
    return meta('child_plan_approvals', {'plan': a['plan'], 'approvalId': a['approvalId']})


def reservation(r):
    return meta('child_reservations', r['task'])


def allocation(a):
    return meta('child_allocations', a['reservation'])


def attachment(o):
    return meta('child_attachments', o['source']['assignmentIdentity']['childSessionId'])


def runtime(x):
    return meta('child_runtime_attachments', pick(x, 'kind workspaceId runId sessionId sidecarGeneration launchGeneration'))


def task_refs(t):
    o = owner(t)
    return [plan_ref(o['source']['plan']), cfg_ref(o['source']['configuration']), meta('child_task_controls', t)]


def row_digest(row):
    v = copy.deepcopy(row['value']); v.pop('lastCommit', None)
    if row['table'] in ['child_plan_approvals', 'child_coordination_receipts', 'child_recovery_decisions']:
        v['receipt'].pop('commit', None)
    return dom('orkworks.hierarchy-storage-row.v4', {'table': row['table'], 'key': row['key'], 'body': v})


# These two artifact versions and one debit are typed synthetic membership inputs,
# not accepted publication/read commands or complete before/after states.
a = S['worker']['source']['assignmentIdentity']
b = S['reviewer']['source']['assignmentIdentity']
target = S['snapshot']['target']
evaluations = []
artifacts = []
for version in [1, 2]:
    e = {'schemaVersion': 1, 'assignmentIdentity': a, 'resultRevision': target['resultRevision'], 'resultDigest': target['resultDigest'],
         'reviewerIdentity': {'kind': 'child', 'assignmentIdentity': b}, 'evaluationRevision': version,
         'predecessor': {'kind': 'no-head'} if version == 1 else {'kind': 'revision', 'evaluationRevision': 1, 'evaluationDigest': dom('orkworks.assignment-evaluation.v1', evaluations[0])},
         'reporterSource': {'kind': 'child', 'authorityGeneration': {'kind': 'child-launch', 'sidecarGeneration': 'c' * 64, 'generation': 'd' * 64}},
         'criteriaSnapshotDigest': dom('orkworks.criteria-snapshot.v1', S['worker']['source']['criteria']),
         'rubricSnapshotDigest': dom('orkworks.rubric-snapshot.v1', S['worker']['source']['rubric']),
         'criterionOutcomes': [{'criterionId': c['id'], 'outcome': 'unassessed', 'rationale': 'Synthetic retained membership only.', 'evidenceReferences': []} for c in S['worker']['source']['criteria']],
         'qualityDimensionOutcomes': [{'dimensionId': d['id'], 'outcome': 'unassessed', 'rationale': 'Synthetic retained membership only.', 'evidenceReferences': []} for d in S['worker']['source']['rubric']['dimensions']],
         'rating': {'rating': 'unassessed', 'rationale': 'Synthetic retained membership only.', 'evidenceReferences': []},
         'findings': [], 'findingDisputes': [], 'reviewedEvaluationRef': None, 'correctionReason': None if version == 1 else 'Synthetic correction, original version remains retained.'}
    evaluations.append(e)
    ref = {**pick(e, 'assignmentIdentity resultRevision resultDigest reviewerIdentity evaluationRevision'), 'evaluationDigest': dom('orkworks.assignment-evaluation.v1', e)}
    artifacts.append({'schemaVersion': 1, 'producerAssignment': b, 'declarationId': 'evaluation', 'artifactId': 'membership-artifact', 'version': version,
                      'evaluation': ref, 'sizeBytes': len(canon(e).encode()), 'contentDigest': sha(canon(e))})
stream = {'key': {'producerAssignment': b, 'declarationId': 'evaluation'}, 'value': {'schemaVersion': 1, 'artifactId': 'membership-artifact', 'target': target, 'lastCommit': S['reviewer']['receipt']['commit']}}
artifact_ref = lambda x: obj('assignment-evaluation-artifact', pick(x, 'producerAssignment declarationId artifactId version'), dom('orkworks.assignment-evaluation-artifact.v1', x))
evaluation_ref = lambda x: obj('assignment-evaluation', x, x['evaluationDigest'])
descriptor = {**pick(artifacts[0], 'artifactId version sizeBytes contentDigest')}
bound = descriptor['sizeBytes'] + len(canon({'schemaVersion': 1, 'artifact': descriptor, 'evaluation': None}).encode()) - 4
intent = {'workspaceId': R['workspaceId'], 'storageEpoch': 'f' * 64, 'admissionEpoch': R['admissionEpoch'], 'reviewerAssignment': b,
          'authorityGeneration': {'kind': 'child-launch', 'sidecarGeneration': 'c' * 64, 'generation': 'd' * 64},
          'producerSource': assignment(b)['reference'], 'artifact': artifact_ref(artifacts[0])['reference'], 'evaluation': artifacts[0]['evaluation'], 'descriptor': descriptor,
          'expectedCommit': S['reviewer']['receipt']['commit'], 'usedBefore': {'assignment': '0', 'run': '0', 'workspace': '0'}, 'replyBound': str(bound)}
debit = {'key': {'sidecarGeneration': 'c' * 64, 'sequence': '1'}, 'value': {'body': {'schemaVersion': 1, 'permit': {'sidecarGeneration': 'c' * 64, 'sequence': '1'}, 'intent': intent},
         'digest': '', 'lastCommit': {**S['reviewer']['receipt']['commit'], 'commitSequence': '3', 'changeDigest': sha('Synthetic debit successor, no manifest.')}}}
debit['value']['digest'] = dom('orkworks.evaluation-artifact-read-debit.v1', debit['value']['body'])
source_metadata = []
for name in ['worker', 'reviewer']:
    o = S[name]; src = o['source']
    for table, key, body, ordinal in [
        ('child_plan_approvals', {'plan': src['plan'], 'approvalId': src['approval']['approvalId']}, o['approval'], 2),
        ('child_reservations', src['configuration']['task'], o['reservation'], 3),
        ('child_allocations', src['reservation'], o['allocation'], 4),
        ('child_attachments', src['assignmentIdentity']['childSessionId'], o['attachment'], 5)]:
        origin = o['receipt']['commit'] if ordinal == 2 else {**o['receipt']['commit'], 'commitSequence': str(ordinal), 'changeDigest': sha('Synthetic owning origin: ' + name + ':' + table)}
        value = {'schemaVersion': 1, 'body': body, 'lastCommit': origin}
        if table == 'child_plan_approvals':
            value |= {'digest': src['approval']['approvalDigest'], 'receipt': o['receipt']}
        r = {'table': table, 'key': key, 'value': value}; r['rowDigest'] = row_digest(r)
        source_metadata.append(r)
components = {'evaluations': evaluations, 'artifacts': artifacts, 'stream': stream, 'debit': debit, 'sourceMetadata': source_metadata}


def own_origin(o, table):
    row = next(r for r in source_metadata if r['table'] == table and r['value']['body'] == o[{'child_plan_approvals': 'approval', 'child_reservations': 'reservation', 'child_allocations': 'allocation', 'child_attachments': 'attachment'}[table]])
    return proof(row['value']['lastCommit'])
entries = []


def add(name, consumer, targets):
    # Reachability tuples collapse only identical endpoints. Original byte/field
    # occurrences stay in their source and are not discounted by this relation.
    targets = sorted({canon(t): t for t in targets}.values(), key=lambda x: canon(x).encode())
    body = {'consumer': consumer, 'targets': targets}
    wire = canon(body)
    entries.append({'id': name, **body, 'canonical': wire, 'bytes': len(wire.encode()), 'sha256': sha(wire)})


for name in ['worker', 'reviewer']:
    o = S[name]; src = o['source']; t = src['configuration']['task']
    inp = [] if src['input'] is None else [input_ref(src['input'])]
    add(name + '-plan', plan_ref(src['plan']), [cfg_ref(src['configuration']), *inp, root(), meta('run_states', pick(R, 'workspaceId runId'))])
    evidence = [obj('child-source-evidence', {'task': t, 'referenceId': e['referenceId'], 'contentDigest': e['contentDigest']}, dom('orkworks.child-source-evidence.v1', e)) for e in o['evidenceObjects']]
    add(name + '-configuration', cfg_ref(src['configuration']), [plan_ref(src['plan']), root(), *evidence, *inp])
    add(name + '-source', assignment(src['assignmentIdentity']), [plan_ref(src['plan']), cfg_ref(src['configuration']), *inp, approval(src['approval']), reservation(src['reservation']), allocation(src['allocation']), attachment(o)])
    add(name + '-approval', approval(src['approval']), [plan_ref(src['plan']), own_origin(o, 'child_plan_approvals')])
    add(name + '-reservation', reservation(src['reservation']), [*task_refs(t), approval(src['approval']), own_origin(o, 'child_reservations')])
    add(name + '-allocation', allocation(src['allocation']), [reservation(src['reservation']), plan_ref(src['plan']), own_origin(o, 'child_allocations')])
    add(name + '-attachment', attachment(o), [reservation(src['reservation']), allocation(src['allocation']), assignment(src['assignmentIdentity']), own_origin(o, 'child_attachments')])
    for i, ev in enumerate(evidence):
        add(name + '-evidence-' + str(i), ev, [])
pair = S['snapshot']['evidence'][0]
report_source, report = report_refs(pair['source'])
add('review-input', input_ref(S['reviewer']['source']['input']), [result_ref(target), report_source, assignment(a), cfg_ref(S['worker']['source']['configuration'])])
add('report-source', report_source, [report])
add('report', report, [])
add('target-result', result_ref(target), [assignment(a)])


def control_targets(row):
    table = row['table']; v = row['value']; x = v.get('body')
    out = [proof(v['lastCommit'])]
    if table == 'child_coordination_receipts':
        x = v['request']; o = owner(x['task'])
        return out + task_refs(x['task']) + [reservation(x['reservation']), attachment(o), proof(v['receipt']['commit'])]
    if table == 'child_control_inputs':
        out += [root(), runtime(v['producer'])]
        for rec in x['records']:
            kind = rec['kind']
            if kind == 'configuration': out.append(cfg_ref(rec['reference']))
            elif kind == 'review-input': out.append(input_ref(rec['reference']))
            elif kind == 'result': out.append(result_ref(rec['reference']))
            elif kind == 'research-report': out += report_refs(rec['source'])
            elif kind == 'prior-plan': out.append(plan_ref(rec['plan']))
            else: raise AssertionError(kind)
    elif table == 'child_runtime_attachments':
        out.append(root())
        if x['kind'] == 'parent': out.append(meta('parent_attachments', x['parent']['parentSessionId']))
        else:
            o = owner(pick(x['assignmentIdentity'], 'workspaceId runId planId planRevision taskId'))
            out += [assignment(x['assignmentIdentity']), attachment(o), reservation(o['source']['reservation'])]
    elif table == 'child_run_controls':
        out.append(root()); out += [plan_ref(p) for p in x['planReferences']]
        if x['currentInput'] is not None: out.append(meta('child_control_inputs', {k: val for k, val in x['currentInput'].items() if k != 'inputDigest'}))
        if x['parentRuntime'] is not None: out.append(runtime(x['parentRuntime']))
        for blocker in x['blockers']:
            if blocker['task'] is not None: out += task_refs(blocker['task'])
    elif table == 'child_plan_controls':
        out += [plan_ref(x['plan']), *task_refs(x['task'])]
        if x['latestApproval'] is not None: out.append(approval(x['latestApproval']))
        if x['replaces'] is not None: out.append(plan_ref(x['replaces']))
        o = owner(x['task']); bind = o['plan']['parentPlanBinding']
        assert bind['preparationRevision'] == 1  # Existing source packet only.
        out.append(meta('run_states', pick(R, 'workspaceId runId')))
    elif table == 'child_task_controls':
        out += task_refs(x['task'])
        if x['reservation'] is not None: out.append(reservation(x['reservation']))
        if x['coordinationResult'] is not None: out.append(meta('child_coordination_receipts', x['task']))
    elif table == 'child_launch_controls':
        o = owner(x['task']); out += task_refs(x['task'])
        out += [reservation(x['reservation']), approval(x['admission']['approval']), runtime(x['admission']['parentRuntime'])]
        if x['allocation'] is not None: out.append(allocation(x['allocation']))
        if x['attachmentId'] is not None: out.append(attachment(o))
        known_original = x['spawnRuntime'] is not None and any(
            v['row']['table'] == 'child_runtime_attachments' and v['row']['value']['body']['kind'] == 'child'
            and v['row']['value']['body']['assignmentIdentity'] == o['source']['assignmentIdentity']
            and pick(v['row']['value']['body'], 'sidecarGeneration launchGeneration') == x['spawnRuntime'] for v in C['rows'])
        if x['spawnRuntime'] is not None and (x['phase'] in ['running', 'ended'] or (x['phase'] == 'interrupted' and known_original)):
            out.append(meta('child_runtime_attachments', {'kind': 'child', **pick(R, 'workspaceId runId'), 'sessionId': o['source']['assignmentIdentity']['childSessionId'], **x['spawnRuntime']}))
    elif table == 'child_session_states':
        o = owner(x['task']); out += [reservation(x['reservation']), attachment(o), assignment(x['assignmentIdentity'])]
        if x['runtime'] is not None: out.append(runtime(x['runtime']))
    elif table == 'child_recovery_decisions':
        o = owner(x['task']); out += task_refs(x['task']); out += [reservation(x['reservation']), proof(x['readSet']['expectedCommit']), proof(v['receipt']['commit'])]
        if x['allocation'] is not None: out.append(allocation(x['allocation']))
        if x['attachmentId'] is not None: out.append(attachment(o))
        for slot in ['run', 'task', 'launch', 'session']:
            snap = x['readSet'][slot]
            if snap is None: continue
            out += [{'kind': 'control-origin', 'table': snap['table'], 'key': snap['key'], 'rowDigest': row_digest(snap), 'origin': snap['value']['lastCommit']},
                    *control_targets(snap)]
    else: raise AssertionError(table)
    return out


for entry in C['rows']:
    row = entry['row']
    add('control-' + entry['id'], meta(row['table'], row['key']), control_targets(row))
stream_loc = meta('evaluation_artifact_streams', stream['key'])
for i, art in enumerate(artifacts):
    ev = evaluation_ref(art['evaluation']); ar = artifact_ref(art)
    add('evaluation-' + str(i + 1), ev, [ar, assignment(a), assignment(b), stream_loc, result_ref(target)])
    add('artifact-' + str(i + 1), ar, [ev, assignment(a), assignment(b), stream_loc])
add('artifact-stream', stream_loc, [assignment(a), assignment(b), cfg_ref(S['reviewer']['source']['configuration']), result_ref(target),
    *[artifact_ref(x) for x in artifacts], *[evaluation_ref(x['evaluation']) for x in artifacts], proof(stream['value']['lastCommit'])])
debit_targets = [assignment(a), {'kind': 'object', 'reference': intent['producerSource']}, {'kind': 'object', 'reference': intent['artifact']}, evaluation_ref(intent['evaluation']), stream_loc,
                 proof(intent['expectedCommit']), proof(debit['value']['lastCommit'])]
for quota_owner in [{'kind': 'assignment', 'assignmentIdentity': b}, {'kind': 'run', **pick(b, 'workspaceId runId')}, {'kind': 'workspace', 'workspaceId': b['workspaceId']}]:
    debit_targets.append(meta('quota_usage', {'owner': quota_owner, 'resource': 'evaluation-artifact-read-bytes'}))
add('historical-debit', meta('artifact_read_debits', debit['key']), debit_targets)
# This reverse relation covers alternatives, not a coherent current store.
reverse = {}
for entry in entries:
    for t in entry['targets']:
        k = canon(t); reverse.setdefault(k, {'target': t, 'consumers': {}})['consumers'][canon(entry['consumer'])] = entry['consumer']
reverse = [{'target': v['target'], 'consumers': sorted(v['consumers'].values(), key=lambda x: canon(x).encode())} for k, v in sorted(reverse.items(), key=lambda kv: kv[0].encode())]
packet = {'schemaVersion': 1, 'classification': 'synthetic-child-membership-components-only', 'components': components, 'entries': entries, 'reverse': reverse}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT / 'child-membership-vectors.json')
args = parser.parse_args()
args.output.write_bytes((json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode())
