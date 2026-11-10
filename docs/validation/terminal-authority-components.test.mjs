import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,mkdtempSync,rmSync,cpSync,statSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {canonical} from './report-publication-components.mjs';
import {initialOwner,control,admit,settle,reconcilePending,canDeliver,makeReceiptV2 as makeReceiptWithProfile,validateReceiptV2,rowCommitment,beginPending,receiptUsage} from './terminal-authority-components.mjs';
const clone=structuredClone;
const old=JSON.parse(readFileSync(new URL('report-publication-vectors.json',import.meta.url)));
const c=old.cases[0], original=c.after.find(r=>r.table==='report_receipts'&&r.key.reportVersion===2);
const seed=JSON.parse(readFileSync(new URL('r2-report-vectors.json',import.meta.url)));
const reportProfile=seed.profiles.find(p=>canonical(p.stream)===canonical(c.request.stream));
const makeReceiptV2=(row,raw,decision)=>makeReceiptWithProfile(row,raw,decision,reportProfile);
const binding={workspaceId:c.request.stream.workspaceId,sidecarGeneration:'b'.repeat(64)};
const scope=n=>({...binding,scopeSequence:String(n)});
// Captured predicates only: these do not implement tokens, clocks, leases or SQL.
const assumed={authenticated:true,originalOwner:true,readRateAdmitted:true,controlSlotAvailable:true,closeSlotAvailable:true,rateAdmitted:true,sourceEligible:true,leaseCurrent:true,publicationSettled:true,rollbackProven:true,ownerHealthy:true,reportProfile};
const wire=(operation,s=scope(1))=>canonical({schemaVersion:1,operation,scope:s});
const opened=()=>control(initialOwner(binding),wire('open'),assumed).state;
const req=canonical(c.request);
const decision=()=>admit(opened(),scope(1),req,assumed);
const digest=x=>createHash('sha256').update(x).digest('hex');
// Isolated receipt-membership witness, deliberately not a complete report-publication transaction.
const context=(row,requestUtf8=req)=>{
 const manifest={schemaVersion:3,workspaceId:row.key.stream.workspaceId,storageEpoch:row.body.lastCommit.storageEpoch,commitSequence:row.body.lastCommit.commitSequence,
 predecessor:clone(old.states.current.find(r=>r.table==='store_header').body.latest),commandKind:'report-publication',changes:[{table:'report_receipts',key:clone(row.key),beforeDigest:null,afterDigest:rowCommitment(row)}]};
 const changeDigest=digest('orkworks.hierarchy-storage-change.v3\n'+canonical(manifest));
 row.body.lastCommit={storageEpoch:manifest.storageEpoch,commitSequence:manifest.commitSequence,changeDigest};
 return {requestUtf8,reportProfile,originalManifest:manifest,originalRowCommitment:rowCommitment(row),originalCommit:clone(row.body.lastCommit),originalProducerQualified:true,currentScopeAuthorized:true,rateAdmitted:true,sourceAccessQualified:true};
};

test('close before delayed open permanently fences that sequence',()=>{
 const first=control(initialOwner(binding),wire('close'),assumed);
 assert.equal(first.status,'ok');assert.equal(first.snapshot.closedThrough,'1');
 const late=control(first.state,wire('open'),assumed);
 assert.equal(late.status,'closed');assert.deepEqual(late.state,first.state);
 assert.equal(control(first.state,wire('open',scope(2)),assumed).status,'ok');
});
test('a delayed old close cannot close a newer open scope',()=>{
 let s=control(opened(),wire('close'),assumed).state;
 s=control(s,wire('open',scope(2)),assumed).state;
 const r=control(s,wire('close'),assumed);
 assert.equal(r.status,'ok');assert.equal(r.snapshot.openScopeSequence,'2');assert.deepEqual(r.state,s);
});
test('duplicate open is idempotent and skipped open/close cannot advance state',()=>{
 const s=opened();assert.deepEqual(control(s,wire('open'),assumed).state,s);
 for(const op of ['open','close']){const r=control(s,wire(op,scope(3)),assumed);assert.equal(r.status,'conflict');assert.deepEqual(r.state,s);}
});
for(const n of ['0','01','-1','18446744073709551616',1,null])test(`noncanonical scope ${String(n)} is refused without mutation`,()=>{
 const s=opened(),r=control(s,wire('close',{...scope(1),scopeSequence:n}),assumed);
 assert.equal(r.status,'bad-request');assert.deepEqual(r.state,s);
});
for(const [name,raw] of [['duplicate property','{"schemaVersion":1,"schemaVersion":1,"operation":"close","scope":{}}'],['oversized',' '.repeat(4097)],['unknown member',canonical({schemaVersion:1,operation:'close',scope:scope(1),hidden:true})]])test(`${name}: strict bounded control refuses`,()=>{
 const s=opened(),r=control(s,raw,assumed);assert.equal(r.status,'bad-request');assert.deepEqual(r.state,s);
});
test('authentication precedes malformed input and foreign lifetime disclosure',()=>{
 const s=opened();assert.equal(control(s,'not json',{...assumed,authenticated:false}).status,'unauthorized');
 assert.equal(control(s,wire('inspect',{...scope(1),sidecarGeneration:'c'.repeat(64)}),assumed).status,'stale');
});
test('close does not consume read-rate admission but still needs its reserved slot',()=>{
 assert.equal(control(opened(),wire('close'),{...assumed,readRateAdmitted:false}).status,'ok');
 assert.equal(control(opened(),wire('inspect'),{...assumed,readRateAdmitted:false}).status,'limit-exceeded');
 const s=opened(),r=control(s,wire('close'),{...assumed,closeSlotAvailable:false});assert.equal(r.status,'unavailable');assert.deepEqual(r.state,s);
});
test('admission-first retains one exact operation after closure; stale delivery stays fenced',()=>{
 const d=decision();assert.equal(d.status,'admitted');
 const closed=control(d.state,wire('close'),assumed);
 assert.equal(closed.snapshot.unsettledScopeSequence,'1');
 assert.equal(admit(closed.state,scope(1),req,assumed).status,'closed');
 assert.equal(control(closed.state,wire('open',scope(2)),assumed).status,'unavailable');
 const done=settle(closed.state,d.decision,'committed',assumed);
 assert.equal(done.status,'settled');assert.equal(done.state.activeDecision,null);
 assert.equal(control(done.state,wire('open',scope(2)),assumed).status,'ok');
});
test('close-first permits no write decision or admission sequence',()=>{
 const s=control(opened(),wire('close'),assumed).state,r=admit(s,scope(1),req,assumed);
 assert.equal(r.status,'closed');assert.equal(r.state.lastAdmissionSequence,'0');assert.equal(r.state.activeDecision,null);
});
for(const field of ['rateAdmitted','sourceEligible','leaseCurrent'])test(`${field} fails before a new write decision`,()=>{
 const s=opened(),r=admit(s,scope(1),req,{...assumed,[field]:false});
 assert.notEqual(r.status,'admitted');assert.deepEqual(r.state,s);
});
test('second pending decision cannot overwrite or retarget the original intent',()=>{
 const d=decision(),r=admit(d.state,scope(1),req,assumed);assert.equal(r.status,'busy');assert.deepEqual(r.state,d.state);
 const other=clone(d.decision);other.requestDigest='0'.repeat(64);
 assert.equal(settle(d.state,other,'committed',assumed).status,'conflict');
 assert.equal(d.state.activeDecision.requestUtf8,req);
});
test('uncertain disposition and ownership loss retain the exact old decision',()=>{
 const d=decision();assert.deepEqual(settle(d.state,d.decision,'uncertain',assumed).state,d.state);
 const r=settle(d.state,d.decision,'committed',{...assumed,leaseCurrent:false});assert.equal(r.status,'unavailable');assert.deepEqual(r.state,d.state);
});
test('rollback retires the slot without reusing the admission sequence',()=>{
 const d=decision(),s=settle(d.state,d.decision,'rolled-back',assumed).state;
 assert.equal(admit(s,scope(1),req,assumed).decision.admissionSequence,'2');
});
test('counter exhaustion does not wrap and still allows final close/inspect',()=>{
 const s={...opened(),lastAdmissionSequence:'18446744073709551615'};
 assert.equal(admit(s,scope(1),req,assumed).status,'exhausted');
 assert.equal(control(s,wire('close'),assumed).status,'ok');
 const max={...initialOwner(binding),closedThrough:'18446744073709551615'};
 assert.equal(control(max,wire('open'),assumed).status,'closed');
 assert.equal(control(max,wire('inspect'),assumed).snapshot.closedThrough,'18446744073709551615');
});
test('unknown local request is not released merely because an open scope has no active decision',()=>{
 const pending={scope:scope(1),requestId:'request-1'};
 const open=control(opened(),wire('inspect'),assumed).snapshot;
 assert.equal(reconcilePending(pending,open).released,false);
 const closed=control(opened(),wire('close'),assumed).snapshot;
 assert.deepEqual(reconcilePending(pending,closed),{released:true,durableDisposition:'unknown'});
 const d=decision(),draining=control(d.state,wire('close'),assumed).snapshot;
 assert.equal(reconcilePending(pending,draining).released,false);
});
test('foreign and delayed newer-scope acknowledgements cannot release an original pending request',()=>{
 const p={scope:scope(2),requestId:'request-2'},oldAck=control(opened(),wire('close'),assumed).snapshot;
 assert.equal(reconcilePending(p,oldAck).released,false);
 assert.equal(reconcilePending({...p,scope:{...scope(2),sidecarGeneration:'c'.repeat(64)}},oldAck).released,false);
});
test('only the original current frame/request/scope can receive success',()=>{
 const captured={scope:scope(1),frameId:'frame-1',requestId:'request-1'};
 assert.equal(canDeliver(captured,{...clone(captured),open:true}),true);
 for(const change of [{open:false},{frameId:'frame-2'},{requestId:'request-2'},{scope:scope(2)}])assert.equal(canDeliver(captured,{...clone(captured),open:true,...change}),false);
});
test('receipt v2 preserves original source request and wire response, and joins the new admitted decision',()=>{
 const d=decision().decision,r=makeReceiptV2(original,req,d);
 assert.equal(r.body.schemaVersion,2);assert.equal(r.body.authorization.scopeSequence,'1');
 assert.deepEqual(r.body.response,original.body.response);assert.equal(r.body.requestDigest,original.body.requestDigest);
 assert.notEqual(r.body.authorization.sidecarGeneration,c.request.report.sidecarGeneration);
 assert.equal(validateReceiptV2(r,context(r)).responseUtf8,canonical(original.body.response));
});
test('historical receipt retry needs no live decision and may use a different current lifetime',()=>{
 const r=makeReceiptV2(original,req,decision().decision);
 const ctx=context(r),before=clone(r),result=validateReceiptV2(r,{...ctx,currentSidecarGeneration:'c'.repeat(64)});
 assert.equal(result.responseUtf8,canonical(original.body.response));assert.deepEqual(r,before);
});
for(const field of ['currentScopeAuthorized','rateAdmitted','sourceAccessQualified','originalProducerQualified'])test(`${field}: an old matching receipt cannot bypass its independent gate`,()=>{
 const r=makeReceiptV2(original,req,decision().decision);
 assert.throws(()=>validateReceiptV2(r,{...context(r),[field]:false}));
});
for(const [name,mutate] of [
 ['authorization substitution',r=>r.body.authorization.scopeSequence='2'],
 ['row commit substitution',r=>r.body.lastCommit.changeDigest='0'.repeat(64)],
 ['response substitution',r=>r.body.response.acceptedHead.reportVersion=3],
 ['request fingerprint substitution',r=>r.body.requestDigest='0'.repeat(64)],
 ['wrong stream',r=>r.key.stream.taskId='foreign'],
 ['unknown field',r=>r.body.authorization.token='forbidden']
])test(`${name}: original provenance cannot be replaced by self-consistent looking data`,()=>{
 const r=makeReceiptV2(original,req,decision().decision),ctx=context(r);mutate(r);assert.throws(()=>validateReceiptV2(r,ctx));
});
test('rehashing changed receipt cannot replace exact original normalized request bytes',()=>{
 const r=makeReceiptV2(original,req,decision().decision),changed=clone(c.request);changed.expectedRunVersion++;
 assert.throws(()=>validateReceiptV2(r,context(r,canonical(changed))));
});
test('new receipt cannot be written using a decision for another descriptor',()=>{
 const d=decision().decision;d.requestUtf8=req.replace('report-c1','other-report');
 assert.throws(()=>makeReceiptV2(original,req,d));
});
test('v1 historical receipt is not silently upgraded by the v2 validator',()=>assert.throws(()=>validateReceiptV2(original,context(original))));
test('maximum-width v2 receipt is rejected under the same 2KiB containing cap',()=>{
 const row={table:'report_receipts',key:clone(old.boundaries.receiptKey),body:{...clone(old.boundaries.receiptBody),schemaVersion:2,
 authorization:{policy:'terminal-ui-correction-v1',sidecarGeneration:'f'.repeat(64),scopeSequence:'18446744073709551615',admissionSequence:'18446744073709551615'}}};
 assert.equal(Buffer.byteLength(canonical(row.key))+Buffer.byteLength(canonical(row.body)),2149);
 assert.throws(()=>validateReceiptV2(row,context(row)));
});

test('owner health failure closes new opens and write admission without discarding existing state',()=>{
 assert.equal(control(initialOwner(binding),wire('open'),{...assumed,ownerHealthy:false}).status,'unavailable');
 const s=opened();assert.equal(admit(s,scope(1),req,{...assumed,ownerHealthy:false}).status,'unavailable');
});
test('one pending main request includes an unknown outcome and cannot be silently replaced',()=>{
 const oldPending={scope:scope(1),frameId:'f',requestId:'r1',outcome:'unknown'};
 const r=beginPending(oldPending,{scope:scope(1),frameId:'f',requestId:'r2'});
 assert.equal(r?.status,'busy');assert.deepEqual(r.pending,oldPending);
 const first=beginPending(null,{scope:scope(1),frameId:'f',requestId:'r2'});
 assert.equal(first.status,'pending');assert.equal(first.pending.requestId,'r2');
});
for(const [name,change] of [
 ['bad digest',ctx=>ctx.originalManifest.changes[0].afterDigest='0'.repeat(64)],
 ['missing receipt membership',ctx=>ctx.originalManifest.changes=[]],
 ['wrong workspace',ctx=>ctx.originalManifest.workspaceId='foreign'],
 ['wrong sequence',ctx=>ctx.originalManifest.commitSequence='99'],
 ['duplicate entry',ctx=>ctx.originalManifest.changes.push(clone(ctx.originalManifest.changes[0]))]
])test(`${name}: historical receipt must join its exact original manifest component`,()=>{
 const r=makeReceiptV2(original,req,decision().decision),ctx=context(r);change(ctx);assert.throws(()=>validateReceiptV2(r,ctx));
});
test('a contradictory control snapshot cannot release an unknown local request',()=>{
 const ack=control(opened(),wire('close'),assumed).snapshot;
 ack.openScopeSequence='1';assert.equal(reconcilePending({scope:scope(1)},ack).released,false);
 ack.openScopeSequence='7';assert.equal(reconcilePending({scope:scope(1)},ack).released,false);
});
test('namespace handoff using the retained owner cannot reset or reuse a closed sequence',()=>{
 const d=decision(),closed=control(d.state,wire('close'),assumed).state;
 const retained=settle(closed,d.decision,'committed',assumed).state;
 assert.equal(retained.closedThrough,'1');assert.equal(retained.lastAdmissionSequence,'1');
 assert.equal(control(retained,wire('open'),assumed).status,'closed');
 const next=control(retained,wire('open',scope(2)),assumed).state;
 assert.equal(admit(next,scope(2),req,assumed).decision.admissionSequence,'2');
});

const golden=JSON.parse(readFileSync(new URL('terminal-authority-vectors.json',import.meta.url)));
for(const trace of golden.traces)test(`${trace.id}: literal independent scope oracle`,()=>{
 let state={...initialOwner(golden.binding),...trace.seed};
 for(const [op,n,want] of trace.steps){
  const sc={...golden.binding,scopeSequence:n};
  const r=op==='admit'?admit(state,sc,golden.requestUtf8,assumed):['committed','rolled-back','uncertain'].includes(op)?settle(state,state.activeDecision,op,assumed):control(state,canonical({schemaVersion:1,operation:op,scope:sc}),assumed);
  state=r.state;
  assert.deepEqual([r.status,state.closedThrough,state.openScopeSequence,state.activeDecision?.scope.scopeSequence??null,state.lastAdmissionSequence],want);
 }
});
test('independent receipt bytes/hash/link and accounting match the real codec, retaining source bytes',()=>{
 assert.equal(digest(readFileSync(new URL('report-publication-vectors.json',import.meta.url))),golden.inputSha256);
 assert.equal(digest(readFileSync(new URL('r2-report-vectors.json',import.meta.url))),golden.reportProfileSha256);
 const d=admit(control(initialOwner(golden.binding),canonical({schemaVersion:1,operation:'open',scope:{...golden.binding,scopeSequence:'1'}}),assumed).state,{...golden.binding,scopeSequence:'1'},golden.requestUtf8,assumed).decision;
 const row=makeReceiptV2(original,golden.requestUtf8,d);row.body.lastCommit=clone(golden.receipt.row.body.lastCommit);
 assert.deepEqual(row,golden.receipt.row);assert.equal(rowCommitment(row),golden.receipt.rowCommitment);
 const result=validateReceiptV2(row,{requestUtf8:golden.requestUtf8,originalRowCommitment:golden.receipt.rowCommitment,originalCommit:golden.receipt.row.body.lastCommit,originalManifest:golden.receipt.manifest,
 currentScopeAuthorized:true,rateAdmitted:true,sourceAccessQualified:true,originalProducerQualified:true,reportProfile});
 assert.equal(result.metadataBytes,golden.receipt.metadataBytes);assert.equal(result.retainedBytes,golden.receipt.retainedBytes);
 assert.equal(result.responseUtf8,golden.responseUtf8);assert.equal(canonical(JSON.parse(golden.requestUtf8).report),golden.reportUtf8);
 assert.equal(digest(golden.requestUtf8),golden.requestSha256);
 const oldBody=clone(original.body);delete oldBody.lastCommit;
 assert.equal(result.retainedBytes-(Buffer.byteLength(canonical(original.key))+Buffer.byteLength(canonical(oldBody))),golden.receipt.authorizationBytes);
 assert.equal(golden.maximum.metadataBytes,2149);assert.equal(golden.maximum.overCapBytes,101);
});
test('independent generator reproduces the complete retained oracle byte-for-byte',()=>{
 const temporary=mkdtempSync(join(tmpdir(),'orkworks-ui-authority-'));
 try{
  const output=join(temporary,'vectors.json');execFileSync('python3',[fileURLToPath(new URL('generate-terminal-authority-vectors.py',import.meta.url)),'--output',output]);
  assert.deepEqual(readFileSync(output),readFileSync(new URL('terminal-authority-vectors.json',import.meta.url)));
 }finally{rmSync(temporary,{recursive:true,force:true});}
});

test('the width witness fails the containing cap independently of any source/request mismatch',()=>{
 assert.throws(()=>receiptUsage(golden.maximum.row),/receipt metadata cap/);
});
test('receipt byte accounting admits exactly 2048 and rejects 2049 complete key/value bytes',()=>{
 // This is only the byte predicate: padding is not an admitted source schema.
 const key={},body={padding:'x'.repeat(2032)};
 assert.equal(Buffer.byteLength(canonical(key))+Buffer.byteLength(canonical(body)),2048);
 assert.equal(receiptUsage({key,body}).metadataBytes,2048);
 body.padding+='x';assert.throws(()=>receiptUsage({key,body}),/receipt metadata cap/);
});

for(const [name,change] of [['extra report property',q=>q.report.hidden='not approved'],['missing report property',q=>delete q.report.limitations],['changed approved questions',q=>q.report.questionIds=['foreign-question']]])test(`${name}: self-consistent hashes still fail closed report normalization`,()=>{
 const q=clone(c.request);change(q);
 const body=clone(q.report);delete body.contentDigest;
 q.report.contentDigest=digest('orkworks.research-report.v1\n'+canonical(body));
 const raw=canonical(q),requestDigest=digest('orkworks.research-report-request.v1\n'+raw);
 const row=clone(original);row.body.requestDigest=requestDigest;row.body.response.acceptedHead.contentDigest=q.report.contentDigest;
 const d={scope:scope(1),admissionSequence:'1',requestUtf8:raw,requestDigest};
 assert.equal(admit(opened(),scope(1),raw,assumed).status,'bad-request');
 assert.throws(()=>makeReceiptV2(row,raw,d));
 row.body.schemaVersion=2;row.body.authorization={policy:'terminal-ui-correction-v1',sidecarGeneration:binding.sidecarGeneration,scopeSequence:'1',admissionSequence:'1'};
 assert.throws(()=>validateReceiptV2(row,context(row,raw)));
});

test('captured profile workspace must match the complete request stream at every receipt path',()=>{
 const foreign=clone(reportProfile);foreign.stream.workspaceId='different-workspace';
 assert.equal(admit(opened(),scope(1),req,{...assumed,reportProfile:foreign}).status,'bad-request');
 assert.throws(()=>makeReceiptWithProfile(original,req,decision().decision,foreign));
 const row=makeReceiptV2(original,req,decision().decision),ctx=context(row);ctx.reportProfile=foreign;
 assert.throws(()=>validateReceiptV2(row,ctx));
});
test('independent regeneration works from a checkout containing spaces and URL escapes',()=>{
 const temporary=mkdtempSync(join(tmpdir(),'orkworks-space-'));
 try{
  const destination=join(temporary,'review % space');
  const source=fileURLToPath(new URL('.',import.meta.url));
  cpSync(source,destination,{recursive:true,filter:entry=>statSync(entry).isDirectory()?resolve(entry)===resolve(source):/\.(mjs|json|py)$/.test(entry)});
  const {NODE_TEST_CONTEXT,...environment}=process.env;
  const output=execFileSync(process.execPath,['--test','--test-reporter=tap','--test-name-pattern=^independent generator reproduces the complete retained oracle byte-for-byte$',join(destination,'terminal-authority-components.test.mjs')],{stdio:'pipe',env:environment}).toString();
  assert.match(output,/^# pass 1$/m);
 }finally{rmSync(temporary,{recursive:true,force:true});}
});

test('missing or malformed frame/request/scope context cannot authorize delivery by equality',()=>{
 assert.equal(canDeliver({},{open:true}),false);
 const captured={scope:scope(1),frameId:'frame-1',requestId:'request-1'};
 assert.equal(canDeliver(captured,{...captured,open:true,frameId:undefined}),false);
 assert.equal(canDeliver({...captured,scope:{}},{...captured,scope:{},open:true}),false);
});
