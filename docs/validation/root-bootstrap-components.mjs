// Documentation-only source-byte checker. Never import into production.
// No authority, native identity, source interpretation or custody qualification.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSourceComponent, validateSourceComponent, validateExecutableBinding} from './root-preparation-source-components.mjs';
import {validateRun} from './root-preparation-components.mjs';

const hash = x => createHash('sha256').update(x).digest('hex');
const order = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const exact = (x, fields) => {
  assert(object(x), 'record object');
  assert.deepEqual(Object.keys(x).sort(), fields.split(' ').sort(), 'closed fields');
};
const text = (x, max, min = 1) => assert(typeof x === 'string' && x.isWellFormed() && Buffer.byteLength(x) >= min && Buffer.byteLength(x) <= max, 'bounded UTF-8');
const id = x => assert(typeof x === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(x), 'ASCII ID');
const digest = x => assert(typeof x === 'string' && /^[a-f0-9]{64}$/.test(x), 'SHA-256');
const tag = (x, choices) => assert(choices.includes(x), 'supported tag');
const integer = (x, min, max) => assert(Number.isSafeInteger(x) && !Object.is(x, -0) && x >= min && x <= max, 'bounded integer');
const list = (x, max, min = 0) => assert(Array.isArray(x) && x.length >= min && x.length <= max, 'bounded array');
const sorted = (xs, key = x => x) => {
  const keys = xs.map(key);
  assert(keys.every((k, i) => i === 0 || order(keys[i - 1], k) < 0), 'unique sorted set');
};
function canonical(x) {
  if (x === null || typeof x === 'boolean') return JSON.stringify(x);
  if (typeof x === 'string') { assert(x.isWellFormed(), 'unpaired surrogate'); return JSON.stringify(x); }
  if (typeof x === 'number') { assert(Number.isSafeInteger(x) && !Object.is(x, -0), 'lossless integer'); return JSON.stringify(x); }
  if (Array.isArray(x)) return '[' + x.map(canonical).join(',') + ']';
  assert(object(x), 'supported JSON value');
  return '{' + Object.keys(x).sort(order).map(k => canonical(k) + ':' + canonical(x[k])).join(',') + '}';
}
const raw = x => hash(canonical(x));
function sealed(x, field, domain, message) {
  digest(x[field]);
  const body = {...x}; delete body[field];
  assert.equal(hash(domain + '\n' + canonical(body)), x[field], message);
}
const actions = ['run.read', 'event.wait', 'clarification.request', 'plan.propose', 'task.launch', 'child-result.record']
  .map(x => 'orkworks.coordination.' + x + '.v1').sort(order);
const resultTools = ['orkworks.result.receipt.read.v1', 'orkworks.result.submit.v1'];
const surfaces = ['instructions', 'skills', 'tools', 'commands', 'filesystem', 'network', 'connectors', 'delegation', 'startup', 'model-policy'].sort(order);
const roles = ['orchestrator', 'research', 'implementation', 'review', 'verification', 'remediation'];
const boundary = 'Coordinate only within the approved root assignment. Binding repository rules remain mandatory. Skills and task text grant no tools or authority. Request changed scope from the user. Approval, current capabilities and exact-plan grants remain separate gates. Do not implement, review code, mutate Git, delegate natively or integrate edits.';

function profile(x, b) {
  exact(x, 'tools readPaths writePaths commands externalSources connectors coordinationActions nativeDelegation accessChanges');
  for (const key of ['readPaths', 'writePaths', 'commands', 'externalSources', 'connectors']) assert.deepEqual(x[key], [], 'empty root access');
  assert.equal(x.nativeDelegation, 'denied', 'native delegation');
  assert.equal(x.accessChanges, 'request-only', 'request-only access');
  list(x.coordinationActions, 16);
  assert.deepEqual(x.coordinationActions.map(a => a.actionId), actions, 'six coordination actions');
  for (const a of x.coordinationActions) {
    exact(a, 'actionId workspaceId runId parentSessionId');
    for (const key of ['workspaceId', 'runId', 'parentSessionId']) assert.equal(a[key], b[key], 'action owner');
  }
  list(x.tools, 64); x.tools.forEach(id);
  const resultCount = resultTools.filter(t => x.tools.includes(t)).length;
  assert(resultCount === 0 || resultCount === 2, 'result action pair');
  assert.deepEqual(x.tools, [...actions, ...(resultCount ? resultTools : [])].sort(order), 'tool pairing');
  if (b.outputContract.declarations.length) assert.equal(resultCount, 2, 'required result reporting');
}

function bindings(xs) {
  list(xs, 32); sorted(xs, x => x.name);
  for (const x of xs) {
    exact(x, 'name value');
    assert(typeof x.name === 'string' && /^[A-Za-z_][A-Za-z0-9_.-]{0,127}$/.test(x.name), 'binding name');
    text(x.value, 2048, 0);
  }
  // Recognized semantics and absence of secrets require adapter qualification.
}
function platform(x) {
  exact(x, 'os architecture osVersion'); tag(x.os, ['macos', 'linux', 'windows']);
  tag(x.architecture, ['arm64', 'x64']); text(x.osVersion, 512);
}
function policy(x) {
  exact(x, 'schemaVersion kind harnessId adapterId adapterVersion policyId mode modelId settings unknownModelAllowed');
  assert.equal(x.schemaVersion, 1); assert.equal(x.kind, 'root-model-policy');
  for (const k of ['harnessId', 'adapterId', 'policyId']) id(x[k]);
  text(x.adapterVersion, 512); tag(x.mode, ['pinned', 'tool-managed']);
  assert.equal(typeof x.unknownModelAllowed, 'boolean');
  if (x.mode === 'pinned') { text(x.modelId, 512); assert.equal(x.unknownModelAllowed, false, 'pinned policy'); }
  else assert.equal(x.modelId, null, 'tool-managed policy');
  bindings(x.settings); assert(Buffer.byteLength(canonical(x)) <= 65536, 'artifact 64 KiB');
}
function projection(x) {
  exact(x, 'schemaVersion kind sourceIdentity sourceRevision bindings');
  assert.equal(x.schemaVersion, 1); assert.equal(x.kind, 'root-effective-configuration');
  text(x.sourceIdentity, 512); text(x.sourceRevision, 512); bindings(x.bindings);
  assert(Buffer.byteLength(canonical(x)) <= 65536, 'artifact 64 KiB');
}
function artifacts(context, evidence) {
  list(context.retainedArtifacts, 16);
  const map = new Map();
  for (const a of context.retainedArtifacts) {
    exact(a, 'content contentDigest'); text(a.content, 1048576); digest(a.contentDigest);
    assert.equal(hash(a.content), a.contentDigest, 'artifact content digest');
    assert(!map.has(a.contentDigest), 'duplicate artifact'); map.set(a.contentDigest, a.content);
  }
  let total = 0;
  for (const r of evidence.references) {
    assert(map.has(r.contentDigest), 'retained artifact unavailable');
    total += Buffer.byteLength(map.get(r.contentDigest));
  }
  assert(total <= 1048576, 'evidence artifact budget');
  // Reject unreferenced side storage even though it cannot confer authority.
  assert.deepEqual([...map.keys()].sort(), [...new Set(evidence.references.map(r => r.contentDigest))].sort(), 'protected artifact closure');
  return map;
}
function artifact(map, d, refs, fixture = true) {
  assert(map.has(d), 'retained artifact unavailable');
  assert(refs.some(r => r.contentDigest === d && (!fixture || r.kind === 'fixture')), 'retained fixture reference');
  return decodeSourceComponent(Buffer.from(map.get(d)));
}
function settings(x, b, map) {
  exact(x, 'schemaVersion kind harnessId adapterId adapterVersion argv nonsecretBindings configurationSources discoveredInstructions deliveryOrder instructionMechanism');
  assert.equal(x.schemaVersion, 1); assert.equal(x.kind, 'root-startup-settings');
  for (const k of ['harnessId', 'adapterId']) id(x[k]); text(x.adapterVersion, 512);
  tag(x.instructionMechanism, ['native-agent', 'startup-context']);
  list(x.argv, 64); x.argv.forEach(v => text(v, 2048, 0)); bindings(x.nonsecretBindings);
  list(x.configurationSources, 32); sorted(x.configurationSources, v => v.sourceIdentity);
  for (const s of x.configurationSources) {
    exact(s, 'sourceIdentity sourceRevision contentDigest');
    text(s.sourceIdentity, 512); text(s.sourceRevision, 512); digest(s.contentDigest);
    const p = artifact(map, s.contentDigest, b.capabilityEvidence.references, false); projection(p);
    assert.equal(p.sourceIdentity, s.sourceIdentity, 'configuration source identity');
    assert.equal(p.sourceRevision, s.sourceRevision, 'configuration source revision');
  }
  const snapshots = {'rule': b.rules, 'role-template': [b.roleTemplate], 'skill': b.skills, 'skill-resource': b.skillResources};
  list(x.discoveredInstructions, 32);
  const seen = new Set();
  for (const [i, d] of x.discoveredInstructions.entries()) {
    exact(d, 'snapshotKind snapshotId contentDigest precedence');
    assert.equal(d.precedence, i, 'discovery precedence'); id(d.snapshotId); digest(d.contentDigest);
    const source = snapshots[d.snapshotKind]?.find(s => s.id === d.snapshotId);
    assert(source && source.contentDigest === d.contentDigest, 'automatic snapshot');
    const key = d.snapshotKind + ':' + d.snapshotId; assert(!seen.has(key), 'duplicate discovery'); seen.add(key);
  }
  list(x.deliveryOrder, 33, 1);
  let supplied = 0; const automatic = [];
  for (const d of x.deliveryOrder) {
    tag(d.kind, ['supplied', 'automatic']);
    exact(d, d.kind === 'supplied' ? 'kind' : 'kind precedence');
    if (d.kind === 'supplied') supplied++; else automatic.push(d.precedence);
  }
  assert.equal(supplied, 1, 'one supplied payload');
  assert.deepEqual(automatic, x.discoveredInstructions.map(d => d.precedence), 'automatic delivery order');
  assert(Buffer.byteLength(canonical(x)) <= 65536, 'artifact 64 KiB');
}
function utc(x) {
  assert(typeof x === 'string' && /^(?!0000)\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(x), 'UTC Gregorian timestamp');
  const date = new Date(x);
  assert(Number.isFinite(date.valueOf()) && date.toISOString() === x.replace('Z', '.000Z'), 'UTC Gregorian timestamp');
}
function evidence(x, b) {
  exact(x, 'schemaVersion evidenceId harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest role requestedProfileDigest effectiveProfileDigest decision checks references assessedAt assessorIdentity evidenceDigest');
  assert.equal(x.schemaVersion, 1); assert.equal(x.role, 'orchestrator');
  for (const k of ['evidenceId', 'harnessId', 'adapterId']) id(x[k]);
  for (const k of ['definitionDigest', 'effectiveSettingsDigest', 'requestedProfileDigest', 'effectiveProfileDigest']) digest(x[k]);
  for (const k of ['adapterVersion', 'toolVersion', 'assessorIdentity']) text(x[k], 512);
  tag(x.instructionMechanism, ['native-agent', 'startup-context']); platform(x.platform); utc(x.assessedAt);
  validateExecutableBinding(x.executableIdentity, x.executableIdentity);
  assert.equal(x.decision, 'unverified', 'unverified slice');
  list(x.references, 16, 1); sorted(x.references, r => r.id);
  for (const r of x.references) {
    exact(r, 'id kind sourceReference contentDigest'); id(r.id);
    tag(r.kind, ['primary-reference', 'fixture']); text(r.sourceReference, 2048); digest(r.contentDigest);
  }
  list(x.checks, 10); assert.deepEqual(x.checks.map(c => c.surface), surfaces, 'ten surfaces');
  let unverified = false;
  for (const c of x.checks) {
    exact(c, 'surface result reason referenceIds'); tag(c.result, ['passed', 'failed', 'unverified', 'not-applicable']);
    text(c.reason, 2048); list(c.referenceIds, 16, 1); sorted(c.referenceIds); c.referenceIds.forEach(id);
    const refs = c.referenceIds.map(key => x.references.find(r => r.id === key));
    assert(refs.every(Boolean), 'evidence reference');
    if (c.result === 'unverified') unverified = true;
    if (c.result === 'passed' || c.result === 'not-applicable') {
      assert(refs.some(r => r.kind === 'primary-reference') && refs.some(r => r.kind === 'fixture'), 'source and fixture evidence');
    }
  }
  assert(unverified, 'unverified check required');
  assert.equal(x.requestedProfileDigest, raw(b.permissions.requested), 'requested profile digest');
  assert.equal(x.effectiveProfileDigest, raw(b.permissions.effective), 'effective profile digest');
  sealed(x, 'evidenceDigest', 'orkworks.capability-evidence.v1', 'evidence digest');
}
function snapshots(b, context) {
  const r = b.roleTemplate;
  exact(r, 'id version role instructions instructionsDigest contentDigest provenance'); id(r.id); integer(r.version, 1, Number.MAX_SAFE_INTEGER);
  assert.equal(r.role, 'orchestrator'); text(r.instructions, 262144);
  assert.equal(hash(r.instructions), r.instructionsDigest, 'role instructions digest');
  const roleBody = {...r}; delete roleBody.contentDigest;
  assert.equal(raw(roleBody), r.contentDigest, 'role content digest');
  validateSourceComponent('provenance', r.provenance, context);
  list(b.rules, 32); list(b.skillResources, 32); assert(b.rules.length + b.skillResources.length <= 32, 'combined rule/resource cap');
  list(b.skills, 16); sorted(b.skills, s => s.id); sorted(b.skillResources, s => s.id);
  for (const [i, rule] of b.rules.entries()) {
    exact(rule, 'id sourcePath content contentDigest precedence authority provenance'); id(rule.id);
    text(rule.sourcePath, 2048);
    assert(!rule.sourcePath.includes('\\') && !rule.sourcePath.includes('\0') && !/^[A-Za-z]:/.test(rule.sourcePath) && rule.sourcePath.split('/').every(s => s && s !== '.' && s !== '..'), 'relative rule path');
    assert.equal(rule.precedence, i); assert.equal(rule.authority, 'binding');
  }
  assert.equal(new Set(b.rules.map(s => s.id)).size, b.rules.length, 'unique rule IDs');
  assert.equal(new Set(b.rules.map(s => s.provenance.sourceIdentity + '\0' + s.sourcePath)).size, b.rules.length, 'unique rule locators');
  for (const s of b.skills) {
    exact(s, 'id version content contentDigest requirement selectionReason requirementSources resourceIds provenance');
    id(s.id); text(s.version, 128); tag(s.requirement, ['mandatory', 'optional']); text(s.selectionReason, 2048);
    list(s.resourceIds, 32); s.resourceIds.forEach(id); sorted(s.resourceIds);
  }
  for (const s of b.skillResources) {
    exact(s, 'id skillId sourceReference content contentDigest provenance'); id(s.id); id(s.skillId); text(s.sourceReference, 2048);
    assert(b.skills.some(skill => skill.id === s.skillId && skill.resourceIds.includes(s.id)), 'resource ownership');
  }
  for (const s of [...b.rules, ...b.skills, ...b.skillResources]) {
    text(s.content, 262144); digest(s.contentDigest); assert.equal(hash(s.content), s.contentDigest, 'snapshot content digest');
    validateSourceComponent('provenance', s.provenance, context);
  }
  for (const s of b.skills) assert.deepEqual(s.resourceIds, b.skillResources.filter(r => r.skillId === s.id).map(r => r.id), 'resource closure');
  const sources = [...b.rules, r].sort((a, b) => order(a.id, b.id));
  assert.equal(new Set(sources.map(s => s.id)).size, sources.length, 'source namespace collision');
  list(b.requirementManifests, 33);
  assert.deepEqual(b.requirementManifests.map(m => m.sourceId), sources.map(s => s.id), 'manifest coverage');
  for (const [i, m] of b.requirementManifests.entries()) {
    exact(m, 'sourceId sourceDigest taskId requiredSkillIds sourceLocations applicabilityReason');
    assert.equal(m.sourceDigest, sources[i].contentDigest, 'manifest source');
    assert.equal(m.taskId, b.rootAssignmentId, 'manifest task'); text(m.applicabilityReason, 2048);
    list(m.requiredSkillIds, 16); m.requiredSkillIds.forEach(id); sorted(m.requiredSkillIds);
    list(m.sourceLocations, 32, 1); m.sourceLocations.forEach(x => text(x, 512)); sorted(m.sourceLocations);
  }
  const required = [...new Set(b.requirementManifests.flatMap(m => m.requiredSkillIds))].sort(order);
  assert.deepEqual(b.skills.filter(s => s.requirement === 'mandatory').map(s => s.id), required, 'mandatory skill union');
  for (const s of b.skills) {
    list(s.requirementSources, 32);
    const expected = b.requirementManifests.filter(m => m.requiredSkillIds.includes(s.id))
      .map(({sourceId, sourceDigest, sourceLocations}) => ({sourceId, sourceDigest, sourceLocations}));
    assert.deepEqual(s.requirementSources, s.requirement === 'mandatory' ? expected : [],
      s.requirement === 'mandatory' ? 'requirement source projection' : 'optional sources');
  }
  exact(b.rubric, 'id version dimensions evaluatorRole'); id(b.rubric.id); integer(b.rubric.version, 1, Number.MAX_SAFE_INTEGER);
  tag(b.rubric.evaluatorRole, roles); list(b.rubric.dimensions, 16, 1);
  for (const d of b.rubric.dimensions) { exact(d, 'id description'); id(d.id); text(d.description, 2048); }
  assert.equal(new Set(b.rubric.dimensions.map(d => d.id)).size, b.rubric.dimensions.length, 'unique dimensions');
}
// Existing #745 repository/worktree descriptors retain their 4096-byte bound.
// New provenance/executable paths use the separate 2048-byte source checker.
function nativePath(x) {
  exact(x, 'encoding base64'); tag(x.encoding, ['unix-bytes', 'windows-utf16le']);
  assert(typeof x.base64 === 'string');
  const bytes = Buffer.from(x.base64, 'base64');
  assert.equal(bytes.toString('base64'), x.base64); assert(bytes.length >= 1 && bytes.length <= 4096, 'native path limit');
  if (x.encoding === 'unix-bytes') { assert.equal(bytes[0], 47); assert(!bytes.includes(0)); }
  else {
    assert(bytes.length % 2 === 0 && /^(?:[A-Za-z]:\\|\\\\[^\\]+\\[^\\]+)/.test(bytes.toString('utf16le')), 'absolute Windows path');
    for (let i = 0; i < bytes.length; i += 2) assert.notEqual(bytes.readUInt16LE(i), 0);
  }
}
function directory(x) {
  exact(x, 'platform mechanismVersion volumeId fileId');
  for (const k of ['platform', 'volumeId', 'fileId']) assert(typeof x[k] === 'string' && /^[\x20-\x7e]{1,128}$/.test(x[k]), 'opaque native object field');
  integer(x.mechanismVersion, 1, Number.MAX_SAFE_INTEGER);
}
function repository(x) {
  exact(x, 'schemaVersion registryEpoch repositoryId canonicalCommonDirectory commonDirectoryObject bindingDigest');
  assert.equal(x.schemaVersion, 1); assert(/^[a-f0-9]{32}$/.test(x.registryEpoch));
  assert(/^repo-[a-f0-9]{32}$/.test(x.repositoryId)); nativePath(x.canonicalCommonDirectory); directory(x.commonDirectoryObject);
  assert(Buffer.byteLength(canonical(x)) <= 8192, 'repository 8 KiB');
  sealed(x, 'bindingDigest', 'orkworks.repository-binding.v1', 'repository digest');
}
function worktree(x, r) {
  exact(x, 'repositoryBinding canonicalRoot rootObject canonicalPrivateGitDirectory privateGitDirectoryObject kind worktreeBindingDigest');
  assert.deepEqual(x.repositoryBinding, r, 'worktree repository'); tag(x.kind, ['primary', 'linked']);
  nativePath(x.canonicalRoot); nativePath(x.canonicalPrivateGitDirectory); directory(x.rootObject); directory(x.privateGitDirectoryObject);
  sealed(x, 'worktreeBindingDigest', 'orkworks.worktree-binding.v1', 'worktree digest');
}
function output(x) {
  exact(x, 'schemaVersion declarations maxSnapshotBytes maxRehashBytes'); assert.equal(x.schemaVersion, 1);
  list(x.declarations, 32); sorted(x.declarations, d => d.id); let total = 0;
  for (const d of x.declarations) {
    exact(d, 'id kind maxBytes source'); id(d.id); id(d.kind);
    assert.equal(d.source.kind, 'server-artifact', 'root server artifact'); exact(d.source, 'kind');
    assert.notEqual(d.kind, 'assignment_evaluation', 'root evaluation'); integer(d.maxBytes, 1, 524288); total += d.maxBytes;
  }
  integer(x.maxSnapshotBytes, 0, 16777216); integer(x.maxRehashBytes, 0, 67108864);
  if (!x.declarations.length) assert(x.maxSnapshotBytes === 0 && x.maxRehashBytes === 0, 'empty output caps');
  else assert(total <= x.maxSnapshotBytes && x.maxRehashBytes >= 2 * x.maxSnapshotBytes, 'output capacity');
}

export function renderRootInstructions(b) {
  const blocks = [['boundary', 'root-v1', boundary], ...b.rules.map(r => ['rule', r.id, r.content]),
    ['role', b.roleTemplate.id, b.roleTemplate.instructions],
    ['assignment', b.rootAssignmentId, canonical({assignment: b.assignment, outputContract: b.outputContract, rubric: b.rubric})]];
  for (const requirement of ['mandatory', 'optional']) for (const s of b.skills.filter(s => s.requirement === requirement)) {
    blocks.push(['skill', s.id, s.content]);
    blocks.push(...b.skillResources.filter(r => r.skillId === s.id).map(r => ['resource', r.id, r.content]));
  }
  const wire = 'ORKWORKS ROOT INSTRUCTIONS v1\n' + blocks.map(([kind, identity, content]) => {
    id(identity); assert(typeof content === 'string' && content.isWellFormed(), 'render UTF-8');
    return `${kind} ${identity} ${Buffer.byteLength(content)}\n${content}\n`;
  }).join('');
  assert(Buffer.byteLength(wire) <= 262144, 'render context ceiling'); return wire;
}
export function validateRootBootstrap(b, context) {
  exact(b, 'schemaVersion bootstrapId repositoryId repositoryBinding sourceWorktreeBinding workspaceId parentSessionId runId rootAssignmentId goalDigest roleTemplate taskCategory assignment rules requirementManifests skills skillResources rubric harness capabilityEvidence model permissions renderedInstructions renderedInstructionsDigest outputContract bootstrapConfigurationDigest');
  assert.equal(b.schemaVersion, 2);
  for (const k of ['bootstrapId', 'repositoryId', 'workspaceId', 'parentSessionId', 'runId', 'rootAssignmentId']) id(b[k]);
  digest(b.goalDigest); text(b.taskCategory, 512);
  exact(b.permissions, 'requested effective'); output(b.outputContract);
  profile(b.permissions.requested, b); profile(b.permissions.effective, b);
  validateRun(context.run); assert.equal(hash(context.goal), b.goalDigest, 'goal digest');
  repository(b.repositoryBinding); worktree(b.sourceWorktreeBinding, b.repositoryBinding);
  assert.equal(b.repositoryId, b.repositoryBinding.repositoryId);
  validateSourceComponent('assignment', b.assignment, context);
  assert.deepEqual(b.assignment.worktreeScope.readPaths, [], 'empty root scope');
  assert.deepEqual(b.assignment.worktreeScope.writePaths, [], 'empty root scope');
  assert.deepEqual(b.assignment.inputs.filter(i => i.kind === 'reference'), [], 'reference custody outside initial source slice');
  snapshots(b, context); evidence(b.capabilityEvidence, b);
  const h = b.harness, e = b.capabilityEvidence;
  exact(h, 'harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest evidenceId evidenceDigest');
  for (const k of Object.keys(h)) assert.equal(canonical(h[k]), canonical(e[k]), 'shared harness ' + k);
  validateSourceComponent('executable', h.executableIdentity, context);
  const map = artifacts(context, e);
  const s = artifact(map, h.effectiveSettingsDigest, e.references); settings(s, b, map);
  for (const k of ['harnessId', 'adapterId', 'adapterVersion', 'instructionMechanism']) assert.equal(s[k], h[k], 'settings binding');
  exact(b.model, 'schemaVersion mode modelId policyId policyDigest adapterGeneration'); assert.equal(b.model.schemaVersion, 1);
  id(b.model.adapterGeneration); digest(b.model.policyDigest);
  const p = artifact(map, b.model.policyDigest, e.references); policy(p);
  for (const k of ['policyId', 'mode', 'modelId']) assert.equal(b.model[k], p[k], 'model policy binding');
  for (const k of ['harnessId', 'adapterId', 'adapterVersion']) assert.equal(p[k], h[k], 'policy adapter binding');
  assert.equal(b.renderedInstructions, renderRootInstructions(b), 'rendered bytes');
  assert.equal(b.renderedInstructionsDigest, hash(b.renderedInstructions), 'rendered digest');
  sealed(b, 'bootstrapConfigurationDigest', 'orkworks.orchestrator-bootstrap.v2', 'bootstrap digest');
  assert(Buffer.byteLength(canonical(b)) <= 1048576, 'bootstrap 1 MiB');
  for (const k of ['workspaceId', 'parentSessionId', 'runId', 'goalDigest', 'bootstrapConfigurationDigest']) assert.equal(b[k], context.run[k], 'run bootstrap ' + k);
  assert.equal(b.repositoryBinding.bindingDigest, context.run.repositoryBindingDigest, 'run repository binding');
  assert.equal(b.sourceWorktreeBinding.worktreeBindingDigest, context.run.sourceWorktreeBindingDigest, 'run source binding');
  return {bootstrapConfigurationDigest: b.bootstrapConfigurationDigest, classification: 'unverified-source-bytes-only'};
}
export function validateRootComponent(kind, value, context) {
  const b = context.bootstrap;
  switch (kind) {
    case 'profile': profile(value, b); break;
    case 'policy': policy(value); break;
    case 'projection': projection(value); break;
    case 'settings': settings(value, b, artifacts(context, b.capabilityEvidence)); break;
    case 'evidence': evidence(value, b); artifacts(context, value); break;
    case 'bootstrap': validateRootBootstrap(value, context); break;
    default: assert.fail('unsupported root component');
  }
  return raw(value); // fixture fingerprint, never an admission token
}
