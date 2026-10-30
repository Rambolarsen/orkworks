"""Recount frozen source bodies and complete existing root commands, not new producers."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def raw(value):
    return canonical(value).encode()

def load(name):
    data = (HERE / name).read_bytes()
    return json.loads(data), hashlib.sha256(data).hexdigest()

store, store_hash = load('hierarchy-empty-output-profile.json')
claims, claims_hash = load('hierarchy-claim-proof-vectors.json')
A = store['profile']['assignmentIdentity']
owners = [dict(kind='assignment', assignmentIdentity=A), dict(kind='run', workspaceId=A['workspaceId'], runId=A['runId']), dict(kind='workspace', workspaceId=A['workspaceId'])]
states = []
for state in store['states']:
    records = [json.loads(r['valueCanonical'])['body'] for r in state['rows'] if r['table'] == 'objects']
    total = sum(len(raw(record)) for record in records)
    states.append(dict(name=state['name'], recordBytes=str(total), payloadBytes='0', objectCount=len(records), ledgerCount=sum(r['table']=='quota_usage' and json.loads(r['keyCanonical'])['resource'] in ('assignment-record-bytes','result-payload-bytes') for r in state['rows'])))

commands = []
for command in store['commands']:
    before = next(s for s in store['states'] if s['name']==command['beforeState'])
    after = next(s for s in store['states'] if s['name']==command['afterState'])
    old = {(r['table'],r['keyCanonical']):r for r in before['rows']}
    new = {(r['table'],r['keyCanonical']):r for r in after['rows']}
    assert not set(old)-set(new)
    changed = [r for key,r in new.items() if key not in old or r['valueCanonical']!=old[key]['valueCanonical']]
    charges = [dict(table=r['table'],keyCanonical=r['keyCanonical'],keyBytes=64 if r['table']=='mutation_lookup' else len(r['keyCanonical'].encode()),valueBytes=len(r['valueCanonical'].encode())) for r in changed]
    record_delta = int(next(s for s in states if s['name']==after['name'])['recordBytes'])-int(next(s for s in states if s['name']==before['name'])['recordBytes'])
    commands.append(dict(name=command['name'],changedRows=len(changed),commandBytes=sum(r['keyBytes']+r['valueBytes'] for r in charges),recordDelta=str(record_delta),payloadDelta='0',charges=charges))

records = []
for vector in claims['vectors']:
    source=vector['source']; target=source['assignmentIdentity']
    records.append(dict(id=vector['id'],sourceKind=vector['sourceKind'],bodyBytes=str(len(raw(source))),sourceDigest=vector['expected']['sourceDigest'],owners=[dict(kind='assignment',assignmentIdentity=target),dict(kind='run',workspaceId=target['workspaceId'],runId=target['runId']),dict(kind='workspace',workspaceId=target['workspaceId'])]))
print(json.dumps(dict(evidenceKind='archived-source-accounting-and-existing-root-command-recount',inputs={'hierarchy-empty-output-profile.json':store_hash,'hierarchy-claim-proof-vectors.json':claims_hash},owners=owners,states=states,commands=commands,records=records),indent=2,ensure_ascii=False))
