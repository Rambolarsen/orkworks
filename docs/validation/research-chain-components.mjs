// Offline synthetic source components only; no admission, history or authority.
import assert from 'node:assert/strict';
import {canonical,hash,exact,text,id,digest,integer,nativePath,worktree} from './root-bootstrap-components.mjs';
import {validateResearchStartup} from './research-startup-components.mjs';
const same=(a,b,label='exact original research source join')=>assert.deepEqual(a,b,label);
const bytes=x=>Buffer.byteLength(canonical(x));
const size=(x,max,label)=>assert(bytes(x)<=max,label);
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
function sealed(x,field,name){
  digest(x[field]);const body={...x};delete body[field];assert.equal(x[field],domain(name,body),field+' source digest');
}
function commit(x){
  exact(x,'storageEpoch commitSequence changeDigest');digest(x.storageEpoch);digest(x.changeDigest);
  assert(typeof x.commitSequence==='string'&&/^[1-9][0-9]{0,19}$/.test(x.commitSequence)&&BigInt(x.commitSequence)<=18446744073709551615n,'positive uint64 commit Counter');
}
function branch(x){
  text(x,256);
  assert(!['main','refs/heads/main','HEAD','@'].includes(x)&&!x.startsWith('-')&&!/[\x00-\x20\x7f~^:?*\[\\]/.test(x)&&!x.includes('..')&&!x.includes('@{'),'selected branch syntax');
  assert(x.split('/').every(p=>p&&!p.startsWith('.')&&!p.endsWith('.')&&!p.endsWith('.lock')),'selected branch components');
}

export function validateResearchChain(p){
  exact(p,'startup unqualified plan request approval receipt reservation allocation attachment assignment fingerprints metadata');
  const measured=validateResearchStartup(p.startup),c=p.startup.configuration,run=p.startup.context.run,t=p.startup.scope.task;
  exact(p.unqualified,'preparedInputBinding metadataCommits');
  const input=p.unqualified.preparedInputBinding,commits=p.unqualified.metadataCommits;
  exact(input,'preparationRevision inputDigest');integer(input.preparationRevision,1,64);digest(input.inputDigest);
  exact(commits,'approval reservation allocation attachment');Object.values(commits).forEach(commit);
  const scope={task:t,scopeDigest:p.startup.scope.scopeDigest},configuration=pick(c,'configurationId configurationDigest');
  const plan=p.plan;exact(plan,'schemaVersion kind parentPlanBinding task batches maxParallelChildren scopeText planDefinitionDigest');
  assert.equal(plan.schemaVersion,1);assert.equal(plan.kind,'single-task-retained-file-research');
  const binding=plan.parentPlanBinding;
  exact(binding,'runId parentSessionId planId planRevision runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest preparationRevision inputDigest');
  same(binding,{...pick(run,'runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),planId:c.planId,planRevision:c.planRevision,...input},'supplied original parent/input binding');
  exact(plan.task,'taskId configuration scope initialPrompt group');assert.equal(plan.task.taskId,c.taskId);
  same(plan.task.configuration,configuration,'original configuration');same(plan.task.scope,scope,'original scope');assert.equal(plan.task.initialPrompt,c.assignment.description,'initial prompt');
  const g=plan.task.group;exact(g,'groupId branchName absolutePath baseCommit');
  assert.equal(g.groupId,c.assignment.worktreeGroupId,'original group');assert.equal(g.baseCommit,c.assignment.baseCommit,'original base');branch(g.branchName);nativePath(g.absolutePath);
  same(plan.batches,[[c.taskId]],'one task/batch');assert.equal(plan.maxParallelChildren,1);text(plan.scopeText,4096);
  sealed(plan,'planDefinitionDigest','orkworks.research-source-plan.v1');size(plan,65536,'plan 64 KiB');
  // Count the bodies actually supplied in the approved source package once per
  // retained occurrence. Digest/reference bytes are already in their bodies;
  // they are not an additional retained copy of each target artifact.
  const approvedPlanBytes=bytes(plan)+measured.configurationBytes+bytes(p.startup.scope)
    +p.startup.captures.reduce((n,v)=>n+bytes(v),0)+measured.rawEvidenceBytes;
  assert(approvedPlanBytes<=2097152,'approved source-plan body capacity');
  const planRef={...pick(t,'workspaceId runId planId planRevision'),planDefinitionDigest:plan.planDefinitionDigest};
  const request=p.request;exact(request,'schemaVersion action plan approvalId reviewContextId');assert.equal(request.schemaVersion,1);
  assert.equal(request.action,'orkworks.research-source-plan.approve.v1');same(request.plan,planRef);id(request.approvalId);digest(request.reviewContextId);
  const requestDigest=domain('orkworks.research-source-plan-request.v1',request),a=p.approval;
  exact(a,'schemaVersion plan approvalId reviewContextId sidecarGeneration requestDigest');assert.equal(a.schemaVersion,1);same(a.plan,planRef);
  assert.equal(a.approvalId,request.approvalId,'original approval ID');assert.equal(a.reviewContextId,request.reviewContextId,'exact review context');
  digest(a.sidecarGeneration);assert.equal(a.requestDigest,requestDigest,'research approval request domain');
  const approvalDigest=domain('orkworks.research-source-plan-approval.v1',a),approvalRef={plan:planRef,approvalId:a.approvalId,approvalDigest};
  same(p.receipt,{schemaVersion:1,plan:planRef,approvalId:a.approvalId,approvalDigest,requestDigest,commit:commits.approval},'original approval receipt');
  size(p.receipt,4096,'approval receipt 4 KiB');
  const r=p.reservation;exact(r,'schemaVersion task scope reservationId childSessionId configurationId configurationDigest approval expectedTaskVersion assignmentTaskVersion reservationDigest');
  assert.equal(r.schemaVersion,1);same(r.task,t);same(r.scope,scope);id(r.reservationId);id(r.childSessionId);
  same(pick(r,'configurationId configurationDigest'),configuration);same(r.approval,approvalRef,'original reserved approval');
  integer(r.expectedTaskVersion,1,2147483647);integer(r.assignmentTaskVersion,1,2147483647);
  assert.equal(r.expectedTaskVersion+1,r.assignmentTaskVersion,'original expected/frozen version pair');
  sealed(r,'reservationDigest','orkworks.research-source-reservation.v1');
  const reservationRef={task:t,...pick(r,'reservationId reservationDigest')},al=p.allocation;
  exact(al,'schemaVersion reservation allocationId group allocatedWorktree allocationDigest');assert.equal(al.schemaVersion,1);same(al.reservation,reservationRef);id(al.allocationId);same(al.group,g,'original allocated group');
  worktree(al.allocatedWorktree,c.repositoryBinding);const allocated=al.allocatedWorktree,source=c.sourceWorktreeBinding;
  assert.equal(allocated.kind,'linked');same(allocated.canonicalRoot,g.absolutePath,'planned allocated root');
  for(const k of ['canonicalRoot','canonicalPrivateGitDirectory','rootObject','privateGitDirectoryObject'])
    assert.notEqual(canonical(allocated[k]),canonical(source[k]),'distinct allocated '+k+' descriptor');
  sealed(al,'allocationDigest','orkworks.research-source-allocation.v1');
  const allocationRef={reservation:reservationRef,...pick(al,'allocationId allocationDigest')};
  const identity={kind:'child',...t,taskVersion:r.assignmentTaskVersion,reservationId:r.reservationId,parentSessionId:c.parentSessionId,childSessionId:r.childSessionId,...configuration};
  const at=p.attachment;exact(at,'schemaVersion attachmentId reservation allocation assignmentIdentity attachmentDigest');assert.equal(at.schemaVersion,1);id(at.attachmentId);
  same(at.reservation,reservationRef);same(at.allocation,allocationRef);same(at.assignmentIdentity,identity,'original frozen assignment identity');
  sealed(at,'attachmentDigest','orkworks.research-source-attachment.v1');
  const criteria=[...c.assignment.criteria].sort((a,b)=>Buffer.compare(Buffer.from(a.id),Buffer.from(b.id)));
  same(p.assignment,{schemaVersion:1,assignmentIdentity:identity,plan:planRef,configuration:{task:t,...configuration},scope,approval:approvalRef,reservation:reservationRef,allocation:allocationRef,attachmentId:at.attachmentId,criteria,rubric:c.rubric},'complete original assignment source projection');
  size(p.assignment,2097152,'assignment body ceiling');
  const assignmentSourceDigest=domain('orkworks.research-assignment-source.v1',p.assignment);
  const evidenceSourceDigests=p.startup.evidenceObjects.map(e=>({referenceId:e.referenceId,sourceDigest:domain('orkworks.research-source-evidence.v1',e)}));
  same(p.fingerprints,{requestDigest,approvalDigest,assignmentSourceDigest,evidenceSourceDigests},'whole-body research source domains');
  // Width examples only. The supplied CommitRefs have no proof manifests,
  // authority, causal ordering or storage-format qualification in this packet.
  const metadata=[{table:'research_plan_approvals',key:{plan:planRef,approvalId:a.approvalId},value:{schemaVersion:1,body:a,digest:approvalDigest,receipt:p.receipt,lastCommit:commits.approval}},
    {table:'research_reservations',key:t,value:{schemaVersion:1,body:r,lastCommit:commits.reservation}},
    {table:'research_allocations',key:reservationRef,value:{schemaVersion:1,body:al,lastCommit:commits.allocation}},
    {table:'research_attachments',key:r.childSessionId,value:{schemaVersion:1,body:at,lastCommit:commits.attachment}}];
  same(p.metadata,metadata,'exact metadata family/key/value examples');
  const metadataWrapperBytes=metadata.map(({key,value})=>bytes({key,value}));
  assert(metadataWrapperBytes.every(n=>n<=16384),'complete metadata key/value wrapper 16 KiB');
  return {planBytes:bytes(plan),approvedPlanBytes,sourceBodyBytes:[plan,request,a,p.receipt,r,al,at,p.assignment].reduce((n,v)=>n+bytes(v),0),metadataWrapperBytes};
}
