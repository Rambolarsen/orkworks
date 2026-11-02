// Independent synthetic golden bytes, no live sources or model calls.
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createHash} from 'node:crypto';
import {verifyChildSources, decodeChildSource, renderChildInstructions, declaredStartup, wholeBodySourceDigest} from './child-source-components.mjs';
const packet = JSON.parse(readFileSync(new URL('./child-source-vectors.json', import.meta.url)));
test('complete synthetic worker/reviewer joins remain unverified', () => {
  assert.deepEqual(verifyChildSources(structuredClone(packet)), {
    classification: 'unverified-source-bytes-only', configurations: 2, sourceObjects: 2,
    sourceDigests: ['worker-source','reviewer-source'].map(id => packet.vectors.find(v => v.id === id).domainDigest)
  });
});

const rejectCases = [
  ['extra configuration field', p => p.worker.configuration.grant = true],
  ['root version as child', p => p.worker.configuration.schemaVersion = 2],
  ['root role substitution', p => p.worker.configuration.roleTemplate.role = 'orchestrator'],
  ['unavailable native evidence promoted', p => p.worker.configuration.capabilityEvidence.decision = 'verified'],
  ['live filesystem profile', p => p.worker.configuration.permissions.effective.readPaths.push('src')],
  ['shell capability', p => p.worker.configuration.permissions.requested.tools.push('shell')],
  ['native delegation', p => p.worker.configuration.permissions.effective.nativeDelegation = 'allowed'],
  ['task dependency', p => p.worker.configuration.assignment.dependencies.push('other')],
  ['reference input', p => p.worker.configuration.assignment.inputs[0].kind = 'reference'],
  ['inline byte drift', p => p.worker.configuration.assignment.inputs[0].content += 'x'],
  ['reviewer extra inline data', p => p.reviewer.configuration.assignment.inputs.push(p.worker.configuration.assignment.inputs[0])],
  ['optional-only criteria', p => p.worker.configuration.assignment.criteria.forEach(c => c.requirement = 'optional')],
  ['root scope substitution', p => p.worker.configuration.assignment.worktreeScope.kind = 'source-worktree'],
  ['missing mandatory skill', p => p.worker.configuration.skills = []],
  ['wrong manifest task', p => p.worker.configuration.requirementManifests[0].taskId = 'other'],
  ['rule content drift', p => p.worker.configuration.rules[0].content += 'x'],
  ['rendered child header drift', p => p.worker.configuration.renderedInstructions = p.worker.configuration.renderedInstructions.replace('CHILD', 'ROOT')],
  ['evidence bytes missing', p => p.worker.evidenceObjects.pop()],
  ['evidence byte drift', p => p.worker.evidenceObjects[0].contentBase64 += 'YQ=='],
  ['wrong evidence owner', p => p.worker.evidenceObjects[0].task.taskId = 'foreign'],
  ['settings masquerade as root', p => replaceArtifact(p.worker, 'child-startup-settings', x => x.kind = 'root-startup-settings')],
  ['duplicate supplied startup entry', p => replaceArtifact(p.worker, 'child-startup-settings', x => x.deliveryOrder.push({kind:'supplied'}))],
  ['review input omitted from startup', p => replaceArtifact(p.reviewer, 'child-startup-settings', x => x.deliveryOrder.pop())],
  ['review input before supplied', p => replaceArtifact(p.reviewer, 'child-startup-settings', x => x.deliveryOrder.reverse())],
  ['worker manufactured output', p => p.worker.configuration.outputContract.declarations.push({id:'out',kind:'report',maxBytes:1,source:{kind:'server-artifact'}})],
  ['reviewer missing evaluation output', p => p.reviewer.configuration.outputContract.declarations = []],
  ['unknown snapshot field', p => p.snapshot.current = true],
  ['wrong target result bytes', p => p.snapshot.result.candidateDigest = '0'.repeat(64)],
  ['foreign target identity', p => p.snapshot.target.assignmentIdentity.childSessionId = 'other'],
  ['snapshot criteria drift', p => p.snapshot.criteria.reverse()],
  ['snapshot rubric drift', p => p.snapshot.rubric.version = 2],
  ['report bytes drift', p => p.snapshot.evidence[0].report.limitations.push('changed')],
  ['report selected twice', p => p.snapshot.evidence.push(structuredClone(p.snapshot.evidence[0]))],
  ['insufficient review read limit', p => p.reviewer.configuration.reviewInput.maxReadBytes = 1],
  ['root subject review', p => p.reviewer.configuration.reviewInput.target.assignmentIdentity.kind = 'root'],
  ['plan has two tasks in batch', p => p.worker.plan.batches[0].push('other')],
  ['plan uses main branch', p => p.worker.plan.task.group.branchName = 'main'],
  ['plan input mismatch', p => p.worker.plan.task.input = p.reviewer.plan.task.input],
  ['plan prompt differs', p => p.worker.plan.task.initialPrompt += 'x'],
  ['approval context differs', p => p.worker.approval.reviewContextId = '0'.repeat(64)],
  ['receipt request differs', p => p.worker.receipt.requestDigest = '0'.repeat(64)],
  ['receipt unknown commit fields', p => p.worker.receipt.commit.commitDigest = '0'.repeat(64)],
  ['reservation version absent', p => delete p.worker.reservation.assignmentTaskVersion],
  ['reservation version overflow', p => p.worker.reservation.assignmentTaskVersion = 2147483648],
  ['reservation session differs', p => p.worker.reservation.childSessionId = 'foreign'],
  ['allocation source substituted', p => p.worker.allocation.allocatedWorktree = p.worker.configuration.sourceWorktreeBinding],
  ['allocation group differs', p => p.worker.allocation.group.branchName = 'other'],
  ['attachment identity differs', p => p.worker.attachment.assignmentIdentity.taskVersion++],
  ['attachment unique ID differs', p => p.worker.source.attachmentId = 'other'],
  ['source adds active flag', p => p.worker.source.active = true],
  ['source criteria drift', p => p.worker.source.criteria.reverse()],
  ['source plan differs', p => p.worker.source.plan.planDefinitionDigest = '0'.repeat(64)],
  ['reviewer and target share child', p => p.reviewer.reservation.childSessionId = p.worker.reservation.childSessionId],
];
for (const [name, mutate] of rejectCases) test('rejects ' + name, () => {
  const p = structuredClone(packet); mutate(p);
  assert.throws(() => verifyChildSources(p), {name:'AssertionError'});
});
const sha = x => createHash('sha256').update(x).digest('hex');
const order = (a,b) => Buffer.compare(Buffer.from(a),Buffer.from(b));
const wire = x => Array.isArray(x) ? '['+x.map(wire).join(',')+']' : x !== null && typeof x === 'object' ? '{'+Object.keys(x).sort(order).map(k => JSON.stringify(k)+':'+wire(x[k])).join(',')+'}' : JSON.stringify(x);
function reseal(x,field,domain) {
  const body={...x}; delete body[field]; x[field]=sha(domain+'\n'+wire(body));
}
// Rebind retained bytes, evidence references and the configuration so malformed
// settings reach semantic checks instead of failing an outer checksum first.
function replaceArtifact(owner,kind,mutate) {
  const e=owner.evidenceObjects.find(e => Buffer.from(e.contentBase64,'base64').toString().includes('"kind":"'+kind+'"'));
  const value=JSON.parse(Buffer.from(e.contentBase64,'base64')); mutate(value);
  const content=wire(value), before=e.contentDigest, after=sha(content);
  e.contentBase64=Buffer.from(content).toString('base64'); e.sizeBytes=Buffer.byteLength(content);
  function replace(x) {for (const k of Object.keys(x)) {if (x[k] === before) x[k]=after; else if (x[k] && typeof x[k] === 'object') replace(x[k]);}}
  replace(owner);
  reseal(owner.configuration.capabilityEvidence,'evidenceDigest','orkworks.capability-evidence.v1');
  owner.configuration.harness.evidenceDigest=owner.configuration.capabilityEvidence.evidenceDigest;
  reseal(owner.configuration,'configurationDigest','orkworks.assignment-configuration.v3');
}
for (const v of packet.vectors) test(v.id + ': independent exact canonical bytes and raw digest', () => {
  const value=decodeChildSource(Buffer.from(v.canonical));
  assert.equal(Buffer.byteLength(v.canonical),v.bytes); assert.equal(sha(v.canonical),v.sha256);
  if (v.domainDigest) assert.equal(wholeBodySourceDigest(v.kind,value),v.domainDigest);
  const p=structuredClone(packet);
  if (v.kind === 'review-snapshot') p.snapshot=value;
  else if (v.kind === 'result') p.snapshot.result=value;
  else {
    const owner=v.id.startsWith('worker-') ? p.worker : p.reviewer;
    if (v.kind === 'evidence-object') owner.evidenceObjects[Number(v.id.split('-').at(-1))]=value;
    else owner[v.kind]=value;
  }
  assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
});
for (const [name, wireBytes] of [
  ['duplicate keys','{"a":1,"a":2}'], ['escaped duplicate','{"a":1,"\\u0061":2}'],
  ['fraction','{"a":1.5}'], ['exponent','{"a":1e3}'], ['negative zero','{"a":-0}'],
  ['rounded integer','{"a":9007199254740993}'], ['unpaired surrogate','{"a":"\\ud800"}'],
  ['noncanonical order','{"b":1,"a":2}'], ['noncanonical whitespace','{ "a":1}'],
  ['invalid UTF-8',Buffer.from([0xff])],
]) test('wire rejects '+name, () => assert.throws(() => decodeChildSource(Buffer.from(wireBytes))));
for (const [name, mutate] of [
  ['label over 512 UTF-8 bytes', p => p.worker.configuration.assignment.label='é'.repeat(257)],
  ['description over 4 KiB', p => p.worker.configuration.assignment.description='x'.repeat(4097)],
  ['inline data over 4 KiB', p => p.worker.configuration.assignment.inputs[0].content='x'.repeat(4097)],
  ['33 inline inputs', p => p.worker.configuration.assignment.inputs=Array.from({length:33},(_,i) => ({...p.worker.configuration.assignment.inputs[0],id:'i'+i}))],
  ['configuration role version overflow', p => p.worker.configuration.roleTemplate.version=2147483648],
  ['rubric version overflow', p => p.worker.configuration.rubric.version=2147483648],
  ['parent revision overflow', p => p.worker.plan.parentPlanBinding.preparationRevision=65],
  ['plan scope over 4 KiB', p => p.worker.plan.scopeText='x'.repeat(4097)],
  ['Git base syntax', p => p.worker.configuration.assignment.baseCommit='B'.repeat(40)],
  ['branch traversal', p => p.worker.plan.task.group.branchName='feature/../other'],
  ['branch reflog syntax', p => p.worker.plan.task.group.branchName='feature@{1}'],
  ['branch lock suffix', p => p.worker.plan.task.group.branchName='feature.lock'],
  ['native path over 4096 bytes', p => p.worker.plan.task.group.absolutePath.base64=Buffer.from('/'+'x'.repeat(4096)).toString('base64')],
  ['native relative path', p => p.worker.plan.task.group.absolutePath.base64=Buffer.from('relative').toString('base64')],
  ['commit Counter precision loss', p => p.worker.receipt.commit.commitSequence=9007199254740992],
  ['commit Counter overflow', p => p.worker.receipt.commit.commitSequence='9223372036854775808'],
  ['rehashed reservation frozen version', p => {p.worker.reservation.assignmentTaskVersion++; reseal(p.worker.reservation,'reservationDigest','orkworks.child-source-reservation.v1');}],
  ['rehashed allocation wrong group', p => {p.worker.allocation.group.branchName='foreign'; reseal(p.worker.allocation,'allocationDigest','orkworks.child-source-allocation.v1');}],
  ['rehashed attachment wrong identity', p => {p.worker.attachment.assignmentIdentity.taskVersion++; reseal(p.worker.attachment,'attachmentDigest','orkworks.child-source-attachment.v1');}],
  ['rehashed plan changed prompt', p => {p.worker.plan.task.initialPrompt='Changed'; reseal(p.worker.plan,'planDefinitionDigest','orkworks.child-source-plan.v1');}],
  ['rehashed snapshot criteria order', p => {p.snapshot.criteria.reverse(); reseal(p.snapshot,'inputDigest','orkworks.child-review-input.v1');}],
]) test('rejects '+name, () => {const p=structuredClone(packet); mutate(p); assert.throws(() => verifyChildSources(p),{name:'AssertionError'});});
test('declared startup keeps data after supplied frames and preserves Unicode/NUL/CRLF', () => {
  const c=packet.reviewer.configuration;
  const settings=JSON.parse(Buffer.from(packet.reviewer.evidenceObjects[0].contentBase64,'base64'));
  const actual=declaredStartup(c,settings,packet.snapshot);
  const data='\nORKWORKS RETAINED REVIEW INPUT v1\n'+Buffer.byteLength(wire(packet.snapshot))+'\n'+wire(packet.snapshot)+'\n';
  assert.equal(actual,packet.reviewer.configuration.rules[0].content+c.renderedInstructions+data);
  assert.equal(renderChildInstructions(packet.worker.configuration),packet.worker.configuration.renderedInstructions);
  assert.match(actual,/café 🛠/); assert.match(actual,/\r\n/);
  assert.equal(packet.worker.configuration.assignment.inputs[0].content,'café\u0000\r\n');
});

function replaceDigest(owner,before,after) {
  for (const k of Object.keys(owner)) {
    if (owner[k] === before) owner[k]=after;
    else if (owner[k] && typeof owner[k] === 'object') replaceDigest(owner[k],before,after);
  }
}
// Arrangement only: recompute the explicit dependency chain after an isolated
// valid mutation. Golden bytes/digests above are independently supplied by Python.
function refreshOwner(owner) {
  function sealAndReplace(value,field,domain) {const before=value[field]; reseal(value,field,domain); replaceDigest(owner,before,value[field]);}
  const c=owner.configuration;
  sealAndReplace(c.capabilityEvidence,'evidenceDigest','orkworks.capability-evidence.v1');
  c.renderedInstructions=renderChildInstructions(c); c.renderedInstructionsDigest=sha(c.renderedInstructions);
  sealAndReplace(c,'configurationDigest','orkworks.assignment-configuration.v3');
  sealAndReplace(owner.plan,'planDefinitionDigest','orkworks.child-source-plan.v1');
  replaceDigest(owner,owner.approval.requestDigest,sha('orkworks.child-source-plan-request.v1\n'+wire(owner.request)));
  replaceDigest(owner,owner.receipt.approvalDigest,sha('orkworks.child-source-plan-approval.v1\n'+wire(owner.approval)));
  sealAndReplace(owner.reservation,'reservationDigest','orkworks.child-source-reservation.v1');
  sealAndReplace(owner.allocation,'allocationDigest','orkworks.child-source-allocation.v1');
  sealAndReplace(owner.attachment,'attachmentDigest','orkworks.child-source-attachment.v1');
}
function appendOpaqueEvidence(owner,content) {
  const referenceId='evidence-4',contentDigest=sha(content);
  owner.configuration.capabilityEvidence.references.push({id:referenceId,kind:'fixture',sourceReference:'synthetic:extra',contentDigest});
  owner.evidenceObjects.push({schemaVersion:1,task:structuredClone(owner.evidenceObjects[0].task),referenceId,
    contentBase64:Buffer.from(content).toString('base64'),sizeBytes:Buffer.byteLength(content),contentDigest});
  refreshOwner(owner);
}
for (const [name,content] of [['empty opaque source',''],['same bytes under distinct reference IDs','Synthetic unverified native evidence; no probe performed.']]) {
  test('retains '+name+' without alias capacity credit', () => {
    const p=structuredClone(packet); appendOpaqueEvidence(p.reviewer,content);
    assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
  });
}
test('inclusive inline and label byte bounds remain valid with fully rebound sources', () => {
  const p=structuredClone(packet); p.reviewer.configuration.assignment.label='é'.repeat(256); refreshOwner(p.reviewer);
  assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
});
for (const version of [1,2147483647]) test('original reservation task version '+version+' survives exact full identity joins', () => {
  const p=structuredClone(packet); p.reviewer.reservation.assignmentTaskVersion=version;
  p.reviewer.attachment.assignmentIdentity.taskVersion=version; p.reviewer.source.assignmentIdentity.taskVersion=version;
  refreshOwner(p.reviewer); assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
});
test('commit Counter beyond 2^53 remains exact TEXT without promoting it to proof', () => {
  const p=structuredClone(packet); p.reviewer.receipt.commit.commitSequence='9223372036854775807';
  assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
});
test('valid native path components can still overflow a complete metadata wrapper', () => {
  const p=structuredClone(packet),path={encoding:'unix-bytes',base64:Buffer.from('/'+'x'.repeat(4095)).toString('base64')};
  p.reviewer.plan.task.group.absolutePath=path; p.reviewer.allocation.group.absolutePath=path;
  p.reviewer.allocation.allocatedWorktree.canonicalRoot=path; p.reviewer.allocation.allocatedWorktree.canonicalPrivateGitDirectory=path;
  reseal(p.reviewer.allocation.allocatedWorktree,'worktreeBindingDigest','orkworks.worktree-binding.v1'); refreshOwner(p.reviewer);
  assert.throws(() => verifyChildSources(p),/metadata complete wrapper 16 KiB/);
});
test('rendered instruction ceiling counts complete frames inclusively', () => {
  const c=structuredClone(packet.worker.configuration);
  let remaining=262144-Buffer.byteLength(renderChildInstructions(c));
  const original=Buffer.byteLength(c.rules[0].content);
  remaining-=String(original+remaining).length-String(original).length;
  c.rules[0].content+='x'.repeat(remaining);
  assert.equal(Buffer.byteLength(renderChildInstructions(c)),262144);
  c.rules[0].content+='x'; assert.throws(() => renderChildInstructions(c),/rendered child byte ceiling/);
});
test('declared startup ceiling includes automatic content and exact input framing', () => {
  const c=structuredClone(packet.worker.configuration),s={deliveryOrder:[{kind:'supplied'}]};
  c.renderedInstructions='x'.repeat(262144); assert.equal(Buffer.byteLength(declaredStartup(c,s,null)),262144);
  s.discoveredInstructions=[{snapshotKind:'rule',snapshotId:c.rules[0].id}]; s.deliveryOrder.push({kind:'automatic',precedence:0});
  assert.throws(() => declaredStartup(c,s,null),/declared composed startup ceiling/);
});

test('independent Python generation reproduces the complete pinned packet', () => {
  const dir=mkdtempSync(join(tmpdir(),'ork-child-source-'));
  try {
    const output=join(dir,'vectors.json');
    const run=spawnSync('python3',[fileURLToPath(new URL('./generate-child-source-vectors.py',import.meta.url)),'--output',output],{encoding:'utf8',timeout:10000});
    assert.equal(run.status,0,run.stderr);
    assert.deepEqual(readFileSync(output),readFileSync(new URL('./child-source-vectors.json',import.meta.url)));
  } finally {rmSync(dir,{recursive:true,force:true});}
});

function verificationReviewer(owner) {
  const c=owner.configuration,old=c.roleTemplate.contentDigest;
  c.roleTemplate.role='verification'; const body={...c.roleTemplate}; delete body.contentDigest;
  replaceDigest(owner,old,sha(wire(body))); c.capabilityEvidence.role='verification';
}
test('reviewer cannot choose its own rubric to override the target evaluator role', () => {
  const p=structuredClone(packet); verificationReviewer(p.reviewer);
  p.reviewer.configuration.rubric.evaluatorRole='verification'; p.reviewer.source.rubric.evaluatorRole='verification';
  refreshOwner(p.reviewer);
  assert.throws(() => verifyChildSources(p),/target rubric reviewer role/);
});
test('verification reviewer eligibility comes from the immutable target rubric', () => {
  const p=structuredClone(packet),oldWorker=p.worker.configuration.configurationDigest;
  p.worker.configuration.rubric.evaluatorRole='verification'; p.worker.source.rubric.evaluatorRole='verification';
  refreshOwner(p.worker); replaceDigest(p,oldWorker,p.worker.configuration.configurationDigest);
  p.snapshot.rubric.evaluatorRole='verification';
  p.snapshot.target.resultDigest=sha('orkworks.assignment-result.v1\n'+wire(p.snapshot.result));
  const oldInput=p.snapshot.inputDigest; reseal(p.snapshot,'inputDigest','orkworks.child-review-input.v1');
  p.reviewer.configuration.reviewInput.target=structuredClone(p.snapshot.target);
  p.reviewer.configuration.reviewInput.maxReadBytes=['result','criteria','rubric'].reduce((n,k) => n+Buffer.byteLength(wire(p.snapshot[k])),0)+p.snapshot.evidence.reduce((n,pair) => n+Buffer.byteLength(wire(pair.report)),0);
  const oldReviewer=p.reviewer.configuration.configurationDigest;
  replaceArtifact(p.reviewer,'child-startup-settings',s => s.deliveryOrder.find(d => d.kind === 'review-input').inputDigest=p.snapshot.inputDigest);
  replaceDigest(p.reviewer,oldReviewer,p.reviewer.configuration.configurationDigest);
  replaceDigest(p.reviewer,oldInput,p.snapshot.inputDigest); verificationReviewer(p.reviewer); refreshOwner(p.reviewer);
  assert.equal(verifyChildSources(p).classification,'unverified-source-bytes-only');
});

test('rehashing cannot make the worker allocation an independent reviewer worktree', () => {
  const p=structuredClone(packet),w=p.worker.allocation;
  p.reviewer.plan.task.group.branchName=w.group.branchName;
  p.reviewer.plan.task.group.absolutePath=structuredClone(w.group.absolutePath);
  p.reviewer.allocation.group=structuredClone(p.reviewer.plan.task.group);
  p.reviewer.allocation.allocatedWorktree=structuredClone(w.allocatedWorktree); refreshOwner(p.reviewer);
  assert.throws(() => verifyChildSources(p),/reviewer allocation independence/);
});
