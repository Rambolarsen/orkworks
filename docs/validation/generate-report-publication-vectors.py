"""Independent conditional terminal-prestate and format-3 correction witnesses."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
raw_seed = (HERE / 'r2-report-vectors.json').read_bytes()
seed = json.loads(raw_seed)
C = lambda x: json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
B = lambda x: len(C(x).encode())
SHA = lambda x: hashlib.sha256(x).hexdigest()
D = lambda domain, x: SHA((domain + '\n' + C(x)).encode())
clone = copy.deepcopy
A = seed['approvedContext']['assignmentIdentity']
W = dict(kind='workspace', workspaceId=A['workspaceId'])
U = dict(kind='run', workspaceId=A['workspaceId'], runId=A['runId'])
epoch = SHA(b'UNVERIFIED-terminal-store-lifetime')
profiles = {x['id']: x for x in seed['profiles']}
reports = {x['id']: json.loads(x['canonicalUtf8']) for x in seed['reports']}
sources = {x['reportEntryId']: json.loads(x['canonicalUtf8']) for x in seed['evidenceSources']}
requests = {x['id']: json.loads(x['canonicalUtf8']) for x in seed['requests']}
evaluation = json.loads(seed['evaluation']['canonicalUtf8'])
er = json.loads(seed['evaluationHead']['canonicalUtf8'])
rr = json.loads(seed['acceptedHead']['canonicalUtf8'])
M = json.loads(seed['materialLineage'][2]['canonicalUtf8'])['state']['material']


def row(table, key, body):
    return dict(table=table, key=clone(key), body=clone(body))


def identity(r, s):
    return dict(workspaceId=s['workspaceId'], **{k: r[k] for k in ['runId', 'planId', 'planRevision', 'taskId', 'reservationId', 'childSessionId', 'configurationDigest', 'sidecarGeneration', 'launchGeneration', 'reportId', 'reportVersion']})


def ref(kind, identity, digest):
    return dict(key=dict(kind=kind, identity=identity), digest=digest)


def report_ref(r, s):
    return ref('research-report', identity(r, s), r['contentDigest'])


def source_ref(source):
    return ref('assignment-evidence-source', source, D('orkworks.assignment-evidence-source.v1', source))


E = ref('assignment-evaluation', er, er['evaluationDigest'])
R = ref('assignment-result', rr, rr['resultDigest'])
head = lambda r: {k: r[k] for k in ['reportId', 'reportVersion', 'contentDigest']}
rid = lambda r: r['table'] + '\n' + C(r['key'])
sorted_rows = lambda rows: sorted(rows, key=lambda r: (r['table'].encode(), C(r['key']).encode()))


def projection(r):
    body = clone(r['body'])
    if r['table'] not in ['store_header', 'commit_proof']:
        body.pop('lastCommit', None)
    if r['table'] == 'summary_state' and body['checkpoint']:
        body['checkpoint'].pop('applied', None)
    if r['table'] == 'pending_material':
        body.pop('origin', None)
    return body


def rd(r):
    return D('orkworks.hierarchy-storage-row.v3', dict(table=r['table'], key=r['key'], body=projection(r)))


def get(rows, table):
    return next(r for r in rows if r['table'] == table)


def usage(rows):
    report = sum(B(r['key']) + B(projection(r)) for r in rows if r['table'] in ['report_heads', 'report_receipts'] or r['table'] == 'objects' and r['key']['kind'] == 'research-report')
    prep = sum(B(r['key']) + B(projection(r)) for r in rows if r['table'] == 'run_states')
    ss = get(rows, 'summary_state')['body']
    pending = [r for r in rows if r['table'] == 'pending_material']
    return {'report-retained-bytes': report, 'preparation-retained-bytes': prep,
            'summary-row-bytes': B(ss['row']) if ss['row'] else 0,
            'summary-row-count': int(ss['row'] is not None),
            'summary-lineage-bytes': (B(ss['checkpoint']['material']) if ss['checkpoint'] else 0) + sum(B(r['body']['before']) + B(r['body']['after']) for r in pending),
            'summary-pending-count': len(pending)}


def resources(rows):
    used = usage(rows)
    for r in rows:
        if r['table'] == 'quota_usage':
            r['body']['used'] = str(used[r['key']['resource']])


def commit(before, desired, kind='report-publication'):
    old = {rid(r): r for r in before}
    prior = get(before, 'store_header')['body']['latest'] if before else None
    sequence = str(int(prior['commitSequence']) + 1) if prior else '1'
    changed = [r for r in desired if rid(r) not in old or C(projection(r)) != C(projection(old[rid(r)]))]
    changes = [dict(table=r['table'], key=r['key'], beforeDigest=rd(old[rid(r)]) if rid(r) in old else None, afterDigest=rd(r)) for r in sorted_rows(changed)]
    manifest = dict(schemaVersion=3, workspaceId=A['workspaceId'], storageEpoch=epoch, commitSequence=sequence, predecessor=prior, commandKind=kind, changes=changes)
    link = dict(storageEpoch=epoch, commitSequence=sequence, changeDigest=D('orkworks.hierarchy-storage-change.v3', manifest))
    for r in changed:
        r['body']['lastCommit'] = link
        if r['table'] == 'summary_state' and r['body']['checkpoint'] and r['body']['checkpoint']['applied'] == 'NEW':
            r['body']['checkpoint']['applied'] = link
        if r['table'] == 'pending_material':
            r['body'].pop('lastCommit', None)
            r['body']['origin'] = link
    desired = [r for r in desired if r['table'] != 'store_header']
    desired += [row('store_header', 1, dict(schemaVersion=3, workspaceId=A['workspaceId'], storageEpoch=epoch, latest=link)),
                row('commit_proof', sequence, dict(body=manifest, changeDigest=link['changeDigest']))]
    desired = sorted_rows(desired)
    actual = [r for r in desired if rid(r) not in old or C(r['body']) != C(old[rid(r)]['body'])]
    return desired, dict(commit=link, changedRows=len(actual), commandBytes=sum(B(r['key']) + B(r['body']) for r in actual))


def receipt(req):
    return dict(schemaVersion=1, operation=req['operation'], expectedTaskVersion=req['expectedTaskVersion'], expectedRunVersion=req['expectedRunVersion'],
                expectedReportHead=req['expectedReportHead'], requestDigest=D('orkworks.research-report-request.v1', req),
                response=dict(schemaVersion=1, acceptedHead=head(req['report'])), lastCommit=None)


def baseline(mode='current', lifecycle='completed'):
    rows = []
    for k in ['c1', 'c2', 'o']:
        req = requests[k + '-v1']; report = req['report']; stream = req['stream']; source = sources[k + '-v1']
        rows += [row('objects', report_ref(report, stream)['key'], dict(body=report, digest=report['contentDigest'], lastCommit=None)),
                 row('objects', source_ref(source)['key'], dict(body=source, digest=source_ref(source)['digest'], lastCommit=None)),
                 row('report_heads', stream, dict(schemaVersion=1, head=head(report), lastCommit=None)),
                 row('report_receipts', dict(stream=stream, reportVersion=1), receipt(req)),
                 row('dependencies', dict(consumer=source_ref(source), target=report_ref(report, stream), purpose='evaluation-evidence'), dict(lastCommit=None)),
                 row('dependencies', dict(consumer=E, target=source_ref(source), purpose='evaluation-evidence'), dict(lastCommit=None))]
    for reference, value in [(R, json.loads(seed['result']['canonicalUtf8'])), (E, evaluation)]:
        rows.append(row('objects', reference['key'], dict(body=value, digest=reference['digest'], lastCommit=None)))
    rows += [row('heads', dict(kind='assignment-result', identity=A), dict(target=R, lastCommit=None)),
             row('heads', dict(kind='assignment-evaluation', identity={k: er[k] for k in ['assignmentIdentity', 'resultRevision', 'resultDigest', 'reviewerIdentity']}), dict(target=E, lastCommit=None))]
    terminal = dict(workspaceId=A['workspaceId'], runId=A['runId'], parentSessionId=A['parentSessionId'], admissionEpoch=A['runId'].split('-')[0], runSequence='1',
                    runDefinitionDigest=SHA(b'UNVERIFIED-terminal-definition'), bootstrapConfigurationDigest=A['bootstrapConfigurationDigest'], state=lifecycle, runVersion=4,
                    inputRevision=1, inputDigest=SHA(b'UNVERIFIED-terminal-input'), plans=[dict(planId='r2-research-plan', planRevision=1, planDefinitionDigest=SHA(b'UNVERIFIED-plan'))],
                    currentPlan=None, currentParentRuntimeGeneration=None, blockers=[], stageContinueRequired=False, pendingClarification=None)
    summary = dict(subjectRevision='10', row=dict(schemaVersion=1, assignmentIdentity=A, summaryRevision='2147483647' if mode == 'exhausted' else '3', state=dict(kind='current', material=M)),
                   checkpoint=dict(material=M, summaryRevision='2147483647' if mode == 'exhausted' else '3', applied='NEW'), admission='initialized', lastCommit=None)
    if mode == 'never-initialized':
        summary.update(row=None, checkpoint=None, admission='never-initialized')
    rows += [row('run_states', dict(workspaceId=A['workspaceId'], runId=A['runId']), dict(schemaVersion=1, terminal=terminal, lastCommit=None)), row('summary_state', A, summary)]
    for resource in ['report-retained-bytes', 'preparation-retained-bytes']:
        for owner in [U, W]:
            rows.append(row('quota_usage', dict(owner=owner, resource=resource), dict(used='0', reserved='0', lastCommit=None)))
    for resource in ['summary-row-bytes', 'summary-row-count', 'summary-lineage-bytes', 'summary-pending-count']:
        rows.append(row('quota_usage', dict(owner=W, resource=resource), dict(used='0', reserved='0', lastCommit=None)))
    resources(rows)
    # Hash-consistent synthetic captured proof, NOT a qualified past producer command.
    # Its complete creation/terminal lifecycle effects are outside this profile.
    return commit([], rows, 'subject-preparation')[0]


def publish(before, req, material, capacity=True):
    rows = clone(before)
    get(rows, 'run_states')['body']['terminal']['runVersion'] += 1
    for r in rows:
        if r['table'] == 'report_heads' and r['key'] == req['stream']:
            r['body']['head'] = head(req['report'])
    reference = report_ref(req['report'], req['stream'])
    rows += [row('objects', reference['key'], dict(body=req['report'], digest=reference['digest'], lastCommit=None)),
             row('report_receipts', dict(stream=req['stream'], reportVersion=req['report']['reportVersion']), receipt(req))]
    ss = get(rows, 'summary_state')['body']; oldss = get(before, 'summary_state')['body']
    # Both original v1 corrections change a selected source-currentness tuple.
    if req['report']['reportVersion'] == 2:
        ss['subjectRevision'] = str(int(ss['subjectRevision']) + 1)
    if ss['row'] and C(material) != C(oldss['checkpoint']['material']):
        if ss['row']['summaryRevision'] == '2147483647':
            ss['row']['state'] = dict(kind='unavailable', reason='exhausted')
        elif capacity:
            revision = str(int(ss['row']['summaryRevision']) + 1)
            ss['row'].update(summaryRevision=revision, state=dict(kind='current', material=material))
            ss['checkpoint'] = dict(material=material, summaryRevision=revision, applied='NEW')
        else:
            ss['row']['state'] = dict(kind='unavailable', reason='reconciling')
            rows.append(row('pending_material', dict(assignmentIdentity=A, assignedRevision='4'), dict(before=oldss['checkpoint']['material'], after=material, origin=None)))
    resources(rows)
    return commit(before, rows)


states = {k: baseline(k) for k in ['current', 'exhausted', 'never-initialized']}
states['cancelled'] = baseline(lifecycle='cancelled')
cases = []
for name, mode, request_id, capacity in [('required-C1', 'current', 'c1-v2', True), ('optional-O', 'current', 'o-v2', True),
                                       ('cancelled-C1', 'cancelled', 'c1-v2', True), ('reconciling-C1', 'current', 'c1-v2', False),
                                       ('exhausted-C1', 'exhausted', 'c1-v2', True), ('never-initialized-C1', 'never-initialized', 'c1-v2', True)]:
    req = requests[request_id]
    history = next(h for h in seed['histories'] if h['correctionRequestId'] == request_id)
    material = json.loads(history['response']['canonicalUtf8'])['detail']['material']
    after, measurement = publish(states[mode], req, material, capacity)
    before_digest = SHA(C(states[mode]).encode()); after_digest = SHA(C(after).encode())
    cases.append(dict(id=name, before=mode, snapshotVersion=7, request=req, currentRowCapacity=capacity, after=after, measurement=measurement,
                      oracle=dict(outcome='accepted', beforeStateDigest=before_digest, afterStateDigest=after_digest, commit=measurement['commit'], response=receipt(req)['response'])))
# Exact retry can refer to v2 after a separately captured v3 correction.
later = clone(requests['c1-v2']); later['expectedRunVersion'] = 5; later['expectedReportHead'] = head(later['report'])
later['report']['reportVersion'] = 3
body = {k: v for k, v in later['report'].items() if k != 'contentDigest'}
later['report']['contentDigest'] = D('orkworks.research-report.v1', body)
material = json.loads(seed['histories'][0]['response']['canonicalUtf8'])['detail']['material']
after_later, later_measure = publish(cases[0]['after'], later, material)
cases.append(dict(id='later-C1', before='required-C1', snapshotVersion=8, request=later, currentRowCapacity=True, after=after_later, measurement=later_measure,
                  oracle=dict(outcome='accepted', beforeStateDigest=SHA(C(cases[0]['after']).encode()), afterStateDigest=SHA(C(after_later).encode()), commit=later_measure['commit'], response=receipt(later)['response'])))

maximum_stream = dict(profiles['c1']['stream'])
for k in ['workspaceId', 'planId', 'taskId', 'reservationId', 'childSessionId']:
    maximum_stream[k] = 'i' * 128
maximum_stream.update(runId='a' * 64 + '-9223372036854775807', planRevision=64, configurationDigest='a' * 64)
maximum_head = dict(reportId='i' * 128, reportVersion=8, contentDigest='a' * 64)
maximum_link = dict(storageEpoch='a' * 64, commitSequence='18446744073709551615', changeDigest='a' * 64)
maximum_receipt = dict(schemaVersion=1, operation='correctResearchReport', expectedTaskVersion=None, expectedRunVersion=2147483647,
                       expectedReportHead=maximum_head, requestDigest='a' * 64, response=dict(schemaVersion=1, acceptedHead=maximum_head), lastCommit=maximum_link)
maximum_receipt_key = dict(stream=maximum_stream, reportVersion=8)
maximum_head_body = dict(schemaVersion=1, head=maximum_head, lastCommit=maximum_link)
boundaries = dict(status='valid field-width components, not a complete stream or accepted history',
                  receiptKey=maximum_receipt_key, receiptBody=maximum_receipt, receiptBytes=B(maximum_receipt_key)+B(maximum_receipt),
                  headBody=maximum_head_body, headBytes=B(maximum_head_body))
denials = []
for name, overrides, changed_req, outcome in [
    ('revoked', {'authority': False}, False, 'denied'), ('old-epoch', {'epochCurrent': False}, False, 'denied'),
    ('rate', {'rateAdmitted': False}, False, 'limit-exceeded'), ('source-proof', {'sourceScope': False}, False, 'unavailable'),
    ('unsupported-consumer', {'noUnsupportedConsumers': False}, False, 'unavailable'),
    ('command-budget', {'maxCommandBytes': 1}, False, 'limit-exceeded'), ('stale-CAS', {}, True, 'conflict')]:
    req = clone(requests['c1-v2'])
    if changed_req:
        req['expectedRunVersion'] = 3
    before_digest = SHA(C(states['current']).encode())
    denials.append(dict(id=name, before='current', request=req, contextOverrides=overrides,
                        oracle=dict(outcome=outcome, beforeStateDigest=before_digest, afterStateDigest=before_digest, commit=None, response=None)))

packet = dict(schemaVersion=1, evidenceKind='conditional-terminal-logical-publication-not-admission-authority-or-native-durability', seedSha256=SHA(raw_seed),
              unverified=['admission and terminal producer effects', 'authority/epoch/lease/rate and source-scope proofs', 'other-consumer absence and physical index completeness', 'native atomicity, snapshots, custody and runtime eligibility'],
              seedProofStatus='hash-consistent synthetic captured prior proof; no producer-command validity claim',
              capturedContext=dict(authority=True, epochCurrent=True, rateAdmitted=True, sourceScope=True, terminationSettled=True, noUnsupportedConsumers=True,
                                   snapshotVersion=7, ownerRuns=[dict(workspaceId=A['workspaceId'], runId=A['runId'])], nonReportPreparationBytes='0'),
              states=states, cases=cases, denials=denials, boundaries=boundaries)
parser = argparse.ArgumentParser(); parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args(); args.output.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
