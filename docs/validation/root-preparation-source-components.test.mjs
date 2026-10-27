// Exact synthetic component oracles; these never exercise runtime admission.
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {decodeSourceComponent, validateSourceComponent, validateExecutableBinding} from './root-preparation-source-components.mjs';

const catalog = JSON.parse(readFileSync(new URL('./root-preparation-source-vectors.json', import.meta.url)));
const context = {...catalog.bindings, initialInput: catalog.initialInput};
const get = name => JSON.parse(catalog.vectors.find(v => v.id === name).canonical);
for (const v of catalog.vectors) test(`${v.id}: matches independent Python byte/hash oracle`, () => {
  const value = decodeSourceComponent(Buffer.from(v.canonical));
  assert.equal(Buffer.byteLength(v.canonical), v.bytes);
  assert.equal(validateSourceComponent(v.kind, value, context), v.sha256);
});

// A validator that accepts mismatched membership, omitted fields or widened
// authority must fail these tests even when the mutated JSON is well formed.
for (const [name, fixture, kind, mutate] of [
  ['root future dependency', 'inline-unicode', 'assignment', x => x.dependencies.push('future-child')],
  ['root child identity', 'inline-unicode', 'assignment', x => x.taskId = 'future'],
  ['missing description', 'inline-unicode', 'assignment', x => delete x.description],
  ['no required criterion', 'inline-unicode', 'assignment', x => x.criteria[0].requirement = 'optional'],
  ['criterion duplicate', 'inline-unicode', 'assignment', x => x.criteria.push({...x.criteria[0]})],
  ['input digest drift', 'inline-unicode', 'assignment', x => x.inputs[0].content += 'x'],
  ['input duplicate', 'inline-unicode', 'assignment', x => x.inputs.push({...x.inputs[0]})],
  ['unknown input tag', 'inline-unicode', 'assignment', x => x.inputs[0].kind = 'fetch'],
  ['reference null revision', 'reference-unbound', 'assignment', x => x.inputs[0].sourceRevision = null],
  ['oversized inline', 'inline-unicode', 'assignment', x => x.inputs[0].content = 'x'.repeat(4097)],
  ['relative path traversal', 'inline-unicode', 'assignment', x => x.worktreeScope.readPaths = ['../secret']],
  ['absolute scope path', 'inline-unicode', 'assignment', x => x.worktreeScope.readPaths = ['/secret']],
  ['Windows relative drive', 'inline-unicode', 'assignment', x => x.worktreeScope.readPaths = ['C:secret']],
  ['scope path duplicate', 'inline-unicode', 'assignment', x => x.worktreeScope.readPaths = ['a', 'a']],
  ['scope path order', 'inline-unicode', 'assignment', x => x.worktreeScope.readPaths = ['z', 'a']],
  ['worktree binding drift', 'inline-unicode', 'assignment', x => x.worktreeScope.sourceWorktreeBindingDigest = 'f'.repeat(64)],
  ['nonlocal nullable path', 'nonlocal', 'provenance', x => x.sourcePath = null],
  ['nonlocal repository source', 'nonlocal', 'provenance', x => x.sourceKind = 'repository'],
  ['provenance unknown source', 'local-native', 'provenance', x => x.sourceKind = 'remote'],
  ['path invalid base64', 'local-native', 'provenance', x => x.sourcePath.base64 = '@@@@'],
  ['path relative native bytes', 'local-native', 'provenance', x => x.sourcePath.base64 = Buffer.from('relative').toString('base64')],
  ['path native NUL', 'local-native', 'provenance', x => x.sourcePath.base64 = Buffer.from('/bad\0path').toString('base64')],
  ['path Windows odd units', 'windows-native', 'provenance', x => x.sourcePath.base64 = Buffer.from([67, 0, 58]).toString('base64')],
  ['direct hidden components', 'direct', 'executable', x => x.components = []],
  ['wrapped empty chain', 'wrapped', 'executable', x => x.components = []],
  ['wrapped duplicate component', 'wrapped', 'executable', x => x.components.push({...x.components[0]})],
  ['wrapped unknown kind', 'wrapped', 'executable', x => x.components[0].kind = 'directory'],
  ['wrapped excess chain', 'wrapped', 'executable', x => x.components = Array.from({length: 65}, (_, i) => ({...x.components[0], path: {encoding: 'unix-bytes', base64: Buffer.from('/' + i).toString('base64')}}))],
  ['executable hash uppercase', 'direct', 'executable', x => x.executable.contentDigest = 'A'.repeat(64)],
  ['input invented assessment', 'initial-input', 'input', x => x.decision = {}],
  ['input future evidence', 'initial-input', 'input', x => x.records.push('report')],
  ['input wrong goal digest', 'initial-input', 'input', x => x.goalDigest = '0'.repeat(64)],
  ['input string revision', 'initial-input', 'input', x => x.inputRevision = '1'],
  ['input wrong workspace', 'initial-input', 'input', x => x.workspaceId = 'other'],
  ['state preattached runtime', 'initial-state', 'state', x => x.parentRuntime = {}],
  ['state invented plan', 'initial-state', 'state', x => x.planReferences.push('plan')],
  ['state fabricated pass', 'initial-state', 'state', x => x.state = 'complete'],
  ['state stale input hash', 'initial-state', 'state', x => x.inputDigest = '0'.repeat(64)],
  ['state wrong definition', 'initial-state', 'state', x => x.runDefinitionDigest = '0'.repeat(64)],
  ['state wrong parent', 'initial-state', 'state', x => x.parentSessionId = 'other'],
  ['reserved nullable generation', 'reserved', 'attachment', x => x.launchGeneration = null],
  ['attached missing generation', 'attached', 'attachment', x => delete x.sidecarGeneration],
  ['attached numeric generation', 'attached', 'attachment', x => x.launchGeneration = 1],
  ['attachment wrong root', 'reserved', 'attachment', x => x.rootAssignmentId = 'other'],
  ['attachment wrong run', 'reserved', 'attachment', x => x.runId = 'other'],
  ['attachment extra bearer', 'attached', 'attachment', x => x.token = 'synthetic-only'],
]) test(`rejects ${name}`, () => {
  const value = get(fixture); mutate(value);
  assert.throws(() => validateSourceComponent(kind, value, context));
});

for (const [name, bytes] of [
  ['duplicate JSON keys', Buffer.from('{"x":null,"x":null}')],
  ['noncanonical whitespace', Buffer.from('{ "x":[]}')],
  ['invalid UTF-8', Buffer.from([0x22, 0xff, 0x22])],
  ['unpaired surrogate', Buffer.from('"\\ud800"')],
  ['unsafe integer', Buffer.from('9007199254740993')],
]) test(`stored source rejects ${name}`, () => assert.throws(() => decodeSourceComponent(bytes)));

test('input identity cannot be checked against a different valid run', () => {
  const otherRun = JSON.parse(readFileSync(new URL('./root-preparation-component-vectors.json', import.meta.url))).vectors[1];
  assert.throws(() => validateSourceComponent('input', get('initial-input'), {...context, run: JSON.parse(otherRun.runCanonical)}));
});

test('scope and input count boundaries accept the ceiling and reject one more', () => {
  const x = get('reference-unbound');
  x.inputs = Array.from({length: 32}, (_, i) => ({...x.inputs[0], id: 'input-' + i}));
  x.worktreeScope.readPaths = Array.from({length: 64}, (_, i) => String(i).padStart(2, '0'));
  assert.doesNotThrow(() => validateSourceComponent('assignment', x, context));
  const y = structuredClone(x); y.inputs.push({...y.inputs[0], id: 'overflow'});
  assert.throws(() => validateSourceComponent('assignment', y, context));
  x.worktreeScope.readPaths.push('overflow');
  assert.throws(() => validateSourceComponent('assignment', x, context));
});

test('executable chain count accepts 64 unique entries and rejects 65', () => {
  const x = get('wrapped');
  x.components = Array.from({length: 64}, (_, i) => ({...x.components[0], path: {encoding: 'unix-bytes', base64: Buffer.from('/' + i).toString('base64')}}));
  assert.doesNotThrow(() => validateSourceComponent('executable', x, context));
  x.components.push({...x.components[0], kind: 'wrapper'});
  assert.throws(() => validateSourceComponent('executable', x, context));
});

test('native path bound measures decoded bytes, preserving non-UTF-8 Unix units', () => {
  const x = get('local-native');
  const raw = Buffer.alloc(2048, 255); raw[0] = 47;
  x.sourcePath.base64 = raw.toString('base64');
  assert.doesNotThrow(() => validateSourceComponent('provenance', x, context));
  x.sourcePath.base64 = Buffer.concat([raw, Buffer.from([255])]).toString('base64');
  assert.throws(() => validateSourceComponent('provenance', x, context));
});

test('executable descriptor byte bound includes base64/JSON overhead', () => {
  const x = get('wrapped');
  x.components = Array.from({length: 64}, (_, i) => ({...x.components[0], path: {encoding: 'unix-bytes', base64: Buffer.from('/' + String(i).padStart(2, '0') + 'x'.repeat(750)).toString('base64')}}));
  assert.throws(() => validateSourceComponent('executable', x, context), /64 KiB/);
});

test('parent attachment data cannot substitute for a source state transition', () => {
  const x = get('initial-state');
  const a = get('attached');
  x.parentRuntime = {sidecarGeneration: a.sidecarGeneration, launchGeneration: a.launchGeneration};
  x.runVersion = 2;
  assert.throws(() => validateSourceComponent('state', x, context));
});

test('executable binding compares both owners against independent golden bytes', () => {
  const v = catalog.vectors.find(v => v.id === 'wrapped');
  assert.equal(validateExecutableBinding(get('wrapped'), get('wrapped')), v.sha256);
});

for (const [name, mutate] of [
  ['file content drift', x => x.executable.contentDigest = '0'.repeat(64)],
  ['native path drift', x => x.executable.path.base64 = Buffer.from('/other/tool').toString('base64')],
  ['invocation order drift', x => x.components.reverse()],
  ['malformed evidence identity', x => delete x.schemaVersion],
]) test(`executable binding rejects ${name}`, () => {
  const evidence = get('wrapped'); mutate(evidence);
  assert.throws(() => validateExecutableBinding(get('wrapped'), evidence));
});
