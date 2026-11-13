"""Offline key/value/charge components only; no storage commitments or history."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SUBJECT = ['reporting-receipt-bytes', 'assignment-record-bytes', 'result-payload-bytes', 'evaluation-artifact-bytes', 'evaluation-artifact-read-bytes']
SUMMARY = ['summary-row-bytes', 'summary-row-count', 'summary-lineage-bytes', 'summary-pending-count']
EXTRA = ['child-source-retained-bytes', 'child-control-retained-bytes', 'report-retained-bytes']
ALL = SUBJECT + SUMMARY + ['preparation-retained-bytes', 'preparation-run-count'] + EXTRA


def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def size(x):
    return len(canonical(x).encode())


def pick(x, fields):
    return {k: x[k] for k in fields.split()}


def location(table, key):
    return dict(table=table, key=copy.deepcopy(key))


def identity(x):
    return x['table'] + '\n' + canonical(x['key'])


def ordered(xs):
    return sorted(xs, key=identity)


def ledger(owner, resource, used, origin):
    return dict(table='quota_usage', key=dict(owner=owner, resource=resource),
                value=dict(used=str(used), reserved='0', lastCommit=origin))


def rows(base, name):
    return [dict(table=e['table'], key=json.loads(e['keyCanonical']), value=json.loads(e['valueCanonical']))
            for e in next(s for s in base['states'] if s['name'] == name)['rows']
            if e['table'] not in ['store_header', 'commit_proof']]


def projection_size(r):
    # Existing preparation/control projection only, never a row/change hash.
    v = copy.deepcopy(r['value'])
    v.pop('lastCommit')
    if r['table'] == 'root_preparations':
        v['receipt'].pop('commit')
    return size(r['key']) + size(v)


def packet():
    raw = (ROOT / 'root-preparation-effects.json').read_bytes()
    base = json.loads(raw)
    d = json.loads(base['contexts']['target']['draftCanonical'])
    b, run = d['bootstrap'], d['runDefinition']
    rk = pick(run, 'workspaceId runId')
    W, R = dict(kind='workspace', workspaceId=run['workspaceId']), dict(kind='run', **rk)
    A = dict(kind='assignment', assignmentIdentity=dict(kind='root', **pick(b, 'workspaceId runId rootAssignmentId parentSessionId bootstrapId bootstrapConfigurationDigest')))
    origins = {name: dict(storageEpoch='f' * 64, commitSequence=str(i + 1),
        changeDigest=hashlib.sha256(('Unqualified initializer component: ' + name).encode()).hexdigest())
        for i, name in enumerate(['initialize', 'captured', 'draft', 'admission'])}
    old = rows(base, 'target-draft')
    admitted = rows(base, 'target-admitted')
    initial = next(r['value']['initialInput'] for r in admitted if r['table'] == 'run_states' and r['key'] == rk)
    control_body = dict(schemaVersion=1, profile='single-task-retained-file-research-prefix-v1',
        **pick(run, 'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
        runVersion=1, state='assessing', currentInput={**pick(initial, 'workspaceId runId preparationRevision inputRevision'),
        'inputDigest': hashlib.sha256(('orkworks.preparation-input.v1\n' + canonical(initial)).encode()).hexdigest()},
        planReferences=[], currentPlan=None, pendingPlan=None, parentRuntime=None, blockers=[],
        stageContinueRequired=False, pendingClarification=None, pauseReason=None, resumeTarget=None)
    control = dict(table='research_run_controls', key=rk, value=dict(schemaVersion=1, body=control_body, lastCommit=origins['admission']))
    control_bytes = size(rk) + size(dict(schemaVersion=1, body=control_body))
    prep_tables = {'root_drafts', 'root_preparations', 'run_states', 'parent_attachments', 'preparation_artifacts'}

    def owns(r):
        if r['table'] == 'parent_attachments':
            return r['key'] == run['parentSessionId']
        return isinstance(r['key'], dict) and r['key'].get('runId') == run['runId'] or r['table'] == 'root_drafts' and r['value']['draft']['runId'] == run['runId']

    before = {identity(r): r for r in old if r['table'] in prep_tables and owns(r)}
    after = {identity(r): r for r in admitted if r['table'] in prep_tables and owns(r)}
    facts = [dict(**location((after.get(k) or before[k])['table'], (after.get(k) or before[k])['key']),
        beforeBytes=projection_size(before[k]) if k in before else 0,
        afterBytes=projection_size(after[k]) if k in after else 0) for k in sorted(before.keys() | after.keys())]
    before_bytes = sum(f['beforeBytes'] for f in facts)
    after_bytes = sum(f['afterBytes'] for f in facts)
    peers = {r['key']['resource']: r['value'] for r in old if r['table'] == 'quota_usage' and r['key']['owner'] == W}
    # These five captured peer balances are hypothetical retained background
    # contributions, not reconstructed inventory/proofs or an existing store.
    background = dict(zip(['evaluation-artifact-bytes', 'evaluation-artifact-read-bytes', *EXTRA], [4096, 8192, 16384, 2048, 8192]))
    captured = ordered([ledger(W, name, background.get(name, int(peers[name]['used']) if name in peers else 0),
        origins['draft'] if name in ['preparation-retained-bytes', 'preparation-run-count'] else origins['captured']) for name in ALL])
    draft_resource = ledger(R, 'preparation-retained-bytes', before_bytes, origins['draft'])
    artifacts = [location(r['table'], r['key']) for r in before.values() if r['table'] == 'preparation_artifacts']
    draft_locations = ordered([location('root_drafts', run['workspaceId']), *artifacts,
        location('quota_usage', draft_resource['key']), location('quota_usage', dict(owner=W, resource='preparation-retained-bytes')),
        location('quota_usage', dict(owner=W, resource='preparation-run-count'))])
    admissions = []
    for name, state_name in [('initialized', 'target-admitted'), ('never-initialized', 'target-never-initialized')]:
        summary = next(r for r in rows(base, state_name) if r['table'] == 'summary_state' and r['key'] == A['assignmentIdentity'])
        summary_delta = dict(zip(SUMMARY, [size(summary['value']['row']) if name == 'initialized' else 0,
            1 if name == 'initialized' else 0, size(summary['value']['checkpoint']['material']) if name == 'initialized' else 0, 0]))
        new_rows = [ledger(owner, resource, 0, origins['admission']) for owner in [A, R] for resource in SUBJECT]
        new_rows += [ledger(R, resource, control_bytes if resource == 'child-control-retained-bytes' else 0, origins['admission']) for resource in EXTRA]
        workspace_after = copy.deepcopy(captured)
        deltas = {**summary_delta, 'preparation-retained-bytes': after_bytes - before_bytes, 'child-control-retained-bytes': control_bytes}
        for r in workspace_after:
            delta = deltas.get(r['key']['resource'], 0)
            if delta:
                r['value'].update(used=str(int(r['value']['used']) + delta), lastCommit=origins['admission'])
        core = [location('root_drafts', run['workspaceId']), location('root_preparations', rk), location('run_states', rk),
                location('parent_attachments', run['parentSessionId']), location('summary_state', A['assignmentIdentity']), location('research_run_controls', rk)]
        replacements = [location('quota_usage', dict(owner=R, resource='preparation-retained-bytes')),
            *[location('quota_usage', r['key']) for old_r, r in zip(captured, workspace_after) if old_r != r]]
        admissions.append(dict(id=name, initialControl=control, newResourceRows=ordered(new_rows),
            runPreparationAfter=ledger(R, 'preparation-retained-bytes', after_bytes, origins['admission']),
            workspaceAfter=workspace_after, summaryChargeDeltas=summary_delta, logicalLocations=ordered(core + [location(r['table'], r['key']) for r in new_rows] + replacements)))
    return dict(classification='offline-research-initialization-components-only', baselineSha256=hashlib.sha256(raw).hexdigest(),
        unqualifiedOrigins=origins, workspaceInitialization=ordered([ledger(W, resource, 0, origins['initialize']) for resource in ALL]),
        capturedWorkspacePeers=captured, draftRunResources=[draft_resource], preparationChargeFacts=facts,
        draftLocations=draft_locations, cancelLocations=draft_locations, retryLocations=[],
        unqualifiedControls=['header replacement', 'new proof'], admissions=admissions,
        assumptions=['owner coverage and freshness not proven', 'captured peer usage is hypothetical', 'original creation proofs not supplied'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
