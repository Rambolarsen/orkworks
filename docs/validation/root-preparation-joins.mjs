// Documentation-only synthetic preparation source joins. Never import in runtime.
// Matching bytes confer no current UI scope, eligibility, fence or commit proof.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSourceComponent} from './root-preparation-source-components.mjs';
import {validateApproval, validateRun} from './root-preparation-components.mjs';
import {validateRootBootstrap} from './root-bootstrap-components.mjs';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const order = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const exact = (x, fields) => {
  assert(object(x), 'record object');
  assert.deepEqual(Object.keys(x).sort(), fields.split(' ').sort(), 'closed fields');
};
const id = x => assert(typeof x === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(x), 'ASCII ID');
const digest = x => assert(typeof x === 'string' && /^[a-f0-9]{64}$/.test(x), 'Digest');
function canonical(x) {
  if (x === null || typeof x === 'boolean') return JSON.stringify(x);
  if (typeof x === 'string') { assert(x.isWellFormed(), 'unpaired surrogate'); return JSON.stringify(x); }
  if (typeof x === 'number') { assert(Number.isSafeInteger(x) && !Object.is(x, -0), 'lossless integer'); return JSON.stringify(x); }
  if (Array.isArray(x)) return '[' + x.map(canonical).join(',') + ']';
  assert(object(x), 'supported JSON');
  return '{' + Object.keys(x).sort(order).map(k => canonical(k) + ':' + canonical(x[k])).join(',') + '}';
}
const domainHash = (domain, value) => hash(domain + '\n' + canonical(value));
const bytes = x => Buffer.from(canonical(x));
const limit = (x, max, message) => assert(bytes(x).length <= max, message);
function commit(x) {
  exact(x, 'storageEpoch commitSequence changeDigest'); digest(x.storageEpoch); digest(x.changeDigest);
  assert(typeof x.commitSequence === 'string' && /^[1-9][0-9]{0,18}$/.test(x.commitSequence), 'positive Counter');
  assert(BigInt(x.commitSequence) <= 9223372036854775807n, 'Counter range');
}
function rootIdentity(b) {
  return {kind: 'root', ...Object.fromEntries(['workspaceId', 'runId', 'rootAssignmentId', 'parentSessionId', 'bootstrapId', 'bootstrapConfigurationDigest'].map(k => [k, b[k]]))};
}
function draft(d, context) {
  exact(d, 'schemaVersion draftId workspaceId runId parentSessionId rootAssignmentId sidecarGeneration reviewContextId bootstrap runDefinition draftDigest');
  assert.equal(d.schemaVersion, 1, 'draft version');
  for (const k of ['draftId', 'workspaceId', 'runId', 'parentSessionId', 'rootAssignmentId']) id(d[k]);
  digest(d.sidecarGeneration); digest(d.reviewContextId); digest(d.draftDigest);
  limit(d, 1081344, 'draft envelope ceiling');
  validateRun(d.runDefinition);
  validateRootBootstrap(d.bootstrap, {...context, run: d.runDefinition});
  for (const k of ['workspaceId', 'runId', 'parentSessionId', 'rootAssignmentId']) assert.equal(d[k], d.bootstrap[k], 'draft ' + k);
  const {draftDigest, ...body} = d;
  assert.equal(domainHash('orkworks.root-preparation-draft.v1', body), draftDigest, 'draftDigest');
  return draftDigest;
}
function reviewedDraft(context) {
  assert(context.reviewedDraftBytes instanceof Uint8Array, 'frozen draft bytes required');
  // Decode before interpretation, including UTF-8, duplicate-key and stored-order checks.
  return decodeSourceComponent(context.reviewedDraftBytes);
}
function approval(a, d, context) {
  draft(d, context);
  const requestDigest = validateApproval(a, d.runDefinition);
  for (const k of ['draftId', 'draftDigest', 'reviewContextId']) assert.equal(a[k], d[k], 'approval ' + k);
  return requestDigest;
}
function originalApproval(context, d) {
  assert(context.approvedRequestBytes instanceof Uint8Array, 'original approval bytes required');
  const a = decodeSourceComponent(context.approvedRequestBytes);
  approval(a, d, context); return a;
}
function receipt(r, d, a, requestDigest) {
  exact(r, 'schemaVersion workspaceId runId rootAssignmentIdentity bootstrapConfigurationDigest runDefinitionDigest draftDigest approvalId requestDigest commit');
  assert.equal(r.schemaVersion, 1, 'receipt version'); limit(r, 4096, 'receipt ceiling');
  assert.deepEqual(r.rootAssignmentIdentity, rootIdentity(d.bootstrap), 'root identity');
  for (const k of ['workspaceId', 'runId', 'draftDigest']) assert.equal(r[k], d[k], 'receipt ' + k);
  assert.equal(r.bootstrapConfigurationDigest, d.bootstrap.bootstrapConfigurationDigest, 'receipt bootstrapConfigurationDigest');
  assert.equal(r.runDefinitionDigest, d.runDefinition.runDefinitionDigest, 'receipt runDefinitionDigest');
  assert.equal(r.approvalId, a.approvalId, 'receipt approvalId');
  assert.equal(r.requestDigest, requestDigest, 'receipt requestDigest'); commit(r.commit);
}
function reconstruct(s) {
  return {schemaVersion: 1, draftId: s.approval.draftId, workspaceId: s.bootstrap.workspaceId,
    runId: s.bootstrap.runId, parentSessionId: s.bootstrap.parentSessionId, rootAssignmentId: s.bootstrap.rootAssignmentId,
    sidecarGeneration: s.approval.sidecarGeneration, reviewContextId: s.approval.reviewContextId,
    bootstrap: s.bootstrap, runDefinition: s.runDefinition, draftDigest: s.approval.draftDigest};
}
function source(s, context) {
  exact(s, 'schemaVersion bootstrap runDefinition approval receipt lastCommit');
  assert.equal(s.schemaVersion, 1, 'source version'); limit(s, 1081344, 'source envelope ceiling');
  exact(s.approval, 'approvalId draftId draftDigest reviewContextId sidecarGeneration requestDigest');
  id(s.approval.approvalId); id(s.approval.draftId);
  for (const k of ['draftDigest', 'reviewContextId', 'sidecarGeneration', 'requestDigest']) digest(s.approval[k]);
  const d = reconstruct(s); draft(d, context);
  const frozen = reviewedDraft(context); draft(frozen, context);
  assert.deepEqual(bytes(d), Buffer.from(context.reviewedDraftBytes), 'frozen draft bytes');
  const a = {schemaVersion: 1, action: 'orkworks.root-preparation.approve.v1',
    workspaceId: s.bootstrap.workspaceId, runId: s.bootstrap.runId,
    draftId: s.approval.draftId, draftDigest: s.approval.draftDigest,
    reviewContextId: s.approval.reviewContextId, approvalId: s.approval.approvalId};
  const requestDigest = approval(a, d, context);
  assert.equal(s.approval.requestDigest, requestDigest, 'requestDigest');
  const original = originalApproval(context, frozen);
  assert.deepEqual(bytes(a), bytes(original), 'original approval bytes');
  receipt(s.receipt, d, a, requestDigest); commit(s.lastCommit);
  assert.deepEqual(s.receipt.commit, s.lastCommit, 'source commit link');
  // Links are syntactically bound only. Their manifests, membership effects,
  // source history, native authority cutoff and storage provenance are unverified.
  return {draft: d, approval: a, requestDigest};
}
export function preparationRowDigest(table, key, value, context) {
  const body = structuredClone(value);
  if (table === 'root_drafts') {
    exact(value, 'schemaVersion draft lastCommit'); assert.equal(value.schemaVersion, 1, 'draft row version');
    draft(value.draft, context); commit(value.lastCommit);
    assert.equal(key, value.draft.workspaceId, 'draft membership key');
    delete body.lastCommit;
  } else {
    assert.equal(table, 'root_preparations', 'preparation table'); source(value, context);
    assert.deepEqual(key, {workspaceId: value.bootstrap.workspaceId, runId: value.bootstrap.runId}, 'source membership key');
    delete body.lastCommit; delete body.receipt.commit;
  }
  return domainHash('orkworks.hierarchy-storage-row.v3', {table, key, body});
}
export function validatePreparationComponent(kind, value, context) {
  switch (kind) {
    case 'draft': return draft(value, context);
    case 'approval': return approval(value, reviewedDraft(context), context);
    case 'receipt': {
      const d = reviewedDraft(context), a = originalApproval(context, d);
      receipt(value, d, a, approval(a, d, context)); break;
    }
    case 'source': source(value, context); break;
    case 'draft-row': return preparationRowDigest('root_drafts', context.sourceKey.workspaceId, value, context);
    case 'source-row': return preparationRowDigest('root_preparations', context.sourceKey, value, context);
    default: assert.fail('unsupported preparation component');
  }
  return hash(bytes(value)); // fixture fingerprint, not an authority or source domain
}
export function validatePreparationPacket(packet, context) {
  exact(packet, 'draftBytes approvalBytes sourceBytes');
  for (const value of Object.values(packet)) assert(value instanceof Uint8Array, 'canonical wire bytes required');
  const d = decodeSourceComponent(packet.draftBytes);
  // An exact-byte comparison is independent of the descriptor's stored digest.
  reviewedDraft(context);
  assert.deepEqual(Buffer.from(packet.draftBytes), Buffer.from(context.reviewedDraftBytes), 'frozen draft bytes');
  const draftDigest = draft(d, context);
  const a = decodeSourceComponent(packet.approvalBytes);
  const requestDigest = approval(a, d, context);
  assert.deepEqual(Buffer.from(packet.approvalBytes), Buffer.from(context.approvedRequestBytes), 'original approval bytes');
  const s = decodeSourceComponent(packet.sourceBytes); source(s, context);
  const sourceRowDigest = preparationRowDigest('root_preparations', context.sourceKey, s, context);
  return {classification: 'synthetic-preparation-joins-only', draftDigest, requestDigest, sourceRowDigest,
    rootAssignmentIdentity: rootIdentity(d.bootstrap)};
}
