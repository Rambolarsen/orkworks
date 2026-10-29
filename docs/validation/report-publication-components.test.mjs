import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {validatePublicationState,executePublication,verifyPublicationCase,detailReadStatus} from './report-publication-components.mjs';
const seed=JSON.parse(readFileSync(new URL('r2-report-vectors.json',import.meta.url)));
const packet=JSON.parse(readFileSync(new URL('report-publication-vectors.json',import.meta.url)));
const clone=structuredClone;
const context=(extra={})=>({...clone(packet.capturedContext),...extra});
const find=(rows,table)=>rows.find(r=>r.table===table);
const baseline=()=>clone(packet.states.current);
const first=packet.cases[0], request=()=>clone(first.request);
const beforeFor=c=>packet.states[c.before]??packet.cases.find(x=>x.id===c.before).after;
for(const c of packet.cases)test(`${c.id}: independently generated complete state, manifest and receipt match`,()=>{
 assert.equal(verifyPublicationCase(c,beforeFor(c),packet,seed),true);
});
test('required correction changes material once and retains C2 failure',()=>{
 const r=executePublication(baseline(),request(),seed,context());
 const s=find(r.rows,'summary_state').body;
 assert.equal(s.row.summaryRevision,'4');assert.equal(s.subjectRevision,'11');
 assert.deepEqual(s.row.state.material.requiredFindings.map(f=>f.findingId),['f-c2']);
 assert.equal(s.row.state.material.overall,'needs-rework');
 assert.equal(r.snapshotVersion,8);assert.equal(r.invalidatedRuns.length,1);
});
test('optional-only correction advances freshness CAS and snapshot, retaining material revision',()=>{
 const c=packet.cases.find(c=>c.id==='optional-O');
 const r=executePublication(baseline(),c.request,seed,context());
 const s=find(r.rows,'summary_state').body;
 assert.equal(s.row.summaryRevision,'3');assert.equal(s.subjectRevision,'11');
 assert.deepEqual(s.row,find(packet.states.current,'summary_state').body.row);
 assert.equal(r.snapshotVersion,8);
});
test('exact v2 retry after v3 returns original receipt before live CAS with no reinvalidation',()=>{
 const r=executePublication(clone(packet.cases.at(-1).after),request(),seed,context({snapshotVersion:9}));
 assert.equal(r.oracle.outcome,'retry');assert.equal(r.oracle.commit,null);
 assert.deepEqual(r.rows,packet.cases.at(-1).after);assert.deepEqual(r.oracle.response,first.oracle.response);
 assert.deepEqual(r.invalidatedRuns,[]);assert.equal(r.snapshotVersion,9);
});
for(const [key,value,want] of [['authority',false,'denied'],['epochCurrent',false,'denied'],['rateAdmitted',false,'limit-exceeded'],
 ['sourceScope',false,'unavailable'],['terminationSettled',false,'unavailable'],['noUnsupportedConsumers',false,'unavailable']])test(`${key}: captured failed precondition cannot use saved retry`,()=>{
 const rows=clone(first.after),r=executePublication(rows,request(),seed,context({[key]:value}));
 assert.equal(r.oracle.outcome,want);assert.equal(r.oracle.commit,null);assert.equal(r.oracle.response,null);assert.deepEqual(r.rows,rows);
});
for(const [name,mutate,want] of [
 ['stale run CAS',r=>r.expectedRunVersion=3,'conflict'],
 ['stale head CAS',r=>r.expectedReportHead.contentDigest='0'.repeat(64),'conflict'],
 ['different report ID',r=>r.report.reportId='branch','conflict'],
 ['version gap',r=>r.report.reportVersion=3,'conflict'],
 ['changed operation',r=>r.operation='submitResearchReport','denied'],
 ['foreign stream',r=>r.stream.workspaceId='foreign','denied'],
 ['unknown request field',r=>r.unknown=true,'denied']])test(`${name}: no partial publication`,()=>{
 const rows=baseline(),req=request();mutate(req);
 // Keep changed descriptor internally hashed, so conflict/schema checks are exercised.
 if(['different report ID','version gap'].includes(name)){
  const b=clone(req.report);delete b.contentDigest;
  const canonical=x=>Array.isArray(x)?'['+x.map(canonical)+']':x&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
  req.report.contentDigest=createHash('sha256').update('orkworks.research-report.v1\n'+canonical(b)).digest('hex');
 }
 const r=executePublication(rows,req,seed,context());assert.equal(r.oracle.outcome,want);
 assert.equal(r.oracle.commit,null);assert.deepEqual(r.rows,rows);assert.deepEqual(rows,packet.states.current);
});
for(const [name,change] of [
 ['missing edge',rows=>rows.splice(rows.findIndex(r=>r.table==='dependencies'),1)],
 ['extra edge',rows=>{const r=clone(find(rows,'dependencies'));r.key.purpose='proposal-input';rows.push(r);}],
 ['missing receipt',rows=>rows.splice(rows.findIndex(r=>r.table==='report_receipts'),1)],
 ['missing report head',rows=>rows.splice(rows.findIndex(r=>r.table==='report_heads'),1)],
 ['wrong ledger',rows=>find(rows,'quota_usage').body.used='1'],
 ['missing ledger',rows=>rows.splice(rows.findIndex(r=>r.table==='quota_usage'),1)],
 ['wrong row link',rows=>find(rows,'run_states').body.lastCommit.changeDigest='0'.repeat(64)],
 ['missing proof',rows=>rows.splice(rows.findIndex(r=>r.table==='commit_proof'),1)],
 ['duplicate row',rows=>rows.push(clone(rows[0]))],
 ['active run',rows=>find(rows,'run_states').body.terminal.state='assessing'],
 ['terminal blockers',rows=>find(rows,'run_states').body.terminal.blockers=['retain me']],
 ['unknown terminal property',rows=>find(rows,'run_states').body.terminal.hidden=true],
 ['run ID/epoch drift',rows=>find(rows,'run_states').body.terminal.admissionEpoch='0'.repeat(64)],
 ['head elected from catalog',rows=>find(rows,'report_heads').body.head.reportVersion=2],
 ['stored descriptor size drift',rows=>rows.find(r=>r.table==='objects'&&r.key.kind==='assignment-evidence-source').body.body.sizeBytes--],
 ['evaluation substitution',rows=>rows.find(r=>r.table==='objects'&&r.key.kind==='assignment-evaluation').body.body.findings=[]],
 ['wrong checkpoint',rows=>find(rows,'summary_state').body.checkpoint.summaryRevision='2']
])test(`${name}: malformed prestate is preserved and unavailable`,()=>{
 const rows=baseline();change(rows);const copy=clone(rows);
 assert.throws(()=>validatePublicationState(rows,seed,context()));
 const r=executePublication(rows,request(),seed,context());assert.equal(r.oracle.outcome,'unavailable');assert.equal(r.oracle.commit,null);
 assert.deepEqual(r.rows,copy);assert.deepEqual(rows,copy);
});
for(const [name,limits] of [['command bytes',{maxCommandBytes:1}],['changed rows',{maxChangedRows:1}],['visited rows',{visitedLimit:1}],
 ['report run quota',{maxRunBytes:1}],['workspace report quota',{maxWorkspaceReportBytes:1}],
 ['lineage fence capacity',{currentRowCapacity:false,maxLineageBytes:1}]])test(`${name}: capacity denial preserves entire old store`,()=>{
 const rows=baseline(),r=executePublication(rows,request(),seed,context(limits));
 assert.equal(r.oracle.outcome,'limit-exceeded');assert.deepEqual(r.rows,rows);assert.equal(r.oracle.commit,null);assert.equal(r.oracle.response,null);
});
for(const [name,mutate] of [
 ['omitted source effect',c=>c.after.splice(c.after.findIndex(r=>r.table==='report_receipts'&&r.key.reportVersion===2),1)],
 ['same material retokened',c=>find(c.after,'summary_state').body.row.summaryRevision='5'],
 ['wrong invalidation oracle',c=>c.oracle.afterStateDigest='0'.repeat(64)],
 ['missing manifest change',c=>find(c.after.filter(r=>r.table==='commit_proof'&&r.key==='2'),'commit_proof').body.body.changes.pop()],
 ['unchanged row relinked',c=>c.after.find(r=>r.table==='objects'&&r.key.kind==='assignment-evaluation').body.lastCommit=c.oracle.commit],
 ['wrong command byte total',c=>c.measurement.commandBytes--]
])test(`${name}: claimed accepted witness rejects`,()=>{
 const c=clone(first);mutate(c);assert.throws(()=>verifyPublicationCase(c,packet.states.current,packet,seed));
});
test('snapshot mismatch rejects optional-only stale capture',()=>assert.equal(detailReadStatus(7,8,true),'stale-reference'));
test('missing proof is unavailable even when snapshot matches',()=>assert.equal(detailReadStatus(8,8,false),'unavailable'));
test('unchanged coherent capture remains current',()=>assert.equal(detailReadStatus(8,8,true),'current'));

test('visited-row budget includes repeated scans and final validation',()=>{
 const rows=baseline();
 const denied=executePublication(rows,request(),seed,context({visitedLimit:rows.length}));
 assert.equal(denied.oracle.outcome,'limit-exceeded');assert.deepEqual(denied.rows,rows);
 const accepted=executePublication(rows,request(),seed,context());
 assert(accepted.logicalRowsVisited>rows.length);assert(accepted.logicalRowsVisited<=100000);
});
// Recommit only a captured synthetic seed to ensure structural attacks fail on
// semantic joins, rather than merely on the old outer row/manifest hashes.
const canon=x=>Array.isArray(x)?'['+x.map(canon).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canon(x[k])).join(',')+'}':JSON.stringify(x);
const sha=x=>createHash('sha256').update(x).digest('hex');
const dh=(d,x)=>sha(d+'\n'+canon(x));
function rebindCaptured(rows){
 const header=find(rows,'store_header'),proof=find(rows,'commit_proof');
 const body=r=>{const b=clone(r.body);delete b.lastCommit;if(r.table==='summary_state'&&b.checkpoint)delete b.checkpoint.applied;return b;};
 const changes=rows.filter(r=>!['store_header','commit_proof'].includes(r.table)).map(r=>({table:r.table,key:r.key,beforeDigest:null,afterDigest:dh('orkworks.hierarchy-storage-row.v3',{table:r.table,key:r.key,body:body(r)})})).sort((a,b)=>Buffer.compare(Buffer.from(a.table),Buffer.from(b.table))||Buffer.compare(Buffer.from(canon(a.key)),Buffer.from(canon(b.key))));
 proof.body.body.changes=changes;proof.body.changeDigest=dh('orkworks.hierarchy-storage-change.v3',proof.body.body);
 const link={storageEpoch:header.body.storageEpoch,commitSequence:'1',changeDigest:proof.body.changeDigest};header.body.latest=link;
 for(const r of rows.filter(r=>!['store_header','commit_proof'].includes(r.table))){r.body.lastCommit=link;if(r.table==='summary_state'&&r.body.checkpoint)r.body.checkpoint.applied=link;}
 rows.sort((a,b)=>Buffer.compare(Buffer.from(a.table),Buffer.from(b.table))||Buffer.compare(Buffer.from(canon(a.key)),Buffer.from(canon(b.key))));
 return rows;
}
for(const [name,change] of [
 ['missing retained edge',rows=>rows.splice(rows.findIndex(r=>r.table==='dependencies'),1)],
 ['unsupported consumer edge',rows=>{const r=clone(find(rows,'dependencies'));r.key.purpose='proposal-input';rows.push(r);}],
 ['missing history receipt',rows=>rows.splice(rows.findIndex(r=>r.table==='report_receipts'),1)],
 ['unaccounted bytes',rows=>find(rows,'quota_usage').body.used='1'],
 ['active lifecycle',rows=>find(rows,'run_states').body.terminal.state='assessing'],
 ['source size uses hash-input bytes',rows=>rows.find(r=>r.table==='objects'&&r.key.kind==='assignment-evidence-source').body.body.sizeBytes-=83],
 ['receipt request binding drift',rows=>find(rows,'report_receipts').body.expectedTaskVersion=2],
 ['receipt claims live head',rows=>find(rows,'report_receipts').body.response.current=true],
 ['terminal state unknown member',rows=>find(rows,'run_states').body.terminal.extra='unsupported'],
 ['material checkpoint drift',rows=>find(rows,'summary_state').body.checkpoint.summaryRevision='2']
])test(`${name}: coherent outer fingerprints cannot bypass source/state rules`,()=>{
 const rows=baseline();change(rows);rebindCaptured(rows);
 assert.throws(()=>validatePublicationState(rows,seed,context()));
 const r=executePublication(rows,request(),seed,context());assert.equal(r.oracle.outcome,'unavailable');assert.deepEqual(r.rows,rows);
});
test('changed canonical request for accepted version conflicts without live CAS',()=>{
 const req=request();req.expectedRunVersion=99;
 const rows=clone(packet.cases.at(-1).after),r=executePublication(rows,req,seed,context());
 assert.equal(r.oracle.outcome,'conflict');assert.deepEqual(r.rows,rows);assert.equal(r.oracle.response,null);
});
test('eight child/user versions share the same stream history',()=>{
 let rows=clone(packet.cases.at(-1).after);
 for(let v=4;v<=8;v++){
  const req=clone(packet.cases.at(-1).request);
  req.expectedRunVersion=v+2;
  req.expectedReportHead=clone(rows.find(r=>r.table==='report_heads'&&r.key.taskId==='research-c1').body.head);
  req.report.reportVersion=v;const body=clone(req.report);delete body.contentDigest;req.report.contentDigest=dh('orkworks.research-report.v1',body);
  const r=executePublication(rows,req,seed,context({snapshotVersion:v+5}));assert.equal(r.oracle.outcome,'accepted');assert.equal(r.snapshotVersion,v+6);rows=r.rows;
 }
 assert.equal(rows.filter(r=>r.table==='report_receipts'&&r.key.stream.taskId==='research-c1').length,8);
 const req=clone(packet.cases.at(-1).request);req.expectedRunVersion=11;
 req.expectedReportHead=clone(rows.find(r=>r.table==='report_heads'&&r.key.taskId==='research-c1').body.head);
 req.report.reportVersion=9;const body=clone(req.report);delete body.contentDigest;req.report.contentDigest=dh('orkworks.research-report.v1',body);
 const r=executePublication(rows,req,seed,context());assert.equal(r.oracle.outcome,'denied');assert.deepEqual(r.rows,rows);
});
test('subject CAS exhaustion cannot wrap or partially publish a correction',()=>{
 const rows=baseline();find(rows,'summary_state').body.subjectRevision='18446744073709551615';rebindCaptured(rows);
 const r=executePublication(rows,request(),seed,context());assert.equal(r.oracle.outcome,'limit-exceeded');assert.deepEqual(r.rows,rows);
});
test('missing tracked-run invalidation membership cannot return accepted publication',()=>{
 const rows=baseline(),r=executePublication(rows,request(),seed,context({ownerRuns:[]}));
 assert.equal(r.oracle.outcome,'unavailable');assert.deepEqual(r.rows,rows);
});
for(const c of packet.denials)test(`${c.id}: independent no-commit denial oracle matches`,()=>{
 const rows=clone(packet.states[c.before]),r=executePublication(rows,c.request,seed,context(c.contextOverrides));
 assert.deepEqual(r.oracle,c.oracle);assert.deepEqual(r.rows,rows);assert.deepEqual(r.invalidatedRuns,[]);
});
test('full maximum-width receipt and head include outer metadata/link bytes',()=>{
 const b=packet.boundaries;
 assert.equal(Buffer.byteLength(canon(b.receiptKey))+Buffer.byteLength(canon(b.receiptBody)),1925);
 assert.equal(b.receiptBytes,1925);assert(b.receiptBytes<=2048);
 assert.equal(Buffer.byteLength(canon(b.headBody)),b.headBytes);assert(b.headBytes<=512);
});

test('catalog binds the unchanged original R2 file bytes',()=>{
 const raw=readFileSync(new URL('r2-report-vectors.json',import.meta.url));
 assert.equal(packet.seedSha256,sha(raw));
});
