// Offline witnesses only: retained synthetic sources do not grant authority.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync, mkdtempSync, rmSync, copyFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {validateLedgerPacket, inspectLedgerReceipt} from './report-ledger-initialization.mjs';
import {canonical} from './report-publication-components.mjs';

const baselineBytes = readFileSync(new URL('root-preparation-effects.json', import.meta.url));
const baseline = JSON.parse(baselineBytes);
function generate() {
  const dir = mkdtempSync(join(tmpdir(), 'report-ledger-'));
  try {
    const output = join(dir, 'vectors.json');
    execFileSync('python3', [fileURLToPath(new URL('generate-report-ledger-initialization.py', import.meta.url)), '--output', output]);
    return readFileSync(output);
  } finally { rmSync(dir, {recursive: true, force: true}); }
}
const generated = generate();
const packet = () => JSON.parse(generated);
const sha = value => createHash('sha256').update(value).digest('hex');
const named = (p, name) => p.states.find(s => s.name === name);
const reportRows = s => s.rows.filter(r => r.table === 'quota_usage'
  && JSON.parse(r.keyCanonical).resource === 'report-retained-bytes');
const check = p => validateLedgerPacket(p, baseline, baselineBytes);

test('generator creates eight separately committed states with one ledger per admitted owner', () => {
  const p = packet();
  assert.equal(p.states.length, 8);
  assert.equal(p.commands.length, 7);
  const counts = p.states.map(s => s.rows.filter(r => r.table === 'quota_usage'
    && JSON.parse(r.keyCanonical).resource === 'report-retained-bytes').length);
  assert.deepEqual(counts, [0, 1, 1, 2, 2, 3, 3, 2]);
});
test('independent verifier accounts complete effects including both summary branches', () => {
  assert.deepEqual(validateLedgerPacket(packet(), baseline, baselineBytes).map(c => c.changedRows),
    // Retained other/target sources have 4/5 reference occurrences: n + 6.
    [12, 10, 19, 11, 19, 16, 11]);
});
test('receipt inspection cannot recreate a discarded source', () => {
  assert.throws(() => inspectLedgerReceipt(packet(), 'target-cancelled', 'target', baseline, baselineBytes), /retained admitted source/);
});

test('retained oracle regenerates independently and preserves baseline bytes', () => {
  assert.deepEqual(generated, readFileSync(new URL('report-ledger-initialization.json', import.meta.url)));
  assert.equal(packet().baselineSha256, sha(baselineBytes));
  assert.deepEqual(readFileSync(new URL('root-preparation-effects.json', import.meta.url)), baselineBytes);
});
test('generator paths containing spaces and URL escapes retain the same oracle', () => {
  const dir = mkdtempSync(join(tmpdir(), 'report ledger % '));
  try {
    for (const name of ['generate-report-ledger-initialization.py', 'root-preparation-effects.json'])
      copyFileSync(fileURLToPath(new URL(name, import.meta.url)), join(dir, name));
    const output = join(dir, 'result.json');
    execFileSync('python3', [join(dir, 'generate-report-ledger-initialization.py'), '--output', output]);
    assert.deepEqual(readFileSync(output), generated);
  } finally { rmSync(dir, {recursive: true, force: true}); }
});
test('byte inspection returns the original admitted receipt without mutating any state', () => {
  const p = packet(), before = canonical(p);
  const result = inspectLedgerReceipt(p, 'target-admitted', 'target', baseline, baselineBytes);
  const source = named(p, 'target-admitted').rows.find(r => r.table === 'root_preparations'
    && JSON.parse(r.keyCanonical).runId === JSON.parse(baseline.contexts.target.draftCanonical).runId);
  assert.equal(result, canonical(JSON.parse(source.valueCanonical).receipt));
  assert.equal(canonical(p), before);
});
test('other admitted run and workspace report ledgers survive target admission and discard unchanged', () => {
  const p = packet();
  const before = reportRows(named(p, 'target-draft'));
  assert.equal(before.length, 2);
  for (const name of ['target-admitted', 'target-never-initialized', 'target-cancelled']) {
    const rows = reportRows(named(p, name));
    for (const row of before) assert.deepEqual(rows.find(r => r.keyCanonical === row.keyCanonical), row);
  }
});

// Reseal latest-row/manifest commitments after a semantic mutation. The validator
// must then reject the wrong creation/effect rather than a stale outer digest.
function reseal(state) {
  const rows = state.rows.map(r => ({table: r.table, key: JSON.parse(r.keyCanonical), value: JSON.parse(r.valueCanonical)}));
  const project = r => {
    const body = structuredClone(r.value);
    if (!['store_header', 'commit_proof'].includes(r.table)) delete body.lastCommit;
    if (r.table === 'root_preparations') delete body.receipt.commit;
    if (r.table === 'summary_state' && body.checkpoint) delete body.checkpoint.applied;
    return body;
  };
  const digest = r => sha('orkworks.hierarchy-storage-row.v3\n' + canonical({table: r.table, key: r.key, body: project(r)}));
  const header = rows.find(r => r.table === 'store_header');
  const seq = header.value.latest.commitSequence;
  const proof = rows.find(r => r.table === 'commit_proof' && r.key === seq);
  for (const change of proof.value.body.changes) if (change.afterDigest !== null) {
    const row = rows.find(r => r.table === change.table && canonical(r.key) === canonical(change.key));
    if (row) change.afterDigest = digest(row);
  }
  proof.value.body.changes.sort((a, b) => Buffer.compare(Buffer.from(a.table + '\n' + canonical(a.key)), Buffer.from(b.table + '\n' + canonical(b.key))));
  proof.value.changeDigest = sha('orkworks.hierarchy-storage-change.v3\n' + canonical(proof.value.body));
  const update = ref => { if (ref?.commitSequence === seq) ref.changeDigest = proof.value.changeDigest; };
  update(header.value.latest);
  for (const r of rows) {
    update(r.value.lastCommit);
    if (r.table === 'root_preparations') update(r.value.receipt.commit);
    if (r.table === 'summary_state' && r.value.checkpoint) update(r.value.checkpoint.applied);
  }
  state.rows = rows.map(r => ({table: r.table, keyCanonical: canonical(r.key), valueCanonical: canonical(r.value), rowDigest: digest(r)}))
    .sort((a, b) => Buffer.compare(Buffer.from(a.table + '\n' + a.keyCanonical), Buffer.from(b.table + '\n' + b.keyCanonical)));
  state.stateSha256 = sha(canonical(state.rows));
}
function mutateReport(p, mutate) {
  const s = named(p, 'target-admitted');
  const row = reportRows(s).find(r => JSON.parse(r.keyCanonical).owner.runId === JSON.parse(baseline.contexts.target.draftCanonical).runId);
  const value = JSON.parse(row.valueCanonical);
  mutate(value, row, s);
  row.valueCanonical = canonical(value);
  reseal(s);
}
for (const [name, mutate] of [
  ['nonzero usage without a report producer', v => { v.used = '1'; }],
  ['reserved report credits', v => { v.reserved = '1'; }],
  ['unknown ledger field', v => { v.credits = '0'; }],
]) test(`self-consistent creation proof rejects ${name}`, () => {
  const p = packet(); mutateReport(p, mutate);
  assert.throws(() => check(p), /exact original creation recipe/);
});
test('self-consistent genesis cannot omit the workspace report ledger and its manifest member', () => {
  const p = packet(), s = named(p, 'initialized');
  s.rows = s.rows.filter(r => !(r.table === 'quota_usage' && JSON.parse(r.keyCanonical).resource === 'report-retained-bytes'));
  const proof = s.rows.find(r => r.table === 'commit_proof');
  const value = JSON.parse(proof.valueCanonical);
  value.body.changes = value.body.changes.filter(c => c.key.resource !== 'report-retained-bytes');
  proof.valueCanonical = canonical(value); reseal(s);
  assert.throws(() => check(p), /exact owner\/row inventory/);
});
test('legacy nine-ledger initialization is not adopted as report-capable', () => {
  const p = packet(); p.states[1] = structuredClone(baseline.states[1]);
  assert.throws(() => check(p), /exact owner\/row inventory/);
});
test('a zero workspace ledger relinked to later admission is not original creation proof', () => {
  const p = packet(), s = named(p, 'target-admitted');
  const row = reportRows(s).find(r => JSON.parse(r.keyCanonical).owner.kind === 'workspace');
  const v = JSON.parse(row.valueCanonical);
  v.lastCommit = JSON.parse(s.rows.find(r => r.table === 'store_header').valueCanonical).latest;
  row.valueCanonical = canonical(v); reseal(s);
  assert.throws(() => check(p), /exact zero ledger and original owner creation/);
});
test('orphan run report ledger cannot be invented during a pending draft', () => {
  const p = packet(), s = named(p, 'target-draft');
  const extra = structuredClone(reportRows(named(p, 'target-admitted')).find(r => JSON.parse(r.keyCanonical).owner.runId
    === JSON.parse(baseline.contexts.target.draftCanonical).runId));
  s.rows.push(extra); reseal(s);
  assert.throws(() => check(p), /exact owner\/row inventory/);
});
test('self-consistent source substitution cannot borrow the original admission proof', () => {
  const p = packet(), s = named(p, 'target-admitted');
  const row = s.rows.find(r => r.table === 'root_preparations' && JSON.parse(r.valueCanonical).bootstrap.runId
    === JSON.parse(baseline.contexts.target.draftCanonical).runId);
  const v = JSON.parse(row.valueCanonical); v.approval.approvalId = 'foreign'; row.valueCanonical = canonical(v); reseal(s);
  assert.throws(() => check(p), /exact original creation recipe/);
});
for (const key of ['commandBytes', 'changedRows']) test(`complete accounting rejects understated ${key}`, () => {
  const p = packet(); p.commands[4][key]--;
  assert.throws(() => check(p), key === 'commandBytes' ? /complete changed logical bytes/ : /complete changed-row count/);
});
test('exact measured whole-command capacity fits and one byte or row less denies', () => {
  const p = packet();
  const bytes = Math.max(...p.commands.map(c => c.commandBytes));
  assert.doesNotThrow(() => validateLedgerPacket(p, baseline, baselineBytes, {commandBytes: bytes, changedRows: 19}));
  assert.throws(() => validateLedgerPacket(p, baseline, baselineBytes, {commandBytes: bytes - 1, changedRows: 19}), /whole command capacity/);
  assert.throws(() => validateLedgerPacket(p, baseline, baselineBytes, {commandBytes: bytes, changedRows: 18}), /whole command capacity/);
});
for (const [name, mutate] of [
  ['extra command', p => p.commands.push(structuredClone(p.commands[0]))],
  ['duplicate state', p => { p.states[1] = structuredClone(p.states[0]); }],
  ['extra packet field', p => { p.runtimeEligible = true; }],
  ['wrong retained input hash', p => { p.baselineSha256 = '0'.repeat(64); }],
  ['wrong command kind', p => { p.commands[4].commandKind = 'root-draft-cancel'; }],
]) test(`finite witness rejects ${name}`, () => {
  assert.throws(() => { const p = packet(); mutate(p); check(p); });
});
test('baseline object cannot differ from the trusted retained bytes', () => {
  const other = structuredClone(baseline); other.contexts.target.retainedArtifacts = [];
  assert.throws(() => validateLedgerPacket(packet(), other, baselineBytes), /exact retained baseline input/);
});
