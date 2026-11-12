import assert from 'node:assert/strict';
import test from 'node:test';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {readFileSync,mkdtempSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {validateResearchChain} from './research-chain-components.mjs';
const generate=()=>execFileSync('python3',[fileURLToPath(new URL('generate-research-chain.py',import.meta.url))]);
test('independent generator supplies research-specific original source records',()=>{
  const p=JSON.parse(generate());assert.equal(p.plan?.kind,'single-task-retained-file-research');
  assert.equal(p.reservation.expectedTaskVersion,1);assert.equal(p.reservation.assignmentTaskVersion,2);
});
test('offline chain validator reports complete source-body measurements',()=>{
  // Literal byte counts independently computed with Python canonical UTF-8.
  assert.deepEqual(validateResearchChain(JSON.parse(generate())),{planBytes:1651,approvedPlanBytes:18193,sourceBodyBytes:10305,metadataWrapperBytes:[1825,1609,2142,1650]});
});
const retained=readFileSync(new URL('research-chain-components.json',import.meta.url));
const packet=()=>JSON.parse(retained);
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const domain=(name,x)=>sha(name+'\n'+canonical(x));
function seal(x,field,name){delete x[field];x[field]=domain(name,x);}
function fingerprintsAndRows(p){
  const a=p.approval,r=p.reservation,al=p.allocation,at=p.attachment,commits=p.unqualified.metadataCommits;
  p.fingerprints={requestDigest:domain('orkworks.research-source-plan-request.v1',p.request),approvalDigest:domain('orkworks.research-source-plan-approval.v1',a),
    assignmentSourceDigest:domain('orkworks.research-assignment-source.v1',p.assignment),evidenceSourceDigests:p.startup.evidenceObjects.map(e=>({referenceId:e.referenceId,sourceDigest:domain('orkworks.research-source-evidence.v1',e)}))};
  p.metadata=[{table:'research_plan_approvals',key:{plan:a.plan,approvalId:a.approvalId},value:{schemaVersion:1,body:a,digest:p.fingerprints.approvalDigest,receipt:p.receipt,lastCommit:commits.approval}},
    {table:'research_reservations',key:r.task,value:{schemaVersion:1,body:r,lastCommit:commits.reservation}},
    {table:'research_allocations',key:al.reservation,value:{schemaVersion:1,body:al,lastCommit:commits.allocation}},
    {table:'research_attachments',key:r.childSessionId,value:{schemaVersion:1,body:at,lastCommit:commits.attachment}}];
}
// Test-only forward relinking makes semantic counter/branch/plan mutations
// self-consistent without an old downstream reference masking the denial.
function relink(p){
  const c=p.startup.configuration,t=p.startup.scope.task,configuration={configurationId:c.configurationId,configurationDigest:c.configurationDigest};
  const scope={task:t,scopeDigest:p.startup.scope.scopeDigest};
  seal(p.plan,'planDefinitionDigest','orkworks.research-source-plan.v1');
  const plan={workspaceId:t.workspaceId,runId:t.runId,planId:t.planId,planRevision:t.planRevision,planDefinitionDigest:p.plan.planDefinitionDigest};
  p.request.plan=structuredClone(plan);p.approval.plan=structuredClone(plan);p.approval.requestDigest=domain('orkworks.research-source-plan-request.v1',p.request);
  const approval={plan,approvalId:p.approval.approvalId,approvalDigest:domain('orkworks.research-source-plan-approval.v1',p.approval)};
  p.receipt={schemaVersion:1,plan,approvalId:p.approval.approvalId,approvalDigest:approval.approvalDigest,requestDigest:p.approval.requestDigest,commit:p.unqualified.metadataCommits.approval};
  p.reservation.approval=structuredClone(approval);seal(p.reservation,'reservationDigest','orkworks.research-source-reservation.v1');
  const reservation={task:t,reservationId:p.reservation.reservationId,reservationDigest:p.reservation.reservationDigest};
  p.allocation.reservation=structuredClone(reservation);seal(p.allocation.allocatedWorktree,'worktreeBindingDigest','orkworks.worktree-binding.v1');seal(p.allocation,'allocationDigest','orkworks.research-source-allocation.v1');
  const allocation={reservation,allocationId:p.allocation.allocationId,allocationDigest:p.allocation.allocationDigest};
  const identity={kind:'child',...t,taskVersion:p.reservation.assignmentTaskVersion,reservationId:p.reservation.reservationId,parentSessionId:c.parentSessionId,childSessionId:p.reservation.childSessionId,...configuration};
  p.attachment.reservation=structuredClone(reservation);p.attachment.allocation=structuredClone(allocation);p.attachment.assignmentIdentity=structuredClone(identity);seal(p.attachment,'attachmentDigest','orkworks.research-source-attachment.v1');
  Object.assign(p.assignment,{assignmentIdentity:identity,plan,configuration:{task:t,...configuration},scope,approval,reservation,allocation,attachmentId:p.attachment.attachmentId});
  fingerprintsAndRows(p);
}
test('independent oracle regenerates and record checks leave it unchanged',()=>{
  assert.deepEqual(generate(),retained);const p=packet(),before=canonical(p);validateResearchChain(p);assert.equal(canonical(p),before);
});
test('generator paths with spaces and percent signs preserve exact UTF-8 LF bytes',()=>{
  const dir=mkdtempSync(join(tmpdir(),'research chain % '));
  try{
    for(const name of ['generate-research-chain.py','research-startup-components.json'])copyFileSync(fileURLToPath(new URL(name,import.meta.url)),join(dir,name));
    const output=join(dir,'out %.json');execFileSync('python3',[join(dir,'generate-research-chain.py'),'--output',output]);assert.deepEqual(readFileSync(output),retained);
  }finally{rmSync(dir,{recursive:true,force:true});}
});
for(const [name,mutate] of [
  ['direct-execution plan kind',p=>{p.plan.kind='single-task-child-source';}],
  ['unmatched parent run',p=>{p.plan.parentPlanBinding.runId='foreign';}],
  ['unmatched prepared input',p=>{p.plan.parentPlanBinding.inputDigest='0'.repeat(64);}],
  ['unknown prepared-input field',p=>{p.unqualified.preparedInputBinding.researchNeeded=false;}],
  ['parallel plan',p=>{p.plan.maxParallelChildren=2;}],
  ['second batch',p=>{p.plan.batches.push(['foreign']);}],
  ['foreign task',p=>{p.plan.task.taskId='foreign';}],
  ['foreign configuration',p=>{p.plan.task.configuration.configurationDigest='0'.repeat(64);}],
  ['foreign scope',p=>{p.plan.task.scope.scopeDigest='0'.repeat(64);}],
  ['opaque inline input',p=>{p.plan.task.input={};}],
  ['different initial prompt',p=>{p.plan.task.initialPrompt='foreign';}],
  ['main branch',p=>{p.plan.task.group.branchName='main';p.allocation.group=structuredClone(p.plan.task.group);}],
  ['traversal-like branch',p=>{p.plan.task.group.branchName='feature/../x';p.allocation.group=structuredClone(p.plan.task.group);}],
  ['foreign group',p=>{p.plan.task.group.groupId='foreign';p.allocation.group=structuredClone(p.plan.task.group);}],
  ['foreign base',p=>{p.plan.task.group.baseCommit='e'.repeat(40);p.allocation.group=structuredClone(p.plan.task.group);}],
  ['empty scope text',p=>{p.plan.scopeText='';}],
  ['legacy approval action',p=>{p.request.action='orkworks.child-source-plan.approve.v1';}],
  ['mixed approval ID',p=>{p.request.approvalId='new-approval';}],
  ['mixed review context',p=>{p.approval.reviewContextId='0'.repeat(64);}],
  ['zero expected version',p=>{p.reservation.expectedTaskVersion=0;}],
  ['nonincrementing frozen version',p=>{p.reservation.expectedTaskVersion=2;p.reservation.assignmentTaskVersion=2;}],
  ['fractional version',p=>{p.reservation.expectedTaskVersion=1.5;}],
  ['task version overflow',p=>{p.reservation.expectedTaskVersion=2147483647;p.reservation.assignmentTaskVersion=2147483648;}],
  ['live CAS field',p=>{p.reservation.currentTaskVersion=3;}],
  ['foreign reservation task',p=>{p.reservation.task.taskId='foreign';}],
  ['foreign reservation scope',p=>{p.reservation.scope.scopeDigest='0'.repeat(64);}],
  ['foreign reserved configuration',p=>{p.reservation.configurationId='foreign';}],
  ['different allocation group',p=>{p.allocation.group.groupId='foreign';}],
  ['source root object adopted',p=>{p.allocation.allocatedWorktree.rootObject=structuredClone(p.startup.configuration.sourceWorktreeBinding.rootObject);}],
  ['source private object adopted',p=>{p.allocation.allocatedWorktree.privateGitDirectoryObject=structuredClone(p.startup.configuration.sourceWorktreeBinding.privateGitDirectoryObject);}],
  ['unplanned allocated root',p=>{p.allocation.allocatedWorktree.canonicalRoot={encoding:'unix-bytes',base64:Buffer.from('/foreign').toString('base64')};}],
  ['unknown source field',p=>{p.assignment.currentTaskVersion=3;}],
])test(`self-consistent source chain denies ${name}`,()=>{const p=packet();mutate(p);relink(p);assert.throws(()=>validateResearchChain(p));});
for(const [name,mutate] of [
  ['receipt from another request',p=>{p.receipt.requestDigest='0'.repeat(64);}],
  ['receipt unknown field',p=>{p.receipt.accepted=true;}],
  ['zero commit counter',p=>{p.unqualified.metadataCommits.approval.commitSequence='0';}],
  ['noncanonical commit counter',p=>{p.unqualified.metadataCommits.approval.commitSequence='02';}],
  ['commit counter overflow',p=>{p.unqualified.metadataCommits.attachment.commitSequence='18446744073709551616';}],
  ['unresolved reservation source',p=>{p.assignment.reservation.reservationDigest='0'.repeat(64);}],
  ['wrong frozen attachment identity',p=>{p.attachment.assignmentIdentity.taskVersion=3;p.assignment.assignmentIdentity.taskVersion=3;seal(p.attachment,'attachmentDigest','orkworks.research-source-attachment.v1');}],
  ['criteria reordered',p=>{p.assignment.criteria.reverse();}],
  ['rubric changed',p=>{p.assignment.rubric.dimensions[0].description='foreign';}],
  ['wrong attachment reference',p=>{p.assignment.attachmentId='foreign';}],
  ['assignment scope alias',p=>{p.assignment.scope.scopeDigest='0'.repeat(64);}],
])test(`closed original-source joins deny ${name}`,()=>{const p=packet();mutate(p);fingerprintsAndRows(p);assert.throws(()=>validateResearchChain(p));});
for(const [name,mutate] of [
  ['missing metadata family',p=>p.metadata.pop()],
  ['extra metadata family',p=>p.metadata.push(structuredClone(p.metadata[0]))],
  ['wrong original commit link',p=>{p.metadata[1].value.lastCommit=p.unqualified.metadataCommits.attachment;}],
  ['extra metadata value field',p=>{p.metadata[1].value.runtimeActive=true;}],
  ['orphan metadata key',p=>{p.metadata[3].key='foreign';}],
  ['wrong assignment domain',p=>{p.fingerprints.assignmentSourceDigest=domain('orkworks.child-assignment-source.v1',p.assignment);}],
  ['wrong evidence source domain',p=>{p.fingerprints.evidenceSourceDigests[0].sourceDigest=domain('orkworks.child-source-evidence.v1',p.startup.evidenceObjects[0]);}],
  ['duplicate evidence source fingerprint',p=>p.fingerprints.evidenceSourceDigests.push({...p.fingerprints.evidenceSourceDigests[0]})],
  ['extra packet authority field',p=>{p.publicationEligible=true;}],
])test(`component packet denies ${name}`,()=>{const p=packet();mutate(p);assert.throws(()=>validateResearchChain(p));});
test('highest original expected/frozen pair fits without introducing a live report counter',()=>{
  const p=packet();p.reservation.expectedTaskVersion=2147483646;p.reservation.assignmentTaskVersion=2147483647;relink(p);assert.doesNotThrow(()=>validateResearchChain(p));
});
test('scope text accepts exactly 4 KiB UTF-8 and denies one more byte',()=>{
  const p=packet();p.plan.scopeText='é'.repeat(2048);relink(p);assert.doesNotThrow(()=>validateResearchChain(p));p.plan.scopeText+='x';relink(p);assert.throws(()=>validateResearchChain(p));
});
test('branch name accepts 256 bytes and denies one more',()=>{
  const p=packet();p.plan.task.group.branchName='x'.repeat(256);p.allocation.group=structuredClone(p.plan.task.group);relink(p);assert.doesNotThrow(()=>validateResearchChain(p));
  p.plan.task.group.branchName+='x';p.allocation.group=structuredClone(p.plan.task.group);relink(p);assert.throws(()=>validateResearchChain(p));
});
test('embedded startup packet is an exact unchanged retained source input',()=>{
  assert.deepEqual(packet().startup,JSON.parse(readFileSync(new URL('research-startup-components.json',import.meta.url))));
});
test('unqualified prepared-input revision accepts 64 and denies 65',()=>{
  const p=packet();p.unqualified.preparedInputBinding.preparationRevision=p.plan.parentPlanBinding.preparationRevision=64;relink(p);assert.doesNotThrow(()=>validateResearchChain(p));
  p.unqualified.preparedInputBinding.preparationRevision=p.plan.parentPlanBinding.preparationRevision=65;relink(p);assert.throws(()=>validateResearchChain(p));
});
test('CommitRef accepts the full uint64 maximum as syntax only',()=>{
  const p=packet();p.unqualified.metadataCommits.attachment.commitSequence='18446744073709551615';fingerprintsAndRows(p);assert.doesNotThrow(()=>validateResearchChain(p));
});
test('reservation retains its original approval rather than adopting a later ID',()=>{
  const p=packet();p.reservation.approval.approvalId='later-approval';seal(p.reservation,'reservationDigest','orkworks.research-source-reservation.v1');fingerprintsAndRows(p);assert.throws(()=>validateResearchChain(p),/original reserved approval/);
});
test('attachment alone cannot adopt a live version while the source stays originally frozen',()=>{
  const p=packet();p.attachment.assignmentIdentity.taskVersion=3;seal(p.attachment,'attachmentDigest','orkworks.research-source-attachment.v1');fingerprintsAndRows(p);
  assert.throws(()=>validateResearchChain(p),/original frozen assignment identity/);
});
test('self-digest records reject the old direct-execution source domains',()=>{
  for(const [key,field,oldDomain] of [['plan','planDefinitionDigest','orkworks.child-source-plan.v1'],['reservation','reservationDigest','orkworks.child-source-reservation.v1'],['allocation','allocationDigest','orkworks.child-source-allocation.v1'],['attachment','attachmentDigest','orkworks.child-source-attachment.v1']]){
    const p=packet();seal(p[key],field,oldDomain);fingerprintsAndRows(p);assert.throws(()=>validateResearchChain(p),new RegExp(field+' source digest'));
  }
});
const native=value=>({encoding:'unix-bytes',base64:Buffer.from(value).toString('base64')});
test('complete metadata key/value wrapper fits 16 KiB and one more byte denies',()=>{
  const p=packet();p.plan.task.group.absolutePath=native('/'+'g'.repeat(3599));p.allocation.group=structuredClone(p.plan.task.group);p.allocation.allocatedWorktree.canonicalRoot=structuredClone(p.plan.task.group.absolutePath);relink(p);
  const width=()=>Buffer.byteLength(canonical({key:p.metadata[2].key,value:p.metadata[2].value}));let best=0;
  for(let n=1;n<=4096;n++){
    p.allocation.allocatedWorktree.canonicalPrivateGitDirectory=native('/'+'p'.repeat(n-1));
    if(width()>16384)break;best=n;
  }
  assert(best>0&&best<4096);p.allocation.allocatedWorktree.canonicalPrivateGitDirectory=native('/'+'p'.repeat(best-1));
  p.plan.task.group.branchName+='x'.repeat(16384-width());p.allocation.group=structuredClone(p.plan.task.group);relink(p);
  assert.equal(width(),16384);assert.doesNotThrow(()=>validateResearchChain(p));
  p.plan.task.group.branchName+='x';p.allocation.group=structuredClone(p.plan.task.group);relink(p);assert.equal(width(),16385);assert.throws(()=>validateResearchChain(p),/metadata key\/value wrapper 16 KiB/);
});
test('equal artifact bytes under a separate retained reference remain a full occurrence',()=>{
  const p=packet(),before=validateResearchChain(p).approvedPlanBytes,c=p.startup.configuration,oldConfigBytes=Buffer.byteLength(canonical(c));
  const evidence=structuredClone(p.startup.evidenceObjects[3]),reference=structuredClone(c.capabilityEvidence.references[3]);
  evidence.referenceId=reference.id='evidence-4';p.startup.evidenceObjects.push(evidence);c.capabilityEvidence.references.push(reference);
  seal(c.capabilityEvidence,'evidenceDigest','orkworks.capability-evidence.v1');c.harness.evidenceDigest=c.capabilityEvidence.evidenceDigest;seal(c,'configurationDigest','orkworks.assignment-configuration.v3');
  p.plan.task.configuration.configurationDigest=p.reservation.configurationDigest=c.configurationDigest;relink(p);
  assert.equal(validateResearchChain(p).approvedPlanBytes,before+Buffer.byteLength(canonical(c))-oldConfigBytes+evidence.sizeBytes);
});
