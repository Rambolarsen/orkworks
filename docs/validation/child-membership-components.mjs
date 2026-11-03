// Documentation-only, synthetic reference derivation. Never import into runtime.
// Alternative control variants are not one store. No physical index, complete
// membership/absence, origin proof, authentication, native or CAS claim follows.
import assert from 'node:assert/strict';
import {canonical,hash,exact,text,digest,integer} from './root-bootstrap-components.mjs';
import {verifyChildSources} from './child-source-components.mjs';
import {validateControlPacket,controlRowDigest} from './child-control-components.mjs';
const same=(a,b,message='exact membership join')=>assert.deepEqual(a,b,message);
const pick=(x,fields)=>Object.fromEntries(fields.split(' ').map(k=>[k,x[k]]));
const domain=(name,x)=>hash(name+'\n'+canonical(x));
const size=x=>Buffer.byteLength(canonical(x));
const order=(a,b)=>Buffer.compare(Buffer.from(canonical(a)),Buffer.from(canonical(b)));
const unique=xs=>[...new Map(xs.map(x=>[canonical(x),x])).values()].sort(order);
const metadata=(table,key)=>({kind:'metadata',table,key:structuredClone(key)});
const object=(kind,identity,d)=>({kind:'object',reference:{key:{kind,identity:structuredClone(identity)},digest:d}});
const commitTarget=commit=>({kind:'commit-proof',commit:structuredClone(commit)});
function counter(x,positive=false){assert(typeof x==='string'&&/^(0|[1-9][0-9]{0,19})$/.test(x)&&BigInt(x)<=18446744073709551615n&&(!positive||BigInt(x)>0n),'canonical u64 TEXT');}
function commit(x){exact(x,'storageEpoch commitSequence changeDigest');digest(x.storageEpoch);digest(x.changeDigest);counter(x.commitSequence,true);}
const plan=p=>object('child-source-plan',p,p.planDefinitionDigest);
const configuration=c=>object('child-source-configuration',c,c.configurationDigest);
const input=i=>object('child-review-input',i,i.inputDigest);
const result=r=>object('assignment-result',r,r.resultDigest);
const approval=a=>metadata('child_plan_approvals',{plan:a.plan,approvalId:a.approvalId});
const reservation=r=>metadata('child_reservations',r.task);
const allocation=a=>metadata('child_allocations',a.reservation);
const runtime=r=>metadata('child_runtime_attachments',pick(r,'kind workspaceId runId sessionId sidecarGeneration launchGeneration'));
const evaluation=r=>object('assignment-evaluation',r,r.evaluationDigest);
const artifact=a=>object('assignment-evaluation-artifact',pick(a,'producerAssignment declarationId artifactId version'),domain('orkworks.assignment-evaluation-artifact.v1',a));
const reports=s=>[object('assignment-evidence-source',s,domain('orkworks.assignment-evidence-source.v1',s)),object('research-report',s.reportIdentity,s.contentDigest)];

function environment(context){
 const s=context.sources;verifyChildSources(s);validateControlPacket(context.controls,{sources:s,rootAssignmentId:'root-1'});
 const run=s.run,runKey=pick(run,'workspaceId runId'),owners=[s.worker,s.reviewer];
 const owner=t=>{const o=owners.find(o=>canonical(o.source.configuration.task)===canonical(t));assert(o,'original retained task');return o;};
 const assignment=a=>{const o=owner(pick(a,'workspaceId runId planId planRevision taskId'));same(a,o.source.assignmentIdentity);return object('child-assignment-source',a,domain('orkworks.child-assignment-source.v1',o.source));};
 const attachment=o=>metadata('child_attachments',o.source.assignmentIdentity.childSessionId);
 const task=t=>{const o=owner(t);return [plan(o.source.plan),configuration(o.source.configuration),metadata('child_task_controls',t)];};
 return {s,run,runKey,owners,owner,assignment,attachment,task,root:metadata('root_preparations',runKey)};
}
function artifactComponents(x,env){
 exact(x,'evaluations artifacts stream debit sourceMetadata');same(x.evaluations.length,2);same(x.artifacts.length,2);
 const {s,run,assignment}=env,A=s.worker.source.assignmentIdentity,B=s.reviewer.source.assignmentIdentity,target=s.snapshot.target;
 for(let i=0;i<2;i++){
  const e=x.evaluations[i];exact(e,'schemaVersion assignmentIdentity resultRevision resultDigest reviewerIdentity evaluationRevision predecessor reporterSource criteriaSnapshotDigest rubricSnapshotDigest criterionOutcomes qualityDimensionOutcomes rating findings findingDisputes reviewedEvaluationRef correctionReason');same(e.schemaVersion,1);
  same(e.assignmentIdentity,A);same(pick(e,'resultRevision resultDigest'),pick(target,'resultRevision resultDigest'));same(e.reviewerIdentity,{kind:'child',assignmentIdentity:B});same(e.evaluationRevision,i+1);
  same(e.predecessor,i===0?{kind:'no-head'}:{kind:'revision',evaluationRevision:1,evaluationDigest:domain('orkworks.assignment-evaluation.v1',x.evaluations[0])});
  exact(e.reporterSource,'kind authorityGeneration');same(e.reporterSource.kind,'child');const g=e.reporterSource.authorityGeneration;exact(g,'kind sidecarGeneration generation');same(g.kind,'child-launch');digest(g.sidecarGeneration);digest(g.generation);
  same(e.criteriaSnapshotDigest,domain('orkworks.criteria-snapshot.v1',s.worker.source.criteria));same(e.rubricSnapshotDigest,domain('orkworks.rubric-snapshot.v1',s.worker.source.rubric));
  same(e.criterionOutcomes.map(c=>c.criterionId),s.worker.source.criteria.map(c=>c.id));
  for(const c of e.criterionOutcomes){exact(c,'criterionId outcome rationale evidenceReferences');same(c.outcome,'unassessed');text(c.rationale,2048);same(c.evidenceReferences,[]);}
  same(e.qualityDimensionOutcomes.map(d=>d.dimensionId),s.worker.source.rubric.dimensions.map(d=>d.id));
  for(const d of e.qualityDimensionOutcomes){exact(d,'dimensionId outcome rationale evidenceReferences');same(d.outcome,'unassessed');text(d.rationale,2048);same(d.evidenceReferences,[]);}
  exact(e.rating,'rating rationale evidenceReferences');same(e.rating.rating,'unassessed');text(e.rating.rationale,2048);same(e.rating.evidenceReferences,[]);
  same(e.findings,[]);same(e.findingDisputes,[]);same(e.reviewedEvaluationRef,null);if(i===0)same(e.correctionReason,null);else text(e.correctionReason,2048);
  const a=x.artifacts[i];exact(a,'schemaVersion producerAssignment declarationId artifactId version evaluation sizeBytes contentDigest');same(a.schemaVersion,1);same(a.producerAssignment,B);same(a.declarationId,s.reviewer.configuration.outputContract.declarations[0].id);same(a.artifactId,'membership-artifact');same(a.version,i+1);
  const ref={...pick(e,'assignmentIdentity resultRevision resultDigest reviewerIdentity evaluationRevision'),evaluationDigest:domain('orkworks.assignment-evaluation.v1',e)};same(a.evaluation,ref);same(a.sizeBytes,size(e));same(a.contentDigest,hash(canonical(e)));
  assert(size(e)<=s.reviewer.configuration.outputContract.declarations[0].maxBytes,'frozen artifact declaration bound');assert(size(a)<=16384);
 }
 same(x.sourceMetadata.length,8);
 for(const [index,o]of env.owners.entries()){
  const cases=[['child_plan_approvals',{plan:o.source.plan,approvalId:o.source.approval.approvalId},o.approval],['child_reservations',o.source.configuration.task,o.reservation],['child_allocations',o.source.reservation,o.allocation],['child_attachments',o.source.assignmentIdentity.childSessionId,o.attachment]];
  cases.forEach(([table,key,body],j)=>{const r=x.sourceMetadata[index*4+j];exact(r,'table key value rowDigest');same(r.table,table);same(r.key,key);same(r.value.body,body);same(r.value.schemaVersion,1);commit(r.value.lastCommit);same(r.value.lastCommit.storageEpoch,o.receipt.commit.storageEpoch);
   if(table==='child_plan_approvals'){exact(r.value,'schemaVersion body digest receipt lastCommit');same(r.value.digest,o.source.approval.approvalDigest);same(r.value.receipt,o.receipt);same(r.value.lastCommit,o.receipt.commit);}else exact(r.value,'schemaVersion body lastCommit');
   const v=structuredClone(r.value);delete v.lastCommit;if(v.receipt)delete v.receipt.commit;same(r.rowDigest,domain('orkworks.hierarchy-storage-row.v4',{table,key,body:v}));assert(size({key,value:r.value})<=16384);
  });
 }
 const stream=x.stream;exact(stream,'key value');same(stream.key,{producerAssignment:B,declarationId:x.artifacts[0].declarationId});exact(stream.value,'schemaVersion artifactId target lastCommit');same(pick(stream.value,'schemaVersion artifactId target'),{schemaVersion:1,artifactId:x.artifacts[0].artifactId,target});commit(stream.value.lastCommit);assert(size(stream)<=8192);
 const d=x.debit;exact(d,'key value');exact(d.key,'sidecarGeneration sequence');digest(d.key.sidecarGeneration);counter(d.key.sequence,true);exact(d.value,'body digest lastCommit');
 const b=d.value.body;exact(b,'schemaVersion permit intent');same(b.schemaVersion,1);same(b.permit,d.key);
 const q=b.intent;exact(q,'workspaceId storageEpoch admissionEpoch reviewerAssignment authorityGeneration producerSource artifact evaluation descriptor expectedCommit usedBefore replyBound');
 same(q.workspaceId,run.workspaceId);same(q.admissionEpoch,run.admissionEpoch);same(q.reviewerAssignment,B);exact(q.authorityGeneration,'kind sidecarGeneration generation');same(q.authorityGeneration.kind,'child-launch');same(q.authorityGeneration.sidecarGeneration,d.key.sidecarGeneration);digest(q.authorityGeneration.generation);
 same(q.producerSource,assignment(B).reference);same(q.artifact,artifact(x.artifacts[0]).reference);same(q.evaluation,x.artifacts[0].evaluation);
 same(q.descriptor,pick(x.artifacts[0],'artifactId version sizeBytes contentDigest'));same(q.expectedCommit,s.reviewer.receipt.commit);commit(q.expectedCommit);same(q.storageEpoch,q.expectedCommit.storageEpoch);
 exact(q.usedBefore,'assignment run workspace');for(const n of Object.values(q.usedBefore))counter(n);counter(q.replyBound,true);
 const Bbytes=q.descriptor.sizeBytes+size({schemaVersion:1,artifact:q.descriptor,evaluation:null})-4;same(q.replyBound,String(Bbytes));
 for(const [name,cap]of Object.entries({assignment:16777216,run:67108864,workspace:134217728}))assert(BigInt(q.usedBefore[name])+BigInt(q.replyBound)<=BigInt(cap));
 same(d.value.digest,domain('orkworks.evaluation-artifact-read-debit.v1',b));commit(d.value.lastCommit);same(d.value.lastCommit.storageEpoch,q.storageEpoch);same(BigInt(d.value.lastCommit.commitSequence),BigInt(q.expectedCommit.commitSequence)+1n);assert(size(d)<=16384);
 // No original proof, actual ledger state, read permit or publication is certified.
}

export function deriveMembershipComponents(context,components){
 const env=environment(context);artifactComponents(components,env);
 const {s,runKey,owners,owner,assignment,attachment,task,root}=env;const out=[];
 const add=(id,consumer,targets)=>out.push({id,consumer,targets:unique(targets)});
 const ownOrigin=(o,table)=>{const field=({child_plan_approvals:'approval',child_reservations:'reservation',child_allocations:'allocation',child_attachments:'attachment'})[table];const r=components.sourceMetadata.find(r=>r.table===table&&canonical(r.value.body)===canonical(o[field]));assert(r);return commitTarget(r.value.lastCommit);};
 for(const [n,o]of [['worker',owners[0]],['reviewer',owners[1]]]){
  const src=o.source,t=src.configuration.task,inputs=src.input===null?[]:[input(src.input)];
  add(n+'-plan',plan(src.plan),[configuration(src.configuration),...inputs,root,metadata('run_states',runKey)]);
  const evidence=o.evidenceObjects.map(e=>object('child-source-evidence',{task:t,referenceId:e.referenceId,contentDigest:e.contentDigest},domain('orkworks.child-source-evidence.v1',e)));
  add(n+'-configuration',configuration(src.configuration),[plan(src.plan),root,...evidence,...inputs]);
  add(n+'-source',assignment(src.assignmentIdentity),[plan(src.plan),configuration(src.configuration),...inputs,approval(src.approval),reservation(src.reservation),allocation(src.allocation),attachment(o)]);
  add(n+'-approval',approval(src.approval),[plan(src.plan),ownOrigin(o,'child_plan_approvals')]);
  add(n+'-reservation',reservation(src.reservation),[...task(t),approval(src.approval),ownOrigin(o,'child_reservations')]);
  add(n+'-allocation',allocation(src.allocation),[reservation(src.reservation),plan(src.plan),ownOrigin(o,'child_allocations')]);
  add(n+'-attachment',attachment(o),[reservation(src.reservation),allocation(src.allocation),assignment(src.assignmentIdentity),ownOrigin(o,'child_attachments')]);
  evidence.forEach((e,i)=>add(n+'-evidence-'+i,e,[]));
 }
 const rs=reports(s.snapshot.evidence[0].source);add('review-input',input(s.reviewer.source.input),[result(s.snapshot.target),rs[0],assignment(s.worker.source.assignmentIdentity),configuration(s.worker.source.configuration)]);
 add('report-source',rs[0],[rs[1]]);add('report',rs[1],[]);add('target-result',result(s.snapshot.target),[assignment(s.worker.source.assignmentIdentity)]);
 const controlTargets=r=>{
  const v=r.value,b=v.body;const targets=[commitTarget(v.lastCommit)];
  switch(r.table){
   case 'child_control_inputs':{
    targets.push(root,runtime(v.producer));
    for(const rec of b.records){switch(rec.kind){
     case 'configuration':targets.push(configuration(rec.reference));break;
     case 'review-input':targets.push(input(rec.reference));break;
     case 'result':targets.push(result(rec.reference));break;
     case 'research-report':targets.push(...reports(rec.source));break;
     case 'prior-plan':targets.push(plan(rec.plan));break;
     default:assert.fail('unsupported prepared-input membership');
    }}break;
   }
   case 'child_runtime_attachments':{
    targets.push(root);
    if(b.kind==='parent')targets.push(metadata('parent_attachments',b.parent.parentSessionId));
    else{const o=owner(pick(b.assignmentIdentity,'workspaceId runId planId planRevision taskId'));targets.push(assignment(b.assignmentIdentity),attachment(o),reservation(o.source.reservation));}break;
   }
   case 'child_run_controls':{
    targets.push(root,...b.planReferences.map(plan));if(b.currentInput!==null)targets.push(metadata('child_control_inputs',pick(b.currentInput,'workspaceId runId preparationRevision inputRevision')));
    if(b.parentRuntime!==null)targets.push(runtime(b.parentRuntime));for(const blocker of b.blockers)if(blocker.task!==null)targets.push(...task(blocker.task));break;
   }
   case 'child_plan_controls':{
    targets.push(plan(b.plan),...task(b.task));if(b.latestApproval!==null)targets.push(approval(b.latestApproval));if(b.replaces!==null)targets.push(plan(b.replaces));
    assert.equal(owner(b.task).plan.parentPlanBinding.preparationRevision,1,'selected original plan input remains revision one');targets.push(metadata('run_states',runKey));break;
   }
   case 'child_task_controls':{
    targets.push(...task(b.task));if(b.reservation!==null)targets.push(reservation(b.reservation));if(b.coordinationResult!==null)targets.push(metadata('child_coordination_receipts',b.task));break;
   }
   case 'child_launch_controls':{
    const o=owner(b.task);targets.push(...task(b.task),reservation(b.reservation),approval(b.admission.approval),runtime(b.admission.parentRuntime));
    if(b.allocation!==null)targets.push(allocation(b.allocation));if(b.attachmentId!==null)targets.push(attachment(o));
    const known=b.spawnRuntime!==null&&context.controls.rows.some(c=>c.row.table==='child_runtime_attachments'&&c.row.value.body.kind==='child'&&canonical(c.row.value.body.assignmentIdentity)===canonical(o.source.assignmentIdentity)&&canonical(pick(c.row.value.body,'sidecarGeneration launchGeneration'))===canonical(b.spawnRuntime));
    if(b.spawnRuntime!==null&&(['running','ended'].includes(b.phase)||(b.phase==='interrupted'&&known)))targets.push(metadata('child_runtime_attachments',{kind:'child',...runKey,sessionId:o.source.assignmentIdentity.childSessionId,...b.spawnRuntime}));break;
   }
   case 'child_session_states':{
    targets.push(reservation(b.reservation),attachment(owner(b.task)),assignment(b.assignmentIdentity));if(b.runtime!==null)targets.push(runtime(b.runtime));break;
   }
   case 'child_coordination_receipts':{
    const q=v.request;targets.push(...task(q.task),reservation(q.reservation),attachment(owner(q.task)),commitTarget(v.receipt.commit));break;
   }
   case 'child_recovery_decisions':{
    targets.push(...task(b.task),reservation(b.reservation),commitTarget(b.readSet.expectedCommit),commitTarget(v.receipt.commit));
    if(b.allocation!==null)targets.push(allocation(b.allocation));if(b.attachmentId!==null)targets.push(attachment(owner(b.task)));
    for(const name of ['run','task','launch','session']){const snapshot=b.readSet[name];if(snapshot===null)continue;
     targets.push({kind:'control-origin',table:snapshot.table,key:snapshot.key,rowDigest:controlRowDigest(snapshot),origin:snapshot.value.lastCommit},...controlTargets(snapshot));
    }break;
   }
   default:assert.fail('unregistered control membership family');
  }
  return targets;
 };
 context.controls.rows.forEach(c=>add('control-'+c.id,metadata(c.row.table,c.row.key),controlTargets(c.row)));
 const A=s.worker.source.assignmentIdentity,B=s.reviewer.source.assignmentIdentity,target=s.snapshot.target,stream=metadata('evaluation_artifact_streams',components.stream.key);
 components.artifacts.forEach((a,i)=>{
  const e=evaluation(a.evaluation),ar=artifact(a);add('evaluation-'+(i+1),e,[ar,assignment(A),assignment(B),stream,result(target)]);add('artifact-'+(i+1),ar,[e,assignment(A),assignment(B),stream]);
 });
 add('artifact-stream',stream,[assignment(A),assignment(B),configuration(s.reviewer.source.configuration),result(target),...components.artifacts.map(artifact),...components.artifacts.map(a=>evaluation(a.evaluation)),commitTarget(components.stream.value.lastCommit)]);
 const d=components.debit,q=d.value.body.intent;
 const debitTargets=[assignment(A),{kind:'object',reference:q.producerSource},{kind:'object',reference:q.artifact},evaluation(q.evaluation),stream,commitTarget(q.expectedCommit),commitTarget(d.value.lastCommit)];
 for(const owner of [{kind:'assignment',assignmentIdentity:B},{kind:'run',...pick(B,'workspaceId runId')},{kind:'workspace',workspaceId:B.workspaceId}])debitTargets.push(metadata('quota_usage',{owner,resource:'evaluation-artifact-read-bytes'}));
 add('historical-debit',metadata('artifact_read_debits',d.key),debitTargets);return out;
}
/** Reverse relation over supplied alternatives only; never a completeness proof. */
export function reverseMembership(entries){
 const targets=new Map();for(const e of entries)for(const t of e.targets){const k=canonical(t);if(!targets.has(k))targets.set(k,{target:t,consumers:[]});targets.get(k).consumers.push(e.consumer);}
 return [...targets.values()].map(e=>({target:e.target,consumers:unique(e.consumers)})).sort((a,b)=>order(a.target,b.target));
}
export function validateMembershipPacket(p,c){
 exact(p,'schemaVersion classification components entries reverse');same(p.schemaVersion,1);same(p.classification,'synthetic-child-membership-components-only');
 const derived=deriveMembershipComponents(c,p.components);same(p.entries.length,derived.length);const ids=new Set();
 p.entries.forEach((e,i)=>{exact(e,'id consumer targets canonical bytes sha256');assert(!ids.has(e.id),'unique membership vector');ids.add(e.id);
  same(pick(e,'id consumer targets'),derived[i]);const body={consumer:e.consumer,targets:e.targets};same(e.canonical,canonical(body));same(e.bytes,size(body));same(e.sha256,hash(e.canonical));
 });
 same(p.reverse,reverseMembership(derived));return {classification:'unverified-membership-components-only',entries:derived.length,reverseTargets:p.reverse.length};
}
/** Conditional synthetic work trace. It does not observe queries, owning bytes,
 * absence, header coherence, native completion or copies in any running store. */
export function measureDiscoveryTrace(events,capture){
 exact(capture,'expectedHeader finalHeader complete');commit(capture.expectedHeader);commit(capture.finalHeader);same(capture.finalHeader,capture.expectedHeader,'changed captured header');same(capture.complete,true,'completion not established by a count');
 let observations=0,canonicalBytes=0;
 for(const e of events){
  assert(['page','occurrence','row','miss','copy','reuse'].includes(e.kind),'closed trace event');
  if(e.kind==='reuse'){exact(e,'kind');continue;}
  exact(e,e.kind==='miss'?'kind key':'kind key value');
  if(e.kind!=='copy')observations++;
  assert(observations<=4096,'discovery observation ceiling');canonicalBytes+=size(e.key)+(e.kind==='miss'?0:size(e.value));
  assert(canonicalBytes<=16777216,'cumulative discovery byte ceiling');
 }
 return {classification:'conditional-discovery-trace-only',observations,canonicalBytes};
}
