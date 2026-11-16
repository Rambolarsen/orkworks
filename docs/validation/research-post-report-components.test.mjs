import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync,mkdtempSync,rmSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {canonical,hash} from './root-bootstrap-components.mjs';
import {validatePostReport,checkPostReportRetry,checkSummaryDrop,checkPostReportWidths,measurePostReportRow} from './research-post-report-components.mjs';
import {checkScopeRetry} from './research-scope-rejection-components.mjs';
const raw=()=>readFileSync(new URL('research-post-report-components.json',import.meta.url)),packet=()=>JSON.parse(raw());
const domain=(n,b)=>hash(n+'\n'+canonical(b));
function reseal(p){for(const c of p.cases){const b=c.after.input.value.body;delete b.inputDigest;b.inputDigest=domain('orkworks.preparation-input.v1',b);
 c.after.run.value.body.currentInput.inputDigest=b.inputDigest;
 const r=c.publication.receiptRow;r.value.requestDigest=domain('orkworks.research-report-request.v1',c.publication.request);
 c.measurements=c.writes.map(k=>measurePostReportRow(k==='head'?c.publication.headRow:k==='receipt'?r:c.after[k]));}}
test('Python independently reproduces UTF-8 LF bytes through a spaced path',()=>{const d=mkdtempSync(join(tmpdir(),'post report % '));try{
 const out=join(d,'result %.json'),r=spawnSync('python3',[fileURLToPath(new URL('generate-research-post-report.py',import.meta.url)),'--output',out],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);assert.deepEqual(readFileSync(out),raw());}finally{rmSync(d,{recursive:true,force:true});}});
test('all four finite alternatives validate against independent measurements',()=>{const p=packet();assert.equal(p.cases.length,4);assert.deepEqual(validatePostReport(p),p.cases.map(c=>c.measurements));});
test('literal contract separates first selection, correction and completed summary invalidation',()=>{
 const p=packet();validatePostReport(p);const [first,correction,empty,summary]=p.cases;
 assert.equal(first.publication.request.operation,'submitResearchReport');assert.equal(first.publication.request.expectedReportHead,null);
 assert.equal(first.before.input.value.body.inputRevision,3);assert.equal(first.after.input.value.body.inputRevision,4);
 for(const c of [correction,empty,summary]){assert.equal(c.publication.request.operation,'correctResearchReport');assert.equal(c.publication.request.expectedReportHead.reportVersion,1);assert.equal(c.publication.report.reportVersion,2);assert.equal(c.after.input.value.body.inputRevision,5);}
 for(const c of p.cases){assert.equal(c.after.input.value.body.decision.inputRevision,3);assert.deepEqual(c.before.aTask,c.after.aTask);assert.deepEqual(c.before.bTask,c.after.bTask);assert.deepEqual(c.before.bPlan,c.after.bPlan);assert.deepEqual(c.before.bExpectation,c.after.bExpectation);assert.equal(c.after.run.value.body.pendingPlan,null);assert.equal(c.after.bPlan.value.body.status,'cancelled');assert.equal(c.before.bridge.value.body.state,'nonterminal');assert.equal(c.publication.request.expectedTaskVersion,c.before.aTask.value.body.taskVersion);}
 assert.equal(empty.before.aTask.value.body.status,'reported_complete');assert.equal(empty.before.aTask.value.body.taskVersion,9);assert.equal(empty.before.bridge.value.body.assignmentIdentity.taskVersion,2);
 assert.deepEqual(empty.before.selector,empty.after.selector);assert.equal(summary.before.selector.value.body.summaryVersion,1);assert.equal(summary.after.selector.value.body.summaryVersion,1);assert.equal(summary.after.selector.value.body.currentSummary,null);assert.deepEqual(summary.before.summary,summary.after.summary);
});
const mutations=[
 ['pending B restored',p=>p.cases[0].after.run.value.body.pendingPlan=p.cases[0].before.bPlan.key],
 ['unknown original B outcome',p=>p.cases[0].original.rejection=null],
 ['missing original completion',p=>p.cases[2].original.completion=null],
 ['wrong original rejected control',p=>p.cases[0].before.bPlan.value.body.status='proposed'],
 ['B made collecting',p=>p.cases[0].before.bExpectation.value.body.collecting=true],
 ['collection retargeted to B',p=>p.cases[0].after.collection.value.body.task=p.cases[0].before.bTask.key],
 ['closed child copied as live',p=>p.cases[2].before.bridge.value.body.state='terminal'],
 ['wrong original runtime',p=>p.cases[2].before.bridge.value.body.runtime.launchGeneration='0'.repeat(64)],
 ['old frozen task CAS',p=>p.cases[2].publication.request.expectedTaskVersion=2],
 ['fresh child-authored assessment',p=>p.cases[0].after.input.value.body.decision.inputRevision=4],
 ['missing original assessment',p=>p.originalAssessment=null],
 ['input rollback to A',p=>p.cases[1].after.input.value.body.records=JSON.parse(readFileSync(new URL('research-prefix-components.json',import.meta.url))).inputs[0].value.body.records],
 ['B configuration dropped',p=>p.cases[1].after.input.value.body.records=p.cases[1].after.input.value.body.records.filter(x=>x.kind!=='configuration')],
 ['prior-plan dropped',p=>p.cases[1].after.input.value.body.records=p.cases[1].after.input.value.body.records.filter(x=>x.kind!=='prior-plan')],
 ['B captures used as A evidence',p=>p.cases[1].publication.source.reportIdentity.configurationDigest=p.cases[1].before.bExpectation.value.body.configuration.configurationDigest],
 ['wrong report producer',p=>p.cases[1].after.input.value.producer=p.originalAssessment.value.producer],
 ['A task version changed by report',p=>p.cases[1].after.aTask.value.body.taskVersion++],
 ['original result rewritten',p=>p.cases[2].after.aTask.value.body.coordinationResult=null],
 ['completed plan reopened',p=>p.cases[2].after.aPlan.value.body.status='active'],
 ['original summary rewritten',p=>p.cases[3].after.summary.value.body.findings=['replacement']],
 ['stale summary pointer retained',p=>p.cases[3].after.selector.value.body.currentSummary=p.cases[3].before.selector.value.body.currentSummary],
 ['summary high-water advanced on invalidation',p=>p.cases[3].after.selector.value.body.summaryVersion++],
 ['completed-plan selector cleared',p=>p.cases[3].after.selector.value.body.completedResearchPlan=null],
 ['null selector rewritten',p=>p.cases[2].after.selector.value.lastCommit=p.cases[2].after.run.value.lastCommit],
 ['false expectation spuriously advanced',p=>p.cases[0].after.bExpectation.value.body.expectationVersion++],
 ['duplicate prospective-removal increment',p=>p.cases[1].after.collection.value.body.collectionVersion+=1],
 ['double run increment',p=>p.cases[1].after.run.value.body.runVersion+=1],
 ['independent blocker cleared',p=>p.cases[0].after.run.value.body.blockers=[]],
 ['invented ordinary revision',p=>p.cases[2].after.bridge.value.body.producerRevision='99'],
 ['wrong closed response',p=>p.cases[1].publication.receiptRow.value.response.acceptedHead.reportVersion=7],
 ['new storage admission',p=>p.storageFormat=57]
];
for(const [name,mutate] of mutations)test('denies '+name,()=>{const p=packet();mutate(p);reseal(p);assert.throws(()=>validatePostReport(p));});
test('exact report retry returns original response despite later live versions',()=>{const p=packet(),c=p.cases[3],before=structuredClone(c);assert.deepEqual(checkPostReportRetry(c.publication,c.publication.request,{...p.facts.child,runVersion:999,inputRevision:64,taskVersion:999}),c.publication.receiptRow.value.response);assert.deepEqual(c,before);});
test('revocation denies before original lookup',()=>{assert.throws(()=>checkPostReportRetry(null,null,{authority:'revoked'}),/authority/);});
for(const k of ['runtime','rate','currentness','membership','sourceProofs','disposition'])test('unknown '+k+' denies report retry',()=>{const p=packet(),c=p.cases[0];assert.throws(()=>checkPostReportRetry(c.publication,c.publication.request,{...p.facts.child,[k]:'unknown'}));});
test('terminal run denies even original report lookup',()=>{const p=packet(),c=p.cases[0];assert.throws(()=>checkPostReportRetry(c.publication,c.publication.request,{...p.facts.child,runState:'cancelled'}));});
test('changed original request conflicts',()=>{const p=packet(),c=p.cases[0],req=structuredClone(c.publication.request);req.expectedTaskVersion++;assert.throws(()=>checkPostReportRetry(c.publication,req,p.facts.child));});
for(const [name,mutate] of [
 ['receipt key',x=>x.receiptRow.key.reportVersion=8],['receipt CAS',x=>x.receiptRow.value.expectedTaskVersion=2],['receipt operation',x=>x.receiptRow.value.operation='wrong'],['receipt C',x=>x.receiptRow.value.lastCommit.commitSequence='999'],['head row',x=>x.headRow.value.head.reportVersion=8]
])test('retry rejects altered '+name,()=>{const p=packet(),c=p.cases[1];mutate(c.publication);assert.throws(()=>checkPostReportRetry(c.publication,c.publication.request,p.facts.child));});
test('missing original outcome proof never becomes current-state retry',()=>{const p=packet(),c=p.cases[1];assert.throws(()=>checkPostReportRetry(c.publication,c.publication.request,{...p.facts.child,outcomes:{}}));});
test('summary invalidation needs no next high-water even at 64',()=>{const c=packet().cases[3],old=structuredClone(c.before.selector),next=structuredClone(c.after.selector);old.value.body.summaryVersion=next.value.body.summaryVersion=64;old.value.body.currentSummary.summaryVersion=64;assert.equal(checkSummaryDrop(old,next),true);});
test('row projection keeps producer and embedded origins but omits outer lastCommit',()=>{const r=packet().cases[1].after.input,m=measurePostReportRow(r);r.value.lastCommit.commitSequence='999';assert.equal(measurePostReportRow(r).chargeBytes,m.chargeBytes);delete r.value.producer;assert(measurePostReportRow(r).chargeBytes<m.chargeBytes);});
test('width-only input65 and257 records deny, no body truncation',()=>{const r=packet().cases[0].after.input;r.value.body.inputRevision=65;assert.throws(()=>checkPostReportWidths([r]));r.value.body.inputRevision=4;r.value.body.records=Array(257).fill({kind:'padding'});assert.throws(()=>checkPostReportWidths([r]));});
test('literal independent represented-row charges include copied producer and summary selector',()=>{const p=packet();validatePostReport(p);
 assert.deepEqual(p.cases.map(c=>c.measurements.reduce((n,x)=>n+x.chargeBytes,0)),[10438,10564,10332,10969]);
 assert.deepEqual(p.cases.map(c=>c.measurements[2]),Array(4).fill({chargeBytes:3861,wrapperBytes:4062}));
});
test('original Main rejection remains its old outcome after a later child report',()=>{const p=packet();for(const c of p.cases){const r=checkScopeRetry(c.original,c.original.rejection.value.body.request,{...p.facts.main,runVersion:999,inputRevision:64});
 assert.equal(r.rejection.runVersion,c.id==='incomplete-first'?21:22);assert.notEqual(r.rejection.runVersion,c.after.run.value.body.runVersion);
 assert.equal(r.completion?.planVersion??null,c.original.completion?3:null);
}});
