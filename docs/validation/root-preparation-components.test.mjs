// Tests of documentation component vectors, never production admission.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {decodeCanonical, validateRun, validateApproval} from './root-preparation-components.mjs';
const catalog = JSON.parse(readFileSync(new URL('./root-preparation-component-vectors.json', import.meta.url)));
const canonical = x => Array.isArray(x) ? '[' + x.map(canonical).join(',') + ']' :
  x !== null && typeof x === 'object' ? '{' + Object.keys(x).sort().map(k => JSON.stringify(k) + ':' + canonical(x[k])).join(',') + '}' : JSON.stringify(x);
const sha = s => createHash('sha256').update(s).digest('hex');
const seal = r => {
  r.goalDigest = sha(r.goal);
  const {runDefinitionDigest, ...body} = r;
  r.runDefinitionDigest = sha('orkworks.orchestration-run.v1\n' + canonical(body));
  return r;
};
for (const v of catalog.vectors) {
  test(v.id + ': independent Python canonical bytes and digests', () => {
    const run = decodeCanonical(Buffer.from(v.runCanonical));
    const approval = decodeCanonical(Buffer.from(v.approvalCanonical));
    assert.equal(Buffer.byteLength(v.runCanonical), v.runBytes);
    assert.equal(Buffer.byteLength(v.approvalCanonical), v.approvalBytes);
    assert.equal(validateRun(run), v.runDigest);
    assert.equal(validateApproval(approval, run), v.requestDigest);
  });
}
test('Unicode normalization changes the exact reviewed goal and run digest', () => {
  assert.notEqual(catalog.vectors[0].runDigest, catalog.vectors[1].runDigest);
  const r = JSON.parse(catalog.vectors[0].runCanonical);
  r.goal = r.goal.normalize('NFD');
  assert.throws(() => validateRun(r), /goalDigest/);
});
const badRuns = [
  ['wrong version', r => r.schemaVersion = 2],
  ['unknown field', r => r.extra = 1],
  ['missing field', r => delete r.parentSessionId],
  ['numeric sequence', r => r.runSequence = 1],
  ['leading zero', r => r.runSequence = '01'],
  ['zero sequence', r => r.runSequence = '0'],
  ['overflow', r => r.runSequence = '9223372036854775808'],
  ['wrong run identity', r => r.runId = 'another-run'],
  ['uppercase digest', r => r.repositoryBindingDigest = 'B'.repeat(64)],
  ['wrong digest type', r => r.sourceWorktreeBindingDigest = null],
  ['empty identity', r => r.workspaceId = ''],
  ['oversized identity', r => r.parentSessionId = 'x'.repeat(129)],
  ['non-ASCII identity', r => r.parentSessionId = 'café'],
  ['zero child limit', r => r.maxParallelChildren = 0],
  ['excess child limit', r => r.maxParallelChildren = 17],
  ['fractional child limit', r => r.maxParallelChildren = 1.5],
  ['string child limit', r => r.maxParallelChildren = '1'],
  ['unsafe JSON integer', r => r.maxParallelChildren = 9007199254740992],
  ['unpaired surrogate', r => r.goal = String.fromCharCode(0xd800)],
];
for (const [name, mutate] of badRuns) test('run rejects ' + name + ' even with a recomputed digest', () => {
  const r = JSON.parse(catalog.vectors[0].runCanonical);
  mutate(r); seal(r);
  assert.throws(() => validateRun(r));
});
test('run rejects one byte over its inclusive 16 KiB limit', () => {
  const r = JSON.parse(catalog.vectors.at(-1).runCanonical);
  r.goal += 'x'; seal(r);
  assert.equal(Buffer.byteLength(canonical(r)), 16385);
  assert.throws(() => validateRun(r), /16384/);
});
for (const field of ['goalDigest', 'runDefinitionDigest']) test('run rejects changed ' + field, () => {
  const r = JSON.parse(catalog.vectors[0].runCanonical);
  r[field] = '0'.repeat(64);
  assert.throws(() => validateRun(r), new RegExp(field));
});
for (const [name, mutate] of [
  ['wrong version', a => a.schemaVersion = 2],
  ['wrong action', a => a.action = 'orkworks.root-preparation.approve.v2'],
  ['extra authority', a => a.token = 'synthetic-not-a-token'],
  ['missing approval', a => delete a.approvalId],
  ['wrong workspace', a => a.workspaceId = 'another-workspace'],
  ['wrong run', a => a.runId = 'another-run'],
  ['bad draft digest', a => a.draftDigest = 'g'.repeat(64)],
  ['bad review context', a => a.reviewContextId = 'short'],
  ['oversized draft ID', a => a.draftId = 'x'.repeat(129)],
]) test('approval rejects ' + name, () => {
  const r = JSON.parse(catalog.vectors[0].runCanonical);
  const a = JSON.parse(catalog.vectors[0].approvalCanonical);
  mutate(a);
  assert.throws(() => validateApproval(a, r));
});
for (const [name, input] of [
  ['duplicate keys', Buffer.from('{"a":1,"a":1}')],
  ['whitespace', Buffer.from('{ "a":1}')],
  ['wrong key order', Buffer.from('{"b":1,"a":2}')],
  ['exponent spelling', Buffer.from('{"a":1e0}')],
  ['unsafe integer', Buffer.from('{"a":9007199254740993}')],
  ['invalid UTF-8', Buffer.from([0x22,0xff,0x22])],
  ['unpaired surrogate', Buffer.from('"\\ud800"')],
]) test('stored canonical bytes reject ' + name, () => assert.throws(() => decodeCanonical(input)));
