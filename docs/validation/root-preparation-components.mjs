// Documentation-only component checks. No bootstrap validation, admission or I/O.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const byteOrder = (a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b));
const isObject = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const exact = (x, fields) => {
  assert(isObject(x), 'expected an object');
  assert.deepEqual(Object.keys(x).sort(), fields.split(' ').sort(), 'closed record fields');
};
const id = x => assert(typeof x === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(x), 'ASCII ID');
const digest = x => assert(typeof x === 'string' && /^[a-f0-9]{64}$/.test(x), 'lowercase SHA-256');
const counter = x => {
  assert(typeof x === 'string' && /^[1-9][0-9]{0,18}$/.test(x), 'positive decimal string counter');
  assert(BigInt(x) <= 9223372036854775807n, 'counter range');
};

// Only the integer/string/object subset required by these closed components.
function canonical(x) {
  if (typeof x === 'string') {
    assert(x.isWellFormed(), 'unpaired surrogate');
    return JSON.stringify(x);
  }
  if (typeof x === 'number') {
    assert(Number.isSafeInteger(x), 'lossless JSON integer');
    return JSON.stringify(x);
  }
  assert(isObject(x), 'unsupported component value');
  return '{' + Object.keys(x).sort(byteOrder).map(k => canonical(k) + ':' + canonical(x[k])).join(',') + '}';
}
const domainHash = (domain, x) => hash(domain + '\n' + canonical(x));

// For retained canonical component bytes only. Equality also rejects duplicate
// keys, alternate number/string spellings and whitespace lost by JSON.parse.
export function decodeCanonical(bytes) {
  const text = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true}).decode(bytes);
  const value = JSON.parse(text);
  assert.equal(canonical(value), text, 'stored bytes must be canonical');
  return value;
}

export function validateRun(run) {
  exact(run, 'schemaVersion runId workspaceId parentSessionId repositoryBindingDigest sourceWorktreeBindingDigest goal goalDigest admissionEpoch runSequence bootstrapConfigurationDigest maxParallelChildren runDefinitionDigest');
  assert.equal(run.schemaVersion, 1);
  for (const field of ['runId', 'workspaceId', 'parentSessionId']) id(run[field]);
  for (const field of ['repositoryBindingDigest', 'sourceWorktreeBindingDigest', 'goalDigest', 'admissionEpoch', 'bootstrapConfigurationDigest', 'runDefinitionDigest']) digest(run[field]);
  counter(run.runSequence);
  assert.equal(run.runId, run.admissionEpoch + '-' + run.runSequence, 'run identity');
  assert(typeof run.goal === 'string' && run.goal.isWellFormed(), 'exact UTF-8 goal');
  assert.equal(hash(run.goal), run.goalDigest, 'goalDigest');
  assert(Number.isInteger(run.maxParallelChildren) && run.maxParallelChildren >= 1 && run.maxParallelChildren <= 16, 'child limit 1..16');
  const bytes = canonical(run);
  assert(Buffer.byteLength(bytes) <= 16384, 'run exceeds 16384 bytes');
  const {runDefinitionDigest, ...body} = run;
  assert.equal(domainHash('orkworks.orchestration-run.v1', body), runDefinitionDigest, 'runDefinitionDigest');
  return runDefinitionDigest;
}

export function validateApproval(approval, run) {
  validateRun(run);
  exact(approval, 'schemaVersion action workspaceId runId draftId draftDigest reviewContextId approvalId');
  assert.equal(approval.schemaVersion, 1);
  assert.equal(approval.action, 'orkworks.root-preparation.approve.v1');
  for (const field of ['workspaceId', 'runId', 'draftId', 'approvalId']) id(approval[field]);
  for (const field of ['draftDigest', 'reviewContextId']) digest(approval[field]);
  assert.equal(approval.workspaceId, run.workspaceId, 'workspace binding');
  assert.equal(approval.runId, run.runId, 'run binding');
  // These checks establish no draft bytes, review-context currentness, caller
  // authentication, fence reservation or referenced source validity.
  return domainHash('orkworks.root-preparation-request.v1', approval);
}
