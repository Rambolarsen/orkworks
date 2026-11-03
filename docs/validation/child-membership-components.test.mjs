// Synthetic relations and conditional cost traces, never a physical index/store.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {deriveMembershipComponents,validateMembershipPacket,reverseMembership,measureDiscoveryTrace} from './child-membership-components.mjs';
const read=n=>JSON.parse(readFileSync(new URL('./'+n,import.meta.url)));
const packet=read('child-membership-vectors.json');
const context={sources:read('child-source-vectors.json'),controls:read('child-control-vectors.json')};
const wire=x=>Array.isArray(x)?'['+x.map(wire).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map(k=>JSON.stringify(k)+':'+wire(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const entry=(p,id)=>p.entries.find(x=>x.id===id);
const has=(xs,predicate)=>xs.some(predicate);
function repin(e){const body={consumer:e.consumer,targets:e.targets};e.canonical=wire(body);e.bytes=Buffer.byteLength(e.canonical);e.sha256=sha(e.canonical);}

test('independent 77-case forward and 58-target reverse packet remains unverified',()=>assert.deepEqual(validateMembershipPacket(packet,context),{classification:'unverified-membership-components-only',entries:77,reverseTargets:58}));
test('every new control family and every existing control variant is derived',()=>{
 const actual=deriveMembershipComponents(context,packet.components);assert.equal(actual.length,77);
 for(const c of context.controls.rows)assert(has(actual,e=>e.id==='control-'+c.id));
});
for(const e of packet.entries)test('independent membership bytes '+e.id,()=>{
 const actual=deriveMembershipComponents(context,packet.components).find(x=>x.id===e.id);
 assert.deepEqual(actual,{id:e.id,consumer:e.consumer,targets:e.targets});
 assert.equal(wire({consumer:e.consumer,targets:e.targets}),e.canonical);assert.equal(Buffer.byteLength(e.canonical),e.bytes);assert.equal(sha(e.canonical),e.sha256);
});
const mutations=[
 ['missing historical approval',p=>entry(p,'worker-source').targets=entry(p,'worker-source').targets.filter(x=>x.table!=='child_plan_approvals')],
 ['current approval masquerades as original',p=>entry(p,'worker-source').targets.find(x=>x.table==='child_plan_approvals').key.approvalId='newer'],
 ['unknown metadata table',p=>entry(p,'worker-source').targets[0]={kind:'metadata',table:'generic_json',key:{}}],
 ['metadata promoted to ObjectRef',p=>entry(p,'worker-source').targets[0]={kind:'object',reference:{key:{kind:'child_reservations',identity:{}},digest:'0'.repeat(64)}}],
 ['missing configuration evidence',p=>entry(p,'reviewer-configuration').targets.pop()],
 ['target subject retargeted',p=>entry(p,'review-input').targets.find(x=>x.kind==='object'&&x.reference.key.kind==='assignment-result').reference.key.identity.resultRevision=2],
 ['prior plan omitted',p=>entry(p,'control-worker-prepared-input').targets=entry(p,'control-worker-prepared-input').targets.filter(x=>x.reference?.key.kind!=='child-source-plan')],
 ['old control origin points at current version',p=>entry(p,'control-recovery-observe-terminal').targets.find(x=>x.kind==='control-origin').rowDigest='0'.repeat(64)],
 ['old direct proof origin changed',p=>entry(p,'control-recovery-observe-terminal').targets.find(x=>x.kind==='control-origin').origin.commitSequence='1'],
 ['read-only snapshot proof omitted',p=>entry(p,'control-recovery-observe-terminal').targets=entry(p,'control-recovery-observe-terminal').targets.filter(x=>x.kind!=='control-origin')],
 ['invented recursive predecessor',p=>entry(p,'historical-debit').targets.push({kind:'commit-proof',commit:{...p.components.debit.value.intent,storageEpoch:'f'.repeat(64),commitSequence:'1',changeDigest:'0'.repeat(64)}})],
 ['debit accounting consumer omitted',p=>entry(p,'historical-debit').targets=entry(p,'historical-debit').targets.filter(x=>x.table!=='quota_usage')],
 ['debit current generation substituted',p=>entry(p,'historical-debit').consumer.key.sidecarGeneration='9'.repeat(64)],
 ['old artifact retargeted to new head',p=>{const old=entry(p,'historical-debit').targets.find(x=>x.reference?.key.kind==='assignment-evaluation-artifact');old.reference=entry(p,'artifact-2').consumer.reference;}],
 ['reverse entry missing historical consumer',p=>p.reverse.find(x=>x.consumers.length>1).consumers.pop()],
 ['reverse target omitted',p=>p.reverse.pop()],
 ['extra reverse consumer',p=>p.reverse[0].consumers.push({kind:'metadata',table:'child_task_controls',key:{}})],
 ['forward alternative case omitted',p=>p.entries.pop()],
 ['unknown packet field',p=>p.completeStore=true],
 ['duplicate target silently discounted',p=>entry(p,'historical-debit').targets.push(entry(p,'historical-debit').targets[0])],
];
for(const [name,mutate]of mutations)test('rejects rehashed '+name,()=>{
 const p=structuredClone(packet);mutate(p);p.entries.forEach(repin);assert.throws(()=>validateMembershipPacket(p,context),{name:'AssertionError'});
});
test('artifact custody is reciprocal and historical v1 remains pinned after v2',()=>{
 const e1=entry(packet,'evaluation-1'),a1=entry(packet,'artifact-1');
 assert(has(e1.targets,x=>wire(x)===wire(a1.consumer)));assert(has(a1.targets,x=>wire(x)===wire(e1.consumer)));
 const d=entry(packet,'historical-debit');assert(has(d.targets,x=>wire(x)===wire(a1.consumer)));assert(!has(d.targets,x=>wire(x)===wire(entry(packet,'artifact-2').consumer)));
});
test('spawn intent creates no successful runtime-history membership',()=>{
 const pending=entry(packet,'control-launch-spawn-pending'),running=entry(packet,'control-launch-running');
 assert.equal(pending.targets.filter(x=>x.table==='child_runtime_attachments'&&x.key.kind==='child').length,0);
 assert.equal(running.targets.filter(x=>x.table==='child_runtime_attachments'&&x.key.kind==='child').length,1);
});
test('reverse membership keeps both original and corrected artifact consumers',()=>{
 const source=entry(packet,'artifact-1').targets.find(x=>x.reference?.key.kind==='child-assignment-source');
 const rev=reverseMembership(packet.entries).find(x=>wire(x.target)===wire(source));assert(rev);
 for(const id of ['artifact-1','artifact-2'])assert(has(rev.consumers,c=>wire(c)===wire(entry(packet,id).consumer)));
});
const header=context.sources.worker.receipt.commit;
const trace=events=>measureDiscoveryTrace(events,{expectedHeader:header,finalHeader:header,complete:true});
test('page/index occurrence/owner/miss each count, copied bytes count again, reuse does not',()=>{
 assert.deepEqual(trace([{kind:'page',key:0,value:null},{kind:'occurrence',key:1,value:null},{kind:'row',key:2,value:null},{kind:'miss',key:3},{kind:'copy',key:2,value:null},{kind:'reuse'}]),{classification:'conditional-discovery-trace-only',observations:4,canonicalBytes:21});
});
for(const n of [4095,4096,4097])test('logical observation boundary '+n,()=>{
 const events=Array.from({length:n},()=>({kind:'miss',key:'x'}));
 if(n<=4096)assert.deepEqual(trace(events),{classification:'conditional-discovery-trace-only',observations:n,canonicalBytes:n*3});else assert.throws(()=>trace(events),{name:'AssertionError'});
});
for(const delta of [-1,0,1])test('cumulative canonical byte boundary '+delta,()=>{
 // Arithmetic-only trace; these strings are not valid producer rows or a store.
 const events=Array.from({length:8},()=>({kind:'row',key:0,value:'x'.repeat(2097152-3)}));events[0].value+='x'.repeat(Math.max(delta,0));if(delta<0)events[0].value=events[0].value.slice(1);
 if(delta<=0)assert.equal(trace(events).canonicalBytes,16777216+delta);else assert.throws(()=>trace(events),{name:'AssertionError'});
});
test('one extra end probe at observation ceiling is denied',()=>assert.throws(()=>trace([...Array.from({length:4096},()=>({kind:'miss',key:'x'})),{kind:'page',key:0,value:null}]),{name:'AssertionError'}));
test('a full trace does not assert completion from the limit',()=>assert.throws(()=>measureDiscoveryTrace(Array.from({length:4096},()=>({kind:'miss',key:0})),{expectedHeader:header,finalHeader:header,complete:false}),{name:'AssertionError'}));
test('changed coherent header rejects rather than patching capture',()=>assert.throws(()=>measureDiscoveryTrace([],{expectedHeader:header,finalHeader:{...header,commitSequence:'3'},complete:true}),{name:'AssertionError'}));
test('unknown trace kind cannot acquire a free visit',()=>assert.throws(()=>trace([{kind:'unmetered',key:0,value:null}]),{name:'AssertionError'}));
test('independent Python regeneration reproduces whole packet',()=>{
 const dir=mkdtempSync(join(tmpdir(),'child-membership-'));
 try{const path=join(dir,'vectors.json');const r=spawnSync('python3',[fileURLToPath(new URL('./generate-child-membership-vectors.py',import.meta.url)),'--output',path],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);assert.deepEqual(readFileSync(path),readFileSync(new URL('./child-membership-vectors.json',import.meta.url)));}finally{rmSync(dir,{recursive:true,force:true});}
});

test('interrupted known successful spawn retains its original runtime-history pin',()=>{
 const c=structuredClone(context),v=c.controls.rows.find(x=>x.id==='launch-interrupted'),b=v.row.value.body;
 b.attachmentId=c.sources.worker.source.attachmentId;b.spawnRuntime={sidecarGeneration:'c'.repeat(64),launchGeneration:'d'.repeat(64)};
 const value=structuredClone(v.row.value);delete value.lastCommit;v.row.rowDigest=sha('orkworks.hierarchy-storage-row.v4\n'+wire({table:v.row.table,key:v.row.key,body:value}));v.canonical=wire(v.row);v.bytes=Buffer.byteLength(v.canonical);v.sha256=sha(v.canonical);
 const d=deriveMembershipComponents(c,packet.components).find(x=>x.id==='control-launch-interrupted');
 assert(has(d.targets,x=>x.table==='child_runtime_attachments'&&x.key.kind==='child'&&x.key.launchGeneration==='d'.repeat(64)));
});
const componentDenials=[
 ['artifact raw content hash',x=>x.artifacts[0].contentDigest='0'.repeat(64)],
 ['artifact declaration',x=>x.artifacts[0].declarationId='other'],
 ['artifact original result',x=>x.artifacts[0].evaluation.resultRevision=2],
 ['correction original predecessor',x=>x.evaluations[1].predecessor.evaluationDigest='0'.repeat(64)],
 ['debit root source substitute',x=>x.debit.value.body.intent.producerSource.key.kind='root-bootstrap'],
 ['debit full permit key',x=>x.debit.value.body.permit.sequence='2'],
 ['debit numeric counter',x=>x.debit.key.sequence=1],
 ['debit counter overflow',x=>{x.debit.key.sequence='18446744073709551616';x.debit.value.body.permit.sequence=x.debit.key.sequence;}],
 ['debit B charge mismatch',x=>x.debit.value.body.intent.replyBound='1'],
 ['debit missing owner counter',x=>delete x.debit.value.body.intent.usedBefore.run],
 ['debit exhausted original assignment pool',x=>x.debit.value.body.intent.usedBefore.assignment='16777216'],
 ['debit unknown extra authority',x=>x.debit.value.body.intent.activeGrant=true],
 ['source metadata foreign task key',x=>x.sourceMetadata[1].key.taskId='foreign'],
 ['approval origin disagrees with original receipt',x=>x.sourceMetadata[0].value.lastCommit.commitSequence='3'],
];
for(const [name,mutate]of componentDenials)test('rejects typed component '+name,()=>{
 const x=structuredClone(packet.components);mutate(x);
 // Rehash debit so semantic denial does not depend on its old checksum.
 x.debit.value.digest=sha('orkworks.evaluation-artifact-read-debit.v1\n'+wire(x.debit.value.body));
 assert.throws(()=>deriveMembershipComponents(context,x),{name:'AssertionError'});
});
test('reservation retains its own synthetic metadata origin rather than approval commit',()=>{
 const d=deriveMembershipComponents(context,packet.components).find(e=>e.id==='worker-reservation');
 assert(has(d.targets,x=>x.kind==='commit-proof'&&x.commit.commitSequence==='3'));
 assert(!has(d.targets,x=>x.kind==='commit-proof'&&x.commit.commitSequence==='2'));
});
