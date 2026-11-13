// Offline values/charges only. No storage format, commitments, reducer or I/O.
import assert from 'node:assert/strict';
import {canonical,hash,exact,digest} from './root-bootstrap-components.mjs';
import {decodeSourceComponent,validateSourceComponent} from './root-preparation-source-components.mjs';
import {validatePreparationComponent} from './root-preparation-joins.mjs';
const same=(a,b,label='exact initializer component')=>assert.deepEqual(a,b,label);
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const bytes=x=>Buffer.byteLength(canonical(x));
const clone=structuredClone;
const subjects=['reporting-receipt-bytes','assignment-record-bytes','result-payload-bytes','evaluation-artifact-bytes','evaluation-artifact-read-bytes'];
const summaries=['summary-row-bytes','summary-row-count','summary-lineage-bytes','summary-pending-count'];
const extras=['child-source-retained-bytes','child-control-retained-bytes','report-retained-bytes'];
const all=[...subjects,...summaries,'preparation-retained-bytes','preparation-run-count',...extras];
const location=(table,key)=>({table,key});
const identity=r=>r.table+'\n'+canonical(r.key);
const sorted=rs=>rs.sort((a,b)=>Buffer.compare(Buffer.from(identity(a)),Buffer.from(identity(b))));
function counter(x,positive=false){
  assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(x),'canonical u64 Counter');
  assert(BigInt(x)<=18446744073709551615n&&(!positive||BigInt(x)>0n),'u64 Counter range');
}
function origin(c){exact(c,'storageEpoch commitSequence changeDigest');digest(c.storageEpoch);digest(c.changeDigest);counter(c.commitSequence,true);}
function ledger(owner,resource,used,lastCommit){return {table:'quota_usage',key:{owner,resource},value:{used:String(used),reserved:'0',lastCommit}};}
function validateLedger(r){
  exact(r,'table key value');same(r.table,'quota_usage');exact(r.key,'owner resource');
  exact(r.value,'used reserved lastCommit');counter(r.value.used);counter(r.value.reserved);origin(r.value.lastCommit);
}
function componentRows(base,name){
  const state=base.states.find(s=>s.name===name);assert(state,'known retained projection sample');
  return state.rows.filter(r=>!['store_header','commit_proof'].includes(r.table)).map(r=>({table:r.table,
    key:decodeSourceComponent(Buffer.from(r.keyCanonical)),value:decodeSourceComponent(Buffer.from(r.valueCanonical))}));
}
function preparationBytes(r){
  const v=clone(r.value);delete v.lastCommit;if(r.table==='root_preparations')delete v.receipt.commit;
  return bytes(r.key)+bytes(v); // projection lengths, never storage row hashes
}
function expected(base,raw){
  const selected=base.contexts.target,d=decodeSourceComponent(Buffer.from(selected.draftCanonical)),approval=decodeSourceComponent(Buffer.from(selected.approvalCanonical));
  const sourceContext={goal:d.runDefinition.goal,retainedArtifacts:selected.retainedArtifacts,reviewedDraftBytes:Buffer.from(selected.draftCanonical),approvedRequestBytes:Buffer.from(selected.approvalCanonical),sourceKey:pick(d,'workspaceId runId')};
  validatePreparationComponent('draft',d,sourceContext);validatePreparationComponent('approval',approval,sourceContext);
  const b=d.bootstrap,run=d.runDefinition,rk=pick(run,'workspaceId runId');
  const W={kind:'workspace',workspaceId:run.workspaceId},R={kind:'run',...rk};
  const A={kind:'assignment',assignmentIdentity:{kind:'root',...pick(b,'workspaceId runId rootAssignmentId parentSessionId bootstrapId bootstrapConfigurationDigest')}};
  const origins=Object.fromEntries(['initialize','captured','draft','admission'].map((name,i)=>[name,{storageEpoch:'f'.repeat(64),commitSequence:String(i+1),changeDigest:hash('Unqualified initializer component: '+name)}]));
  const old=componentRows(base,'target-draft'),admitted=componentRows(base,'target-admitted');
  const seed=admitted.find(r=>r.table==='run_states'&&canonical(r.key)===canonical(rk));assert(seed,'original initial source sample');
  const initial=seed.value.initialInput;validateSourceComponent('input',initial,{run});validateSourceComponent('state',seed.value.state,{run,initialInput:initial});
  const body={schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',
    ...pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
    runVersion:1,state:'assessing',currentInput:{...pick(initial,'workspaceId runId preparationRevision inputRevision'),inputDigest:hash('orkworks.preparation-input.v1\n'+canonical(initial))},
    planReferences:[],currentPlan:null,pendingPlan:null,parentRuntime:null,blockers:[],stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null};
  const control={table:'research_run_controls',key:rk,value:{schemaVersion:1,body,lastCommit:origins.admission}};
  const controlBytes=bytes(rk)+bytes({schemaVersion:1,body});assert(bytes({key:rk,value:control.value})<=1048576,'complete control wrapper bound');
  const prepTables=['root_drafts','root_preparations','run_states','parent_attachments','preparation_artifacts'];
  const owned=r=>r.table==='root_drafts'?r.value.draft.runId===run.runId:r.table==='parent_attachments'?r.key===run.parentSessionId:r.key?.runId===run.runId;
  const before=new Map(old.filter(r=>prepTables.includes(r.table)&&owned(r)).map(r=>[identity(r),r]));
  const after=new Map(admitted.filter(r=>prepTables.includes(r.table)&&owned(r)).map(r=>[identity(r),r]));
  const facts=sorted([...new Set([...before.keys(),...after.keys()])].map(key=>{
    const first=before.get(key),last=after.get(key),r=last??first;
    return {...location(r.table,r.key),beforeBytes:first?preparationBytes(first):0,afterBytes:last?preparationBytes(last):0};
  }));
  const beforeBytes=facts.reduce((n,f)=>n+f.beforeBytes,0),afterBytes=facts.reduce((n,f)=>n+f.afterBytes,0);
  const oldPeers=new Map(old.filter(r=>r.table==='quota_usage'&&canonical(r.key.owner)===canonical(W)).map(r=>[r.key.resource,r.value]));
  const background={'evaluation-artifact-bytes':'4096','evaluation-artifact-read-bytes':'8192','child-source-retained-bytes':'16384','child-control-retained-bytes':'2048','report-retained-bytes':'8192'};
  const captured=sorted(all.map(resource=>ledger(W,resource,background[resource]??oldPeers.get(resource)?.used??'0',
    ['preparation-retained-bytes','preparation-run-count'].includes(resource)?origins.draft:origins.captured)));
  const draftResource=ledger(R,'preparation-retained-bytes',beforeBytes,origins.draft);
  const artifactLocations=[...before.values()].filter(r=>r.table==='preparation_artifacts').map(r=>location(r.table,r.key));
  const draftLocations=sorted([location('root_drafts',run.workspaceId),...artifactLocations,location('quota_usage',draftResource.key),
    ...['preparation-retained-bytes','preparation-run-count'].map(resource=>location('quota_usage',{owner:W,resource}))]);
  const admissions=['initialized','never-initialized'].map(id=>{
    const summary=componentRows(base,id==='initialized'?'target-admitted':'target-never-initialized').find(r=>r.table==='summary_state'&&canonical(r.key)===canonical(A.assignmentIdentity));assert(summary);
    const delta={'summary-row-bytes':id==='initialized'?bytes(summary.value.row):0,'summary-row-count':id==='initialized'?1:0,
      'summary-lineage-bytes':id==='initialized'?bytes(summary.value.checkpoint.material):0,'summary-pending-count':0};
    const newResources=sorted([...[A,R].flatMap(owner=>subjects.map(resource=>ledger(owner,resource,0,origins.admission))),
      ...extras.map(resource=>ledger(R,resource,resource==='child-control-retained-bytes'?controlBytes:0,origins.admission))]);
    const deltas={...delta,'preparation-retained-bytes':afterBytes-beforeBytes,'child-control-retained-bytes':controlBytes};
    const workspaceAfter=captured.map(r=>{
      const amount=deltas[r.key.resource]??0;if(amount===0)return clone(r);
      const next=ledger(W,r.key.resource,BigInt(r.value.used)+BigInt(amount),origins.admission);validateLedger(next);return next;
    });
    const core=[location('root_drafts',run.workspaceId),location('root_preparations',rk),location('run_states',rk),location('parent_attachments',run.parentSessionId),location('summary_state',A.assignmentIdentity),location('research_run_controls',rk)];
    const replacedWorkspace=workspaceAfter.filter((r,i)=>canonical(r)!==canonical(captured[i])).map(r=>location(r.table,r.key));
    const logicalLocations=sorted([...core,...newResources.map(r=>location(r.table,r.key)),location('quota_usage',{owner:R,resource:'preparation-retained-bytes'}),...replacedWorkspace]);
    assert.equal(new Set(logicalLocations.map(identity)).size,logicalLocations.length,'coalesced logical keys');
    return {id,initialControl:control,newResourceRows:newResources,runPreparationAfter:ledger(R,'preparation-retained-bytes',afterBytes,origins.admission),workspaceAfter,summaryChargeDeltas:delta,logicalLocations};
  });
  return {packet:{classification:'offline-research-initialization-components-only',baselineSha256:hash(raw),unqualifiedOrigins:origins,
    workspaceInitialization:sorted(all.map(resource=>ledger(W,resource,0,origins.initialize))),capturedWorkspacePeers:captured,draftRunResources:[draftResource],preparationChargeFacts:facts,
    draftLocations,cancelLocations:draftLocations,retryLocations:[],unqualifiedControls:['header replacement','new proof'],admissions,
    assumptions:['owner coverage and freshness not proven','captured peer usage is hypothetical','original creation proofs not supplied']},controlBytes,beforeBytes,afterBytes};
}
export function validateResearchInitialization(p,baseline,baselineBytes){
  exact(p,'classification baselineSha256 unqualifiedOrigins workspaceInitialization capturedWorkspacePeers draftRunResources preparationChargeFacts draftLocations cancelLocations retryLocations unqualifiedControls admissions assumptions');
  same(baseline,JSON.parse(baselineBytes),'exact retained documentation input');const want=expected(baseline,baselineBytes),q=want.packet;
  same(p.classification,q.classification);same(p.baselineSha256,q.baselineSha256,'retained baseline bytes');
  exact(p.unqualifiedOrigins,'initialize captured draft admission');Object.values(p.unqualifiedOrigins).forEach(origin);same(p.unqualifiedOrigins,q.unqualifiedOrigins,'unqualified origin labels');
  for(const r of [...p.workspaceInitialization,...p.capturedWorkspacePeers,...p.draftRunResources,...p.admissions.flatMap(b=>[...b.newResourceRows,...b.workspaceAfter,b.runPreparationAfter])])validateLedger(r);
  same(p.workspaceInitialization,q.workspaceInitialization,'fourteen fresh workspace resources');same(p.capturedWorkspacePeers,q.capturedWorkspacePeers,'hypothetical captured peers');
  same(p.draftRunResources,q.draftRunResources,'draft lifetime resource boundary');same(p.preparationChargeFacts,q.preparationChargeFacts,'exact retained occurrence projections');
  same(p.draftLocations,q.draftLocations,'draft logical locations');same(p.cancelLocations,q.cancelLocations,'cancel logical locations');same(p.retryLocations,[],'retry is read only');
  same(p.unqualifiedControls,q.unqualifiedControls,'labels only, no storage controls');same(p.assumptions,q.assumptions,'explicit qualification limits');
  same(p.admissions,q.admissions,'exact admission components');
  return {classification:q.classification,workspaceResources:p.workspaceInitialization.length,initializeLogicalRows:p.workspaceInitialization.length+2,
    draftLogicalRows:p.draftLocations.length+2,cancelLogicalRows:p.cancelLocations.length+2,retryLogicalRows:0,
    controlChargeBytes:want.controlBytes,preparationBeforeBytes:want.beforeBytes,preparationAfterBytes:want.afterBytes,
    controlWrapperBytes:bytes({key:p.admissions[0].initialControl.key,value:p.admissions[0].initialControl.value}),
    workspaceWrapperBytes:p.workspaceInitialization.reduce((n,r)=>n+bytes({key:r.key,value:r.value}),0),
    admissions:p.admissions.map(b=>({id:b.id,newResourceRows:b.newResourceRows.length,logicalRows:b.logicalLocations.length+2}))};
}
