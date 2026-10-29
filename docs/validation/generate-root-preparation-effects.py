"""Independent synthetic initial-effect witnesses; never native admission evidence."""
import argparse
import base64
import copy
import hashlib
import json
from pathlib import Path


def clone(x):
    return copy.deepcopy(x)


def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def size(x):
    return len(canonical(x).encode('utf-8'))


def sha(x):
    return hashlib.sha256(x.encode('utf-8')).hexdigest()


def domain(name, x):
    return sha(name + '\n' + canonical(x))


def seal(x, field, name):
    body = {k: v for k, v in x.items() if k != field}
    x[field] = domain(name, body)


def row(table, key, value):
    return {'table': table, 'key': clone(key), 'value': clone(value)}


def rid(r):
    return r['table'] + '\n' + canonical(r['key'])


def project(r):
    v = clone(r['value'])
    if r['table'] not in ['store_header', 'commit_proof']:
        del v['lastCommit']
    if r['table'] == 'root_preparations':
        del v['receipt']['commit']
    if r['table'] == 'summary_state' and v['checkpoint'] is not None:
        del v['checkpoint']['applied']
    return v


def row_digest(r):
    return domain('orkworks.hierarchy-storage-row.v3', {'table': r['table'], 'key': r['key'], 'body': project(r)})


bootstrap_path = Path('docs/validation/root-bootstrap-vectors.json')
join_path = Path('docs/validation/root-preparation-join-vectors.json')
base = json.loads(bootstrap_path.read_bytes())
joins = json.loads(join_path.read_bytes())
seed_draft = json.loads(next(v['canonical'] for v in joins['vectors'] if v['kind'] == 'draft'))
seed_approval = json.loads(next(v['canonical'] for v in joins['vectors'] if v['kind'] == 'approval'))


def bind_case(name, admission_epoch, sequence, duplicate_gap=False):
    d = clone(seed_draft)
    b = d['bootstrap']
    previous_root = b['rootAssignmentId']
    b.update({'runId': admission_epoch + '-' + sequence, 'parentSessionId': name + '-effect-parent',
              'rootAssignmentId': name + '-effect-root', 'bootstrapId': name + '-effect-bootstrap'})
    b['model']['adapterGeneration'] = name + '-effect-adapter-generation'
    for profile in b['permissions'].values():
        for action in profile['coordinationActions']:
            action.update({k: b[k] for k in ['workspaceId', 'runId', 'parentSessionId']})
    for manifest in b['requirementManifests']:
        manifest['taskId'] = b['rootAssignmentId']
    e = b['capabilityEvidence']
    e['evidenceId'] = name + '-effect-evidence'
    if duplicate_gap:
        e['references'].append({**clone(e['references'][-1]), 'id': 'artifact-4', 'sourceReference': 'synthetic:duplicate-gap'})
    e['requestedProfileDigest'] = sha(canonical(b['permissions']['requested']))
    e['effectiveProfileDigest'] = sha(canonical(b['permissions']['effective']))
    seal(e, 'evidenceDigest', 'orkworks.capability-evidence.v1')
    b['harness'].update({'evidenceId': e['evidenceId'], 'evidenceDigest': e['evidenceDigest']})
    # Only the ASCII framing ID changes; assignment/output/rubric content is exact.
    b['renderedInstructions'] = b['renderedInstructions'].replace('assignment ' + previous_root + ' ', 'assignment ' + b['rootAssignmentId'] + ' ')
    b['renderedInstructionsDigest'] = sha(b['renderedInstructions'])
    seal(b, 'bootstrapConfigurationDigest', 'orkworks.orchestrator-bootstrap.v2')
    r = d['runDefinition']
    r.update({k: b[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'bootstrapConfigurationDigest']})
    r.update({'admissionEpoch': admission_epoch, 'runSequence': sequence})
    seal(r, 'runDefinitionDigest', 'orkworks.orchestration-run.v1')
    d.update({k: b[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'rootAssignmentId']})
    d.update({'draftId': name + '-effect-draft', 'reviewContextId': ('8' if name == 'target' else '9') * 64})
    seal(d, 'draftDigest', 'orkworks.root-preparation-draft.v1')
    a = {**clone(seed_approval), **{k: d[k] for k in ['workspaceId', 'runId', 'draftId', 'draftDigest', 'reviewContextId']},
         'approvalId': name + '-effect-approval'}
    return d, a


draft, approval = bind_case('target', 'a' * 64, '2', True)
other_draft, other_approval = bind_case('other', 'e' * 64, '1')
contexts = {}
for name, d, a in [('target', draft, approval), ('other', other_draft, other_approval)]:
    contexts[name] = {'draftCanonical': canonical(d), 'approvalCanonical': canonical(a),
                      'retainedArtifacts': clone(base['retainedArtifacts'])}
workspace = draft['workspaceId']
epoch = 'f' * 64
REPORT = ['reporting-receipt-bytes', 'assignment-record-bytes', 'result-payload-bytes']
SUMMARY = ['summary-row-bytes', 'summary-row-count', 'summary-lineage-bytes', 'summary-pending-count']
PREPARATION = ['preparation-retained-bytes', 'preparation-run-count']
PREP_TABLES = ['root_drafts', 'root_preparations', 'run_states', 'parent_attachments', 'preparation_artifacts']
W = {'kind': 'workspace', 'workspaceId': workspace}


def root_identity(b):
    return {'kind': 'root', **{k: b[k] for k in ['workspaceId', 'runId', 'rootAssignmentId', 'parentSessionId', 'bootstrapId', 'bootstrapConfigurationDigest']}}


def scoped_owner(ws, run):
    return {'kind': 'run', 'workspaceId': ws, 'runId': run}


def owner_for(r):
    v = r['value']
    owner = v['draft'] if r['table'] == 'root_drafts' else v['bootstrap'] if r['table'] == 'root_preparations' else v['state'] if r['table'] == 'run_states' else v['attachment'] if r['table'] == 'parent_attachments' else r['key']
    return scoped_owner(owner['workspaceId'], owner['runId'])


def material(b):
    outcomes = [{'target': {'kind': 'criterion', 'id': c['id']}, 'disposition': 'unassessed'} for c in b['assignment']['criteria'] if c['requirement'] == 'required']
    outcomes += [{'target': {'kind': 'quality-dimension', 'id': d['id']}, 'disposition': 'unassessed'} for d in b['rubric']['dimensions']]
    outcomes += [{'target': {'kind': 'rating'}, 'disposition': 'unassessed'}]
    return {'schemaVersion': 1, 'assignmentIdentity': root_identity(b), 'accepted': {'kind': 'none'},
            'criteriaSnapshotDigest': domain('orkworks.criteria-snapshot.v1', sorted(b['assignment']['criteria'], key=lambda c: c['id'])),
            'rubricSnapshotDigest': domain('orkworks.rubric-snapshot.v1', b['rubric']), 'overall': 'unassessed',
            'requiredOutcomes': sorted(outcomes, key=lambda o: canonical(o['target'])), 'requiredFindings': []}


def desired_quotas(rows):
    q = {}
    def put(owner, resource, used):
        key = {'owner': clone(owner), 'resource': resource}
        q[canonical(key)] = row('quota_usage', key, {'used': str(used), 'reserved': '0', 'lastCommit': None})
    for resource in REPORT + SUMMARY + PREPARATION:
        put(W, resource, 0)
    prep_rows = [r for r in rows.values() if r['table'] in PREP_TABLES]
    owners = {canonical(owner_for(r)): owner_for(r) for r in prep_rows}
    for owner in owners.values():
        used = sum(size(r['key']) + size(project(r)) for r in prep_rows if owner_for(r) == owner)
        put(owner, 'preparation-retained-bytes', used)
    put(W, 'preparation-retained-bytes', sum(size(r['key']) + size(project(r)) for r in prep_rows))
    run_ids = {r['value']['draft']['runId'] if r['table'] == 'root_drafts' else r['value']['bootstrap']['runId'] for r in rows.values() if r['table'] in ['root_drafts', 'root_preparations']}
    put(W, 'preparation-run-count', len(run_ids))
    for source in [r for r in rows.values() if r['table'] == 'root_preparations']:
        b = source['value']['bootstrap']
        for owner in [{'kind': 'assignment', 'assignmentIdentity': root_identity(b)}, scoped_owner(b['workspaceId'], b['runId'])]:
            for resource in REPORT:
                put(owner, resource, 0)
    summaries = [r['value'] for r in rows.values() if r['table'] == 'summary_state']
    put(W, 'summary-row-count', sum(v['row'] is not None for v in summaries))
    put(W, 'summary-row-bytes', sum(size(v['row']) for v in summaries if v['row'] is not None))
    put(W, 'summary-lineage-bytes', sum(size(v['checkpoint']['material']) for v in summaries if v['checkpoint'] is not None))
    return q


caps = {'classification': 'fixture-only-not-production', 'preparationRunBytes': '16777216', 'preparationWorkspaceBytes': '33554432',
        'preparationRunCount': '8', 'summaryCount': '4096', 'summaryBytes': '268435456', 'lineageBytes': '67108864', 'pendingCount': '4096',
        'commandBytes': 16777216, 'changedRows': 4096}
full_caps = {**caps, 'summaryCount': '1'}
states = {'absent': {}}
commands = []


def commit(before_name, after_name, kind, subject, core, cap_name='fitting', branch=None):
    before = states[before_name]
    after = clone(before)
    for r in core:
        if r['value'] is None:
            after.pop(rid(r))
        else:
            after[rid(r)] = clone(r)
    expected = desired_quotas(after)
    for k in list(after):
        if after[k]['table'] == 'quota_usage' and canonical(after[k]['key']) not in expected:
            del after[k]
    for r in expected.values():
        old = before.get(rid(r))
        after[rid(r)] = clone(old) if old and project(old) == project(r) else r
    header = before.get('store_header\n1')
    sequence = str(int(header['value']['latest']['commitSequence']) + 1) if header else '1'
    changed = [key for key in set(before) | set(after) if key not in before or key not in after or canonical(project(before[key])) != canonical(project(after[key]))]
    changes = []
    for key in changed:
        old, new = before.get(key), after.get(key)
        r = new or old
        changes.append({'table': r['table'], 'key': r['key'], 'beforeDigest': row_digest(old) if old else None, 'afterDigest': row_digest(new) if new else None})
    changes.sort(key=lambda c: (c['table'], canonical(c['key'])))
    manifest = {'schemaVersion': 3, 'workspaceId': workspace, 'storageEpoch': epoch, 'commitSequence': sequence,
                'predecessor': header['value']['latest'] if header else None, 'commandKind': kind, 'changes': changes}
    change_digest = domain('orkworks.hierarchy-storage-change.v3', manifest)
    link = {'storageEpoch': epoch, 'commitSequence': sequence, 'changeDigest': change_digest}
    for key in changed:
        if key not in after:
            continue
        r = after[key]
        r['value']['lastCommit'] = clone(link)
        if r['table'] == 'root_preparations':
            r['value']['receipt']['commit'] = clone(link)
        if r['table'] == 'summary_state' and r['value']['checkpoint'] is not None:
            r['value']['checkpoint']['applied'] = clone(link)
    control = [row('store_header', 1, {'schemaVersion': 3, 'workspaceId': workspace, 'storageEpoch': epoch, 'latest': link}),
               row('commit_proof', sequence, {'body': manifest, 'changeDigest': change_digest})]
    for r in control:
        after[rid(r)] = r
    all_changed = changed + [rid(r) for r in control]
    command_bytes = sum(size(after[k]['key']) + size(after[k]['value']) if k in after else size(before[k]['key']) for k in all_changed)
    assert len(set(all_changed)) == len(all_changed)
    assert command_bytes <= caps['commandBytes']
    states[after_name] = after
    commands.append({'beforeState': before_name, 'afterState': after_name, 'commandKind': kind, 'subject': subject,
                     'summaryBranch': branch, 'caps': cap_name, 'manifestCanonical': canonical(manifest), 'changeDigest': change_digest,
                     'changedRows': len(all_changed), 'commandBytes': command_bytes})


def prepare(name):
    d = json.loads(contexts[name]['draftCanonical'])
    b = d['bootstrap']
    core = [row('root_drafts', workspace, {'schemaVersion': 1, 'draft': d, 'lastCommit': None})]
    lookup = {a['contentDigest']: a['content'].encode('utf-8') for a in contexts[name]['retainedArtifacts']}
    for ref in b['capabilityEvidence']['references']:
        raw = lookup[ref['contentDigest']]
        core.append(row('preparation_artifacts', {'workspaceId': workspace, 'runId': b['runId'], 'referenceId': ref['id']},
                        {'schemaVersion': 1, 'contentBase64': base64.b64encode(raw).decode(), 'sizeBytes': str(len(raw)),
                         'contentDigest': ref['contentDigest'], 'lastCommit': None}))
    return core


def admit(name, initialized):
    d = json.loads(contexts[name]['draftCanonical'])
    a = json.loads(contexts[name]['approvalCanonical'])
    b, run = d['bootstrap'], d['runDefinition']
    request_digest = domain('orkworks.root-preparation-request.v1', a)
    identity = root_identity(b)
    receipt = {'schemaVersion': 1, 'workspaceId': workspace, 'runId': b['runId'], 'rootAssignmentIdentity': identity,
               'bootstrapConfigurationDigest': b['bootstrapConfigurationDigest'], 'runDefinitionDigest': run['runDefinitionDigest'],
               'draftDigest': d['draftDigest'], 'approvalId': a['approvalId'], 'requestDigest': request_digest, 'commit': None}
    source = {'schemaVersion': 1, 'bootstrap': b, 'runDefinition': run,
              'approval': {**{k: a[k] for k in ['approvalId', 'draftId', 'draftDigest', 'reviewContextId']},
                           'sidecarGeneration': d['sidecarGeneration'], 'requestDigest': request_digest}, 'receipt': receipt, 'lastCommit': None}
    input_value = {'schemaVersion': 1, 'workspaceId': workspace, 'runId': b['runId'], 'preparationRevision': 1, 'inputRevision': 1,
                   'goalDigest': b['goalDigest'], 'bootstrapConfigurationDigest': b['bootstrapConfigurationDigest'], 'decision': None, 'records': []}
    state = {'schemaVersion': 1, 'workspaceId': workspace, 'runId': b['runId'], 'parentSessionId': b['parentSessionId'],
             'runDefinitionDigest': run['runDefinitionDigest'], 'bootstrapConfigurationDigest': b['bootstrapConfigurationDigest'],
             'runVersion': 1, 'state': 'assessing', 'preparationRevision': 1, 'inputRevision': 1,
             'inputDigest': domain('orkworks.preparation-input.v1', input_value), 'planReferences': [], 'currentPlan': None,
             'parentRuntime': None, 'blockers': [], 'stageContinueRequired': False, 'pendingClarification': None, 'pauseReason': None, 'resumeTarget': None}
    attachment = {'schemaVersion': 1, 'kind': 'reserved', **{k: b[k] for k in ['workspaceId', 'runId', 'parentSessionId', 'rootAssignmentId', 'bootstrapConfigurationDigest']}}
    summary = {'subjectRevision': '1', 'row': None, 'checkpoint': None, 'admission': 'never-initialized', 'lastCommit': None}
    if initialized:
        m = material(b)
        summary.update({'row': {'schemaVersion': 1, 'assignmentIdentity': identity, 'summaryRevision': '1', 'state': {'kind': 'current', 'material': m}},
                        'checkpoint': {'material': m, 'summaryRevision': '1', 'applied': None}, 'admission': 'initialized'})
    key = {'workspaceId': workspace, 'runId': b['runId']}
    return [row('root_drafts', workspace, None), row('root_preparations', key, source),
            row('run_states', key, {'schemaVersion': 1, 'initialInput': input_value, 'state': state, 'lastCommit': None}),
            row('parent_attachments', b['parentSessionId'], {'schemaVersion': 1, 'attachment': attachment, 'lastCommit': None}),
            row('summary_state', identity, summary)]


commit('absent', 'initialized', 'initialize-store', None, [])
commit('initialized', 'other-draft', 'root-draft-prepare', 'other', prepare('other'))
commit('other-draft', 'other-admitted', 'subject-preparation', 'other', admit('other', True), branch='initialized')
commit('other-admitted', 'target-draft', 'root-draft-prepare', 'target', prepare('target'))
commit('target-draft', 'target-admitted', 'subject-preparation', 'target', admit('target', True), branch='initialized')
commit('target-draft', 'target-never-initialized', 'subject-preparation', 'target', admit('target', False), cap_name='summary-full', branch='never-initialized')
cancel = [row('root_drafts', workspace, None)] + [row(r['table'], r['key'], None) for r in states['target-draft'].values() if r['table'] == 'preparation_artifacts' and r['key']['runId'] == draft['runId']]
commit('target-draft', 'target-cancelled', 'root-draft-cancel', 'target', cancel)
encoded_states = []
for name, rows in states.items():
    encoded_states.append({'name': name, 'rows': [{'table': r['table'], 'keyCanonical': canonical(r['key']),
        'valueCanonical': canonical(r['value']), 'rowDigest': row_digest(r)} for r in sorted(rows.values(), key=lambda r: (r['table'], canonical(r['key'])))]})
    encoded_states[-1]['stateSha256'] = sha(canonical(encoded_states[-1]['rows']))
catalog = {'schemaVersion': 1, 'classification': 'synthetic-logical-effects-only-not-runtime-admission',
           'inputs': {'bootstrapCatalogSha256': hashlib.sha256(bootstrap_path.read_bytes()).hexdigest(),
                      'joinCatalogSha256': hashlib.sha256(join_path.read_bytes()).hexdigest()},
           'contexts': contexts, 'caps': {'fitting': caps, 'summary-full': full_caps}, 'states': encoded_states, 'commands': commands,
           'unverified': ['actual UI authority and reviewed presentation', 'durable admission fence and native revocation ordering',
                          'capability/native/source evidence and source custody', 'SQL atomicity/recovery, physical/query/resource limits',
                          'production preparation/history/shared-custody caps and runtime eligibility']}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=Path('docs/validation/root-preparation-effects.json'))
args = parser.parse_args()
args.output.write_bytes((json.dumps(catalog, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
