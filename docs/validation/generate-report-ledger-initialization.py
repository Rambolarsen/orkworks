"""Independent offline report-ledger creation witnesses; no native/store access."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
BASE_BYTES = (HERE / 'root-preparation-effects.json').read_bytes()
BASE = json.loads(BASE_BYTES)


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'))


def sha(value):
    return hashlib.sha256(value.encode('utf-8')).hexdigest()


def domain(name, value):
    return sha(name + '\n' + canonical(value))


def row_id(row):
    return row['table'] + '\n' + canonical(row['key'])


def row_digest(row):
    body = copy.deepcopy(row['value'])
    if row['table'] not in ('store_header', 'commit_proof'):
        body.pop('lastCommit')
    if row['table'] == 'root_preparations':
        body['receipt'].pop('commit')
    if row['table'] == 'summary_state' and body['checkpoint'] is not None:
        body['checkpoint'].pop('applied')
    return domain('orkworks.hierarchy-storage-row.v3',
                  {'table': row['table'], 'key': row['key'], 'body': body})


def extend(state):
    rows = [{'table': r['table'], 'key': json.loads(r['keyCanonical']),
             'value': json.loads(r['valueCanonical'])} for r in state['rows']]
    extras = []
    header = next((r for r in rows if r['table'] == 'store_header'), None)
    if header:
        owners = [({'kind': 'workspace', 'workspaceId': header['value']['workspaceId']}, '1')]
        for r in rows:
            if r['table'] == 'root_preparations':
                b = r['value']['bootstrap']
                owners.append(({'kind': 'run', 'workspaceId': b['workspaceId'], 'runId': b['runId']},
                               r['value']['receipt']['commit']['commitSequence']))
        for owner, sequence in owners:
            extras.append((sequence, {'table': 'quota_usage',
                'key': {'owner': owner, 'resource': 'report-retained-bytes'},
                'value': {'used': '0', 'reserved': '0', 'lastCommit': None}}))
    proofs = {}
    refs = {}
    for r in sorted((r for r in rows if r['table'] == 'commit_proof'), key=lambda r: int(r['key'])):
        manifest = copy.deepcopy(r['value']['body'])
        seq = manifest['commitSequence']
        if manifest['predecessor'] is not None:
            manifest['predecessor'] = refs[manifest['predecessor']['commitSequence']]
        for created, extra in extras:
            if created == seq:
                manifest['changes'].append({'table': extra['table'], 'key': extra['key'],
                    'beforeDigest': None, 'afterDigest': row_digest(extra)})
        manifest['changes'].sort(key=lambda c: c['table'] + '\n' + canonical(c['key']))
        digest = domain('orkworks.hierarchy-storage-change.v3', manifest)
        proofs[seq] = {'body': manifest, 'changeDigest': digest}
        refs[seq] = {'storageEpoch': manifest['storageEpoch'], 'commitSequence': seq, 'changeDigest': digest}
    for r in rows:
        v = r['value']
        if r['table'] == 'commit_proof':
            r['value'] = proofs[r['key']]
        elif r['table'] == 'store_header':
            v['latest'] = refs[v['latest']['commitSequence']]
        else:
            v['lastCommit'] = refs[v['lastCommit']['commitSequence']]
            if r['table'] == 'root_preparations':
                v['receipt']['commit'] = refs[v['receipt']['commit']['commitSequence']]
            if r['table'] == 'summary_state' and v['checkpoint'] is not None:
                v['checkpoint']['applied'] = refs[v['checkpoint']['applied']['commitSequence']]
    for seq, r in extras:
        r['value']['lastCommit'] = refs[seq]
        rows.append(r)
    encoded = [{'table': r['table'], 'keyCanonical': canonical(r['key']),
                'valueCanonical': canonical(r['value']), 'rowDigest': row_digest(r)}
               for r in sorted(rows, key=row_id)]
    return {'name': state['name'], 'rows': encoded, 'stateSha256': sha(canonical(encoded))}


states = [extend(s) for s in BASE['states']]
by_name = {s['name']: s for s in states}
commands = []
for original in BASE['commands']:
    c = copy.deepcopy(original)
    before, after = by_name[c['beforeState']], by_name[c['afterState']]
    old = {(r['table'], r['keyCanonical']): r for r in before['rows']}
    new = {(r['table'], r['keyCanonical']): r for r in after['rows']}
    changed = [k for k in old.keys() | new.keys() if old.get(k) != new.get(k)]
    c['changedRows'] = len(changed)
    c['commandBytes'] = sum(len(k[1].encode('utf-8')) +
        (len(new[k]['valueCanonical'].encode('utf-8')) if k in new else 0) for k in changed)
    header = next(json.loads(r['valueCanonical']) for r in after['rows'] if r['table'] == 'store_header')
    seq = header['latest']['commitSequence']
    proof = next(json.loads(r['valueCanonical']) for r in after['rows']
                 if r['table'] == 'commit_proof' and json.loads(r['keyCanonical']) == seq)
    c['manifestCanonical'] = canonical(proof['body'])
    c['changeDigest'] = proof['changeDigest']
    commands.append(c)

packet = {'schemaVersion': 1, 'classification': 'offline-fresh-report-ledger-creation-only',
          'baselineSha256': hashlib.sha256(BASE_BYTES).hexdigest(), 'states': states, 'commands': commands,
          'unverified': ['real admission and report/terminal producers', 'complete consumers and physical indexes',
                         'UI authority, lease, storage/native durability and executable plans']}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.write_bytes((json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
