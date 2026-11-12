// Offline, finite creation witnesses only. Never import into application code.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSourceComponent} from './root-preparation-source-components.mjs';
import {validateEffectsState, validateEffectsTransition} from './root-preparation-effects.mjs';
import {canonical} from './report-publication-components.mjs';

const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const domain = (name, value) => sha(name + '\n' + canonical(value));
const size = value => Buffer.byteLength(canonical(value));
const order = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const identity = r => r.table + '\n' + canonical(r.key);
const clone = structuredClone;
const exact = (value, keys) => {
  assert(value && typeof value === 'object' && !Array.isArray(value), 'closed record');
  assert.deepEqual(Object.keys(value).sort(), keys.split(' ').sort(), 'closed fields');
};
const parse = text => decodeSourceComponent(Buffer.from(text));
function projection(r) {
  const value = clone(r.value);
  if (!['store_header', 'commit_proof'].includes(r.table)) delete value.lastCommit;
  if (r.table === 'root_preparations') delete value.receipt.commit;
  if (r.table === 'summary_state' && value.checkpoint) delete value.checkpoint.applied;
  return value;
}
const rowDigest = r => domain('orkworks.hierarchy-storage-row.v3', {table: r.table, key: r.key, body: projection(r)});
const reportLedger = (owner, ref) => ({table: 'quota_usage', key: {owner, resource: 'report-retained-bytes'},
  value: {used: '0', reserved: '0', lastCommit: ref}});
function decode(state) {
  exact(state, 'name rows stateSha256');
  assert(Array.isArray(state.rows) && state.rows.length <= 8192, 'bounded complete logical rows');
  const rows = new Map();
  let prior;
  let bytes = 0;
  let proofBytes = 0;
  let proofs = 0;
  for (const encoded of state.rows) {
    exact(encoded, 'table keyCanonical valueCanonical rowDigest');
    const r = {table: encoded.table, key: parse(encoded.keyCanonical), value: parse(encoded.valueCanonical)};
    const id = identity(r);
    assert(prior === undefined || order(prior, id) < 0, 'unique sorted rows');
    prior = id;
    assert.equal(encoded.rowDigest, rowDigest(r), 'row commitment');
    bytes += size(r.key) + size(r.value);
    if (r.table === 'commit_proof') { proofs++; proofBytes += size(r.key) + size(r.value); }
    rows.set(id, r);
  }
  assert(bytes <= 536870912 && proofBytes <= 67108864 && proofs <= 4096, 'retained inventory/proof capacity');
  assert.equal(state.stateSha256, sha(canonical(state.rows)), 'state fingerprint');
  return rows;
}
function validateExtendedState(state, original, baseline) {
  assert.equal(state.name, original.name, 'same retained scenario');
  const caps = baseline.caps[state.name === 'target-never-initialized' ? 'summary-full' : 'fitting'];
  validateEffectsState(original, baseline.contexts, caps);
  const before = decode(original);
  const actual = decode(state);
  if (before.size === 0) { assert.equal(actual.size, 0, 'no implicit initialization'); return actual; }
  const oldHeader = before.get('store_header\n1');
  const refs = new Map();
  const oldProofs = [...before.values()].filter(r => r.table === 'commit_proof')
    .sort((a, b) => BigInt(a.key) < BigInt(b.key) ? -1 : 1);
  const additions = [{sequence: '1', owner: {kind: 'workspace', workspaceId: oldHeader.value.workspaceId}}];
  for (const r of before.values()) if (r.table === 'root_preparations') {
    const b = r.value.bootstrap;
    additions.push({sequence: r.value.receipt.commit.commitSequence,
      owner: {kind: 'run', workspaceId: b.workspaceId, runId: b.runId}});
  }
  assert.equal(actual.size, before.size + additions.length, 'exact owner/row inventory');
  for (const old of oldProofs) {
    const found = actual.get(identity(old));
    assert(found, 'retained original proof');
    const expected = clone(old.value.body);
    if (expected.predecessor) expected.predecessor = refs.get(expected.predecessor.commitSequence);
    for (const extra of additions.filter(a => a.sequence === old.key)) {
      const row = reportLedger(extra.owner, null);
      expected.changes.push({table: row.table, key: row.key, beforeDigest: null, afterDigest: rowDigest(row)});
    }
    expected.changes.sort((a, b) => order(identity(a), identity(b)));
    const digest = domain('orkworks.hierarchy-storage-change.v3', expected);
    assert.deepEqual(found.value, {body: expected, changeDigest: digest}, 'exact original creation recipe');
    refs.set(old.key, {storageEpoch: expected.storageEpoch, commitSequence: old.key, changeDigest: digest});
  }
  const replace = old => {
    assert(old && refs.has(old.commitSequence), 'known original reference');
    return refs.get(old.commitSequence);
  };
  for (const old of before.values()) {
    const found = actual.get(identity(old));
    assert(found, 'all original rows retained');
    if (old.table === 'commit_proof') continue;
    const expected = clone(old.value);
    if (old.table === 'store_header') expected.latest = replace(expected.latest);
    else {
      expected.lastCommit = replace(expected.lastCommit);
      if (old.table === 'root_preparations') expected.receipt.commit = replace(expected.receipt.commit);
      if (old.table === 'summary_state' && expected.checkpoint) expected.checkpoint.applied = replace(expected.checkpoint.applied);
    }
    assert.deepEqual(found.value, expected, 'original source/ledger bytes and exact proof links');
  }
  for (const extra of additions) {
    const expected = reportLedger(extra.owner, refs.get(extra.sequence));
    assert.deepEqual(actual.get(identity(expected)), expected, 'exact zero ledger and original owner creation');
  }
  return actual;
}

// Validates only this finite extension of a retained synthetic packet. It cannot
// discover a real store inventory, authenticate a request or validate later report
// producers. baselineBytes are the caller's trusted retained documentation input.
export function validateLedgerPacket(packet, baseline, baselineBytes, limits = {commandBytes: 16777216, changedRows: 4096}) {
  exact(packet, 'schemaVersion classification baselineSha256 states commands unverified');
  assert.equal(packet.schemaVersion, 1);
  assert.equal(packet.classification, 'offline-fresh-report-ledger-creation-only');
  assert.deepEqual(baseline, JSON.parse(baselineBytes), 'exact retained baseline input');
  assert.equal(packet.baselineSha256, sha(baselineBytes), 'retained baseline bytes');
  assert.deepEqual(packet.unverified, ['real admission and report/terminal producers', 'complete consumers and physical indexes',
    'UI authority, lease, storage/native durability and executable plans']);
  exact(limits, 'commandBytes changedRows');
  assert(Number.isSafeInteger(limits.commandBytes) && limits.commandBytes > 0 && limits.commandBytes <= 16777216);
  assert(Number.isSafeInteger(limits.changedRows) && limits.changedRows > 0 && limits.changedRows <= 4096);
  assert(Array.isArray(packet.states) && packet.states.length === baseline.states.length, 'complete finite state set');
  assert(Array.isArray(packet.commands) && packet.commands.length === baseline.commands.length, 'complete finite command set');
  const states = new Map();
  for (let i = 0; i < baseline.states.length; i++) {
    const state = packet.states[i];
    assert(!states.has(state.name), 'unique state');
    states.set(state.name, validateExtendedState(state, baseline.states[i], baseline));
  }
  const results = [];
  for (let i = 0; i < baseline.commands.length; i++) {
    const original = baseline.commands[i];
    const command = packet.commands[i];
    exact(command, 'beforeState afterState commandKind subject summaryBranch caps manifestCanonical changeDigest changedRows commandBytes');
    const baseBefore = baseline.states.find(s => s.name === original.beforeState);
    const baseAfter = baseline.states.find(s => s.name === original.afterState);
    validateEffectsTransition(baseBefore, baseAfter, original, baseline.contexts, baseline.caps[original.caps]);
    for (const key of ['beforeState', 'afterState', 'commandKind', 'subject', 'summaryBranch', 'caps'])
      assert.deepEqual(command[key], original[key], 'same original conditional command');
    const before = states.get(command.beforeState);
    const after = states.get(command.afterState);
    const changed = [...new Set([...before.keys(), ...after.keys()])]
      .filter(key => canonical(before.get(key) ?? null) !== canonical(after.get(key) ?? null));
    assert.equal(command.changedRows, changed.length, 'complete changed-row count');
    const bytes = changed.reduce((sum, key) => {
      const row = after.get(key) ?? before.get(key);
      return sum + size(row.key) + (after.has(key) ? size(row.value) : 0);
    }, 0);
    assert.equal(command.commandBytes, bytes, 'complete changed logical bytes');
    assert(bytes <= limits.commandBytes && changed.length <= limits.changedRows, 'whole command capacity');
    const ref = after.get('store_header\n1').value.latest;
    const manifest = parse(command.manifestCanonical);
    const proofId = 'commit_proof\n' + canonical(ref.commitSequence);
    assert.deepEqual(after.get(proofId).value, {body: manifest, changeDigest: command.changeDigest}, 'exact command proof');
    assert.equal(ref.changeDigest, command.changeDigest);
    const effects = changed.filter(key => key !== 'store_header\n1' && key !== proofId).map(key => {
      const old = before.get(key), next = after.get(key), row = next ?? old;
      return {table: row.table, key: row.key, beforeDigest: old ? rowDigest(old) : null, afterDigest: next ? rowDigest(next) : null};
    }).sort((a, b) => order(identity(a), identity(b)));
    assert.deepEqual(manifest.changes, effects, 'complete before/after manifest');
    results.push({afterState: command.afterState, changedRows: changed.length, commandBytes: bytes, changeDigest: command.changeDigest});
  }
  return results;
}

export function inspectLedgerReceipt(packet, stateName, subject, baseline, baselineBytes) {
  validateLedgerPacket(packet, baseline, baselineBytes);
  const state = packet.states.find(s => s.name === stateName);
  assert(state, 'known exact state');
  const context = baseline.contexts[subject];
  assert(context, 'known exact subject');
  const runId = parse(context.draftCanonical).runId;
  const source = [...decode(state).values()].find(r => r.table === 'root_preparations' && r.value.bootstrap.runId === runId);
  assert(source, 'retained admitted source required');
  return canonical(source.value.receipt); // Byte inspection, never an authenticated retry.
}
