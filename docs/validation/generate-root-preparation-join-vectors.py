"""Independent synthetic draft/approval/receipt source joins, never admission proof."""
import argparse
import copy
import hashlib
import json
from pathlib import Path


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(value):
    return hashlib.sha256(value.encode('utf-8')).hexdigest()


def seal(domain, value, field):
    return {**value, field: sha(domain + '\n' + canonical(value))}


def vector(kind, value, domain_digest=None):
    wire = canonical(value)
    entry = {'kind': kind, 'canonical': wire, 'utf8Bytes': len(wire.encode()), 'sha256': sha(wire)}
    if domain_digest is not None:
        entry['domainDigest'] = domain_digest
    return entry


base_path = Path('docs/validation/root-bootstrap-vectors.json')
base_bytes = base_path.read_bytes()
base = json.loads(base_bytes)
bootstrap = json.loads(next(v['canonical'] for v in base['vectors'] if v['kind'] == 'bootstrap'))
run = base['run']
draft = seal('orkworks.root-preparation-draft.v1', {
    'schemaVersion': 1, 'draftId': 'draft-join-1', 'workspaceId': bootstrap['workspaceId'],
    'runId': bootstrap['runId'], 'parentSessionId': bootstrap['parentSessionId'],
    'rootAssignmentId': bootstrap['rootAssignmentId'], 'sidecarGeneration': 'b' * 64,
    'reviewContextId': 'c' * 64, 'bootstrap': bootstrap, 'runDefinition': run}, 'draftDigest')
approval = {'schemaVersion': 1, 'action': 'orkworks.root-preparation.approve.v1',
            **{k: draft[k] for k in ['workspaceId', 'runId', 'draftId', 'draftDigest', 'reviewContextId']},
            'approvalId': 'approval-join-1'}
request_digest = sha('orkworks.root-preparation-request.v1\n' + canonical(approval))
identity = {'kind': 'root', **{k: bootstrap[k] for k in ['workspaceId', 'runId', 'rootAssignmentId',
            'parentSessionId', 'bootstrapId', 'bootstrapConfigurationDigest']}}
# These structurally valid links do not reference real manifests or a store.
# Their digests label synthetic gaps rather than inventing transaction proof.
draft_commit = {'storageEpoch': 'd' * 64, 'commitSequence': '1',
                'changeDigest': sha('Synthetic draft commit link; no retained manifest proof.')}
source_commit = {**draft_commit, 'commitSequence': '2',
                 'changeDigest': sha('Synthetic source commit link; no retained manifest proof.')}
receipt = {'schemaVersion': 1, **{k: draft[k] for k in ['workspaceId', 'runId', 'draftDigest']},
           'rootAssignmentIdentity': identity, 'bootstrapConfigurationDigest': bootstrap['bootstrapConfigurationDigest'],
           'runDefinitionDigest': run['runDefinitionDigest'], 'approvalId': approval['approvalId'],
           'requestDigest': request_digest, 'commit': source_commit}
source = {'schemaVersion': 1, 'bootstrap': bootstrap, 'runDefinition': run,
          'approval': {**{k: approval[k] for k in ['approvalId', 'draftId', 'draftDigest', 'reviewContextId']},
                       'sidecarGeneration': draft['sidecarGeneration'], 'requestDigest': request_digest},
          'receipt': receipt, 'lastCommit': source_commit}
draft_row = {'schemaVersion': 1, 'draft': draft, 'lastCommit': draft_commit}
source_key = {k: bootstrap[k] for k in ['workspaceId', 'runId']}


def row_digest(table, key, value):
    body = copy.deepcopy(value)
    del body['lastCommit']
    if table == 'root_preparations':
        del body['receipt']['commit']
    envelope = {'table': table, 'key': key, 'body': body}
    return sha('orkworks.hierarchy-storage-row.v3\n' + canonical(envelope))


vectors = [vector('draft', draft, draft['draftDigest']), vector('approval', approval, request_digest),
           vector('receipt', receipt), vector('source', source),
           vector('draft-row', draft_row, row_digest('root_drafts', bootstrap['workspaceId'], draft_row)),
           vector('source-row', source, row_digest('root_preparations', source_key, source))]
catalog = {'schemaVersion': 1, 'classification': 'synthetic-preparation-joins-only',
           'bootstrapCatalogSha256': hashlib.sha256(base_bytes).hexdigest(),
           'frozenDraftSha256': sha(canonical(draft)), 'sourceKey': source_key,
           'unverified': ['native identity and source custody', 'capability eligibility and instruction delivery',
                          'current UI authority, review context and durable admission fence',
                          'commit manifests, complete changed rows and aggregate accounting',
                          'summary initialization, parent attachment and runtime authority'],
           'vectors': vectors}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=Path('docs/validation/root-preparation-join-vectors.json'))
args = parser.parse_args()
args.output.write_bytes((json.dumps(catalog, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
