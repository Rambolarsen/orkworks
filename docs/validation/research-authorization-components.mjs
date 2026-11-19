// Finite decoded documentation components only. No store, publisher or cutoff.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {canonical,hash,exact,digest} from './root-bootstrap-components.mjs';
import {validateResearchPrefix} from './research-prefix-components.mjs';
const raw=readFileSync(new URL('research-prefix-components.json',import.meta.url));
const prefix=JSON.parse(raw);
const same=(a,b,label)=>assert.deepEqual(a,b,label);
const bytes=x=>Buffer.byteLength(canonical(x));
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const pick=(x,keys)=>Object.fromEntries(keys.split(' ').map(k=>[k,x[k]]));
const origin=(label,n)=>({storageEpoch:'f'.repeat(64),commitSequence:String(n),changeDigest:hash('UNQUALIFIED research authorization label: '+label)});
const assumptions=['source/native identity and allocation verified externally','original authority and permit acquisition supplied',
  'complete retained/tombstone membership and uniqueness supplied','source/control origin proofs and full profile supplied',
  'paired ownership, H/G currentness and original commit outcome supplied','resource balances and full job/physical capacity unqualified'];
function counter(x,max,positive=false){
  assert(typeof x==='string'&&/^(0|[1-9][0-9]*)$/.test(x)&&x.length<=20,'canonical counter');
  assert(BigInt(x)<=max&&(!positive||BigInt(x)>0n),'counter range');
}
// Independent component widths: a lower-bound success is never full job fit.
export function validateAuthorizationLimits(p,ordinaryRecordBytes,jointJobBytes){
  for(const [kind,max] of Object.entries({run:1048576,plan:16384,task:16384,launch:32768})){
    const r=p.authorization.body.intent.controls[kind];
    assert(bytes({key:r.key,value:r.value})<=max,'control wrapper limit');
  }
  assert(bytes({key:p.authorizationKey,value:p.authorization})<=2097152,'authorization wrapper limit');
  assert(Number.isSafeInteger(ordinaryRecordBytes)&&ordinaryRecordBytes>=0&&ordinaryRecordBytes<=1048576,'ordinary record limit');
  assert(Number.isSafeInteger(jointJobBytes)&&jointJobBytes>=0&&jointJobBytes<=1048576,'joint job limit');
}
export function validateResearchAuthorization(p){
  exact(p,'inputSha256 assumptions authorizationKey authorization ordinaryMember resolution schedules measurements');
  assert.equal(p.inputSha256,hash(raw),'unchanged documentation input');
  validateResearchPrefix(prefix);
  same(p.assumptions,assumptions,'supplied unqualified evidence boundary');
  const chain=prefix.chain,a=chain.assignment,P=a.assignmentIdentity;
  const parentRow=prefix.runtimeHistory[0],parent={...parentRow.key,attachmentDigest:parentRow.value.digest};
  const auth=p.authorization,b=auth.body,i=b.intent;
  exact(auth,'body digest lastCommit');exact(b,'schemaVersion permit intent');assert.equal(b.schemaVersion,1);
  exact(b.permit,'sidecarGeneration sequence');digest(b.permit.sidecarGeneration);
  counter(b.permit.sequence,18446744073709551615n,true);
  assert.equal(b.permit.sidecarGeneration,parent.sidecarGeneration,'original parent generation');
  exact(i,'workspaceId storageEpoch admissionEpoch expectedCommit ordinary assignmentIdentity authority attachment producerSource controls');
  same(p.authorizationKey,P,'full original authorization key');same(i.assignmentIdentity,P,'frozen original P');
  assert.equal(i.workspaceId,P.workspaceId);assert.equal(i.storageEpoch,prefix.context.unqualifiedCommit.storageEpoch);
  assert.equal(i.admissionEpoch,'e'.repeat(64),'supplied admission epoch');
  same(i.expectedCommit,origin('captured-H',3),'captured H/currentness example');
  same(auth.lastCommit,origin('supplied-original-C',4),'supplied original C, not a proof');
  exact(i.ordinary,'databaseEpoch historyGeneration createdAt');
  assert(/^[a-f0-9]{8}(-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(i.ordinary.databaseEpoch),'ordinary epoch UUID');
  counter(i.ordinary.historyGeneration,9223372036854775807n);
  same(i.ordinary,{databaseEpoch:'11111111-2222-4333-8444-555555555555',historyGeneration:'0',createdAt:'synthetic original creation — not a timestamp proof'},'captured ordinary epoch/G/creation');
  same(i.authority,{kind:'original-parent-research-source',parentRuntime:parent},'original parent source authority');
  const attachment={childSessionId:P.childSessionId,...pick(chain.attachment,'attachmentId attachmentDigest')};
  same(i.attachment,attachment,'deterministic intended new attachment');
  const source={key:{kind:'research-assignment-source',identity:P},digest:domain('orkworks.research-assignment-source.v1',a)};
  same(i.producerSource,source,'whole-body original research source');
  exact(i.controls,'run plan task launch');
  // Derive all prestate fields directly from original records. Later control
  // views are never templates or substitutes for an allocated prestate.
  const run=chain.startup.context.run,task=chain.startup.scope.task;
  const inputRef=pick(prefix.inputs[0].value.body,'workspaceId runId preparationRevision inputRevision inputDigest');
  const runBody={schemaVersion:1,profile:'single-task-retained-file-research-prefix-v1',
    ...pick(run,'workspaceId runId parentSessionId runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest'),
    runVersion:7,state:'researching',currentInput:inputRef,planReferences:[a.plan],currentPlan:a.plan,pendingPlan:null,
    parentRuntime:parent,blockers:[],stageContinueRequired:false,pendingClarification:null,pauseReason:null,resumeTarget:null};
  const planBody={schemaVersion:1,plan:a.plan,planVersion:2,status:'active',latestApproval:a.approval,task,replaces:null};
  const taskBody={schemaVersion:1,task,taskVersion:4,status:'launching',reservation:a.reservation,coordinationResult:null};
  const w=chain.allocation.allocatedWorktree;
  const launchBody={schemaVersion:1,task,reservation:a.reservation,
    admission:{approval:a.approval,parentRuntime:parent,expectedTaskVersion:chain.reservation.expectedTaskVersion,assignmentTaskVersion:P.taskVersion},
    phase:'allocated',allocation:a.allocation,attachmentId:null,spawnRuntime:null,
    observedDirectories:[{kind:'private-git',path:w.canonicalPrivateGitDirectory,object:w.privateGitDirectoryObject},
      {kind:'root',path:w.canonicalRoot,object:w.rootObject}],interruptedAt:null,reasonCode:null};
  for(const [kind,key,body] of [['run',pick(run,'workspaceId runId'),runBody],['plan',a.plan,planBody],['task',task,taskBody],['launch',task,launchBody]]){
    const expected={table:'research_'+kind+'_controls',key,value:{schemaVersion:1,body,lastCommit:origin(kind+'-prestate',2)}};
    same(i.controls[kind],expected,'allocated '+kind+' snapshot/key/origin join');
  }
  digest(auth.digest);assert.equal(auth.digest,domain('orkworks.research-preparation-authorization.v1',b),'research authorization source digest');
  const member={schemaVersion:1,assignmentIdentity:P,attachmentId:attachment.attachmentId,assignmentSourceDigest:source.digest,preparationCommit:auth.lastCommit};
  same(p.ordinaryMember,member,'unchanged reserved ordinary member');
  const r=p.resolution;exact(r,'basis originalCommand authorizationFamily source attachment assignment ordinaryId lineage authorizations');
  assert.equal(r.basis,'supplied-unqualified');
  same(r.originalCommand,{commit:auth.lastCommit,commandKind:'research-source-attach'},'recognized original research command only');
  assert.equal(r.authorizationFamily,'research_preparation_authorizations','command-selected family');
  same(r.source,source,'original selected source');same(r.attachment,chain.attachment,'complete original attachment');
  same(r.assignment,a,'complete original assignment');assert.equal(r.ordinaryId,P.childSessionId);
  same(r.lineage,{createdAt:i.ordinary.createdAt,linkage:member},'original ordinary identity and linkage');
  // Finite inventory includes only the intended row. Complete external inventory
  // and direct proof coverage remain assumptions, not conclusions of this loop.
  assert(Array.isArray(r.authorizations));
  for(const row of r.authorizations){
    exact(row,'family key value');
    assert(['research_preparation_authorizations','child_preparation_authorizations'].includes(row.family),'supported uniqueness family');
  }
  const sameP=r.authorizations.filter(x=>canonical(x.key)===canonical(P));
  assert.equal(sameP.length,1,'unique P across supplied families');
  const samePermit=r.authorizations.filter(x=>canonical(x.value.body.permit)===canonical(b.permit));
  assert.equal(samePermit.length,1,'unique preparation permit across supplied families');
  same(r.authorizations,[{family:'research_preparation_authorizations',key:P,value:auth}],'exact supplied original authorization');
  const outcomes=[['revoke-first','deny'],['permit-then-volatile-close','may-save/no-spawn'],
    ['durable-H-drift','deny/rollback'],['ordinary-G-drift','deny/rollback'],['ownership-lost','deny/rollback'],
    ['proven-rollback','old-state/sequence-spent'],['unknown-disposition','retain/no-retry'],
    ['commit-response-lost','original-result/no-spawn'],['workspace-switch','detach/retain-original-pair']];
  same(p.schedules,outcomes.map(([id,expected])=>({id,expected})),'policy oracles, not executed schedules');
  const measured={controlWrapperBytes:Object.values(i.controls).map(row=>bytes({key:row.key,value:row.value})),
    authorizationCharge:bytes({key:P,value:{body:b,digest:auth.digest}}),
    authorizationWrapperBytes:bytes({key:P,value:auth}),ordinaryMemberBytes:bytes(member),
    suppliedJointPartsBytes:bytes({authorization:auth,ordinaryMember:member})};
  same(p.measurements,measured,'independent UTF-8 measurements');
  validateAuthorizationLimits(p,measured.ordinaryMemberBytes,measured.suppliedJointPartsBytes);
  return measured;
}
