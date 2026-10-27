"""Independent stdlib golden values; synthetic components, never native evidence."""
import argparse
import base64
import hashlib
import json
from pathlib import Path

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def sha(value):
    return hashlib.sha256(value.encode('utf-8')).hexdigest()

def native(value, encoding='unix-bytes'):
    codec = 'utf-8' if encoding == 'unix-bytes' else 'utf-16le'
    return {'encoding': encoding, 'base64': base64.b64encode(value.encode(codec)).decode()}

def vector(kind, name, value):
    wire = canonical(value)
    return {'id': name, 'kind': kind, 'canonical': wire,
            'bytes': len(wire.encode('utf-8')), 'sha256': sha(wire)}

run = json.loads(Path('docs/validation/root-preparation-component-vectors.json').read_text())['vectors'][0]
run = json.loads(run['runCanonical'])
bindings = {'run': run, 'rootAssignmentId': 'root-source-fixture'}
scope = {'kind': 'source-worktree', 'repositoryBindingDigest': run['repositoryBindingDigest'],
         'sourceWorktreeBindingDigest': run['sourceWorktreeBindingDigest'],
         'readPaths': ['docs/spec.md'], 'writePaths': []}
assignment = {'schemaVersion': 1, 'label': 'Coordinate café 🛠', 'description': 'Review supplied evidence.\n',
              'inputs': [{'kind': 'inline', 'id': 'goal-note', 'content': 'Exact café 🛠\n',
                          'contentDigest': sha('Exact café 🛠\n')}],
              'expectedOutput': 'A bounded proposal.',
              'criteria': [{'id': 'C1', 'requirement': 'required', 'description': 'Retain uncertainty.'}],
              'dependencies': [], 'worktreeScope': scope}
input_value = {'schemaVersion': 1, 'workspaceId': run['workspaceId'], 'runId': run['runId'],
               'preparationRevision': 1, 'inputRevision': 1, 'goalDigest': run['goalDigest'],
               'bootstrapConfigurationDigest': run['bootstrapConfigurationDigest'],
               'decision': None, 'records': []}
input_digest = sha('orkworks.preparation-input.v1\n' + canonical(input_value))
state = {'schemaVersion': 1, 'workspaceId': run['workspaceId'], 'runId': run['runId'],
         'parentSessionId': run['parentSessionId'], 'runDefinitionDigest': run['runDefinitionDigest'],
         'bootstrapConfigurationDigest': run['bootstrapConfigurationDigest'], 'runVersion': 1,
         'state': 'assessing', 'preparationRevision': 1, 'inputRevision': 1,
         'inputDigest': input_digest, 'planReferences': [], 'currentPlan': None,
         'parentRuntime': None, 'blockers': [], 'stageContinueRequired': False,
         'pendingClarification': None, 'pauseReason': None, 'resumeTarget': None}
reserved = {'schemaVersion': 1, 'kind': 'reserved', 'workspaceId': run['workspaceId'],
            'runId': run['runId'], 'parentSessionId': run['parentSessionId'],
            'rootAssignmentId': bindings['rootAssignmentId'],
            'bootstrapConfigurationDigest': run['bootstrapConfigurationDigest']}
attached = {**reserved, 'kind': 'attached', 'sidecarGeneration': 'a' * 64, 'launchGeneration': 'b' * 64}
provenance = {'schemaVersion': 1, 'kind': 'local', 'sourceKind': 'repository',
              'sourceIdentity': 'fixture-catalog', 'sourceRevision': 'fixture-rev-1',
              'sourcePath': native('/fixture/repo/AGENTS.md')}
exe = {'path': native('/fixture/bin/tool'), 'contentDigest': 'c' * 64}
wrapped = {'schemaVersion': 1, 'kind': 'wrapped', 'executable': exe,
           'components': [{'kind': kind, 'path': native('/fixture/' + kind), 'contentDigest': digit * 64}
                          for kind, digit in [('interpreter', 'd'), ('wrapper', 'e'),
                                              ('package-manifest', 'f'), ('package-file', '0')]]}
reference_assignment = json.loads(json.dumps(assignment))
reference_assignment['inputs'] = [{'kind': 'reference', 'id': 'retained', 'sourceIdentity': 'catalog',
                                  'sourceRevision': 'v1', 'reference': 'reviewed-note', 'contentDigest': '1' * 64}]
vectors = [vector('assignment', 'inline-unicode', assignment),
           vector('assignment', 'reference-unbound', reference_assignment),
           vector('provenance', 'local-native', provenance),
           vector('provenance', 'nonlocal', {k: v for k, v in {**provenance, 'kind': 'nonlocal', 'sourceKind': 'bundled'}.items() if k != 'sourcePath'}),
           vector('provenance', 'windows-native', {**provenance, 'sourcePath': native('C:\\fixture\\AGENTS.md', 'windows-utf16le')}),
           vector('executable', 'direct', {'schemaVersion': 1, 'kind': 'direct', 'executable': exe}),
           vector('executable', 'wrapped', wrapped),
           vector('input', 'initial-input', input_value),
           vector('state', 'initial-state', state),
           vector('attachment', 'reserved', reserved), vector('attachment', 'attached', attached)]
catalog = {'schemaVersion': 1, 'classification': 'synthetic-component-only',
           'bindings': bindings, 'initialInput': input_value, 'inputDigest': input_digest,
           'unverified': ['native paths and executable contents', 'source and reference custody',
                          'permission enforcement and adapter eligibility', 'session attachment proof',
                          'atomic storage effects and runtime authority'], 'vectors': vectors}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=Path('docs/validation/root-preparation-source-vectors.json'))
args = parser.parse_args()
args.output.write_bytes((json.dumps(catalog, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
