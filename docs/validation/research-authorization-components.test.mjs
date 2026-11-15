import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {validateResearchAuthorization} from './research-authorization-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-authorization.py',import.meta.url))]);
test('Python derives allocated snapshots with frozen version distinct from live version',()=>{
  const p=JSON.parse(generate()),i=p.authorization?.body?.intent;
  assert.equal(i?.controls?.launch?.value?.body?.phase,'allocated');
  assert.equal(i.assignmentIdentity.taskVersion,2);
  assert.equal(i.controls.task.value.body.taskVersion,4);
  assert.equal(i.controls.run.value.body.runVersion,7);
});
test('checker denies empty packaging',()=>assert.throws(()=>validateResearchAuthorization({})));

import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {validateAuthorizationLimits} from './research-authorization-components.mjs';
const retained=readFileSync(new URL('research-authorization-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const domain=(name,x)=>hash(name+'\n'+canonical(x));
function reseal(p){
  p.authorization.digest=domain('orkworks.research-preparation-authorization.v1',p.authorization.body);
  p.resolution.authorizations[0].value=structuredClone(p.authorization);
}
test('retained oracle regenerates byte-exactly; checker preserves inputs and measures source charge',()=>{
  assert.deepEqual(generate(),retained);
  const p=packet(),before=canonical(p),result=validateResearchAuthorization(p);
  assert.equal(result.authorizationCharge,p.measurements.authorizationCharge);
  assert.equal(canonical(p),before);
});
for(const [name,change] of [
  ['child source family',i=>{i.producerSource.key.kind='child-assignment-source';}],
  ['frozen task version reset to live',i=>{i.assignmentIdentity.taskVersion=4;}],
  ['live task version reset to frozen',i=>{i.controls.task.value.body.taskVersion=2;}],
  ['running prestate',i=>{i.controls.launch.value.body.phase='running';}],
  ['existing attachment in allocated prestate',i=>{i.controls.launch.value.body.attachmentId=i.attachment.attachmentId;}],
  ['copied snapshot origin',i=>{i.controls.plan.value.lastCommit=i.expectedCommit;}],
  ['foreign snapshot key',i=>{i.controls.task.key.taskId='other';}],
  ['wrong original approval',i=>{i.controls.launch.value.body.admission.approval.approvalId='other';}],
  ['current H drift',i=>{i.expectedCommit.commitSequence='4';}],
  ['ordinary G drift',i=>{i.ordinary.historyGeneration='1';}],
  ['foreign parent generation',i=>{i.authority.parentRuntime.sidecarGeneration='0'.repeat(64);}],
  ['wrong deterministic attachment',i=>{i.attachment.attachmentDigest='0'.repeat(64);}],
  ['unknown intent field',i=>{i.spawnAllowed=true;}],
])test('resealed intent denies '+name,()=>{const p=packet();change(p.authorization.body.intent);reseal(p);assert.throws(()=>validateResearchAuthorization(p),/whole-body original research source|frozen original P|allocated .* snapshot|captured H|captured ordinary|original parent source authority|deterministic intended new attachment|fields|keys/);});
for(const [name,change] of [
  ['body byte drift',p=>{p.authorization.body.intent.ordinary.createdAt+='!';}],
  ['child digest domain',p=>{p.authorization.digest=domain('orkworks.child-preparation-authorization.v1',p.authorization.body);}],
  ['zero permit sequence',p=>{p.authorization.body.permit.sequence='0';reseal(p);}],
  ['counter leading zero',p=>{p.authorization.body.permit.sequence='01';reseal(p);}],
  ['permit overflow',p=>{p.authorization.body.permit.sequence='18446744073709551616';reseal(p);}],
  ['mismatched authorization key',p=>{p.authorizationKey.taskVersion=3;}],
  ['duplicate P across families',p=>{p.resolution.authorizations.push({...structuredClone(p.resolution.authorizations[0]),family:'child_preparation_authorizations'});}],
  ['same permit for another P',p=>{const r=structuredClone(p.resolution.authorizations[0]);r.key.childSessionId='different';p.resolution.authorizations.push(r);}],
  ['unknown source command',p=>{p.resolution.originalCommand.commandKind='caller-attach';}],
  ['wrong child command owner',p=>{p.resolution.originalCommand.commandKind='child-source-attach';}],
  ['missing original command',p=>{delete p.resolution.originalCommand;}],
  ['original C mismatch',p=>{p.resolution.originalCommand.commit.commitSequence='5';}],
  ['authorization digest used as source',p=>{p.ordinaryMember.assignmentSourceDigest=p.authorization.digest;}],
  ['source-kind member extension',p=>{p.ordinaryMember.sourceKind='research';}],
  ['historical creation string changed',p=>{p.resolution.lineage.createdAt='new';}],
  ['source assignment byte drift',p=>{p.resolution.assignment.criteria=[];}],
  ['synthesized proof claim',p=>{p.resolution.verified=true;}],
  ['new storage format',p=>{p.formatVersion=7;}],
  ['volatile close claims spawn',p=>{p.schedules[1].expected='may-save/and-spawn';}],
  ['unknown outcome claims rollback',p=>{p.schedules[6].expected='rollback/retry';}],
  ['missing native assumption',p=>{p.assumptions.shift();}],
  ['forged charge measurement',p=>{p.measurements.authorizationCharge--;}],
])test('component denies '+name,()=>{const p=packet();change(p);p.resolution.authorizations[0].value=structuredClone(p.authorization);assert.throws(()=>validateResearchAuthorization(p));});
test('width checks enforce lower ordinary record/job and smaller control limits independently',()=>{
  const p=packet();assert.doesNotThrow(()=>validateAuthorizationLimits(p,1048576,1048576));
  assert.throws(()=>validateAuthorizationLimits(p,1048577,1048576),/ordinary record/);
  assert.throws(()=>validateAuthorizationLimits(p,1048576,1048577),/joint job/);
  for(const [k,max] of [['run',1048576],['plan',16384],['task',16384],['launch',32768]]){
    const q=packet();q.authorization.body.intent.controls[k].value.padding='x'.repeat(max);
    assert.throws(()=>validateAuthorizationLimits(q,1,1),/control wrapper/);
  }
  const q=packet();q.authorization.body.padding='x'.repeat(2097152);
  assert.throws(()=>validateAuthorizationLimits(q,1,1),/authorization wrapper/);
});
test('generator resolves input beside its script under spaces and percent paths',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research authorization % '));
  try{
    for(const name of ['generate-research-authorization.py','research-prefix-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const output=join(dir,'out %.json');execFileSync('python3',[join(dir,'generate-research-authorization.py'),'--output',output]);
    assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});

test('literal Python measurements and whole-body digest remain independently pinned',()=>{
  const p=packet();assert.deepEqual(validateResearchAuthorization(p),{
    controlWrapperBytes:[2277,1348,989,2742],authorizationCharge:10439,
    authorizationWrapperBytes:10639,ordinaryMemberBytes:803,suppliedJointPartsBytes:11033});
  assert.equal(p.authorization.digest,'162c06f4b3e8f028d1eb1be3fa47605c6d2aba6c1bcbb38fe7d2ed961bb152f8');
});
test('a width-only run at its wrapper ceiling can still exceed the independent joint job limit',()=>{
  const p=packet(),r=p.authorization.body.intent.controls.run;
  r.value.widthOnlyPadding='';
  r.value.widthOnlyPadding='x'.repeat(1048576-Buffer.byteLength(canonical({key:r.key,value:r.value})));
  assert.equal(Buffer.byteLength(canonical({key:r.key,value:r.value})),1048576);
  assert(Buffer.byteLength(canonical({key:p.authorizationKey,value:p.authorization}))<2097152);
  const lowerBound=Buffer.byteLength(canonical({authorization:p.authorization,ordinaryMember:p.ordinaryMember}));
  assert(lowerBound>1048576);
  assert.throws(()=>validateAuthorizationLimits(p,803,lowerBound),/joint job/);
});
