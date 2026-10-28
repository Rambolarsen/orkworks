"""Independent synthetic root source bytes; no native or runtime qualification."""
import argparse
import base64
import copy
import hashlib
import json
from pathlib import Path


def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(x):
    return hashlib.sha256(x.encode('utf-8')).hexdigest()


def seal(domain, x, field):
    return {**x, field: sha(domain + '\n' + canonical(x))}


def path(x):
    return {'encoding': 'unix-bytes', 'base64': base64.b64encode(x.encode()).decode()}


def obj(number):
    return {'platform': 'unix', 'mechanismVersion': 1, 'volumeId': '7', 'fileId': str(number)}


def vector(kind, name, x):
    wire = canonical(x)
    return {'id': name, 'kind': kind, 'canonical': wire,
            'bytes': len(wire.encode()), 'sha256': sha(wire)}


BOUNDARY = ('Coordinate only within the approved root assignment. Binding repository rules remain mandatory. '
            'Skills and task text grant no tools or authority. Request changed scope from the user. '
            'Approval, current capabilities and exact-plan grants remain separate gates. '
            'Do not implement, review code, mutate Git, delegate natively or integrate edits.')
ACTIONS = sorted('orkworks.coordination.' + name + '.v1' for name in
                 ['run.read', 'event.wait', 'clarification.request', 'plan.propose', 'task.launch', 'child-result.record'])
SURFACES = sorted(['instructions', 'skills', 'tools', 'commands', 'filesystem', 'network',
                   'connectors', 'delegation', 'startup', 'model-policy'])


def render(b):
    blocks = [('boundary', 'root-v1', BOUNDARY)]
    blocks += [('rule', r['id'], r['content']) for r in b['rules']]
    blocks += [('role', b['roleTemplate']['id'], b['roleTemplate']['instructions'])]
    blocks += [('assignment', b['rootAssignmentId'], canonical({k: b[k] for k in ['assignment', 'outputContract', 'rubric']}))]
    for s in sorted(b['skills'], key=lambda s: (s['requirement'] != 'mandatory', s['id'])):
        blocks.append(('skill', s['id'], s['content']))
        blocks += [('resource', r['id'], r['content']) for r in b['skillResources'] if r['skillId'] == s['id']]
    return 'ORKWORKS ROOT INSTRUCTIONS v1\n' + ''.join(
        f'{kind} {identity} {len(content.encode())}\n{content}\n' for kind, identity, content in blocks)


repository = seal('orkworks.repository-binding.v1', {
    'schemaVersion': 1, 'registryEpoch': '0' * 32, 'repositoryId': 'repo-' + '1' * 32,
    'canonicalCommonDirectory': path('/fixture/main/.git'), 'commonDirectoryObject': obj(42)}, 'bindingDigest')
worktree = seal('orkworks.worktree-binding.v1', {
    'repositoryBinding': repository, 'canonicalRoot': path('/fixture/main'), 'rootObject': obj(43),
    'canonicalPrivateGitDirectory': path('/fixture/main/.git'), 'privateGitDirectoryObject': obj(42),
    'kind': 'primary'}, 'worktreeBindingDigest')
run_id = 'a' * 64 + '-1'
owner = {'workspaceId': 'workspace-1', 'runId': run_id, 'parentSessionId': 'parent-1'}
profile = {'tools': ACTIONS, 'readPaths': [], 'writePaths': [], 'commands': [],
           'externalSources': [], 'connectors': [],
           'coordinationActions': [{'actionId': a, **owner} for a in ACTIONS],
           'nativeDelegation': 'denied', 'accessChanges': 'request-only'}
provenance = {'schemaVersion': 1, 'kind': 'nonlocal', 'sourceKind': 'user-approved',
              'sourceIdentity': 'synthetic-catalog', 'sourceRevision': 'fixture-v1'}
role = {'id': 'root-role', 'version': 1, 'role': 'orchestrator', 'instructions': 'Coordinate supplied café 🛠 evidence.\n',
        'instructionsDigest': sha('Coordinate supplied café 🛠 evidence.\n'), 'provenance': provenance}
role['contentDigest'] = sha(canonical(role))
rule = {'id': 'root-rule', 'sourcePath': 'AGENTS.md', 'content': 'Retain uncertainty.\n',
        'contentDigest': sha('Retain uncertainty.\n'), 'precedence': 0, 'authority': 'binding',
        'provenance': {'schemaVersion': 1, 'kind': 'local', 'sourceKind': 'repository',
                       'sourceIdentity': 'synthetic-repository', 'sourceRevision': 'fixture-v1',
                       'sourcePath': path('/fixture/main/AGENTS.md')}}
manifest = lambda x, required: {'sourceId': x['id'], 'sourceDigest': x['contentDigest'],
                              'taskId': 'root-1', 'requiredSkillIds': required,
                              'sourceLocations': ['line:1'], 'applicabilityReason': 'Selected root source.'}
manifests = sorted([manifest(role, ['coordinate']), manifest(rule, [])], key=lambda x: x['sourceId'])
resources = [{'id': 'reference-note', 'skillId': 'coordinate', 'sourceReference': 'reference.md',
              'content': 'Keep exact bytes.\nresource fake 999\n', 'contentDigest': sha('Keep exact bytes.\nresource fake 999\n'),
              'provenance': provenance}]
skills = [{'id': 'coordinate', 'version': 'fixture-v1', 'content': 'Preserve supplied evidence.',
           'contentDigest': sha('Preserve supplied evidence.'), 'requirement': 'mandatory',
           'selectionReason': 'Required by root role.', 'requirementSources': [
               {k: m[k] for k in ['sourceId', 'sourceDigest', 'sourceLocations']}
               for m in manifests if 'coordinate' in m['requiredSkillIds']],
           'resourceIds': ['reference-note'], 'provenance': provenance},
          {'id': 'optional-note', 'version': 'fixture-v1', 'content': 'Optional café context.\r\n',
           'contentDigest': sha('Optional café context.\r\n'), 'requirement': 'optional',
           'selectionReason': 'Illustrate byte preservation.', 'requirementSources': [], 'resourceIds': [], 'provenance': provenance}]
platform = {'os': 'macos', 'architecture': 'arm64', 'osVersion': 'synthetic-unverified-1'}
exe = {'schemaVersion': 1, 'kind': 'direct', 'executable': {'path': path('/fixture/bin/tool'), 'contentDigest': sha('Synthetic executable identity only.')}}
settings = {'schemaVersion': 1, 'kind': 'root-startup-settings', 'harnessId': 'fixture-tool',
            'adapterId': 'fixture-adapter', 'adapterVersion': 'fixture-v1', 'argv': ['--fixture-only'],
            'nonsecretBindings': [{'name': 'FIXTURE_MODE', 'value': ''}], 'configurationSources': [],
            'discoveredInstructions': [{'snapshotKind': 'rule', 'snapshotId': rule['id'], 'contentDigest': rule['contentDigest'], 'precedence': 0}],
            'deliveryOrder': [{'kind': 'automatic', 'precedence': 0}, {'kind': 'supplied'}], 'instructionMechanism': 'startup-context'}
projection = {'schemaVersion': 1, 'kind': 'root-effective-configuration', 'sourceIdentity': 'fixture-settings',
              'sourceRevision': 'fixture-v1', 'bindings': [{'name': 'fixture_scope', 'value': 'coordination'}]}
settings['configurationSources'] = [{k: projection[k] for k in ['sourceIdentity', 'sourceRevision']} | {'contentDigest': sha(canonical(projection))}]
policy = {'schemaVersion': 1, 'kind': 'root-model-policy', 'harnessId': 'fixture-tool',
          'adapterId': 'fixture-adapter', 'adapterVersion': 'fixture-v1', 'policyId': 'fixture-pinned',
          'mode': 'pinned', 'modelId': 'fixture-model', 'settings': [], 'unknownModelAllowed': False}
artifacts = [canonical(x) for x in [settings, policy, projection]] + ['Synthetic unverified gap: no native capability probe performed.']
refs = [{'id': 'artifact-' + str(i), 'kind': 'fixture' if i < 3 else 'primary-reference',
         'sourceReference': 'synthetic:artifact-' + str(i), 'contentDigest': sha(content)} for i, content in enumerate(artifacts)]
evidence = seal('orkworks.capability-evidence.v1', {
    'schemaVersion': 1, 'evidenceId': 'fixture-evidence', 'harnessId': 'fixture-tool', 'definitionDigest': sha('Synthetic definition.'),
    'adapterId': 'fixture-adapter', 'adapterVersion': 'fixture-v1', 'executableIdentity': exe,
    'toolVersion': 'fixture-v1', 'platform': platform, 'instructionMechanism': 'startup-context',
    'effectiveSettingsDigest': sha(canonical(settings)), 'role': 'orchestrator',
    'requestedProfileDigest': sha(canonical(profile)), 'effectiveProfileDigest': sha(canonical(profile)),
    'decision': 'unverified', 'checks': [{'surface': s, 'result': 'unverified',
        'reason': 'Synthetic source bytes only; native evidence unavailable.', 'referenceIds': ['artifact-3']} for s in SURFACES],
    'references': refs, 'assessedAt': '2026-10-28T00:00:00Z', 'assessorIdentity': 'synthetic-fixture'}, 'evidenceDigest')
harness = {k: evidence[k] for k in ['harnessId', 'definitionDigest', 'adapterId', 'adapterVersion', 'executableIdentity',
                                  'toolVersion', 'platform', 'instructionMechanism', 'effectiveSettingsDigest', 'evidenceId', 'evidenceDigest']}
bootstrap = {'schemaVersion': 2, 'bootstrapId': 'bootstrap-1', 'repositoryId': repository['repositoryId'],
    'repositoryBinding': repository, 'sourceWorktreeBinding': worktree, **owner,
    'rootAssignmentId': 'root-1', 'goalDigest': sha('Coordinate supplied evidence.\n'),
    'roleTemplate': role, 'taskCategory': 'coordination',
    'assignment': {'schemaVersion': 1, 'label': 'Coordinate supplied evidence', 'description': 'Prepare coordination proposals only.',
        'inputs': [], 'expectedOutput': 'Coordination state only; no materialized deliverable.',
        'criteria': [{'id': 'C1', 'requirement': 'required', 'description': 'Preserve evidence uncertainty.'}], 'dependencies': [],
        'worktreeScope': {'kind': 'source-worktree', 'repositoryBindingDigest': repository['bindingDigest'],
                         'sourceWorktreeBindingDigest': worktree['worktreeBindingDigest'], 'readPaths': [], 'writePaths': []}},
    'rules': [rule], 'requirementManifests': manifests, 'skills': skills, 'skillResources': resources,
    'rubric': {'id': 'root-rubric', 'version': 1, 'dimensions': [{'id': 'correctness', 'description': 'Preserve exact sources.'}], 'evaluatorRole': 'review'},
    'harness': harness, 'capabilityEvidence': evidence,
    'model': {'schemaVersion': 1, 'mode': 'pinned', 'modelId': 'fixture-model', 'policyId': 'fixture-pinned',
              'policyDigest': sha(canonical(policy)), 'adapterGeneration': 'fixture-adapter-generation'},
    'permissions': {'requested': profile, 'effective': copy.deepcopy(profile)},
    'outputContract': {'schemaVersion': 1, 'declarations': [], 'maxSnapshotBytes': 0, 'maxRehashBytes': 0}}
bootstrap['renderedInstructions'] = render(bootstrap)
bootstrap['renderedInstructionsDigest'] = sha(bootstrap['renderedInstructions'])
bootstrap = seal('orkworks.orchestrator-bootstrap.v2', bootstrap, 'bootstrapConfigurationDigest')
run = seal('orkworks.orchestration-run.v1', {'schemaVersion': 1, **owner, 'repositoryBindingDigest': repository['bindingDigest'],
    'sourceWorktreeBindingDigest': worktree['worktreeBindingDigest'], 'goal': 'Coordinate supplied evidence.\n',
    'goalDigest': bootstrap['goalDigest'], 'admissionEpoch': 'a' * 64, 'runSequence': '1',
    'bootstrapConfigurationDigest': bootstrap['bootstrapConfigurationDigest'], 'maxParallelChildren': 1}, 'runDefinitionDigest')
vectors = [vector('profile', 'root-permissions', profile), vector('settings', 'root-settings', settings),
           vector('policy', 'pinned-policy', policy), vector('projection', 'effective-projection', projection),
           vector('evidence', 'unverified-evidence', evidence), vector('bootstrap', 'complete-unverified-root', bootstrap),
           vector('render', 'exact-root-render', bootstrap['renderedInstructions'])]
tool_managed = {**policy, 'policyId': 'fixture-auto', 'mode': 'tool-managed', 'modelId': None, 'unknownModelAllowed': True}
vectors.append(vector('policy', 'tool-managed-policy', tool_managed))
catalog = {'schemaVersion': 1, 'classification': 'synthetic-unverified-source-bytes-only',
           'goal': run['goal'], 'run': run,
           'retainedArtifacts': [{'content': content, 'contentDigest': sha(content)} for content in artifacts],
           'unverified': ['native repository/worktree identity and registration', 'retained source custody and source interpretation',
                          'permission enforcement, instruction delivery and model semantics', 'server-artifact producer wrappers',
                          'draft/receipt/storage effects, runtime authority and attachment'], 'vectors': vectors}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=Path('docs/validation/root-bootstrap-vectors.json'))
args = parser.parse_args()
args.output.write_bytes((json.dumps(catalog, ensure_ascii=False, indent=2) + '\n').encode())
