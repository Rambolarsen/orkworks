// Exact-source joins and adversarial mutations; never live admission calls.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {decodeSourceComponent} from './root-preparation-source-components.mjs';
import {validatePreparationComponent, validatePreparationPacket, preparationRowDigest} from './root-preparation-joins.mjs';
const bytes = name => readFileSync(new URL(name, import.meta.url));
const catalog = JSON.parse(bytes('./root-preparation-join-vectors.json'));
const bootstrapCatalog = JSON.parse(bytes('./root-bootstrap-vectors.json'));
const entry = kind => catalog.vectors.find(v => v.kind === kind);
const get = kind => JSON.parse(entry(kind).canonical);
const hash = raw => createHash('sha256').update(raw).digest('hex');
const context = () => ({goal: bootstrapCatalog.goal, retainedArtifacts: structuredClone(bootstrapCatalog.retainedArtifacts),
  reviewedDraftBytes: Buffer.from(entry('draft').canonical), approvedRequestBytes: Buffer.from(entry('approval').canonical),
  sourceKey: structuredClone(catalog.sourceKey)});
const packet = () => ({draftBytes: Buffer.from(entry('draft').canonical),
  approvalBytes: Buffer.from(entry('approval').canonical), sourceBytes: Buffer.from(entry('source').canonical)});
function canonical(x) {
  if (Array.isArray(x)) return '[' + x.map(canonical).join(',') + ']';
  if (x && typeof x === 'object') return '{' + Object.keys(x).sort().map(k => JSON.stringify(k) + ':' + canonical(x[k])).join(',') + '}';
  return JSON.stringify(x);
}
const seal = (x, field, domain) => {const body = {...x}; delete body[field]; x[field] = hash(domain + '\n' + canonical(body));};
for (const v of catalog.vectors) test(`${v.kind}: independent canonical bytes and raw/domain hashes`, () => {
  const x = decodeSourceComponent(Buffer.from(v.canonical));
  assert.equal(Buffer.byteLength(v.canonical), v.utf8Bytes);
  assert.equal(hash(v.canonical), v.sha256);
  const result = validatePreparationComponent(v.kind, x, context());
  assert.equal(result, v.domainDigest ?? v.sha256);
});
test('original frozen draft, approval and retained source join without admission claims', () => {
  assert.deepEqual(validatePreparationPacket(packet(), context()), {
    classification: 'synthetic-preparation-joins-only', draftDigest: entry('draft').domainDigest,
    requestDigest: entry('approval').domainDigest, sourceRowDigest: entry('source-row').domainDigest,
    rootAssignmentIdentity: get('receipt').rootAssignmentIdentity});
  assert.equal(hash(bytes('./root-bootstrap-vectors.json')), catalog.bootstrapCatalogSha256);
  assert.equal(hash(context().reviewedDraftBytes), catalog.frozenDraftSha256);
});
for (const [name, mutate, message] of [
  ['root identity', x => x.rootAssignmentId = 'foreign', /draft rootAssignmentId/],
  ['parent identity', x => x.parentSessionId = 'foreign', /draft parentSessionId/],
  ['run identity', x => x.runId = 'foreign', /draft runId/],
  ['workspace identity', x => x.workspaceId = 'foreign', /draft workspaceId/],
  ['unknown field', x => x.bearer = 'synthetic-only', /closed fields/],
  ['schema version', x => x.schemaVersion = 2, /draft version/],
  ['missing generation', x => delete x.sidecarGeneration, /closed fields/],
  ['noncanonical generation', x => x.sidecarGeneration = 'A'.repeat(64), /Digest/],
  ['noncanonical context', x => x.reviewContextId = '0'.repeat(63), /Digest/],
  ['changed digest', x => x.draftDigest = '0'.repeat(64), /draftDigest/],
  ['bootstrap drift', x => x.bootstrap.assignment.description += 'x', /rendered bytes/],
  ['run drift', x => x.runDefinition.maxParallelChildren = 2, /runDefinitionDigest/],
  ['quoted bootstrap', x => x.bootstrap = canonical(x.bootstrap), /record object/],
]) test(`draft rejects ${name}`, () => {
  const x = get('draft'); mutate(x);
  assert.throws(() => validatePreparationComponent('draft', x, context()), message);
});
for (const [name, mutate, message] of [
  ['different draft', x => x.draftId = 'other', /approval draftId/],
  ['different digest', x => x.draftDigest = '0'.repeat(64), /approval draftDigest/],
  ['different review context', x => x.reviewContextId = '0'.repeat(64), /approval reviewContextId/],
  ['different run', x => x.runId = 'foreign', /run binding/],
  ['foreign action', x => x.action = 'approve-everything', error => error.code === 'ERR_ASSERTION' && error.expected === 'orkworks.root-preparation.approve.v1'],
  ['extra grant', x => x.grant = null, /closed record fields/],
]) test(`approval rejects ${name}`, () => {
  const x = get('approval'); mutate(x);
  assert.throws(() => validatePreparationComponent('approval', x, context()), message);
});
for (const [name, mutate, message] of [
  ['receipt subject kind', x => x.receipt.rootAssignmentIdentity.kind = 'child', /root identity/],
  ['receipt child fields', x => x.receipt.rootAssignmentIdentity.taskId = 'future', /root identity/],
  ['receipt bootstrap digest', x => x.receipt.bootstrapConfigurationDigest = '0'.repeat(64), /receipt bootstrapConfigurationDigest/],
  ['receipt run digest', x => x.receipt.runDefinitionDigest = '0'.repeat(64), /receipt runDefinitionDigest/],
  ['receipt approval ID', x => x.receipt.approvalId = 'other', /receipt approvalId/],
  ['receipt request hash', x => x.receipt.requestDigest = '0'.repeat(64), /receipt requestDigest/],
  ['receipt draft hash', x => x.receipt.draftDigest = '0'.repeat(64), /receipt draftDigest/],
  ['reconstruction generation', x => x.approval.sidecarGeneration = '0'.repeat(64), /draftDigest/],
  ['reconstruction context', x => x.approval.reviewContextId = '0'.repeat(64), /draftDigest/],
  ['reconstruction draft ID', x => x.approval.draftId = 'other', /draftDigest/],
  ['request reconstruction', x => x.approval.requestDigest = '0'.repeat(64), /requestDigest/],
  ['commit epoch', x => x.lastCommit.storageEpoch = '0'.repeat(64), /source commit link/],
  ['commit digest', x => x.receipt.commit.changeDigest = '0'.repeat(64), /source commit link/],
  ['zero commit sequence', x => x.receipt.commit.commitSequence = '0', /positive Counter/],
  ['numeric commit sequence', x => x.lastCommit.commitSequence = 2, /positive Counter/],
  ['oversized commit sequence', x => x.lastCommit.commitSequence = '9223372036854775808', /Counter range/],
  ['nullable commit', x => x.lastCommit = null, /record object/],
  ['additional source field', x => x.latestConfiguration = {}, /closed fields/],
  ['missing receipt', x => delete x.receipt, /closed fields/],
]) test(`retained source rejects ${name}`, () => {
  const x = get('source'); mutate(x);
  assert.throws(() => validatePreparationComponent('source', x, context()), message);
});
test('source storage membership key cannot be substituted', () => {
  const c = context(); c.sourceKey.runId = 'other';
  assert.throws(() => validatePreparationComponent('source-row', get('source'), c), /source membership key/);
});
test('draft row key is the exact workspace and cannot use the run tuple', () => {
  assert.throws(() => preparationRowDigest('root_drafts', catalog.sourceKey, get('draft-row'), context()), /draft membership key/);
});
test('unregistered storage table is unavailable', () => {
  assert.throws(() => preparationRowDigest('objects', catalog.sourceKey, get('source'), context()), /preparation table/);
});
test('row commitment excludes only named commit links while validating their equality', () => {
  const x = get('source'), c = context();
  x.lastCommit.changeDigest = '0'.repeat(64); x.receipt.commit.changeDigest = '0'.repeat(64);
  assert.equal(preparationRowDigest('root_preparations', c.sourceKey, x, c), entry('source-row').domainDigest);
  x.lastCommit.changeDigest = '1'.repeat(64);
  assert.throws(() => preparationRowDigest('root_preparations', c.sourceKey, x, c), /source commit link/);
});
test('draft row storage links are outside the draft/source commitment', () => {
  const x = get('draft-row'); x.lastCommit.commitSequence = '9223372036854775807';
  assert.equal(preparationRowDigest('root_drafts', catalog.sourceKey.workspaceId, x, context()), entry('draft-row').domainDigest);
});
test('approval metadata is retained in row commitment even after all source hashes are repaired', () => {
  const x = get('source'), c = context();
  x.approval.approvalId = 'changed-approval';
  const a = get('approval'); a.approvalId = 'changed-approval';
  const digest = hash('orkworks.root-preparation-request.v1\n' + canonical(a));
  x.approval.requestDigest = digest; x.receipt.approvalId = a.approvalId; x.receipt.requestDigest = digest;
  c.approvedRequestBytes = Buffer.from(canonical(a));
  assert.notEqual(preparationRowDigest('root_preparations', c.sourceKey, x, c), entry('source-row').domainDigest);
});
test('same logical draft with altered frozen wire bytes fails rather than rehashing the review snapshot', () => {
  const c = context(); c.reviewedDraftBytes = Buffer.from(entry('draft').canonical + '\n');
  assert.throws(() => validatePreparationPacket(packet(), c), /canonical stored bytes|frozen draft bytes/);
});
test('self-consistent changed draft cannot replace original reviewed bytes', () => {
  const p = packet(); const d = get('draft'); d.draftId = 'substituted-draft';
  seal(d, 'draftDigest', 'orkworks.root-preparation-draft.v1'); p.draftBytes = Buffer.from(canonical(d));
  assert.throws(() => validatePreparationPacket(p, context()), /frozen draft bytes/);
});
test('same approval tuple with changed bytes conflicts against original retained approval', () => {
  const p = packet(); const a = get('approval'); a.approvalId = 'changed'; p.approvalBytes = Buffer.from(canonical(a));
  assert.throws(() => validatePreparationPacket(p, context()), /original approval bytes/);
});
for (const wire of ['{"schemaVersion":1,"schemaVersion":1}', ' {"schemaVersion":1}', '"\\ud800"'])
  test(`stored preparation bytes reject ${JSON.stringify(wire)}`, () => {
    const p = packet(); p.draftBytes = Buffer.from(wire);
    assert.throws(() => validatePreparationPacket(p, context()));
  });
test('invalid UTF-8 stored source is rejected before reconstruction', () => {
  const p = packet(); p.sourceBytes = Buffer.from([34,255,34]);
  assert.throws(() => validatePreparationPacket(p, context()));
});


test('draft and retained-source limits include the complete envelopes', () => {
  const d = get('draft'); d.bootstrap.assignment.description = 'x'.repeat(1081344);
  assert.throws(() => validatePreparationComponent('draft', d, context()), /draft envelope ceiling/);
  const s = get('source'); s.bootstrap.assignment.description = 'x'.repeat(1081344);
  assert.throws(() => validatePreparationComponent('source', s, context()), /source envelope ceiling/);
});
test('receipt bytes cannot escape their smaller ceiling through nested identity', () => {
  const r = get('receipt'); r.rootAssignmentIdentity.rootAssignmentId = 'x'.repeat(4096);
  assert.throws(() => validatePreparationComponent('receipt', r, context()), /receipt ceiling/);
});
for (const sequence of ['01', '+1', '1e2', '-1', '1.0', '']) test(`CommitRef rejects noncanonical Counter ${JSON.stringify(sequence)}`, () => {
  const x = get('draft-row'); x.lastCommit.commitSequence = sequence;
  assert.throws(() => validatePreparationComponent('draft-row', x, context()), /positive Counter/);
});
test('a repaired retained source cannot replace the original frozen draft', () => {
  const s = get('source'), d = get('draft'), a = get('approval');
  d.draftId = 'substituted'; seal(d, 'draftDigest', 'orkworks.root-preparation-draft.v1');
  Object.assign(a, {draftId:d.draftId, draftDigest:d.draftDigest});
  const requestDigest = hash('orkworks.root-preparation-request.v1\n' + canonical(a));
  Object.assign(s.approval, {draftId:d.draftId, draftDigest:d.draftDigest, requestDigest});
  Object.assign(s.receipt, {draftDigest:d.draftDigest, requestDigest});
  assert.throws(() => validatePreparationComponent('source', s, context()), /frozen draft bytes/);
});
test('the draft retains source content rather than allowing only its lookup hash', () => {
  const d = get('draft'); d.bootstrap = {bootstrapConfigurationDigest:d.bootstrap.bootstrapConfigurationDigest};
  assert.throws(() => validatePreparationComponent('draft', d, context()), /closed fields/);
});
