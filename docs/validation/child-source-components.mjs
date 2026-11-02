// Documentation-only, synthetic source-byte checker. Never import into runtime.
// No native event, unique attachment, custody, currentness or authority proof.
import assert from 'node:assert/strict';
import {canonical, hash, exact, text, id, digest, integer, list, sorted, bindings,
  nativePath, repository, worktree, artifact,
  validateInstructionSnapshots, validateUnverifiedEvidence} from './root-bootstrap-components.mjs';
import {validateRun} from './root-preparation-components.mjs';
import {decodeSummary} from './hierarchy-summary-components.mjs';
import {validateSourceComponent, validateExecutableBinding, decodeSourceComponent} from './root-preparation-source-components.mjs';
import {validateReportEvidenceSource} from './r2-report-components.mjs';

const same = (a,b,message = 'exact source join') => assert.deepEqual(a,b,message);
const domain = (name,x) => hash(name + '\n' + canonical(x));
const size = (x,max,message='component byte ceiling') => assert(Buffer.byteLength(canonical(x)) <= max,message);
const baseTools = ['orkworks.result.receipt.read.v1','orkworks.result.submit.v1'];
const reviewTools = [...baseTools,'orkworks.evaluation.submit.v1','orkworks.evaluation.receipt.read.v1','orkworks.evaluation.artifact.read.v1'].sort();
const boundary = 'Act only within the approved child assignment. Binding repository rules remain mandatory. Skills and task data grant no tools or authority. Use only current exact reporting and retained-input capabilities. Request changed scope from the user. Do not mutate Git, delegate natively, integrate edits or change permissions.';
function sealed(x, field, name) {
  digest(x[field]); const body = {...x}; delete body[field];
  assert.equal(domain(name,body),x[field],name + ' digest');
}
const pick = (x,keys) => Object.fromEntries(keys.split(' ').map(k => [k,x[k]]));
function taskKey(c,run) {return {workspaceId:c.workspaceId,runId:run.runId,...pick(c,'planId planRevision taskId')};}
function childIdentity(x) {
  exact(x,'kind workspaceId runId planId planRevision taskId taskVersion reservationId parentSessionId childSessionId configurationId configurationDigest');
  assert.equal(x.kind,'child');
  for (const k of ['workspaceId','runId','planId','taskId','reservationId','parentSessionId','childSessionId','configurationId']) id(x[k]);
  integer(x.planRevision,1,64); integer(x.taskVersion,1,2147483647); digest(x.configurationDigest);
}
function criteria(xs, ordered=false) {
  list(xs,32,1); const ids=[];
  for (const c of xs) {exact(c,'id requirement description'); id(c.id); text(c.description,2048); assert(['required','optional'].includes(c.requirement)); ids.push(c.id);}
  assert.equal(new Set(ids).size,ids.length,'unique criterion IDs');
  assert(xs.some(c => c.requirement === 'required'),'required criterion');
  if (ordered) sorted(xs,c => c.id);
}
function profile(x,reviewing) {
  exact(x,'tools readPaths writePaths commands externalSources connectors coordinationActions nativeDelegation accessChanges');
  same(x.tools,reviewing ? reviewTools : baseTools,'selected child tool set');
  for (const k of ['readPaths','writePaths','commands','externalSources','connectors','coordinationActions']) same(x[k],[],'no live child access');
  assert.equal(x.nativeDelegation,'denied'); assert.equal(x.accessChanges,'request-only');
}
function assignment(x,reviewing) {
  exact(x,'schemaVersion label description inputs expectedOutput criteria dependencies worktreeScope worktreeGroupId baseCommit');
  assert.equal(x.schemaVersion,1); text(x.label,512); text(x.description,4096); text(x.expectedOutput,2048);
  same(x.dependencies,[]); exact(x.worktreeScope,'readPaths writePaths'); same(x.worktreeScope.readPaths,[]); same(x.worktreeScope.writePaths,[]);
  id(x.worktreeGroupId); assert(typeof x.baseCommit === 'string' && /^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(x.baseCommit),'Git object ID syntax only');
  list(x.inputs,32); const ids=[];
  for (const v of x.inputs) {exact(v,'kind id content contentDigest'); assert.equal(v.kind,'inline'); id(v.id); text(v.content,4096); digest(v.contentDigest); assert.equal(hash(v.content),v.contentDigest); ids.push(v.id);}
  assert.equal(new Set(ids).size,ids.length); if (reviewing) same(x.inputs,[],'reviewer has no inline inputs');
  criteria(x.criteria);
}
function output(x,reviewing,reviewInput) {
  exact(x,'schemaVersion declarations maxSnapshotBytes maxRehashBytes'); assert.equal(x.schemaVersion,1);
  integer(x.maxSnapshotBytes,0,16777216); integer(x.maxRehashBytes,0,67108864);
  if (!reviewing) {same(x.declarations,[],'worker empty output'); assert.equal(x.maxSnapshotBytes,0); assert.equal(x.maxRehashBytes,0); return;}
  list(x.declarations,1,1); const d=x.declarations[0]; exact(d,'id kind maxBytes source'); id(d.id);
  assert.equal(d.kind,'assignment_evaluation'); same(d.source,{kind:'server-artifact'}); integer(d.maxBytes,1,524288);
  assert.equal(d.id,reviewInput.evaluationOutputDeclarationId); assert(d.maxBytes <= x.maxSnapshotBytes && x.maxRehashBytes >= 2*x.maxSnapshotBytes,'output aggregate capacity');
}
function projection(x) {
  exact(x,'schemaVersion kind sourceIdentity sourceRevision bindings'); assert.equal(x.schemaVersion,1); assert.equal(x.kind,'child-effective-configuration');
  text(x.sourceIdentity,512); text(x.sourceRevision,512); bindings(x.bindings); size(x,65536);
}
function policy(x) {
  exact(x,'schemaVersion kind harnessId adapterId adapterVersion policyId mode modelId settings unknownModelAllowed');
  assert.equal(x.schemaVersion,1); assert.equal(x.kind,'child-model-policy');
  for (const k of ['harnessId','adapterId','policyId']) id(x[k]); text(x.adapterVersion,512); assert.equal(typeof x.unknownModelAllowed,'boolean');
  if (x.mode === 'pinned') {text(x.modelId,512); assert.equal(x.unknownModelAllowed,false);}
  else {assert.equal(x.mode,'tool-managed'); assert.equal(x.modelId,null);}
  bindings(x.settings); size(x,65536);
}
function settings(x,c,map,snapshot) {
  exact(x,'schemaVersion kind harnessId adapterId adapterVersion argv nonsecretBindings configurationSources discoveredInstructions deliveryOrder instructionMechanism');
  assert.equal(x.schemaVersion,1); assert.equal(x.kind,'child-startup-settings'); assert.equal(x.instructionMechanism,'startup-context');
  for (const k of ['harnessId','adapterId','adapterVersion','instructionMechanism']) assert.equal(x[k],c.harness[k],'settings adapter binding');
  list(x.argv,64); x.argv.forEach(v => text(v,2048,0)); bindings(x.nonsecretBindings);
  list(x.configurationSources,32); sorted(x.configurationSources,s => s.sourceIdentity);
  for (const s of x.configurationSources) {
    exact(s,'sourceIdentity sourceRevision contentDigest'); text(s.sourceIdentity,512); text(s.sourceRevision,512); digest(s.contentDigest);
    const p=artifact(map,s.contentDigest,c.capabilityEvidence.references,false); projection(p);
    assert.equal(p.sourceIdentity,s.sourceIdentity); assert.equal(p.sourceRevision,s.sourceRevision);
  }
  list(x.discoveredInstructions,32); const lookup={'rule':c.rules,'role-template':[c.roleTemplate],'skill':c.skills,'skill-resource':c.skillResources}; const seen=new Set();
  for (const [i,d] of x.discoveredInstructions.entries()) {
    exact(d,'snapshotKind snapshotId contentDigest precedence'); assert.equal(d.precedence,i); id(d.snapshotId); digest(d.contentDigest);
    const source=lookup[d.snapshotKind]?.find(s => s.id === d.snapshotId); assert(source && source.contentDigest === d.contentDigest,'automatic snapshot binding');
    const key=d.snapshotKind+':'+d.snapshotId; assert(!seen.has(key),'unique discovery'); seen.add(key);
  }
  list(x.deliveryOrder,snapshot ? 34 : 33,snapshot ? 2 : 1); let supplied=0,inputs=0; const automatic=[];
  for (const [i,d] of x.deliveryOrder.entries()) {
    assert(['supplied','automatic','review-input'].includes(d.kind)); exact(d,d.kind === 'automatic' ? 'kind precedence' : d.kind === 'review-input' ? 'kind inputDigest' : 'kind');
    if (d.kind === 'supplied') supplied++;
    else if (d.kind === 'automatic') automatic.push(d.precedence);
    else {inputs++; assert(snapshot,'worker review data denied'); assert.equal(d.inputDigest,snapshot.inputDigest); assert.equal(x.deliveryOrder[i-1]?.kind,'supplied','review data immediately follows supplied');}
  }
  assert.equal(supplied,1,'one supplied payload'); assert.equal(inputs,snapshot ? 1 : 0,'one reviewer data block'); same(automatic,x.discoveredInstructions.map(d => d.precedence),'automatic order');
  size(x,65536); return x;
}
export function renderChildInstructions(c) {
  const blocks=[['boundary','child-v1',boundary],...c.rules.map(r => ['rule',r.id,r.content]),['role',c.roleTemplate.id,c.roleTemplate.instructions],
    ['assignment',c.taskId,canonical(pick(c,'assignment outputContract reviewInput rubric'))]];
  for (const requirement of ['mandatory','optional']) for (const s of c.skills.filter(s => s.requirement === requirement)) {
    blocks.push(['skill',s.id,s.content],...c.skillResources.filter(r => r.skillId === s.id).map(r => ['resource',r.id,r.content]));
  }
  const wire='ORKWORKS CHILD INSTRUCTIONS v1\n'+blocks.map(([kind,identity,content]) => {id(identity); text(content,262144,0); return `${kind} ${identity} ${Buffer.byteLength(content)}\n${content}\n`;}).join('');
  assert(Buffer.byteLength(wire) <= 262144,'rendered child byte ceiling'); return wire;
}
export function declaredStartup(c,s,snapshot) {
  const lookup={'rule':c.rules,'role-template':[c.roleTemplate],'skill':c.skills,'skill-resource':c.skillResources};
  const wire=s.deliveryOrder.map(d => {
    if (d.kind === 'supplied') return c.renderedInstructions;
    if (d.kind === 'review-input') {assert(snapshot); const bytes=canonical(snapshot); return '\nORKWORKS RETAINED REVIEW INPUT v1\n'+Buffer.byteLength(bytes)+'\n'+bytes+'\n';}
    const v=s.discoveredInstructions[d.precedence]; return lookup[v.snapshotKind].find(x => x.id === v.snapshotId).content ?? lookup[v.snapshotKind].find(x => x.id === v.snapshotId).instructions;
  }).join('');
  assert(Buffer.byteLength(wire) <= 262144,'declared composed startup ceiling'); return wire;
}
function reviewInput(x,snapshot) {
  if (!snapshot) {assert.equal(x,null,'worker reviewInput null'); return;}
  exact(x,'schemaVersion kind target outputDeclarationIds evidenceSources evaluationOutputDeclarationId maxReadBytes');
  assert.equal(x.schemaVersion,1); assert.equal(x.kind,'accepted-child-result'); same(x.target,snapshot.target,'exact review target');
  same(x.outputDeclarationIds,[]); same(x.evidenceSources,snapshot.evidence.map(p => p.source),'exact selected reports'); id(x.evaluationOutputDeclarationId);
  integer(x.maxReadBytes,1,262144);
  const required=['result','criteria','rubric'].reduce((n,k) => n+Buffer.byteLength(canonical(snapshot[k])),0)+snapshot.evidence.reduce((n,p) => n+Buffer.byteLength(canonical(p.report)),0);
  assert(required <= x.maxReadBytes,'review read capacity');
}
function configuration(owner,context,snapshot) {
  const c=owner.configuration, reviewing=!!snapshot;
  exact(c,'schemaVersion configurationId repositoryId repositoryBinding sourceWorktreeBinding workspaceId parentSessionId planId planRevision taskId assignmentKind roleTemplate taskCategory assignment rules requirementManifests skills skillResources rubric harness capabilityEvidence model permissions renderedInstructions renderedInstructionsDigest configurationDigest outputContract reviewInput');
  assert.equal(c.schemaVersion,3); assert.equal(c.assignmentKind,'child');
  for (const k of ['configurationId','repositoryId','workspaceId','parentSessionId','planId','taskId']) id(c[k]); integer(c.planRevision,1,64); text(c.taskCategory,512);
  for (const k of ['workspaceId','parentSessionId']) assert.equal(c[k],context.run[k],'run/configuration owner');
  repository(c.repositoryBinding); worktree(c.sourceWorktreeBinding,c.repositoryBinding);
  assert.equal(c.repositoryId,c.repositoryBinding.repositoryId); assert.equal(c.repositoryBinding.bindingDigest,context.run.repositoryBindingDigest); assert.equal(c.sourceWorktreeBinding.worktreeBindingDigest,context.run.sourceWorktreeBindingDigest,'original source worktree');
  const role=c.roleTemplate.role; assert(reviewing ? ['review','verification'].includes(role) : role === 'verification','selected role');
  if (reviewing) assert.equal(role,snapshot.rubric.evaluatorRole,'target rubric reviewer role');
  assignment(c.assignment,reviewing); reviewInput(c.reviewInput,snapshot); output(c.outputContract,reviewing,c.reviewInput);
  exact(c.permissions,'requested effective'); profile(c.permissions.requested,reviewing); profile(c.permissions.effective,reviewing);
  validateInstructionSnapshots(c,context,role,c.taskId); validateUnverifiedEvidence(c.capabilityEvidence,c,role);
  const h=c.harness,e=c.capabilityEvidence; exact(h,'harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest evidenceId evidenceDigest');
  for (const k of Object.keys(h)) same(h[k],e[k],'harness/evidence binding'); validateExecutableBinding(h.executableIdentity,e.executableIdentity);
  const t=taskKey(c,context.run); list(owner.evidenceObjects,16); same(owner.evidenceObjects.map(e => e.referenceId),e.references.map(r => r.id),'exact retained evidence ID closure');
  const retainedArtifacts=owner.evidenceObjects.map((v,i) => {
    exact(v,'schemaVersion task referenceId contentBase64 sizeBytes contentDigest'); assert.equal(v.schemaVersion,1); same(v.task,t,'evidence task owner'); id(v.referenceId);
    assert(typeof v.contentBase64 === 'string' && v.contentBase64.length <= 87384,'encoded evidence ceiling'); const bytes=Buffer.from(v.contentBase64,'base64');
    assert.equal(bytes.toString('base64'),v.contentBase64,'canonical base64'); integer(v.sizeBytes,0,65536); assert.equal(bytes.length,v.sizeBytes); assert.equal(hash(bytes),v.contentDigest);
    assert.equal(v.contentDigest,e.references[i].contentDigest,'evidence reference digest');
    const content=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes); return {content,contentDigest:v.contentDigest};
  });
  // Child evidence bodies explicitly allow zero bytes and repeated content under
  // distinct reference IDs. Charge every occurrence; the lookup map is no quota discount.
  assert(retainedArtifacts.reduce((n,a) => n+Buffer.byteLength(a.content),0) <= 1048576,'evidence raw aggregate');
  const map=new Map(retainedArtifacts.map(a => [a.contentDigest,a.content])); const s=settings(artifact(map,h.effectiveSettingsDigest,e.references),c,map,snapshot);
  exact(c.model,'schemaVersion mode modelId policyId policyDigest adapterGeneration'); assert.equal(c.model.schemaVersion,1); id(c.model.adapterGeneration); digest(c.model.policyDigest);
  const p=artifact(map,c.model.policyDigest,e.references); policy(p);
  for (const k of ['mode','modelId','policyId']) assert.equal(c.model[k],p[k],'model policy binding');
  for (const k of ['harnessId','adapterId','adapterVersion']) assert.equal(p[k],h[k],'model adapter binding');
  assert.equal(c.renderedInstructions,renderChildInstructions(c),'rendered child bytes'); assert.equal(c.renderedInstructionsDigest,hash(c.renderedInstructions));
  declaredStartup(c,s,snapshot); sealed(c,'configurationDigest','orkworks.assignment-configuration.v3'); size(c,1048576,'configuration 1 MiB');
  return {c,t,map,s};
}
function commit(x) {
  exact(x,'storageEpoch commitSequence changeDigest'); digest(x.storageEpoch); digest(x.changeDigest);
  assert(typeof x.commitSequence === 'string' && /^[1-9][0-9]{0,19}$/.test(x.commitSequence) && BigInt(x.commitSequence) <= 18446744073709551615n,'positive lossless commit Counter');
}
function branch(x) {
  text(x,256); assert(x !== 'main' && x !== 'HEAD' && x !== '@' && !x.startsWith('-') && !/[\x00-\x20\x7f~^:?*\[\\]/.test(x) && !x.includes('..') && !x.includes('@{'),'Git ref syntax only');
  assert(x.split('/').every(p => p && !p.startsWith('.') && !p.endsWith('.') && !p.endsWith('.lock')),'Git ref components');
}
function planAndEvents(owner,context,snapshot,validated) {
  const {c,t,map}=validated, p=owner.plan;
  exact(p,'schemaVersion kind parentPlanBinding task batches maxParallelChildren scopeText planDefinitionDigest'); assert.equal(p.schemaVersion,1); assert.equal(p.kind,'single-task-child-source');
  const b=p.parentPlanBinding; exact(b,'runId parentSessionId planId planRevision runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest preparationRevision inputDigest');
  for (const k of ['runId','parentSessionId','runDefinitionDigest','bootstrapConfigurationDigest','repositoryBindingDigest','sourceWorktreeBindingDigest']) assert.equal(b[k],context.run[k],'parent run binding');
  assert.equal(b.planId,c.planId); assert.equal(b.planRevision,c.planRevision); integer(b.preparationRevision,1,64);
  assert.equal(b.preparationRevision,context.parentInput.preparationRevision); validateSourceComponent('input',context.parentInput,{run:context.run}); assert.equal(b.inputDigest,domain('orkworks.preparation-input.v1',context.parentInput));
  exact(p.task,'taskId configuration input initialPrompt group'); assert.equal(p.task.taskId,c.taskId); same(p.task.configuration,pick(c,'configurationId configurationDigest'),'plan configuration');
  const input=snapshot ? {task:t,inputDigest:snapshot.inputDigest} : null; same(p.task.input,input,'plan input'); assert.equal(p.task.initialPrompt,c.assignment.description);
  const g=p.task.group; exact(g,'groupId branchName absolutePath baseCommit'); assert.equal(g.groupId,c.assignment.worktreeGroupId); assert.equal(g.baseCommit,c.assignment.baseCommit); branch(g.branchName); nativePath(g.absolutePath);
  same(p.batches,[[c.taskId]],'single-task batch'); assert.equal(p.maxParallelChildren,1); text(p.scopeText,4096); sealed(p,'planDefinitionDigest','orkworks.child-source-plan.v1'); size(p,65536,'plan 64 KiB');
  // Count occurrences in every closed artifact reference, without dedup credit.
  const artifactRefs=[c.harness.effectiveSettingsDigest,c.capabilityEvidence.effectiveSettingsDigest,c.model.policyDigest,...c.capabilityEvidence.references.map(r => r.contentDigest)];
  const settingsValue=decodeSourceComponent(Buffer.from(map.get(c.harness.effectiveSettingsDigest)));
  artifactRefs.push(...settingsValue.configurationSources.map(s => s.contentDigest));
  const total=Buffer.byteLength(canonical(p))+Buffer.byteLength(canonical(c))+(snapshot ? Buffer.byteLength(canonical(snapshot)) : 0)+artifactRefs.reduce((n,d) => n+Buffer.byteLength(map.get(d)),0);
  assert(total <= 2097152,'complete approved-plan capacity');
  const plan={...pick(t,'workspaceId runId planId planRevision'),planDefinitionDigest:p.planDefinitionDigest};
  const request=owner.request; exact(request,'schemaVersion action plan approvalId reviewContextId'); assert.equal(request.schemaVersion,1); assert.equal(request.action,'orkworks.child-source-plan.approve.v1'); same(request.plan,plan); id(request.approvalId); digest(request.reviewContextId);
  const a=owner.approval; exact(a,'schemaVersion plan approvalId reviewContextId sidecarGeneration requestDigest'); assert.equal(a.schemaVersion,1); same(a.plan,plan); assert.equal(a.approvalId,request.approvalId); assert.equal(a.reviewContextId,request.reviewContextId); digest(a.sidecarGeneration); assert.equal(a.requestDigest,domain('orkworks.child-source-plan-request.v1',request));
  const approvalDigest=domain('orkworks.child-source-plan-approval.v1',a), approval={plan,approvalId:a.approvalId,approvalDigest};
  const receipt=owner.receipt; exact(receipt,'schemaVersion plan approvalId approvalDigest requestDigest commit'); same(pick(receipt,'schemaVersion plan approvalId approvalDigest requestDigest'),{schemaVersion:1,plan,approvalId:a.approvalId,approvalDigest,requestDigest:a.requestDigest}); commit(receipt.commit); size(receipt,4096,'approval receipt 4 KiB');
  const r=owner.reservation; exact(r,'schemaVersion task reservationId childSessionId configurationId configurationDigest approval assignmentTaskVersion reservationDigest'); assert.equal(r.schemaVersion,1); same(r.task,t); id(r.reservationId); id(r.childSessionId); same(pick(r,'configurationId configurationDigest'),p.task.configuration); same(r.approval,approval); integer(r.assignmentTaskVersion,1,2147483647); sealed(r,'reservationDigest','orkworks.child-source-reservation.v1');
  const reservation={task:t,...pick(r,'reservationId reservationDigest')};
  const allocation=owner.allocation; exact(allocation,'schemaVersion reservation allocationId group allocatedWorktree allocationDigest'); assert.equal(allocation.schemaVersion,1); same(allocation.reservation,reservation); id(allocation.allocationId); same(allocation.group,g); worktree(allocation.allocatedWorktree,c.repositoryBinding);
  same(allocation.allocatedWorktree.canonicalRoot,g.absolutePath,'allocated planned path'); assert.equal(allocation.allocatedWorktree.kind,'linked');
  assert.notEqual(allocation.allocatedWorktree.worktreeBindingDigest,c.sourceWorktreeBinding.worktreeBindingDigest,'source is not allocated worktree');
  assert.notEqual(canonical(allocation.allocatedWorktree.rootObject),canonical(c.sourceWorktreeBinding.rootObject),'distinct allocated object descriptor');
  sealed(allocation,'allocationDigest','orkworks.child-source-allocation.v1');
  const allocationRef={reservation,...pick(allocation,'allocationId allocationDigest')};
  const identity={kind:'child',...t,taskVersion:r.assignmentTaskVersion,reservationId:r.reservationId,parentSessionId:c.parentSessionId,childSessionId:r.childSessionId,...p.task.configuration}; childIdentity(identity);
  const attachment=owner.attachment; exact(attachment,'schemaVersion attachmentId reservation allocation assignmentIdentity attachmentDigest'); assert.equal(attachment.schemaVersion,1); id(attachment.attachmentId); same(attachment.reservation,reservation); same(attachment.allocation,allocationRef); same(attachment.assignmentIdentity,identity,'frozen child identity'); sealed(attachment,'attachmentDigest','orkworks.child-source-attachment.v1');
  const source=owner.source; exact(source,'schemaVersion assignmentIdentity plan configuration approval reservation allocation attachmentId input criteria rubric');
  const projectedCriteria=[...c.assignment.criteria].sort((a,b) => Buffer.compare(Buffer.from(a.id),Buffer.from(b.id))); criteria(source.criteria,true);
  same(source,{schemaVersion:1,assignmentIdentity:identity,plan,configuration:{task:t,...p.task.configuration},approval,reservation,allocation:allocationRef,attachmentId:attachment.attachmentId,input,criteria:projectedCriteria,rubric:c.rubric},'complete assignment source projection');
  size(source,2097152);
  // Bound only exact supplied metadata wrappers; excluded links are not proofs.
  const rows=[{key:{plan,approvalId:a.approvalId},value:{schemaVersion:1,body:a,digest:approvalDigest,receipt,lastCommit:receipt.commit}},
    {key:t,value:{schemaVersion:1,body:r,lastCommit:receipt.commit}},
    {key:reservation,value:{schemaVersion:1,body:allocation,lastCommit:receipt.commit}},
    {key:r.childSessionId,value:{schemaVersion:1,body:attachment,lastCommit:receipt.commit}}];
  for (const row of rows) size(row,16384,'metadata complete wrapper 16 KiB');
  return source;
}
function result(x,worker) {
  exact(x,'schemaVersion assignmentIdentity predecessor authorityGeneration outputs resultRevision candidateDigest timeCostEvidence'); assert.equal(x.schemaVersion,1); same(x.assignmentIdentity,worker.source.assignmentIdentity); childIdentity(x.assignmentIdentity);
  integer(x.resultRevision,1,32); digest(x.candidateDigest); same(x.outputs,[],'selected target has no materialized outputs');
  exact(x.authorityGeneration,'kind sidecarGeneration generation'); assert.equal(x.authorityGeneration.kind,'child-launch'); digest(x.authorityGeneration.sidecarGeneration); digest(x.authorityGeneration.generation);
  if (x.predecessor.kind === 'no-head') {same(x.predecessor,{kind:'no-head'}); assert.equal(x.resultRevision,1);}
  else {exact(x.predecessor,'kind resultRevision resultDigest'); assert.equal(x.predecessor.kind,'revision'); integer(x.predecessor.resultRevision,1,31); assert.equal(x.predecessor.resultRevision,x.resultRevision-1); digest(x.predecessor.resultDigest);}
  list(x.timeCostEvidence,32); sorted(x.timeCostEvidence,e => e.kind+'\0'+(e.usageRecordId ?? ''));
  for (const e of x.timeCostEvidence) {
    if (e.kind === 'elapsed') {exact(e,'kind elapsedMilliseconds authorityGeneration'); integer(e.elapsedMilliseconds,0,604800000); same(e.authorityGeneration,x.authorityGeneration);}
    else {exact(e,'kind costMicroUsd providerId modelId pricingSnapshotId usageRecordId usageDigest'); assert.equal(e.kind,'cost'); integer(e.costMicroUsd,0,1000000000000); for (const k of ['providerId','modelId','pricingSnapshotId','usageRecordId']) id(e[k]); digest(e.usageDigest);}
    size(e,512);
  }
  size(x,524288); return domain('orkworks.assignment-result.v1',x);
}
function snapshot(p,worker) {
  const s=p.snapshot; exact(s,'schemaVersion task target result criteria rubric evidence inputDigest'); assert.equal(s.schemaVersion,1);
  same(s.task,taskKey(p.reviewer.configuration,p.run),'snapshot reviewing task'); exact(s.target,'assignmentIdentity resultRevision resultDigest');
  const d=result(s.result,worker); same(s.target,{assignmentIdentity:worker.source.assignmentIdentity,resultRevision:s.result.resultRevision,resultDigest:d},'snapshot target/result binding');
  same(s.criteria,worker.source.criteria,'target criteria projection'); same(s.rubric,worker.source.rubric,'target rubric projection');
  list(s.evidence,113); sorted(s.evidence,pair => domain('orkworks.assignment-evidence-source.v1',pair.source));
  same(p.reportContext.repositoryBindingDigest,worker.configuration.repositoryBinding.bindingDigest,'report repository context');
  for (const pair of s.evidence) {
    exact(pair,'source report'); assert.equal(pair.source.reportIdentity.workspaceId,p.run.workspaceId,'workspace-local report');
    validateReportEvidenceSource(pair.source,pair.report,p.reportContext.profile,domain('orkworks.assignment-evidence-source.v1',pair.source));
  }
  sealed(s,'inputDigest','orkworks.child-review-input.v1'); size(s,262144,'review snapshot 256 KiB'); return s;
}
export function verifyChildSources(p) {
  exact(p,'schemaVersion classification run parentInput worker reviewer snapshot reportContext vectors unverified'); assert.equal(p.schemaVersion,1); assert.equal(p.classification,'synthetic-unverified-child-source-bytes-only');
  validateRun(p.run); validateSourceComponent('input',p.parentInput,{run:p.run});
  for (const owner of [p.worker,p.reviewer]) exact(owner,'configuration evidenceObjects plan request approval receipt reservation allocation attachment source');
  const worker=configuration(p.worker,p,null); planAndEvents(p.worker,p,null,worker);
  const s=snapshot(p,p.worker), reviewer=configuration(p.reviewer,p,s); planAndEvents(p.reviewer,p,s,reviewer);
  for (const k of ['taskId','reservationId','childSessionId','configurationId','configurationDigest']) assert.notEqual(p.worker.source.assignmentIdentity[k],p.reviewer.source.assignmentIdentity[k],'reviewer independence');
  assert.notEqual(p.worker.attachment.attachmentId,p.reviewer.attachment.attachmentId,'distinct synthetic attachments');
  for (const key of ['canonicalRoot','rootObject','privateGitDirectoryObject']) assert.notEqual(canonical(p.worker.allocation.allocatedWorktree[key]),canonical(p.reviewer.allocation.allocatedWorktree[key]),'reviewer allocation independence');
  assert.notEqual(p.worker.allocation.group.branchName,p.reviewer.allocation.group.branchName,'reviewer allocation independence');
  return {classification:'unverified-source-bytes-only',configurations:2,sourceObjects:2,sourceDigests:[p.worker.source,p.reviewer.source].map(s => domain('orkworks.child-assignment-source.v1',s))};
}

export function decodeChildSource(bytes) {
  const value=decodeSummary(bytes,2097152);
  assert.equal(canonical(value),new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes),'canonical stored child source');
  return value;
}

export function wholeBodySourceDigest(kind,value) {
  const domains={source:'orkworks.child-assignment-source.v1','evidence-object':'orkworks.child-source-evidence.v1',
    request:'orkworks.child-source-plan-request.v1',approval:'orkworks.child-source-plan-approval.v1',result:'orkworks.assignment-result.v1'};
  assert(Object.hasOwn(domains,kind),'whole-body source kind');
  return domain(domains[kind],value);
}
