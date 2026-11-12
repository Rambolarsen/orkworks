"""Independent offline membership samples only; not a store or legal history."""
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
    return hashlib.sha256(x.encode()).hexdigest()


def domain(name, x):
    return sha(name + '\n' + canonical(x))


def pick(x, fields):
    return {k: x[k] for k in fields.split()}


def unique(values):
    return [v for _, v in sorted({canonical(v): v for v in values}.items())]


def meta(table, key):
    return dict(kind='metadata', table=table, key=copy.deepcopy(key))


def obj(kind, identity, digest):
    return dict(kind='object', reference=dict(key=dict(kind=kind, identity=copy.deepcopy(identity)), digest=digest))


def proof(commit):
    return dict(kind='commit-proof', commit=copy.deepcopy(commit))


def group(name, entries):
    reverse = {}
    for e in entries:
        for t in e['targets']:
            key = canonical(t)
            reverse.setdefault(key, dict(target=t, consumers=[]))['consumers'].append(e['consumer'])
    return dict(id=name, entries=entries, reverse=[dict(target=v['target'], consumers=unique(v['consumers'])) for _, v in sorted(reverse.items())])


def packet():
    p = json.loads((ROOT / 'research-prefix-components.json').read_text())
    ch = p['chain']
    s, a = ch['startup'], ch['assignment']
    run, t = s['context']['run'], s['scope']['task']
    rk = pick(run, 'workspaceId runId')
    root = meta('root_preparations', rk)
    seed = meta('run_states', rk)
    plan = obj('research-source-plan', a['plan'], ch['plan']['planDefinitionDigest'])
    cfg = obj('research-source-configuration', a['configuration'], s['configuration']['configurationDigest'])
    scope = obj('research-source-scope', a['scope'], s['scope']['scopeDigest'])
    caps = [obj('research-source-capture', dict(task=t, sourceId=c['sourceId'], captureDigest=c['captureDigest']), c['captureDigest']) for c in s['captures']]
    evidence = [obj('research-source-evidence', dict(task=t, **pick(e, 'referenceId contentDigest')), domain('orkworks.research-source-evidence.v1', e)) for e in s['evidenceObjects']]
    assignment = obj('research-assignment-source', a['assignmentIdentity'], ch['fingerprints']['assignmentSourceDigest'])
    report = obj('research-report', p['reportAssumption']['source']['reportIdentity'], p['reportAssumption']['report']['contentDigest'])
    approval = meta('research_plan_approvals', dict(plan=a['plan'], approvalId=a['approval']['approvalId']))
    reservation = meta('research_reservations', t)
    allocation = meta('research_allocations', a['reservation'])
    attachment = meta('research_attachments', a['assignmentIdentity']['childSessionId'])
    task = meta('research_task_controls', t)
    runtime = lambda r: meta('research_runtime_attachments', pick(r, 'kind workspaceId runId sessionId sidecarGeneration launchGeneration'))
    parent, child = [runtime(r['key']) for r in p['runtimeHistory']]
    input_location = lambda b: meta('research_control_inputs', pick(b, 'workspaceId runId preparationRevision inputRevision'))

    def entry(name, consumer, targets):
        return dict(id=name, consumer=consumer, targets=unique(targets))

    sources = [entry('plan', plan, [root, cfg, scope, input_location(p['inputs'][0]['value']['body'])]),
        entry('configuration', cfg, [root, plan, scope, *evidence]),
        entry('scope', scope, caps),
        *[entry('capture-' + str(i), c, [root, plan, cfg, scope]) for i, c in enumerate(caps)],
        *[entry('evidence-' + str(i), e, [cfg]) for i, e in enumerate(evidence)],
        entry('assignment', assignment, [root, plan, cfg, scope, approval, reservation, allocation, attachment]),
        entry('report', report, [assignment, scope, child])]
    history = []
    for r in ch['metadata']:
        targets = {'research_plan_approvals': [plan],
            'research_reservations': [plan, cfg, scope, approval],
            'research_allocations': [reservation, plan],
            'research_attachments': [reservation, allocation, assignment]}[r['table']]
        if r['table'] == 'research_plan_approvals':
            targets.append(proof(r['value']['receipt']['commit']))
        history.append(entry(r['table'], meta(r['table'], r['key']), [*targets, proof(r['value']['lastCommit'])]))
    for r in p['inputs']:
        b, targets = r['value']['body'], [root, cfg, scope, *caps, runtime(r['value']['producer'])]
        if b['inputRevision'] == 3:
            targets += [report, input_location(p['inputs'][0]['value']['body']), parent]
        history.append(entry('input-' + str(b['inputRevision']), input_location(b), [*targets, proof(r['value']['lastCommit'])]))
    for r in p['runtimeHistory']:
        extra = [meta('parent_attachments', run['parentSessionId'])] if r['key']['kind'] == 'parent' else [assignment, attachment, reservation]
        history.append(entry('runtime-' + r['key']['kind'], runtime(r['key']), [root, *extra, proof(r['value']['lastCommit'])]))

    stream = pick(a['assignmentIdentity'], 'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest')
    request = dict(schemaVersion=1, operation='submitResearchReport', stream=stream, expectedTaskVersion=7,
                   expectedRunVersion=None, expectedReportHead=None, report=p['reportAssumption']['report'])
    head_row = dict(table='report_heads', key=stream, value=dict(schemaVersion=1, head=p['reportAssumption']['head'], lastCommit=p['context']['unqualifiedCommit']))
    receipt_row = dict(table='report_receipts', key=dict(stream=stream, reportVersion=1), value=dict(schemaVersion=1,
        **pick(request, 'operation expectedTaskVersion expectedRunVersion expectedReportHead'),
        requestDigest=domain('orkworks.research-report-request.v1', request),
        response=dict(schemaVersion=1, acceptedHead=p['reportAssumption']['head']), lastCommit=p['context']['unqualifiedCommit']))
    registry = [entry(r['table'], meta(r['table'], r['key']), [report, assignment, proof(r['value']['lastCommit'])]) for r in [head_row, receipt_row]]
    groups = [group('sources', sources), group('history', history), group('registry', registry)]
    for view in p['views']:
        entries = []
        for r in view['rows']:
            b = r['value']['body']
            targets = [proof(r['value']['lastCommit'])]
            if r['table'] == 'research_run_controls':
                targets += [root]
                targets += [seed] if b['currentInput']['inputRevision'] == 1 else [input_location(b['currentInput'])]
                targets += [plan] if b['planReferences'] else []
                targets += [parent] if b['parentRuntime'] is not None else []
            elif r['table'] == 'research_plan_controls':
                targets += [plan, cfg, task]
                targets += [approval] if b['latestApproval'] is not None else []
            elif r['table'] == 'research_task_controls':
                targets += [plan, cfg]
                targets += [reservation] if b['reservation'] is not None else []
            elif r['table'] == 'research_launch_controls':
                targets += [plan, cfg, reservation, approval, parent, allocation, attachment, child]
            elif r['table'] == 'research_session_states':
                targets += [reservation, attachment, assignment]
                targets += [child] if b['runtime'] is not None else []
            elif r['table'] == 'research_collections':
                targets += [meta('research_run_controls', rk), task]
            elif r['table'] == 'research_expectations':
                targets += [meta('research_collections', rk), cfg, scope]
                targets += [reservation] if b['reservation'] is not None else []
                targets += [report] if b['selectedReport'] is not None else []
            entries.append(entry(r['table'], meta(r['table'], r['key']), targets))
        groups.append(group(view['id'], entries))
    before, after = [next(v for v in p['views'] if v['id'] == name)['rows'] for name in ['before-selection', 'selected-source']]
    changes, unchanged = [], []
    for old, new in zip(before, after):
        location = meta(old['table'], old['key'])
        if old['value']['body'] == new['value']['body']:
            unchanged.append(location)
        else:
            changes.append(dict(location=location, before=old['value'], after=new['value']))
    changes.append(dict(location=input_location(p['inputs'][1]['value']['body']), before=None, after=p['inputs'][1]['value']))
    standalone = entry('separate-evidence-source-example', obj('assignment-evidence-source', p['reportAssumption']['source'], p['reportAssumption']['sourceKey']), [report])
    return dict(classification='conditional-research-membership-components-only',
        prefixFingerprint=sha(canonical(p)), groups=groups, registryComponents=dict(request=request, headRow=head_row, receiptRow=receipt_row),
        standaloneSourceExample=standalone,
        effectComparison=dict(beforeView='before-selection', afterView='selected-source', changes=changes, unchanged=unchanged),
        discoveryAssumption=dict(expectedHeader=p['context']['unqualifiedCommit'], finalHeader=p['context']['unqualifiedCommit'], complete=True),
        discoveryTrace=[dict(kind='occurrence', key=dict(collection=rk, task=t), value=dict(selectedReport=None)),
                        dict(kind='row', key=t, value=before[-1]['value']), dict(kind='miss', key=receipt_row['key'])],
        prospectiveOwnerAssumption=[dict(collection=rk, task=t)])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, indent=2) + '\n').encode()
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
