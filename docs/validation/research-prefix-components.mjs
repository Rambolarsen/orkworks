// Offline finite component oracle. No store, reducer, authority or publication.
import assert from 'node:assert/strict';
import {canonical,hash,exact,id,digest,integer,list,text} from './root-bootstrap-components.mjs';
import {validateSourceComponent} from './root-preparation-source-components.mjs';
import {validateResearchChain} from './research-chain-components.mjs';
import {validateReportEvidenceSource} from './r2-report-components.mjs';
const same=(a,b,label='finite component join')=>assert.deepEqual(a,b,label);
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const bytes=x=>Buffer.byteLength(canonical(x));
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const size=(x,max,label)=>assert(bytes(x)<=max,label);
const inputKey=b=>pick(b,'workspaceId runId preparationRevision inputRevision');
const runtimeRef=r=>({...r.key,attachmentDigest:r.value.digest});
const inputRef=b=>({...inputKey(b),inputDigest:b.inputDigest??domain('orkworks.preparation-input.v1',b)});
const wrapperBytes=r=>bytes({key:r.key,value:r.value});
const wrapperCaps={research_control_inputs:65536,research_runtime_attachments:4096,research_run_controls:1048576,
  research_plan_controls:16384,research_task_controls:16384,research_launch_controls:32768,
  research_session_states:16384,research_collections:16384,research_expectations:16384};
function commit(x){
  exact(x,'storageEpoch commitSequence changeDigest');digest(x.storageEpoch);digest(x.changeDigest);
  assert(typeof x.commitSequence==='string'&&/^[1-9][0-9]{0,19}$/.test(x.commitSequence)&&BigInt(x.commitSequence)<=18446744073709551615n,'positive uint64 commit Counter');
}
function wrapper(r,fields){
  exact(r,'table key value');exact(r.value,'schemaVersion body'+(fields?' '+fields:'')+' lastCommit');
  assert.equal(r.value.schemaVersion,1);commit(r.value.lastCommit);
  assert(Object.hasOwn(wrapperCaps,r.table),'supported component family');
  assert(wrapperBytes(r)<=wrapperCaps[r.table],'complete component wrapper bound');
}
function context(p){
  validateResearchChain(p.chain);
  const {startup,assignment}=p.chain,run=startup.context.run,t=startup.scope.task;
  exact(p.context,'rootAssignmentId initialInput unqualifiedCommit');id(p.context.rootAssignmentId);commit(p.context.unqualifiedCommit);
  validateSourceComponent('input',p.context.initialInput,{run});
  list(p.runtimeHistory,2,2);
  const refs=[];
  for(const [i,r] of p.runtimeHistory.entries()){
    wrapper(r,'digest');assert.equal(r.table,'research_runtime_attachments');
    const b=r.value.body;assert.equal(b.schemaVersion,1);
    exact(r.key,'kind workspaceId runId sessionId sidecarGeneration launchGeneration');
    digest(r.key.sidecarGeneration);digest(r.key.launchGeneration);
    const kind=i===0?'parent':'child';
    if(kind==='parent'){
      exact(b,'schemaVersion kind parent');
      validateSourceComponent('attachment',b.parent,{run,rootAssignmentId:p.context.rootAssignmentId});
      assert.equal(b.parent.kind,'attached');
    }else{
      exact(b,'schemaVersion kind assignmentIdentity attachmentId sidecarGeneration launchGeneration');
      same(b.assignmentIdentity,assignment.assignmentIdentity);assert.equal(b.attachmentId,assignment.attachmentId);
    }
    assert.equal(b.kind,kind);const gens=kind==='parent'?b.parent:b;
    same(r.key,{kind,...pick(run,'workspaceId runId'),sessionId:kind==='parent'?run.parentSessionId:assignment.assignmentIdentity.childSessionId,...pick(gens,'sidecarGeneration launchGeneration')},'runtime source key');
    assert.equal(r.value.digest,domain('orkworks.research-source-runtime-attachment.v1',b),'research runtime source digest');
    same(r.value.lastCommit,p.context.unqualifiedCommit,'symbolic runtime origin');refs.push(runtimeRef(r));
  }
  const [parent,child]=refs;
  exact(p.reportAssumption,'report source sourceKey head');const a=p.reportAssumption;
  const profile={stream:pick(assignment.assignmentIdentity,'workspaceId runId planId planRevision taskId reservationId childSessionId configurationDigest'),
    lineage:pick(child,'sidecarGeneration launchGeneration'),questionIds:startup.scope.questions.map(q=>q.questionId),captures:[]};
  validateReportEvidenceSource(a.source,a.report,profile,a.sourceKey);
  assert.equal(a.report.author.kind,'child','synthetic first child selection');assert.equal(a.report.reportVersion,1);
  same(a.head,pick(a.report,'reportId reportVersion contentDigest'),'supplied head bytes only');
  return {run,t,scope:{task:t,scopeDigest:startup.scope.scopeDigest},configuration:{task:t,...pick(startup.configuration,'configurationId configurationDigest')},parent,child};
}
const recordOrder=(a,b)=>Buffer.compare(Buffer.from(a.kind),Buffer.from(b.kind))||Buffer.compare(Buffer.from(canonical(a)),Buffer.from(canonical(b)));
function input(row,p,c,selection){
  wrapper(row,'producer');assert.equal(row.table,'research_control_inputs');const b=row.value.body,d=b.decision;
  exact(b,'schemaVersion workspaceId runId preparationRevision inputRevision goalDigest bootstrapConfigurationDigest decision records inputDigest');
  assert.equal(b.schemaVersion,1);same(pick(b,'workspaceId runId goalDigest bootstrapConfigurationDigest'),pick(c.run,'workspaceId runId goalDigest bootstrapConfigurationDigest'));
  integer(b.preparationRevision,2,64);integer(b.inputRevision,2,64);assert.equal(b.preparationRevision,b.inputRevision);
  same(row.key,inputKey(b),'exact input key');
  exact(d,'schemaVersion inputRevision clarificationNeeded researchNeeded reasons scope');assert.equal(d.schemaVersion,1);
  integer(d.inputRevision,2,b.inputRevision);assert.equal(d.clarificationNeeded,false);assert.equal(d.researchNeeded,true);
  list(d.reasons,16,1);d.reasons.forEach(x=>text(x,2048));same(d.scope,c.scope,'exact decision scope');
  list(b.records,256);
  const expected=[{kind:'configuration',reference:c.configuration},{kind:'research-scope',reference:c.scope},
    ...p.chain.startup.scope.captures.map(reference=>({kind:'research-capture',reference}))];
  if(selection!==null){same(selection,p.reportAssumption.source,'supplied selection source');expected.push({kind:'research-report',source:selection});}
  expected.sort(recordOrder);same(b.records,expected,'exact selected input records');
  digest(b.inputDigest);const body={...b};delete body.inputDigest;
  assert.equal(b.inputDigest,domain('orkworks.preparation-input.v1',body),'research input digest');size(b,32768,'input body 32 KiB');
  same(row.value.lastCommit,p.context.unqualifiedCommit,'symbolic input origin');
  if(d.inputRevision===b.inputRevision){
    same(row.value.producer,c.parent,'parent assessment producer');
  }else{
    assert(selection!==null,'child report invalidation requires selected source');
    same(row.value.producer,c.child,'child invalidation producer');
    // Find the original by its exact k/k key, never by copied decision values.
    const key={...pick(c.run,'workspaceId runId'),preparationRevision:d.inputRevision,inputRevision:d.inputRevision};
    const originals=p.inputs.filter(r=>canonical(r.key)===canonical(key));
    assert.equal(originals.length,1,'unique original assessment row');const original=originals[0];
    assert.equal(original.value.body.decision.inputRevision,d.inputRevision,'fresh original assessment');
    input(original,p,c,null);same(d,original.value.body.decision,'exact original assessment');
  }
  return bytes(b);
}
// Standalone bounded input source check. Does not validate arbitrary views or
// bind hypothetical revisions to the finite packet's approved plan.
export function validateResearchInput(row,p,selection=null){return input(row,p,context(p),selection);}

function expectedViews(p,c){
  const {run,t,configuration,scope,parent,child}=c,a=p.chain.assignment;
  const key=pick(run,'workspaceId runId');
  const row=(table,key,body)=>({table,key,value:{schemaVersion:1,body,lastCommit:p.context.unqualifiedCommit}});
  // These explicit samples are independent views, not an event dispatcher or
  // initializer. Missing events/versions/proofs cannot be inferred between them.
  return ['initial-control','proposed','before-selection','selected-source'].map((name,stage)=>{
    const runBody={schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',
      ...pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
      runVersion:[1,3,10,11][stage],state:['assessing','awaiting_research_approval','researching','researching'][stage],
      currentInput:inputRef(stage===0?p.context.initialInput:p.inputs[stage===3?1:0].value.body),
      planReferences:stage===0?[]:[a.plan],currentPlan:stage>=2?a.plan:null,pendingPlan:stage===1?a.plan:null,
      parentRuntime:stage===0?null:parent,blockers:[],stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null};
    const rows=[row('research_run_controls',key,runBody)];
    if(stage){
      rows.push(row('research_plan_controls',a.plan,{schemaVersion:1,plan:a.plan,planVersion:stage===1?1:2,status:stage===1?'proposed':'active',latestApproval:stage===1?null:a.approval,task:t,replaces:null}),
        row('research_task_controls',t,{schemaVersion:1,task:t,taskVersion:stage===1?1:7,status:stage===1?'pending':'running',reservation:stage===1?null:a.reservation,coordinationResult:null}));
      if(stage>=2){
        const w=p.chain.allocation.allocatedWorktree;
        const observedDirectories=[{kind:'private-git',path:w.canonicalPrivateGitDirectory,object:w.privateGitDirectoryObject},{kind:'root',path:w.canonicalRoot,object:w.rootObject}];
        rows.push(row('research_launch_controls',t,{schemaVersion:1,task:t,reservation:a.reservation,
          admission:{approval:a.approval,parentRuntime:parent,expectedTaskVersion:1,assignmentTaskVersion:2},phase:'running',allocation:a.allocation,attachmentId:a.attachmentId,
          spawnRuntime:pick(child,'sidecarGeneration launchGeneration'),observedDirectories,interruptedAt:null,reasonCode:null}),
          row('research_session_states',t,{schemaVersion:1,task:t,reservation:a.reservation,assignmentIdentity:a.assignmentIdentity,attachmentId:a.attachmentId,
            sessionId:a.assignmentIdentity.childSessionId,producerRevision:'2',state:'nonterminal',runtime:child}));
      }
      rows.push(row('research_collections',key,{schemaVersion:1,...key,collectionVersion:stage,active:true,task:t}),
        row('research_expectations',t,{schemaVersion:1,task:t,collection:key,collecting:true,configuration,scope,reservation:stage===1?null:a.reservation,
          selectedReport:stage===3?p.reportAssumption.source:null,expectationVersion:stage}));
    }
    return {id:name,rows};
  });
}
export function validateResearchPrefix(p){
  exact(p,'context chain reportAssumption inputs runtimeHistory views');const c=context(p);
  list(p.inputs,2,2);assert.equal(p.inputs[0].value.body.inputRevision,2);assert.equal(p.inputs[1].value.body.inputRevision,3);
  const inputBytes=input(p.inputs[0],p,c,null)+input(p.inputs[1],p,c,p.reportAssumption.source);
  same(p.inputs[1].value.body.decision,p.inputs[0].value.body.decision,'retained decision-2');
  same(p.chain.unqualified.preparedInputBinding,{preparationRevision:2,inputDigest:p.inputs[0].value.body.inputDigest},'new plan binds exact input-2');
  same(p.views,expectedViews(p,c),'finite current-control views');
  for(const v of p.views)for(const r of v.rows)wrapper(r,'');
  return {inputBytes,runtimeBytes:p.runtimeHistory.reduce((n,r)=>n+bytes(r.value.body),0),
    viewWrapperBytes:p.views.map(v=>v.rows.reduce((n,r)=>n+wrapperBytes(r),0)),prospectiveExpectations:1};
}
