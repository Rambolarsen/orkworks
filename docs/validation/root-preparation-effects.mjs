// Documentation-only, closed initial-effect witnesses. Never import into runtime.
// Logical coherence establishes no current UI/fence/capability/native authority.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {decodeSourceComponent, validateSourceComponent} from './root-preparation-source-components.mjs';
import {validatePreparationComponent} from './root-preparation-joins.mjs';
const sha = x => createHash('sha256').update(x).digest('hex');
const object = x => x !== null && typeof x === 'object' && !Array.isArray(x);
const order = (a,b) => Buffer.compare(Buffer.from(a),Buffer.from(b));
const exact = (x,fields) => {
  assert(object(x),'record object');
  assert.deepEqual(Object.keys(x).sort(),fields.split(' ').sort(),'closed fields');
};
const id = x => assert(typeof x === 'string' && /^[A-Za-z0-9._-]{1,128}$/.test(x),'ASCII ID');
const digest = x => assert(typeof x === 'string' && /^[a-f0-9]{64}$/.test(x),'Digest');
const counter = (x,positive=false) => {
  assert(typeof x === 'string' && /^(?:0|[1-9][0-9]{0,18})$/.test(x),'canonical Counter');
  const n=BigInt(x); assert(n<=9223372036854775807n && (!positive || n>0n),'Counter range'); return n;
};
function canonical(x) {
  if(x===null || typeof x==='boolean')return JSON.stringify(x);
  if(typeof x==='string'){assert(x.isWellFormed(),'unpaired surrogate');return JSON.stringify(x);}
  if(typeof x==='number'){assert(Number.isSafeInteger(x)&&!Object.is(x,-0),'lossless integer');return JSON.stringify(x);}
  if(Array.isArray(x))return '['+x.map(canonical).join(',')+']';
  assert(object(x),'supported JSON');
  return '{'+Object.keys(x).sort(order).map(k=>canonical(k)+':'+canonical(x[k])).join(',')+'}';
}
const size=x=>Buffer.byteLength(canonical(x));
const hash=(name,x)=>sha(name+'\n'+canonical(x));
const rid=(table,key)=>table+'\n'+canonical(key);
const clone=structuredClone;
const REPORT=['reporting-receipt-bytes','assignment-record-bytes','result-payload-bytes'];
const SUMMARY=['summary-row-bytes','summary-row-count','summary-lineage-bytes','summary-pending-count'];
const PREPARATION=['preparation-retained-bytes','preparation-run-count'];
const PREP_TABLES=['root_drafts','root_preparations','run_states','parent_attachments','preparation_artifacts'];
const TABLES=[...PREP_TABLES,'summary_state','quota_usage','store_header','commit_proof'];
function link(x){exact(x,'storageEpoch commitSequence changeDigest');digest(x.storageEpoch);counter(x.commitSequence,true);digest(x.changeDigest);}
function project(r){
  const v=clone(r.value);
  if(!['store_header','commit_proof'].includes(r.table))delete v.lastCommit;
  if(r.table==='root_preparations')delete v.receipt.commit;
  if(r.table==='summary_state'&&v.checkpoint!==null)delete v.checkpoint.applied;
  return v;
}
const rowDigest=r=>hash('orkworks.hierarchy-storage-row.v3',{table:r.table,key:r.key,body:project(r)});
const rootIdentity=b=>({kind:'root',...Object.fromEntries(['workspaceId','runId','rootAssignmentId','parentSessionId','bootstrapId','bootstrapConfigurationDigest'].map(k=>[k,b[k]]))});
const runOwner=b=>({kind:'run',workspaceId:b.workspaceId,runId:b.runId});
const workspaceOwner=workspaceId=>({kind:'workspace',workspaceId});
function contextMap(input){
  assert(object(input),'fixture source contexts');
  const result=new Map(), identities={run:new Set(),root:new Set(),parent:new Set()};
  let workspace;
  for(const [name,c] of Object.entries(input)){
    exact(c,'draftCanonical approvalCanonical retainedArtifacts');
    const d=decodeSourceComponent(Buffer.from(c.draftCanonical)),a=decodeSourceComponent(Buffer.from(c.approvalCanonical));
    const context={goal:d.runDefinition.goal,retainedArtifacts:c.retainedArtifacts,reviewedDraftBytes:Buffer.from(c.draftCanonical),
      approvedRequestBytes:Buffer.from(c.approvalCanonical),sourceKey:{workspaceId:d.workspaceId,runId:d.runId}};
    validatePreparationComponent('draft',d,context);validatePreparationComponent('approval',a,context);
    const b=d.bootstrap;
    assert.equal(b.assignment.inputs.filter(i=>i.kind==='reference').length,0,'no reference input profile');
    assert.deepEqual(b.outputContract.declarations,[],'empty-output profile');
    for(const [kind,value] of [['run',b.runId],['root',b.rootAssignmentId],['parent',b.parentSessionId]]){
      assert(!identities[kind].has(value),'unique source '+kind);identities[kind].add(value);
    }
    if(workspace===undefined)workspace=b.workspaceId;else assert.equal(workspace,b.workspaceId,'one fixture workspace');
    result.set(name,{name,d,a,b,context,workspace});
  }
  assert(result.size>0,'nonempty fixture source contexts');return result;
}
const caseForRun=(cases,runId)=>{
  const found=[...cases.values()].find(c=>c.b.runId===runId);assert(found,'known exact run source');return found;
};
function decodeState(s){
  exact(s,'name rows stateSha256');assert(typeof s.name==='string');assert(Array.isArray(s.rows));
  const map=new Map();let prior;
  for(const encoded of s.rows){
    exact(encoded,'table keyCanonical valueCanonical rowDigest');assert(TABLES.includes(encoded.table),'closed preparation table');
    const key=decodeSourceComponent(Buffer.from(encoded.keyCanonical)),value=decodeSourceComponent(Buffer.from(encoded.valueCanonical));
    const r={table:encoded.table,key,value};const rowId=rid(r.table,key);
    assert(!map.has(rowId),'unique storage row');assert(prior===undefined||order(prior,rowId)<0,'canonical row order');prior=rowId;
    assert.equal(encoded.rowDigest,rowDigest(r),'row commitment');map.set(rowId,r);
  }
  return map;
}
function material(b){
  const targets=[...b.assignment.criteria.filter(c=>c.requirement==='required').map(c=>({kind:'criterion',id:c.id})),
    ...b.rubric.dimensions.map(d=>({kind:'quality-dimension',id:d.id})),{kind:'rating'}];
  targets.sort((a,b)=>order(canonical(a),canonical(b)));
  return {schemaVersion:1,assignmentIdentity:rootIdentity(b),accepted:{kind:'none'},
    criteriaSnapshotDigest:hash('orkworks.criteria-snapshot.v1',[...b.assignment.criteria].sort((a,b)=>order(a.id,b.id))),
    rubricSnapshotDigest:hash('orkworks.rubric-snapshot.v1',b.rubric),overall:'unassessed',
    requiredOutcomes:targets.map(target=>({target,disposition:'unassessed'})),requiredFindings:[]};
}
function expectedSummary(b,initialized,commit){
  const value={subjectRevision:'1',row:null,checkpoint:null,admission:'never-initialized',lastCommit:commit};
  if(initialized){const m=material(b);Object.assign(value,{admission:'initialized',
    row:{schemaVersion:1,assignmentIdentity:rootIdentity(b),summaryRevision:'1',state:{kind:'current',material:m}},
    checkpoint:{material:m,summaryRevision:'1',applied:commit}});}
  return value;
}
function preparationOwner(r){
  const v=r.value;
  const source=r.table==='root_drafts'?v.draft:r.table==='root_preparations'?v.bootstrap:r.table==='run_states'?v.state:r.table==='parent_attachments'?v.attachment:r.key;
  return runOwner(source);
}
function expectedQuotas(rows,workspace){
  const result=new Map(),W=workspaceOwner(workspace);
  const put=(owner,resource,used)=>{const key={owner,resource};result.set(rid('quota_usage',key),{table:'quota_usage',key,value:{used:String(used),reserved:'0',lastCommit:null}});};
  for(const resource of [...REPORT,...SUMMARY,...PREPARATION])put(W,resource,0);
  const prep=[...rows.values()].filter(r=>PREP_TABLES.includes(r.table));
  const owners=new Map(prep.map(r=>{const owner=preparationOwner(r);return [canonical(owner),owner];}));
  for(const owner of owners.values())put(owner,'preparation-retained-bytes',prep.filter(r=>canonical(preparationOwner(r))===canonical(owner)).reduce((n,r)=>n+size(r.key)+size(project(r)),0));
  put(W,'preparation-retained-bytes',prep.reduce((n,r)=>n+size(r.key)+size(project(r)),0));
  const owningRuns=new Set();
  for(const r of rows.values())if(['root_drafts','root_preparations'].includes(r.table))owningRuns.add(preparationOwner(r).runId);
  put(W,'preparation-run-count',owningRuns.size);
  for(const r of rows.values())if(r.table==='root_preparations')for(const owner of [{kind:'assignment',assignmentIdentity:rootIdentity(r.value.bootstrap)},runOwner(r.value.bootstrap)])
    for(const resource of REPORT)put(owner,resource,0);
  const summaries=[...rows.values()].filter(r=>r.table==='summary_state').map(r=>r.value);
  put(W,'summary-row-count',summaries.filter(v=>v.row!==null).length);
  put(W,'summary-row-bytes',summaries.reduce((n,v)=>n+(v.row===null?0:size(v.row)),0));
  put(W,'summary-lineage-bytes',summaries.reduce((n,v)=>n+(v.checkpoint===null?0:size(v.checkpoint.material)),0));
  return result;
}
function validateCaps(c){
  exact(c,'classification preparationRunBytes preparationWorkspaceBytes preparationRunCount summaryCount summaryBytes lineageBytes pendingCount commandBytes changedRows');
  assert.equal(c.classification,'fixture-only-not-production','fixture-only ceilings');
  for(const k of ['preparationRunBytes','preparationWorkspaceBytes','preparationRunCount','summaryCount','summaryBytes','lineageBytes','pendingCount'])counter(c[k]);
  assert(Number.isSafeInteger(c.commandBytes)&&c.commandBytes>0&&c.commandBytes<=16777216,'complete command cap');
  assert(Number.isSafeInteger(c.changedRows)&&c.changedRows>0&&c.changedRows<=4096,'changed row cap');
}
function quotaCap(key,c){
  const resource=key.resource,kind=key.owner.kind;
  if(resource==='preparation-retained-bytes')return BigInt(kind==='run'?c.preparationRunBytes:c.preparationWorkspaceBytes);
  if(resource==='preparation-run-count')return BigInt(c.preparationRunCount);
  const summary={'summary-row-count':'summaryCount','summary-row-bytes':'summaryBytes','summary-lineage-bytes':'lineageBytes','summary-pending-count':'pendingCount'};
  if(resource in summary)return BigInt(c[summary[resource]]);
  if(resource==='reporting-receipt-bytes')return BigInt({assignment:17,run:64,workspace:256}[kind])*1048576n;
  if(resource==='assignment-record-bytes')return 1048576n; // declared empty-output fixture comparison only
  assert.equal(resource,'result-payload-bytes');return 0n;
}
export function validateArtifactOccurrences(rows,references,owner){
  exact(owner,'workspaceId runId');id(owner.workspaceId);id(owner.runId);
  assert(Array.isArray(references)&&references.length<=16,'reference count');
  assert(Array.isArray(rows)&&rows.length===references.length,'artifact occurrence closure');
  const expected=new Map();for(const r of references){
    exact(r,'id kind sourceReference contentDigest');id(r.id);digest(r.contentDigest);
    assert(['fixture','primary-reference'].includes(r.kind));assert(typeof r.sourceReference==='string'&&r.sourceReference.isWellFormed()&&Buffer.byteLength(r.sourceReference)>0&&Buffer.byteLength(r.sourceReference)<=2048);
    assert(!expected.has(r.id),'unique reference ID');expected.set(r.id,r);
  }
  let total=0;const seen=new Set();
  for(const r of rows){
    exact(r,'table key value');assert.equal(r.table,'preparation_artifacts');exact(r.key,'workspaceId runId referenceId');
    assert.equal(r.key.workspaceId,owner.workspaceId,'artifact workspace');assert.equal(r.key.runId,owner.runId,'artifact run');
    const reference=expected.get(r.key.referenceId);assert(reference&&!seen.has(r.key.referenceId),'exact artifact reference');seen.add(r.key.referenceId);
    const v=r.value;exact(v,'schemaVersion contentBase64 sizeBytes contentDigest lastCommit');assert.equal(v.schemaVersion,1);
    assert(size(v)<=2097152,'artifact encoded row ceiling');link(v.lastCommit);digest(v.contentDigest);
    assert(typeof v.contentBase64==='string','artifact Base64');const raw=Buffer.from(v.contentBase64,'base64');
    assert.equal(raw.toString('base64'),v.contentBase64,'canonical artifact Base64');
    assert.equal(BigInt(raw.length),counter(v.sizeBytes),'artifact raw byte count');assert(raw.length<=1048576,'artifact raw occurrence ceiling');
    assert.equal(sha(raw),v.contentDigest,'artifact raw digest');assert.equal(v.contentDigest,reference.contentDigest,'frozen reference digest');total+=raw.length;
  }
  assert(total<=1048576,'raw artifact aggregate ceiling');return total;
}
function validateRows(map,cases,caps){
  if(map.size===0)return;
  const first=cases.values().next().value,workspace=first.b.workspaceId;
  const header=map.get(rid('store_header',1));assert(header,'store header required');
  exact(header.value,'schemaVersion workspaceId storageEpoch latest');assert.equal(header.value.schemaVersion,3);assert.equal(header.value.workspaceId,workspace);digest(header.value.storageEpoch);link(header.value.latest);
  assert.equal(header.value.storageEpoch,header.value.latest.storageEpoch,'header epoch');
  const proofs=new Map();
  for(const r of map.values())if(r.table==='commit_proof'){
    counter(r.key,true);exact(r.value,'body changeDigest');const m=r.value.body;
    exact(m,'schemaVersion workspaceId storageEpoch commitSequence predecessor commandKind changes');
    assert.equal(m.schemaVersion,3);assert.equal(m.workspaceId,workspace);assert.equal(m.storageEpoch,header.value.storageEpoch);
    assert.equal(m.commitSequence,r.key);assert(counter(m.commitSequence,true)<=counter(header.value.latest.commitSequence,true),'no future proof');
    assert(['initialize-store','root-draft-prepare','root-draft-cancel','subject-preparation'].includes(m.commandKind),'closed initial command');
    if(m.predecessor===null)assert.equal(m.commitSequence,'1');else{link(m.predecessor);assert.equal(m.predecessor.storageEpoch,m.storageEpoch);assert.equal(counter(m.predecessor.commitSequence,true)+1n,counter(m.commitSequence,true));}
    assert(Array.isArray(m.changes)&&m.changes.length>0&&m.changes.length<=4096,'manifest change count');let prior;
    for(const change of m.changes){
      exact(change,'table key beforeDigest afterDigest');assert(TABLES.includes(change.table)&&!['store_header','commit_proof'].includes(change.table),'closed changed table');
      for(const k of ['beforeDigest','afterDigest'])if(change[k]!==null)digest(change[k]);
      assert(change.beforeDigest!==change.afterDigest&&(change.beforeDigest!==null||change.afterDigest!==null),'actual body change');
      const key=rid(change.table,change.key);assert(prior===undefined||order(prior,key)<0,'unique sorted manifest');prior=key;
    }
    assert.equal(r.value.changeDigest,hash('orkworks.hierarchy-storage-change.v3',m),'manifest digest');proofs.set(r.key,r.value);
  }
  // This finite packet explicitly prunes no proofs. Do not export this as a
  // production requirement to retain an unbounded predecessor chain.
  for(const p of proofs.values())if(p.body.predecessor!==null){
    const prior=proofs.get(p.body.predecessor.commitSequence);
    assert(prior&&prior.changeDigest===p.body.predecessor.changeDigest,'retained predecessor proof');
  }
  const prove=(ref,r)=>{
    link(ref);assert.equal(ref.storageEpoch,header.value.storageEpoch,'same storage epoch');const p=proofs.get(ref.commitSequence);assert(p&&p.changeDigest===ref.changeDigest,'required retained proof');
    if(r){const change=p.body.changes.find(c=>c.table===r.table&&canonical(c.key)===canonical(r.key));assert(change&&change.afterDigest===rowDigest(r),'row link proof commitment');}
  };
  prove(header.value.latest);
  const drafts=[...map.values()].filter(r=>r.table==='root_drafts');assert(drafts.length<=1,'one pending workspace draft');
  const owned=new Set(),admitted=new Set(),parents=new Set();
  for(const r of map.values()){
    if(['store_header','commit_proof'].includes(r.table))continue;
    link(r.value.lastCommit);prove(r.value.lastCommit,r);
    if(r.table==='root_drafts'){
      const c=caseForRun(cases,r.value.draft.runId);assert(!owned.has(c.b.runId),'one source owner per run');owned.add(c.b.runId);
      assert.equal(r.key,workspace);assert.deepEqual(r.value.draft,c.d,'original frozen draft');
      validatePreparationComponent('draft-row',r.value,c.context);
    }else if(r.table==='root_preparations'){
      const c=caseForRun(cases,r.value.bootstrap.runId);assert(!owned.has(c.b.runId),'one source owner per run');owned.add(c.b.runId);admitted.add(c.b.runId);
      assert.deepEqual(r.key,c.context.sourceKey,'source membership key');validatePreparationComponent('source-row',r.value,c.context);
    }else if(r.table==='run_states'){
      exact(r.value,'schemaVersion initialInput state lastCommit');assert.equal(r.value.schemaVersion,1);assert(size(r.value)<=4096,'run state row ceiling');
      const c=caseForRun(cases,r.value.state.runId);assert.deepEqual(r.key,c.context.sourceKey);assert(map.has(rid('root_preparations',r.key)),'run source required');
      const sourceContext={run:c.d.runDefinition,rootAssignmentId:c.b.rootAssignmentId,initialInput:r.value.initialInput};
      validateSourceComponent('input',r.value.initialInput,sourceContext);validateSourceComponent('state',r.value.state,sourceContext);
    }else if(r.table==='parent_attachments'){
      exact(r.value,'schemaVersion attachment lastCommit');assert.equal(r.value.schemaVersion,1);assert(size(r.value)<=4096,'parent row ceiling');
      const c=caseForRun(cases,r.value.attachment.runId);assert.equal(r.key,c.b.parentSessionId);assert(!parents.has(r.key),'unique parent');parents.add(r.key);
      assert.equal(r.value.attachment.kind,'reserved','reserved parent only');
      validateSourceComponent('attachment',r.value.attachment,{run:c.d.runDefinition,rootAssignmentId:c.b.rootAssignmentId});
      assert(map.has(rid('root_preparations',c.context.sourceKey)),'parent source required');
    }else if(r.table==='summary_state'){
      const c=caseForRun(cases,r.key.runId);assert.deepEqual(r.key,rootIdentity(c.b),'summary root identity');
      assert(['initialized','never-initialized'].includes(r.value.admission));assert.deepEqual(r.value,expectedSummary(c.b,r.value.admission==='initialized',r.value.lastCommit),'initial summary exact bytes');
      assert(map.has(rid('root_preparations',c.context.sourceKey)),'summary source required');
      if(r.value.checkpoint)prove(r.value.checkpoint.applied);
      if(r.value.row)assert(size(r.value.row)<=2097152,'summary row bound');
    }else if(r.table==='quota_usage'){
      exact(r.key,'owner resource');exact(r.value,'used reserved lastCommit');counter(r.value.used);counter(r.value.reserved);
    }else assert.equal(r.table,'preparation_artifacts');
  }
  for(const run of owned){
    const c=caseForRun(cases,run),rows=[...map.values()].filter(r=>r.table==='preparation_artifacts'&&r.key.runId===run);
    validateArtifactOccurrences(rows,c.b.capabilityEvidence.references,c.context.sourceKey);
    const originals=new Map(c.context.retainedArtifacts.map(a=>[a.contentDigest,Buffer.from(a.content)]));
    for(const r of rows)assert.deepEqual(Buffer.from(r.value.contentBase64,'base64'),originals.get(r.value.contentDigest),'original retained artifact bytes');
    if(admitted.has(run)){
      assert(map.has(rid('run_states',c.context.sourceKey)),'initial run membership required');assert(map.has(rid('parent_attachments',c.b.parentSessionId)),'reserved parent membership required');
      assert(map.has(rid('summary_state',rootIdentity(c.b))),'summary marker required');
    }
  }
  for(const r of map.values())if(r.table==='preparation_artifacts')assert(owned.has(r.key.runId),'artifact owner required');
  const expected=expectedQuotas(map,workspace),actual=[...map.values()].filter(r=>r.table==='quota_usage');
  assert.equal(actual.length,expected.size,'complete shared ledger membership');
  for(const r of actual){const q=expected.get(rid(r.table,r.key));assert(q,'known exact quota key');assert.deepEqual(project(r),project(q),'reconciled shared ledger');
    assert(counter(r.value.used)+counter(r.value.reserved)<=quotaCap(r.key,caps),'fixture owner capacity');}
}
function validateState(s,cases,caps){const map=decodeState(s);validateRows(map,cases,caps);assert.equal(sha(canonical(s.rows)),s.stateSha256,'full-state fingerprint');return map;}
export function validateEffectsState(s,contexts,caps){validateCaps(caps);validateState(s,contextMap(contexts),caps);return s.stateSha256;}
function coreRows(c,initialized){
  const d=c.d,a=c.a,b=c.b,key=c.context.sourceKey,requestDigest=hash('orkworks.root-preparation-request.v1',a);
  const initialInput={schemaVersion:1,workspaceId:b.workspaceId,runId:b.runId,preparationRevision:1,inputRevision:1,goalDigest:b.goalDigest,bootstrapConfigurationDigest:b.bootstrapConfigurationDigest,decision:null,records:[]};
  const state={schemaVersion:1,workspaceId:b.workspaceId,runId:b.runId,parentSessionId:b.parentSessionId,runDefinitionDigest:d.runDefinition.runDefinitionDigest,
    bootstrapConfigurationDigest:b.bootstrapConfigurationDigest,runVersion:1,state:'assessing',preparationRevision:1,inputRevision:1,inputDigest:hash('orkworks.preparation-input.v1',initialInput),
    planReferences:[],currentPlan:null,parentRuntime:null,blockers:[],stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null};
  const attachment={schemaVersion:1,kind:'reserved',...Object.fromEntries(['workspaceId','runId','parentSessionId','rootAssignmentId','bootstrapConfigurationDigest'].map(k=>[k,b[k]]))};
  const receipt={schemaVersion:1,workspaceId:b.workspaceId,runId:b.runId,rootAssignmentIdentity:rootIdentity(b),bootstrapConfigurationDigest:b.bootstrapConfigurationDigest,
    runDefinitionDigest:d.runDefinition.runDefinitionDigest,draftDigest:d.draftDigest,approvalId:a.approvalId,requestDigest,commit:null};
  const source={schemaVersion:1,bootstrap:b,runDefinition:d.runDefinition,approval:{...Object.fromEntries(['approvalId','draftId','draftDigest','reviewContextId'].map(k=>[k,a[k]])),sidecarGeneration:d.sidecarGeneration,requestDigest},receipt,lastCommit:null};
  return [{table:'root_drafts',key:b.workspaceId,value:null},{table:'root_preparations',key,value:source},
    {table:'run_states',key,value:{schemaVersion:1,initialInput,state,lastCommit:null}},
    {table:'parent_attachments',key:b.parentSessionId,value:{schemaVersion:1,attachment,lastCommit:null}},
    {table:'summary_state',key:rootIdentity(b),value:expectedSummary(b,initialized,null)}];
}
function willInitialize(before,c,caps){
  const used=resource=>counter(before.get(rid('quota_usage',{owner:workspaceOwner(c.b.workspaceId),resource})).value.used);
  const v=expectedSummary(c.b,true,null);
  return used('summary-row-count')+1n<=BigInt(caps.summaryCount)&&used('summary-row-bytes')+BigInt(size(v.row))<=BigInt(caps.summaryBytes)
    &&used('summary-lineage-bytes')+BigInt(size(v.checkpoint.material))<=BigInt(caps.lineageBytes);
}
function predict(before,command,cases,caps){
  const next=new Map([...before].map(([k,r])=>[k,clone(r)])),first=cases.values().next().value;
  const c=command.subject===null?null:cases.get(command.subject);assert(command.subject===null||c,'known command subject');
  const insert=r=>{assert(!next.has(rid(r.table,r.key)),'fresh initial key');next.set(rid(r.table,r.key),clone(r));};
  const remove=(table,key)=>{assert(next.delete(rid(table,key)),'exact row to delete');};
  if(command.commandKind==='initialize-store'){
    assert.equal(before.size,0,'explicit new store only');assert.equal(command.subject,null);assert.equal(command.summaryBranch,null);
  }else{
    assert(before.has(rid('store_header',1))&&c,'existing store and exact subject');
    const draftRow=before.get(rid('root_drafts',c.b.workspaceId)),sourceKey=c.context.sourceKey;
    if(command.commandKind==='root-draft-prepare'){
      assert.equal(command.summaryBranch,null);assert(!draftRow,'no silent pending draft replacement');
      assert(!before.has(rid('root_preparations',sourceKey))&&!before.has(rid('run_states',sourceKey))&&!before.has(rid('parent_attachments',c.b.parentSessionId))&&!before.has(rid('summary_state',rootIdentity(c.b))),'fresh source/membership');
      assert(![...before.values()].some(r=>PREP_TABLES.includes(r.table)&&preparationOwner(r).runId===c.b.runId),'no orphan ownership');
      insert({table:'root_drafts',key:c.b.workspaceId,value:{schemaVersion:1,draft:c.d,lastCommit:null}});
      const originals=new Map(c.context.retainedArtifacts.map(a=>[a.contentDigest,Buffer.from(a.content)]));
      for(const ref of c.b.capabilityEvidence.references){const raw=originals.get(ref.contentDigest);assert(raw,'exact source bytes');
        insert({table:'preparation_artifacts',key:{...sourceKey,referenceId:ref.id},value:{schemaVersion:1,contentBase64:raw.toString('base64'),sizeBytes:String(raw.length),contentDigest:ref.contentDigest,lastCommit:null}});}
    }else{
      assert(draftRow&&canonical(draftRow.value.draft)===c.context.reviewedDraftBytes.toString(),'exact frozen pending draft');
      assert(!before.has(rid('root_preparations',sourceKey)),'not already admitted');
      if(command.commandKind==='root-draft-cancel'){
        assert.equal(command.summaryBranch,null);remove('root_drafts',c.b.workspaceId);
        for(const r of before.values())if(r.table==='preparation_artifacts'&&r.key.runId===c.b.runId)remove(r.table,r.key);
      }else{
        assert.equal(command.commandKind,'subject-preparation','closed initial command');const initialized=willInitialize(before,c,caps);
        assert.equal(command.summaryBranch,initialized?'initialized':'never-initialized','capacity-derived summary branch');
        for(const r of coreRows(c,initialized)){if(r.value===null)remove(r.table,r.key);else insert(r);}
      }
    }
  }
  const quotas=expectedQuotas(next,first.b.workspaceId);
  for(const [key,r] of next)if(r.table==='quota_usage'&&!quotas.has(key))next.delete(key);
  for(const [key,r] of quotas){const old=before.get(key);next.set(key,old&&canonical(project(old))===canonical(project(r))?clone(old):r);}
  return next;
}
export function validateEffectsTransition(beforeState,afterState,command,contexts,caps){
  exact(command,'beforeState afterState commandKind subject summaryBranch caps manifestCanonical changeDigest changedRows commandBytes');
  validateCaps(caps);const cases=contextMap(contexts),before=validateState(beforeState,cases,caps),after=validateState(afterState,cases,caps);
  assert.equal(command.beforeState,beforeState.name);assert.equal(command.afterState,afterState.name);
  const expected=predict(before,command,cases,caps);
  const m=decodeSourceComponent(Buffer.from(command.manifestCanonical));
  const excluded=key=>key===rid('store_header',1)||key===rid('commit_proof',m.commitSequence);
  const actualData=[...after.entries()].filter(([key])=>!excluded(key));const expectedData=[...expected.entries()].filter(([key])=>!excluded(key));
  assert.deepEqual(actualData.map(([k,r])=>[k,project(r)]).sort((a,b)=>order(a[0],b[0])),expectedData.map(([k,r])=>[k,project(r)]).sort((a,b)=>order(a[0],b[0])),'complete predicted logical state');
  const header=before.get(rid('store_header',1));
  assert.equal(m.commandKind,command.commandKind);assert.equal(m.commitSequence,header?String(counter(header.value.latest.commitSequence,true)+1n):'1');
  assert.deepEqual(m.predecessor,header?.value.latest??null,'exact predecessor');
  const changed=[...new Set([...before.keys(),...after.keys()])].filter(key=>!before.has(key)||!after.has(key)||canonical(before.get(key).value)!==canonical(after.get(key).value));
  const headerId=rid('store_header',1),proofId=rid('commit_proof',m.commitSequence);assert(changed.includes(headerId)&&changed.includes(proofId),'both controls');
  const changes=changed.filter(key=>key!==headerId&&key!==proofId).map(key=>{
    const old=before.get(key),next=after.get(key),r=next??old;
    return {table:r.table,key:r.key,beforeDigest:old?rowDigest(old):null,afterDigest:next?rowDigest(next):null};
  }).sort((a,b)=>order(rid(a.table,a.key),rid(b.table,b.key)));
  assert(changes.every(c=>c.beforeDigest!==c.afterDigest),'no link-only churn');assert.deepEqual(m.changes,changes,'complete before/after manifest');
  const changeDigest=hash('orkworks.hierarchy-storage-change.v3',m);assert.equal(command.changeDigest,changeDigest);
  const ref={storageEpoch:m.storageEpoch,commitSequence:m.commitSequence,changeDigest};
  assert.deepEqual(after.get(headerId).value.latest,ref);assert.deepEqual(after.get(proofId).value,{body:m,changeDigest});
  for(const key of changed){const r=after.get(key);if(r&&key!==headerId&&key!==proofId)assert.deepEqual(r.value.lastCommit,ref,'changed row link');}
  assert.equal(command.changedRows,changed.length);assert(changed.length<=caps.changedRows,'complete changed-row budget');
  const commandBytes=changed.reduce((sum,key)=>{const r=after.get(key)??before.get(key);return sum+size(r.key)+(after.has(key)?size(r.value):0);},0);
  assert.equal(command.commandBytes,commandBytes,'complete changed logical bytes');assert(commandBytes<=caps.commandBytes,'whole command capacity');
  return {classification:'synthetic-logical-effects-only-not-runtime-admission',changeDigest,changedRows:changed.length,commandBytes};
}
export function inspectLogicalPreparationReceipt(s,subject,contexts,caps){
  validateCaps(caps);const cases=contextMap(contexts),map=validateState(s,cases,caps),c=cases.get(subject);assert(c,'known subject');
  const source=map.get(rid('root_preparations',c.context.sourceKey));assert(source,'retained source required');
  return canonical(source.value.receipt); // inspection only; no authenticated mutation retry
}
