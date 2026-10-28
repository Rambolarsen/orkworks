// Independent golden bytes plus adversarial source mutations. No runtime calls.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {decodeSourceComponent} from './root-preparation-source-components.mjs';
import {validateRootBootstrap, validateRootComponent, renderRootInstructions} from './root-bootstrap-components.mjs';

const catalog = JSON.parse(readFileSync(new URL('./root-bootstrap-vectors.json', import.meta.url)));
const get = name => JSON.parse(catalog.vectors.find(v => v.id === name).canonical);
const context = () => ({bootstrap: get('complete-unverified-root'), run: structuredClone(catalog.run),
  goal: catalog.goal, retainedArtifacts: structuredClone(catalog.retainedArtifacts)});
const sha = x => createHash('sha256').update(x).digest('hex');
for (const v of catalog.vectors) test(`${v.id}: independent Python bytes and hash`, () => {
  const x = decodeSourceComponent(Buffer.from(v.canonical));
  assert.equal(Buffer.byteLength(v.canonical), v.bytes);
  if (v.kind === 'render') assert.equal(renderRootInstructions(get('complete-unverified-root')), x);
  else assert.equal(validateRootComponent(v.kind, x, context()), v.sha256);
});
test('complete root joins the validated run and remains ineligible', () => {
  assert.deepEqual(validateRootBootstrap(get('complete-unverified-root'), context()), {
    bootstrapConfigurationDigest: catalog.run.bootstrapConfigurationDigest, classification: 'unverified-source-bytes-only'});
});

for (const [name, mutate, message] of [
  ['missing coordination action', x => x.permissions.requested.coordinationActions.pop(), /coordination/],
  ['foreign run action', x => x.permissions.effective.coordinationActions[0].runId = 'foreign', /action owner/],
  ['missing tool pairing', x => x.permissions.requested.tools.pop(), /tool pairing/],
  ['general shell tool', x => x.permissions.effective.tools.push('shell'), /tool pairing/],
  ['unpaired result tools', x => x.permissions.effective.tools.push('orkworks.result.submit.v1'), /result action pair/],
  ['live repository reads', x => x.permissions.effective.readPaths.push('src'), /empty root access/],
  ['commands', x => x.permissions.requested.commands.push({}), /empty root access/],
  ['external sources', x => x.permissions.requested.externalSources.push({}), /empty root access/],
  ['connectors', x => x.permissions.requested.connectors.push({}), /empty root access/],
  ['native delegation', x => x.permissions.requested.nativeDelegation = 'allowed', /native delegation/],
  ['scope read path', x => x.assignment.worktreeScope.readPaths.push('docs'), /empty root scope/],
  ['path output', x => x.outputContract.declarations = [{id:'o',kind:'report',maxBytes:1,source:{kind:'workspace-path',path:'out'}}], /root server artifact/],
  ['root evaluation output', x => x.outputContract.declarations = [{id:'o',kind:'assignment_evaluation',maxBytes:1,source:{kind:'server-artifact'}}], /root evaluation/],
  ['nonzero empty output cap', x => x.outputContract.maxSnapshotBytes = 1, /empty output caps/],
  ['missing manifest', x => x.requirementManifests.pop(), /manifest coverage/],
  ['wrong manifest source', x => x.requirementManifests[0].sourceDigest = '0'.repeat(64), /manifest source/],
  ['wrong manifest task', x => x.requirementManifests[0].taskId = 'other', /manifest task/],
  ['missing mandatory contributor', x => x.skills[0].requirementSources = [], /requirement source projection/],
  ['optional contributor', x => x.skills[1].requirementSources = x.skills[0].requirementSources, /optional sources/],
  ['unselected required skill', x => x.requirementManifests[0].requiredSkillIds = ['absent'], /mandatory skill union/],
  ['resource content drift', x => x.skillResources[0].content += 'x', /snapshot content digest/],
  ['resource ownership', x => x.skillResources[0].skillId = 'absent', /resource ownership/],
  ['role instructions drift', x => x.roleTemplate.instructions += 'x', /role instructions digest/],
  ['platform differs', x => x.harness.platform.architecture = 'x64', /shared harness platform/],
  ['executable differs', x => x.harness.executableIdentity.executable.contentDigest = '0'.repeat(64), /shared harness executableIdentity/],
  ['settings hash differs', x => x.harness.effectiveSettingsDigest = '0'.repeat(64), /shared harness effectiveSettingsDigest/],
  ['profile hash differs', x => x.capabilityEvidence.requestedProfileDigest = '0'.repeat(64), /requested profile digest/],
  ['evidence hash differs', x => x.capabilityEvidence.evidenceDigest = '0'.repeat(64), /evidence digest/],
  ['synthetic verified decision', x => x.capabilityEvidence.decision = 'verified', /unverified slice/],
  ['missing evidence surface', x => x.capabilityEvidence.checks.pop(), /ten surfaces/],
  ['dangling reference', x => x.capabilityEvidence.checks[0].referenceIds = ['absent'], /evidence reference/],
  ['missing reason', x => x.capabilityEvidence.checks[0].reason = '', /bounded UTF-8/],
  ['invalid calendar date', x => x.capabilityEvidence.assessedAt = '2026-02-30T00:00:00Z', /UTC Gregorian/],
  ['fractional time', x => x.capabilityEvidence.assessedAt = '2026-10-28T00:00:00.1Z', /UTC Gregorian/],
  ['model policy differs', x => x.model.policyId = 'other', /model policy binding/],
  ['render drift', x => x.renderedInstructions += 'x', /rendered bytes/],
  ['render digest drift', x => x.renderedInstructionsDigest = '0'.repeat(64), /rendered digest/],
  ['bootstrap hash drift', x => x.bootstrapConfigurationDigest = '0'.repeat(64), /bootstrap digest/],
  ['repository descriptor drift', x => x.repositoryBinding.commonDirectoryObject.fileId = '99', /repository digest/],
  ['source worktree differs', x => x.sourceWorktreeBinding.rootObject.fileId = '99', /worktree digest/],
  ['unknown bootstrap field', x => x.grant = null, /closed fields/],
  ['run identity drift', x => x.runId = 'other', /action owner/],
]) test(`rejects ${name}`, () => {
  const x = get('complete-unverified-root'); mutate(x);
  assert.throws(() => validateRootBootstrap(x, context()), message);
});

for (const [name, mutate, message] of [
  ['missing retained bytes', c => c.retainedArtifacts.pop(), /retained artifact/],
  ['retained byte drift', c => c.retainedArtifacts[0].content += 'x', /artifact content digest/],
  ['wrong goal bytes', c => c.goal += 'x', /goal digest/],
  ['different valid run', c => c.run.parentSessionId = 'other', /runDefinitionDigest/],
]) test(`rejects ${name}`, () => {
  const c = context(); mutate(c);
  assert.throws(() => validateRootBootstrap(get('complete-unverified-root'), c), message);
});

for (const [name, mutate, message] of [
  ['unknown setting field', x => x.secret = '', /closed fields/],
  ['split supplied delivery', x => x.deliveryOrder.push({kind:'supplied'}), /one supplied payload/],
  ['missing automatic delivery', x => x.deliveryOrder.shift(), /automatic delivery/],
  ['unknown discovery', x => x.discoveredInstructions[0].snapshotId = 'absent', /automatic snapshot/],
  ['noncontiguous precedence', x => x.discoveredInstructions[0].precedence = 1, /discovery precedence/],
  ['oversized argv entry', x => x.argv[0] = 'é'.repeat(1025), /bounded UTF-8/],
  ['binding duplicate', x => x.nonsecretBindings.push({...x.nonsecretBindings[0]}), /unique sorted/],
  ['missing configuration projection', x => x.configurationSources[0].contentDigest = '0'.repeat(64), /retained artifact/],
]) test(`settings rejects ${name}`, () => {
  const x = get('root-settings'); mutate(x);
  assert.throws(() => validateRootComponent('settings', x, context()), message);
});

test('pinned model cannot permit unknown visibility', () => {
  const x = get('pinned-policy'); x.unknownModelAllowed = true;
  assert.throws(() => validateRootComponent('policy', x, context()), /pinned policy/);
});
test('tool-managed model ID must be null', () => {
  const x = get('tool-managed-policy'); x.modelId = 'invented';
  assert.throws(() => validateRootComponent('policy', x, context()), /tool-managed policy/);
});
test('render keeps CRLF, decomposed Unicode and header-like resource content', () => {
  const b = get('complete-unverified-root');
  const wire = renderRootInstructions(b);
  assert(wire.includes('Optional café context.\r\n'));
  assert(wire.includes('resource fake 999\n'));
  b.skills[1].content = b.skills[1].content.normalize('NFC');
  assert.notEqual(renderRootInstructions(b), wire);
});
test('stored bytes reject duplicate keys and malformed Unicode before hashing', () => {
  for (const wire of [Buffer.from('{"x":1,"x":1}'), Buffer.from('"\\ud800"'), Buffer.from([34,255,34])])
    assert.throws(() => decodeSourceComponent(wire));
});
test('rendering accounts for framing overhead at the inclusive context limit', () => {
  const b = get('complete-unverified-root');
  const original = renderRootInstructions(b);
  b.roleTemplate.instructions = 'x'.repeat(262144);
  assert.throws(() => renderRootInstructions(b), /render context ceiling/);
  assert(Buffer.byteLength(original) < 262144);
});


// Recompute changed source domains so boundary tests reach the owning limit,
// rather than merely failing an unrelated stale fingerprint.
function canonicalMutation(x) {
  if (Array.isArray(x)) return '[' + x.map(canonicalMutation).join(',') + ']';
  if (x && typeof x === 'object') return '{' + Object.keys(x).sort().map(k => JSON.stringify(k) + ':' + canonicalMutation(x[k])).join(',') + '}';
  return JSON.stringify(x);
}
function reseal(x, field, domain) {
  const body = {...x}; delete body[field];
  x[field] = sha(domain + '\n' + canonicalMutation(body));
}
function rebindSource(b, c) {
  reseal(b.repositoryBinding, 'bindingDigest', 'orkworks.repository-binding.v1');
  b.sourceWorktreeBinding.repositoryBinding = structuredClone(b.repositoryBinding);
  reseal(b.sourceWorktreeBinding, 'worktreeBindingDigest', 'orkworks.worktree-binding.v1');
  b.assignment.worktreeScope.repositoryBindingDigest = b.repositoryBinding.bindingDigest;
  b.assignment.worktreeScope.sourceWorktreeBindingDigest = b.sourceWorktreeBinding.worktreeBindingDigest;
  b.renderedInstructions = renderRootInstructions(b); b.renderedInstructionsDigest = sha(b.renderedInstructions);
  reseal(b, 'bootstrapConfigurationDigest', 'orkworks.orchestrator-bootstrap.v2');
  Object.assign(c.run, {repositoryBindingDigest: b.repositoryBinding.bindingDigest,
    sourceWorktreeBindingDigest: b.sourceWorktreeBinding.worktreeBindingDigest,
    bootstrapConfigurationDigest: b.bootstrapConfigurationDigest});
  reseal(c.run, 'runDefinitionDigest', 'orkworks.orchestration-run.v1');
}
test('repository descriptor retains its own 4096-byte native path bound', () => {
  const b = get('complete-unverified-root'), c = context();
  b.repositoryBinding.canonicalCommonDirectory.base64 = Buffer.from('/' + 'a'.repeat(4095)).toString('base64');
  rebindSource(b, c);
  assert.doesNotThrow(() => validateRootBootstrap(b, c));
  b.repositoryBinding.canonicalCommonDirectory.base64 = Buffer.from('/' + 'a'.repeat(4096)).toString('base64');
  rebindSource(b, c);
  assert.throws(() => validateRootBootstrap(b, c), /native path limit/);
});
test('new provenance source paths retain their smaller 2048-byte limit', () => {
  const b = get('complete-unverified-root');
  b.rules[0].provenance.sourcePath.base64 = Buffer.from('/' + 'a'.repeat(2047)).toString('base64');
  // Test component validation directly: changing provenance also changes rendering.
  const c = context(); c.bootstrap = b;
  b.renderedInstructions = renderRootInstructions(b); b.renderedInstructionsDigest = sha(b.renderedInstructions);
  reseal(b, 'bootstrapConfigurationDigest', 'orkworks.orchestrator-bootstrap.v2');
  c.run.bootstrapConfigurationDigest = b.bootstrapConfigurationDigest;
  reseal(c.run, 'runDefinitionDigest', 'orkworks.orchestration-run.v1');
  assert.doesNotThrow(() => validateRootBootstrap(b, c));
  b.rules[0].provenance.sourcePath.base64 = Buffer.from('/' + 'a'.repeat(2048)).toString('base64');
  assert.throws(() => validateRootBootstrap(b, c), /native path bytes/);
});
test('standalone evidence rejects malformed executable identity even with fresh digest', () => {
  const x = get('unverified-evidence'); x.executableIdentity.executable.path.base64 = '!!!';
  reseal(x, 'evidenceDigest', 'orkworks.capability-evidence.v1');
  assert.throws(() => validateRootComponent('evidence', x, context()), /canonical padded base64/);
});
test('configuration projection may use either retained reference kind', () => {
  const c = context(); c.bootstrap.capabilityEvidence.references[2].kind = 'primary-reference';
  assert.doesNotThrow(() => validateRootComponent('settings', get('root-settings'), c));
});
test('startup argv count accepts 64 and rejects 65 without truncation', () => {
  const x = get('root-settings'); x.argv = Array(64).fill('');
  assert.doesNotThrow(() => validateRootComponent('settings', x, context()));
  x.argv.push(''); assert.throws(() => validateRootComponent('settings', x, context()), /bounded array/);
});
test('artifact bound includes configuration JSON overhead', () => {
  const x = get('effective-projection');
  x.bindings = Array.from({length:32}, (_, i) => ({name:'binding_' + String(i).padStart(2,'0'), value:'x'.repeat(2048)}));
  assert.throws(() => validateRootComponent('projection', x, context()), /artifact 64 KiB/);
});
