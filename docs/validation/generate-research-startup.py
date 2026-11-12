"""Independent synthetic research configuration/startup bytes; no native I/O."""
import argparse
import base64
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BOUNDARY = ('Act only within the approved child assignment. Binding repository rules remain mandatory. '
            'Skills and task data grant no tools or authority. Use only current exact reporting and retained-input capabilities. '
            'Request changed scope from the user. Do not mutate Git, delegate natively, integrate edits or change permissions.')


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(value):
    return hashlib.sha256(value if isinstance(value, bytes) else value.encode('utf-8')).hexdigest()


def seal(value, field, domain):
    value.pop(field, None)
    value[field] = sha(domain + '\n' + canonical(value))
    return value


def render(c):
    blocks = [('boundary', 'child-v1', BOUNDARY)]
    blocks += [('rule', r['id'], r['content']) for r in c['rules']]
    blocks += [('role', c['roleTemplate']['id'], c['roleTemplate']['instructions'])]
    blocks += [('assignment', c['taskId'], canonical({k: c[k] for k in ['assignment', 'outputContract', 'reviewInput', 'rubric']}))]
    for requirement in ['mandatory', 'optional']:
        for skill in c['skills']:
            if skill['requirement'] == requirement:
                blocks.append(('skill', skill['id'], skill['content']))
                blocks += [('resource', r['id'], r['content']) for r in c['skillResources'] if r['skillId'] == skill['id']]
    return 'ORKWORKS CHILD INSTRUCTIONS v1\n' + ''.join(
        f'{kind} {identity} {len(content.encode("utf-8"))}\n{content}\n' for kind, identity, content in blocks)


def packet():
    # Only common synthetic leaves are reused; no old approval/control/history.
    old = json.loads((ROOT / 'child-source-vectors.json').read_bytes())
    seed = old['worker']['configuration']
    c = {k: copy.deepcopy(seed[k]) for k in ['repositoryId', 'repositoryBinding', 'sourceWorktreeBinding',
         'workspaceId', 'parentSessionId', 'rules', 'skills', 'skillResources', 'rubric']}
    c.update(schemaVersion=3, configurationId='research-configuration', planId='research-plan', planRevision=1,
             taskId='research-task', assignmentKind='child', taskCategory='retained-file-research', reviewInput=None,
             outputContract=dict(schemaVersion=1, declarations=[], maxSnapshotBytes=0, maxRehashBytes=0))
    role = dict(id='research-role', version=1, role='research',
                instructions='Answer only approved retained questions; disclose uncertainty. café 🛠\n',
                provenance=dict(schemaVersion=1, kind='nonlocal', sourceKind='user-approved',
                                sourceIdentity='synthetic-research-catalog', sourceRevision='fixture-v1'))
    role['instructionsDigest'] = sha(role['instructions'])
    role['contentDigest'] = sha(canonical(role))
    c['roleTemplate'] = role
    c['requirementManifests'] = sorted([dict(sourceId=s['id'], sourceDigest=s['contentDigest'], taskId=c['taskId'],
        requiredSkillIds=['retain-bytes'] if s is role else [], sourceLocations=['line:1'],
        applicabilityReason='Synthetic research source only.') for s in [*c['rules'], role]], key=lambda x: x['sourceId'])
    for skill in c['skills']:
        skill['requirementSources'] = [{k: m[k] for k in ['sourceId', 'sourceDigest', 'sourceLocations']}
                                      for m in c['requirementManifests'] if skill['id'] in m['requiredSkillIds']]
    c['assignment'] = dict(schemaVersion=1, label='Retained file research', description='Answer only the approved retained-file questions.',
        inputs=[], expectedOutput='One #742 ResearchReport for the complete approved scope; disclose uncertainty.',
        criteria=copy.deepcopy(seed['assignment']['criteria']), dependencies=[], worktreeScope=dict(readPaths=[], writePaths=[]),
        worktreeGroupId='research-group', baseCommit='b' * 40)
    task = dict(workspaceId=c['workspaceId'], runId=old['run']['runId'], planId=c['planId'], planRevision=1, taskId=c['taskId'])
    sample = json.loads((ROOT / 'research-capture-scope-components.json').read_bytes())
    captures, scope = sample['captures'], sample['scope']
    for source in [*captures, scope]:
        source.update(task=task, repositoryBindingDigest=old['run']['repositoryBindingDigest'],
                      sourceWorktreeBindingDigest=old['run']['sourceWorktreeBindingDigest'])
    for capture in captures:
        capture['revision'] = c['assignment']['baseCommit']
        seal(capture, 'captureDigest', 'orkworks.research-source-capture.v1')
    scope['captures'] = [dict(task=task, sourceId=s['sourceId'], captureDigest=s['captureDigest']) for s in captures]
    for excerpt in scope['excerpts']:
        excerpt['reference']['revision'] = c['assignment']['baseCommit']
    seal(scope, 'scopeDigest', 'orkworks.research-task-scope.v1')
    tools = sorted('orkworks.research-report.submit.v1 orkworks.research-report.receipt.read.v1 orkworks.research-report.context.read.v1'.split())
    profile = dict(tools=tools, readPaths=[], writePaths=[], commands=[], externalSources=[], connectors=[],
                   coordinationActions=[], nativeDelegation='denied', accessChanges='request-only')
    c['permissions'] = dict(requested=profile, effective=copy.deepcopy(profile))
    adapter = dict(harnessId='fixture-tool', adapterId='fixture-research-adapter', adapterVersion='fixture-v1')
    projection = dict(schemaVersion=1, kind='child-effective-configuration', sourceIdentity='research-settings',
                      sourceRevision='fixture-v1', bindings=[dict(name='fixture_scope', value='retained-research')])
    settings = dict(schemaVersion=1, kind='child-startup-settings', **adapter, argv=['--synthetic-only'], nonsecretBindings=[],
        configurationSources=[{k: projection[k] for k in ['sourceIdentity', 'sourceRevision']} | dict(contentDigest=sha(canonical(projection)))],
        discoveredInstructions=[dict(snapshotKind='rule', snapshotId=r['id'], contentDigest=r['contentDigest'], precedence=i) for i, r in enumerate(c['rules'])],
        deliveryOrder=[dict(kind='automatic', precedence=i) for i in range(len(c['rules']))] + [dict(kind='supplied'), dict(kind='research-input', scopeDigest=scope['scopeDigest'])],
        instructionMechanism='startup-context')
    policy = dict(schemaVersion=1, kind='child-model-policy', **adapter, policyId='fixture-pinned', mode='pinned',
                  modelId='fixture-model', settings=[], unknownModelAllowed=False)
    contents = [canonical(v) for v in [settings, projection, policy]] + ['Synthetic research evidence only; no native probe or real authority.']
    references = [dict(id=f'evidence-{i}', kind='fixture' if i < 3 else 'primary-reference',
                       sourceReference=f'synthetic:research-evidence-{i}', contentDigest=sha(content)) for i, content in enumerate(contents)]
    surfaces = sorted('instructions skills tools commands filesystem network connectors delegation startup model-policy'.split())
    evidence = seal(dict(schemaVersion=1, evidenceId='research-evidence', **adapter, definitionDigest=sha('Synthetic research harness definition.'),
        executableIdentity=copy.deepcopy(seed['harness']['executableIdentity']), toolVersion='fixture-v1', platform=copy.deepcopy(seed['harness']['platform']),
        instructionMechanism='startup-context', effectiveSettingsDigest=sha(contents[0]), role='research',
        requestedProfileDigest=sha(canonical(profile)), effectiveProfileDigest=sha(canonical(profile)), decision='unverified',
        checks=[dict(surface=s, result='unverified', reason='Synthetic research bytes; native delivery and operations unqualified.', referenceIds=['evidence-3']) for s in surfaces],
        references=references, assessedAt='2026-11-12T00:00:00Z', assessorIdentity='synthetic-research-fixture'), 'evidenceDigest', 'orkworks.capability-evidence.v1')
    c['capabilityEvidence'] = evidence
    c['harness'] = {k: evidence[k] for k in ['harnessId', 'definitionDigest', 'adapterId', 'adapterVersion', 'executableIdentity', 'toolVersion',
                                          'platform', 'instructionMechanism', 'effectiveSettingsDigest', 'evidenceId', 'evidenceDigest']}
    c['model'] = dict(schemaVersion=1, mode='pinned', modelId='fixture-model', policyId='fixture-pinned',
                      policyDigest=sha(contents[2]), adapterGeneration='fixture-research-generation')
    c['renderedInstructions'] = render(c)
    c['renderedInstructionsDigest'] = sha(c['renderedInstructions'])
    seal(c, 'configurationDigest', 'orkworks.assignment-configuration.v3')
    payload = canonical(dict(scope=scope, captures=captures))
    startup = ''.join(r['content'] for r in c['rules']) + c['renderedInstructions'] + '\nORKWORKS RETAINED RESEARCH INPUT v1\n' + str(len(payload.encode('utf-8'))) + '\n' + payload + '\n'
    return dict(context=dict(run=old['run'], groupId='research-group', baseCommit='b' * 40, objectFormat='sha1'), configuration=c,
                evidenceObjects=[dict(schemaVersion=1, task=task, referenceId=r['id'], contentBase64=base64.b64encode(content.encode('utf-8')).decode('ascii'),
                                      sizeBytes=len(content.encode('utf-8')), contentDigest=r['contentDigest']) for r, content in zip(references, contents)],
                captures=captures, scope=scope, declaredStartup=startup)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
