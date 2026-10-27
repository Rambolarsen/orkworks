// Documentation-only structural components. Never import into production.
// No filesystem/native evidence, source custody, admission or authority checks.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {validateRun} from './root-preparation-components.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const order = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const exact = (x, fields) => {
  assert(object(x), 'record object');
  assert.deepEqual(Object.keys(x).sort(), fields.split(' ').sort(), 'closed fields');
};
const text = (x, max) => assert(typeof x === 'string' && x.isWellFormed() && Buffer.byteLength(x) >= 1 && Buffer.byteLength(x) <= max, 'bounded UTF-8 text');
const id = x => assert(typeof x === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(x), 'ASCII ID');
const digest = x => assert(typeof x === 'string' && /^[0-9a-f]{64}$/.test(x), 'lowercase SHA-256');
const oneOf = (x, values) => assert(values.includes(x), 'supported tag');
const list = (x, max) => assert(Array.isArray(x) && x.length <= max, 'bounded array');
const unique = xs => assert.equal(new Set(xs).size, xs.length, 'unique entries');

function canonical(x) {
  if (x === null || typeof x === 'boolean') return JSON.stringify(x);
  if (typeof x === 'string') {
    assert(x.isWellFormed(), 'unpaired surrogate');
    return JSON.stringify(x);
  }
  if (typeof x === 'number') {
    assert(Number.isSafeInteger(x) && !Object.is(x, -0), 'lossless integer');
    return JSON.stringify(x);
  }
  if (Array.isArray(x)) return '[' + x.map(canonical).join(',') + ']';
  assert(object(x), 'supported component value');
  return '{' + Object.keys(x).sort(order).map(k => canonical(k) + ':' + canonical(x[k])).join(',') + '}';
}

export function decodeSourceComponent(bytes) {
  const wire = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(bytes);
  const value = JSON.parse(wire);
  assert.equal(canonical(value), wire, 'canonical stored bytes');
  return value;
}

// Plausible path bytes still need native evidence for canonicality or identity.
function path(x) {
  exact(x, 'encoding base64');
  oneOf(x.encoding, ['unix-bytes', 'windows-utf16le']);
  assert(typeof x.base64 === 'string' && x.base64.length <= 2732, 'bounded base64');
  const bytes = Buffer.from(x.base64, 'base64');
  assert.equal(bytes.toString('base64'), x.base64, 'canonical padded base64');
  assert(bytes.length > 0 && bytes.length <= 2048, 'native path bytes');
  if (x.encoding === 'unix-bytes') {
    assert.equal(bytes[0], 47, 'absolute Unix path');
    assert(!bytes.includes(0), 'no native NUL');
  } else {
    assert.equal(bytes.length % 2, 0, 'UTF-16 native units');
    for (let i = 0; i < bytes.length; i += 2) assert(bytes.readUInt16LE(i) !== 0, 'no native NUL');
    assert(/^(?:[A-Za-z]:\\|\\\\[^\\]+\\[^\\]+)/.test(bytes.toString('utf16le')), 'absolute Windows path');
  }
}

function scopePaths(xs) {
  list(xs, 64);
  for (const p of xs) {
    text(p, 2048);
    assert(!p.includes('\\') && !p.includes('\0') && !/^[A-Za-z]:/.test(p), 'relative scope path');
    assert(p.split('/').every(c => c !== '' && c !== '.' && c !== '..'), 'relative path components');
  }
  unique(xs);
  assert.deepEqual(xs, [...xs].sort(order), 'path set byte order');
}

function assignment(x, run) {
  exact(x, 'schemaVersion label description inputs expectedOutput criteria dependencies worktreeScope');
  assert.equal(x.schemaVersion, 1);
  text(x.label, 512); text(x.description, 4096); text(x.expectedOutput, 2048);
  list(x.inputs, 32);
  for (const input of x.inputs) {
    oneOf(input.kind, ['inline', 'reference']);
    exact(input, input.kind === 'inline' ? 'kind id content contentDigest' : 'kind id sourceIdentity sourceRevision reference contentDigest');
    id(input.id); digest(input.contentDigest);
    if (input.kind === 'inline') {
      text(input.content, 4096);
      assert.equal(hash(input.content), input.contentDigest, 'inline digest');
    } else {
      text(input.sourceIdentity, 512); text(input.sourceRevision, 512); text(input.reference, 2048);
      // This establishes no retained reference bytes/custody.
    }
  }
  unique(x.inputs.map(i => i.id));
  list(x.criteria, 32);
  assert(x.criteria.some(c => c.requirement === 'required'), 'required criterion');
  for (const c of x.criteria) {
    exact(c, 'id requirement description'); id(c.id);
    oneOf(c.requirement, ['required', 'optional']); text(c.description, 2048);
  }
  unique(x.criteria.map(c => c.id));
  assert.deepEqual(x.dependencies, [], 'no future root dependencies');
  exact(x.worktreeScope, 'kind repositoryBindingDigest sourceWorktreeBindingDigest readPaths writePaths');
  const s = x.worktreeScope;
  assert.equal(s.kind, 'source-worktree');
  assert.equal(s.repositoryBindingDigest, run.repositoryBindingDigest, 'repository scope');
  assert.equal(s.sourceWorktreeBindingDigest, run.sourceWorktreeBindingDigest, 'worktree scope');
  scopePaths(s.readPaths); scopePaths(s.writePaths);
}

function provenance(x) {
  oneOf(x.kind, ['local', 'nonlocal']);
  exact(x, 'schemaVersion kind sourceKind sourceIdentity sourceRevision' + (x.kind === 'local' ? ' sourcePath' : ''));
  assert.equal(x.schemaVersion, 1);
  oneOf(x.sourceKind, ['repository', 'bundled', 'user-approved']);
  text(x.sourceIdentity, 512); text(x.sourceRevision, 512);
  if (x.kind === 'local') path(x.sourcePath);
  else assert.notEqual(x.sourceKind, 'repository', 'repository file source must be local');
}

function executable(x) {
  oneOf(x.kind, ['direct', 'wrapped']);
  exact(x, 'schemaVersion kind executable' + (x.kind === 'wrapped' ? ' components' : ''));
  assert.equal(x.schemaVersion, 1);
  exact(x.executable, 'path contentDigest');
  path(x.executable.path); digest(x.executable.contentDigest);
  if (x.kind === 'wrapped') {
    list(x.components, 64); assert(x.components.length >= 1, 'nonempty chain');
    for (const c of x.components) {
      exact(c, 'kind path contentDigest');
      oneOf(c.kind, ['wrapper', 'interpreter', 'package-manifest', 'package-file']);
      path(c.path); digest(c.contentDigest);
    }
    unique(x.components.map(c => canonical({kind: c.kind, path: c.path})));
  }
  assert(Buffer.byteLength(canonical(x)) <= 65536, 'executable identity 64 KiB');
  // Inventory completeness and actual invocation remain UNVERIFIED.
}

function input(x, run) {
  assert.deepEqual(x, {
    schemaVersion: 1, workspaceId: run.workspaceId, runId: run.runId,
    preparationRevision: 1, inputRevision: 1, goalDigest: run.goalDigest,
    bootstrapConfigurationDigest: run.bootstrapConfigurationDigest,
    decision: null, records: [],
  }, 'initial input exact source binding');
  return hash('orkworks.preparation-input.v1\n' + canonical(x));
}

function state(x, context) {
  const r = context.run;
  const inputDigest = input(context.initialInput, r);
  assert.deepEqual(x, {
    schemaVersion: 1, workspaceId: r.workspaceId, runId: r.runId,
    parentSessionId: r.parentSessionId, runDefinitionDigest: r.runDefinitionDigest,
    bootstrapConfigurationDigest: r.bootstrapConfigurationDigest, runVersion: 1,
    state: 'assessing', preparationRevision: 1, inputRevision: 1, inputDigest,
    planReferences: [], currentPlan: null, parentRuntime: null, blockers: [],
    stageContinueRequired: false, pendingClarification: null, pauseReason: null, resumeTarget: null,
  }, 'initial state exact membership');
}

function attachment(x, context) {
  oneOf(x.kind, ['reserved', 'attached']);
  const fields = 'schemaVersion kind workspaceId runId parentSessionId rootAssignmentId bootstrapConfigurationDigest';
  exact(x, fields + (x.kind === 'attached' ? ' sidecarGeneration launchGeneration' : ''));
  assert.equal(x.schemaVersion, 1);
  for (const field of ['workspaceId', 'runId', 'parentSessionId', 'bootstrapConfigurationDigest']) assert.equal(x[field], context.run[field], 'attachment ' + field);
  id(context.rootAssignmentId);
  assert.equal(x.rootAssignmentId, context.rootAssignmentId, 'root assignment binding');
  if (x.kind === 'attached') {
    digest(x.sidecarGeneration); digest(x.launchGeneration);
  }
}

export function validateSourceComponent(kind, value, context) {
  // Complete closed run component, never a partial caller projection.
  validateRun(context.run);
  switch (kind) {
    case 'assignment': assignment(value, context.run); break;
    case 'provenance': provenance(value); break;
    case 'executable': executable(value); break;
    case 'input': input(value, context.run); break;
    case 'state': state(value, context); break;
    case 'attachment': attachment(value, context); break;
    default: assert.fail('unsupported source component');
  }
  return hash(canonical(value)); // raw fixture fingerprint, not a source domain
}

// Compare the two source-owned field values only, not full adapter eligibility.
export function validateExecutableBinding(adapterIdentity, evidenceIdentity) {
  executable(adapterIdentity); executable(evidenceIdentity);
  const bytes = canonical(adapterIdentity);
  assert.equal(bytes, canonical(evidenceIdentity), 'shared executable identity bytes');
  return hash(bytes);
}
