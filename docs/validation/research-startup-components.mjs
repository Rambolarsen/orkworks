// Offline synthetic components only. No producer/native/adapter authority.
import assert from 'node:assert/strict';
import {canonical,hash,exact,text,id,digest,integer,list,sorted,bindings,repository,worktree,artifact,
  validateInstructionSnapshots,validateUnverifiedEvidence} from './root-bootstrap-components.mjs';
import {validateRun} from './root-preparation-components.mjs';
import {validateExecutableBinding} from './root-preparation-source-components.mjs';
import {renderChildInstructions} from './child-source-components.mjs';
import {validateResearchScope} from './research-capture-scope-components.mjs';

const same=(a,b,label='exact research source join')=>assert.deepEqual(a,b,label);
const size=(x,max,label)=>assert(Buffer.byteLength(canonical(x))<=max,label);
const tools=['orkworks.research-report.context.read.v1','orkworks.research-report.receipt.read.v1','orkworks.research-report.submit.v1'];
function profile(p){
  exact(p,'tools readPaths writePaths commands externalSources connectors coordinationActions nativeDelegation accessChanges');
  same(p.tools,tools,'research report tools');
  for(const k of ['readPaths','writePaths','commands','externalSources','connectors','coordinationActions'])same(p[k],[],'no live access');
  assert.equal(p.nativeDelegation,'denied');assert.equal(p.accessChanges,'request-only');
}
function assignment(a,context){
  exact(a,'schemaVersion label description inputs expectedOutput criteria dependencies worktreeScope worktreeGroupId baseCommit');
  assert.equal(a.schemaVersion,1);text(a.label,512);text(a.description,4096);text(a.expectedOutput,2048);
  same(a.inputs,[]);same(a.dependencies,[]);exact(a.worktreeScope,'readPaths writePaths');
  same(a.worktreeScope.readPaths,[]);same(a.worktreeScope.writePaths,[]);id(a.worktreeGroupId);
  assert.equal(a.worktreeGroupId,context.groupId,'supplied group binding');assert.equal(a.baseCommit,context.baseCommit,'supplied base binding');
  list(a.criteria,32,1);const ids=[];
  for(const c of a.criteria){exact(c,'id requirement description');id(c.id);text(c.description,2048);assert(['required','optional'].includes(c.requirement));ids.push(c.id);}
  assert.equal(new Set(ids).size,ids.length,'unique criteria');assert(a.criteria.some(c=>c.requirement==='required'),'required criterion');
}
function projection(x){
  exact(x,'schemaVersion kind sourceIdentity sourceRevision bindings');assert.equal(x.schemaVersion,1);assert.equal(x.kind,'child-effective-configuration');
  text(x.sourceIdentity,512);text(x.sourceRevision,512);bindings(x.bindings);size(x,65536,'projection bytes');
}
function policy(x){
  exact(x,'schemaVersion kind harnessId adapterId adapterVersion policyId mode modelId settings unknownModelAllowed');
  assert.equal(x.schemaVersion,1);assert.equal(x.kind,'child-model-policy');
  for(const k of ['harnessId','adapterId','policyId'])id(x[k]);text(x.adapterVersion,512);assert.equal(typeof x.unknownModelAllowed,'boolean');
  if(x.mode==='pinned'){text(x.modelId,512);assert.equal(x.unknownModelAllowed,false);}
  else{assert.equal(x.mode,'tool-managed');assert.equal(x.modelId,null);}
  bindings(x.settings);size(x,65536,'policy bytes');
}
function settings(s,c,map,scope){
  exact(s,'schemaVersion kind harnessId adapterId adapterVersion argv nonsecretBindings configurationSources discoveredInstructions deliveryOrder instructionMechanism');
  assert.equal(s.schemaVersion,1);assert.equal(s.kind,'child-startup-settings');assert.equal(s.instructionMechanism,'startup-context');
  for(const k of ['harnessId','adapterId','adapterVersion','instructionMechanism'])assert.equal(s[k],c.harness[k],'settings adapter binding');
  list(s.argv,64);s.argv.forEach(v=>text(v,2048,0));bindings(s.nonsecretBindings);
  list(s.configurationSources,32);sorted(s.configurationSources,v=>v.sourceIdentity);
  for(const v of s.configurationSources){
    exact(v,'sourceIdentity sourceRevision contentDigest');text(v.sourceIdentity,512);text(v.sourceRevision,512);digest(v.contentDigest);
    const p=artifact(map,v.contentDigest,c.capabilityEvidence.references,false);projection(p);
    assert.equal(p.sourceIdentity,v.sourceIdentity);assert.equal(p.sourceRevision,v.sourceRevision);
  }
  const lookup={'rule':c.rules,'role-template':[c.roleTemplate],'skill':c.skills,'skill-resource':c.skillResources};
  list(s.discoveredInstructions,32);const seen=new Set();
  for(const [i,d] of s.discoveredInstructions.entries()){
    exact(d,'snapshotKind snapshotId contentDigest precedence');id(d.snapshotId);digest(d.contentDigest);assert.equal(d.precedence,i,'discovery precedence');
    const source=lookup[d.snapshotKind]?.find(v=>v.id===d.snapshotId);
    assert(source&&source.contentDigest===d.contentDigest,'automatic snapshot binding');
    const key=d.snapshotKind+':'+d.snapshotId;assert(!seen.has(key),'unique discovery');seen.add(key);
  }
  list(s.deliveryOrder,34,2);let supplied=0,inputs=0;const automatic=[];
  for(const [i,d] of s.deliveryOrder.entries()){
    assert(['supplied','automatic','research-input'].includes(d.kind),'research delivery variant');
    exact(d,d.kind==='supplied'?'kind':d.kind==='automatic'?'kind precedence':'kind scopeDigest');
    if(d.kind==='supplied')supplied++;
    else if(d.kind==='automatic')automatic.push(d.precedence);
    else{
      inputs++;assert.equal(d.scopeDigest,scope.scopeDigest,'research scope binding');
      assert.equal(s.deliveryOrder[i-1]?.kind,'supplied','research input immediately follows supplied');
    }
  }
  assert.equal(supplied,1,'one supplied payload');assert.equal(inputs,1,'one research input');
  same(automatic,s.discoveredInstructions.map(d=>d.precedence),'automatic delivery order');size(s,65536,'settings bytes');
  return lookup;
}

export function validateResearchStartup(p){
  exact(p,'context configuration evidenceObjects captures scope declaredStartup');
  exact(p.context,'run groupId baseCommit objectFormat');const context=p.context,run=context.run,c=p.configuration;
  validateRun(run);id(context.groupId);
  exact(c,'schemaVersion configurationId repositoryId repositoryBinding sourceWorktreeBinding workspaceId parentSessionId planId planRevision taskId assignmentKind roleTemplate taskCategory assignment rules requirementManifests skills skillResources rubric harness capabilityEvidence model permissions renderedInstructions renderedInstructionsDigest configurationDigest outputContract reviewInput');
  assert.equal(c.schemaVersion,3);assert.equal(c.assignmentKind,'child');
  for(const k of ['configurationId','repositoryId','workspaceId','parentSessionId','planId','taskId'])id(c[k]);integer(c.planRevision,1,64);text(c.taskCategory,512);
  for(const k of ['workspaceId','parentSessionId'])assert.equal(c[k],run[k],'run/configuration owner');
  repository(c.repositoryBinding);worktree(c.sourceWorktreeBinding,c.repositoryBinding);
  assert.equal(c.repositoryId,c.repositoryBinding.repositoryId);assert.equal(c.repositoryBinding.bindingDigest,run.repositoryBindingDigest);
  assert.equal(c.sourceWorktreeBinding.worktreeBindingDigest,run.sourceWorktreeBindingDigest,'source worktree');
  assert.equal(c.roleTemplate.role,'research','research role');assignment(c.assignment,context);
  same(c.outputContract,{schemaVersion:1,declarations:[],maxSnapshotBytes:0,maxRehashBytes:0},'empty research outputs');assert.equal(c.reviewInput,null);
  exact(c.permissions,'requested effective');profile(c.permissions.requested);profile(c.permissions.effective);
  validateInstructionSnapshots(c,{run},'research',c.taskId);validateUnverifiedEvidence(c.capabilityEvidence,c,'research');
  const h=c.harness,e=c.capabilityEvidence;
  exact(h,'harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest evidenceId evidenceDigest');
  for(const k of Object.keys(h))same(h[k],e[k],'harness/evidence binding');validateExecutableBinding(h.executableIdentity,e.executableIdentity);
  const task={workspaceId:c.workspaceId,runId:run.runId,planId:c.planId,planRevision:c.planRevision,taskId:c.taskId};
  const measured=validateResearchScope(p.captures,p.scope,{task,repositoryBindingDigest:run.repositoryBindingDigest,sourceWorktreeBindingDigest:run.sourceWorktreeBindingDigest,baseCommit:context.baseCommit,objectFormat:context.objectFormat});
  list(p.evidenceObjects,16);same(p.evidenceObjects.map(v=>v.referenceId),e.references.map(r=>r.id),'complete retained evidence occurrences');
  const map=new Map();let rawEvidenceBytes=0;
  for(const [i,v] of p.evidenceObjects.entries()){
    exact(v,'schemaVersion task referenceId contentBase64 sizeBytes contentDigest');assert.equal(v.schemaVersion,1);same(v.task,task,'evidence task binding');id(v.referenceId);
    assert(typeof v.contentBase64==='string'&&v.contentBase64.length<=87384,'encoded evidence bytes');
    const raw=Buffer.from(v.contentBase64,'base64');assert.equal(raw.toString('base64'),v.contentBase64,'canonical padded Base64');
    integer(v.sizeBytes,0,65536);assert.equal(v.sizeBytes,raw.length,'raw evidence size');digest(v.contentDigest);assert.equal(v.contentDigest,hash(raw),'raw evidence hash');
    assert.equal(v.contentDigest,e.references[i].contentDigest,'evidence reference binding');rawEvidenceBytes+=raw.length;
    map.set(v.contentDigest,new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(raw));
  }
  assert(rawEvidenceBytes<=1048576,'evidence raw aggregate');
  const s=artifact(map,h.effectiveSettingsDigest,e.references),lookup=settings(s,c,map,p.scope);
  exact(c.model,'schemaVersion mode modelId policyId policyDigest adapterGeneration');assert.equal(c.model.schemaVersion,1);id(c.model.adapterGeneration);digest(c.model.policyDigest);
  const m=artifact(map,c.model.policyDigest,e.references);policy(m);
  for(const k of ['mode','modelId','policyId'])assert.equal(c.model[k],m[k],'model policy binding');
  for(const k of ['harnessId','adapterId','adapterVersion'])assert.equal(m[k],h[k],'model adapter binding');
  assert.equal(c.renderedInstructions,renderChildInstructions(c),'complete child instruction framing');assert.equal(c.renderedInstructionsDigest,hash(c.renderedInstructions));
  const payload=canonical({scope:p.scope,captures:p.captures});
  const startup=s.deliveryOrder.map(d=>{
    if(d.kind==='supplied')return c.renderedInstructions;
    if(d.kind==='research-input')return '\nORKWORKS RETAINED RESEARCH INPUT v1\n'+Buffer.byteLength(payload)+'\n'+payload+'\n';
    const source=s.discoveredInstructions[d.precedence],v=lookup[source.snapshotKind].find(x=>x.id===source.snapshotId);return v.content??v.instructions;
  }).join('');
  const startupBytes=Buffer.byteLength(startup);assert(startupBytes<=262144,'complete composed startup ceiling');assert.equal(p.declaredStartup,startup,'exact declared startup bytes');
  digest(c.configurationDigest);const body={...c};delete body.configurationDigest;
  assert.equal(c.configurationDigest,hash('orkworks.assignment-configuration.v3\n'+canonical(body)),'unchanged v3 configuration digest');
  size(c,1048576,'configuration 1 MiB');
  return {configurationBytes:Buffer.byteLength(canonical(c)),startupBytes,rawEvidenceBytes,rawCaptureBytes:measured.rawCaptureBytes};
}
