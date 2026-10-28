import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {validateReport,validateReportEvidenceSource,emptyCapturedState,applyCaptured,verifyR2} from './r2-report-components.mjs';
const packet=JSON.parse(readFileSync(new URL('r2-report-vectors.json',import.meta.url)));
const canonical=x=>Array.isArray(x)?'['+x.map(canonical).join(',')+']':x!==null&&typeof x==='object'?'{'+Object.keys(x).sort().map(k=>JSON.stringify(k)+':'+canonical(x[k])).join(',')+'}':JSON.stringify(x);
const hash=raw=>createHash('sha256').update(raw).digest('hex');
const domain=(d,x)=>hash(d+'\n'+canonical(x));
const report=id=>JSON.parse(packet.reports.find(r=>r.id===id).canonicalUtf8);
const profile=id=>structuredClone(packet.profiles.find(p=>p.id===id));
const request=id=>JSON.parse(packet.requests.find(r=>r.id===id).canonicalUtf8);
const seal=r=>{delete r.contentDigest;r.contentDigest=domain('orkworks.research-report.v1',r);return r;};
const initial=()=>{
  let state=emptyCapturedState(packet.profiles);
  for(const id of ['c1-v1','c2-v1','o-v1'])state=applyCaptured(state,request(id),profile(id.split('-')[0])).state;
  return state;
};
for(const entry of [...packet.reports,{...packet.urlComponent,id:'URL component'}])test(`${entry.id}: independent canonical report bytes preserve stored/hash-input distinction`,()=>{
  const r=JSON.parse(entry.canonicalUtf8);assert.equal(Buffer.byteLength(entry.canonicalUtf8),entry.utf8Bytes);
  assert.equal(hash(entry.canonicalUtf8),entry.sha256);assert.equal(canonical(r),entry.canonicalUtf8);
  assert.equal(validateReport(r,profile(entry.profileId)),r.contentDigest);
  const {contentDigest,...body}=r;assert.notEqual(domain('orkworks.research-report.v1',r),contentDigest);
  assert.equal(domain('orkworks.research-report.v1',body),contentDigest);
});
for(const [name,mutate] of [
  ['unknown report field',r=>r.workspaceId='injected'],['missing array',r=>delete r.limitations],
  ['unknown author',r=>r.author={kind:'parent'}],['missing schema version',r=>delete r.schemaVersion],
  ['version nine',r=>r.reportVersion=9],['string version',r=>r.reportVersion='1'],
  ['wrong assignment',r=>r.reservationId='other'],['old generation',r=>r.launchGeneration='0'.repeat(64)],
  ['wrong question set',r=>r.questionIds=['foreign']],['duplicate questions',r=>r.questionIds.push(r.questionIds[0])],
  ['unknown finding field',r=>r.findings[0].id='invented'],['unmapped finding question',r=>r.findings[0].questionIds=['foreign']],
  ['unknown classification',r=>r.findings[0].classification='verified'],['established without reference',r=>r.findings[0].evidenceReferenceIds=[]],
  ['unknown finding reference',r=>r.findings[0].evidenceReferenceIds=['foreign']],['duplicate finding references',r=>r.findings[0].evidenceReferenceIds.push('e-c1')],
  ['unknown reference property',r=>r.evidenceReferences[0].token='forged'],['unknown reference kind',r=>r.evidenceReferences[0].kind='connector'],
  ['absolute path',r=>r.evidenceReferences[0].path='/research/c1.txt'],['parent path',r=>r.evidenceReferences[0].path='../c1.txt'],
  ['wrong revision',r=>r.evidenceReferences[0].revision='newer'],['wrong excerpt digest',r=>r.evidenceReferences[0].excerptDigest='0'.repeat(64)],
  ['combined uncertainty/blocker overflow',r=>{r.uncertainties=Array(8).fill('unknown');r.blockers=Array(9).fill('blocked');}],
  ['combined recommendation/limitation overflow',r=>{r.recommendations=Array(8).fill('next');r.limitations=Array(9).fill('limited');}],
  ['finding count overflow',r=>r.findings=Array(33).fill(r.findings[0])],['explanation byte overflow',r=>r.limitations=['æ'.repeat(1025)]],
])test(`report rejects ${name} even with recomputed content hash`,()=>{
  const r=report('c1-v1');mutate(r);seal(r);assert.throws(()=>validateReport(r,profile('c1')));
});
test('a declared unverified finding may have no references without inventing evidence',()=>{
  const r=report('c1-v1');r.findings[0].classification='unverified';r.findings[0].evidenceReferenceIds=[];
  assert.equal(validateReport(seal(r),profile('c1')),r.contentDigest);
});
test('exact 2 KiB raw excerpt is accepted; empty, oversized and changed payloads are rejected',()=>{
  for(const count of [0,2048,2049]){
    const r=report('c1-v1'),p=profile('c1'),raw=Buffer.alloc(count,120);
    r.evidenceReferences[0].excerptDigest=hash(raw);
    const capture=p.captures.find(c=>c.reference.kind==='file'&&c.reference.revision==='fixture-v1');
    capture.reference=structuredClone(r.evidenceReferences[0]);capture.bytesBase64=raw.toString('base64');seal(r);
    if(count===2048)assert.equal(validateReport(r,p),r.contentDigest);
    else assert.throws(()=>validateReport(r,p));
  }
  const p=profile('c1');p.captures[0].bytesBase64=Buffer.from('different').toString('base64');
  assert.throws(()=>validateReport(report('c1-v1'),p));
});
function urlCase(url,time){
  const r=JSON.parse(packet.urlComponent.canonicalUtf8),p=profile('c1');
  if(url!==undefined)r.evidenceReferences[0].url=url;
  if(time!==undefined)r.evidenceReferences[0].retrievedAt=time;
  p.captures.find(c=>c.reference.kind==='url').reference=structuredClone(r.evidenceReferences[0]);
  return [seal(r),p];
}
for(const url of ['https://example.test','http://example.test:0/a?x=y#part','HTTPS://[::1]:65535/a%2fb','https://[v1.a:b]/'])test(`URI profile preserves valid spelling ${url}`,()=>{
  const [r,p]=urlCase(url);assert.equal(validateReport(r,p),r.contentDigest);assert.equal(r.evidenceReferences[0].url,url);
});
for(const url of ['file:///tmp/a','https:example.test','https://','https://user@example.test/a','https://example.test:','https://example.test:65536/a',
  'https://example.test/%Q0','https://example.test/a b','https://example.test/æ','https://example.test\\a','https://[bad]/'])test(`URI profile rejects ${url}`,()=>{
    const [r,p]=urlCase(url);assert.throws(()=>validateReport(r,p));
});
for(const time of ['2026-02-30T12:00:00Z','2026-10-28T12:00:00+01:00','2026-10-28T25:00:00Z','2026-10-28T12:00:00.1234567890Z'])test(`retrieval timestamp rejects ${time}`,()=>{
  const [r,p]=urlCase(undefined,time);assert.throws(()=>validateReport(r,p));
});
test('request digest hashes the complete accepted descriptor and exact nullable expected fields',()=>{
  for(const entry of packet.requests){
    const r=JSON.parse(entry.canonicalUtf8);assert.equal(hash(entry.canonicalUtf8),entry.sha256);
    assert.equal(domain('orkworks.research-report-request.v1',r),entry.requestDigest);
    assert.equal(r.report.contentDigest,validateReport(r.report,profile(entry.profileId)));
  }
});
test('two logical corrections keep one immutable evaluation but distinguish C1 currentness and optional O',()=>{
  const result=verifyR2(packet);
  assert.equal(result.evaluationDigest,packet.evaluation.evaluationDigest);
  assert.deepEqual(result.histories,[{id:'required-C1-correction',sourceStates:{c1:'stale',c2:'current',o:'current'},freshness:'historical',requiredFindingIds:['f-c2']},
    {id:'optional-O-correction',sourceStates:{c1:'current',c2:'current',o:'stale'},freshness:'historical',requiredFindingIds:['f-c1','f-c2']}]);
});
test('initial publication and user correction advance run/head once; exact retry preserves the newer head',()=>{
  const before=initial();assert.equal(before.runVersion,4);
  const accepted=applyCaptured(before,request('c1-v2'),profile('c1'));assert.equal(accepted.kind,'accepted');assert.equal(accepted.state.runVersion,5);
  const after=structuredClone(accepted.state);
  const replay=applyCaptured(after,request('c1-v1'),profile('c1'));
  assert.equal(replay.kind,'retry');assert.equal(replay.acceptedHead.reportVersion,1);assert.deepEqual(replay.state,after);
  const replayCorrection=applyCaptured(after,request('c1-v2'),profile('c1'));assert.equal(replayCorrection.kind,'retry');assert.deepEqual(replayCorrection.state,after);
  assert.deepEqual(before,initial());
});
for(const [name,mutate] of [
  ['stale run CAS',r=>r.expectedRunVersion=3],['head digest drift',r=>r.expectedReportHead.contentDigest='0'.repeat(64)],
  ['version gap',r=>{r.report.reportVersion=3;seal(r.report);}],['changed fixed report ID',r=>{r.report.reportId='replacement';seal(r.report);}],
  ['caller-supplied wrong author',r=>{r.report.author.kind='child';seal(r.report);}],['foreign workspace',r=>r.stream.workspaceId='foreign'],
  ['missing expected member',r=>delete r.expectedTaskVersion],['unknown request field',r=>r.bearer='injected'],
])test(`captured correction rejects ${name} without changing state`,()=>{
  const before=initial(),snapshot=structuredClone(before),r=request('c1-v2');mutate(r);
  assert.throws(()=>applyCaptured(before,r,profile('c1')));assert.deepEqual(before,snapshot);
});
test('same-version changed bytes conflict and a stale generation fails before saved-request lookup',()=>{
  const state=initial(),changed=request('c1-v1');changed.report.findings[0].answer='Changed';seal(changed.report);
  assert.throws(()=>applyCaptured(state,changed,profile('c1')));
  const p=profile('c1');p.lineage.launchGeneration='0'.repeat(64);
  assert.throws(()=>applyCaptured(state,request('c1-v1'),p));
});
for(const [name,mutate] of [
  ['evaluation byte',p=>p.evaluation.canonicalUtf8=p.evaluation.canonicalUtf8.replace('c1 cites','changed cites')],
  ['report byte',p=>p.reports[0].canonicalUtf8=p.reports[0].canonicalUtf8.replace('original observation.','changed observation.')],
  ['source size',p=>{const s=JSON.parse(p.evidenceSources[0].canonicalUtf8);s.sizeBytes--;p.evidenceSources[0].canonicalUtf8=canonical(s);}],
  ['corrected evidence substitution',p=>{const e=JSON.parse(p.evaluation.canonicalUtf8);e.criterionOutcomes[0].evidenceReferences=[p.evidenceSources.find(s=>s.reportEntryId==='c1-v2').sourceKey];p.evaluation.canonicalUtf8=canonical(e);}],
  ['claimed currentness',p=>p.histories[0].expectedSourceStates.c1='current'],
  ['wrong committed head',p=>p.histories[0].expectedState.heads[0].head.reportVersion=1],
  ['summary hides C2',p=>{const r=JSON.parse(p.histories[0].response.canonicalUtf8);r.detail.material.requiredFindings=[];p.histories[0].response.canonicalUtf8=canonical(r);}],
])test(`R2 joins reject ${name}`,()=>{
  const p=structuredClone(packet);mutate(p);assert.throws(()=>verifyR2(p));
});

test('serialized report, finding, reference and complete request caps remain independent',()=>{
  const p=profile('c1'),r=report('c1-v1');
  r.uncertainties=[...Array(15).fill('x'.repeat(2000)),'x'];seal(r);
  const needed=32768-Buffer.byteLength(canonical(r));assert(needed>=0&&needed+1<=2048);
  r.uncertainties[15]='x'.repeat(needed+1);seal(r);assert.equal(Buffer.byteLength(canonical(r)),32768);
  assert.equal(validateReport(r,p),r.contentDigest);
  const req=request('c1-v1');req.report=r;
  assert.throws(()=>applyCaptured(emptyCapturedState(packet.profiles),req,p),'32 KiB descriptor does not imply request fits');
  r.uncertainties[15]+='x';seal(r);assert.throws(()=>validateReport(r,p));
  const f=report('c1-v1');f.findings[0].answer='x'.repeat(2048);seal(f);assert.throws(()=>validateReport(f,p),'finding includes IDs and punctuation');
  const reference=report('c1-v1'),scope=profile('c1');reference.evidenceReferences[0].location='x'.repeat(1000);
  scope.captures[0].reference=structuredClone(reference.evidenceReferences[0]);seal(reference);assert.throws(()=>validateReport(reference,scope),'reference includes all fields');
});
test('64 references are permitted; the 65th is rejected without truncation',()=>{
  const r=report('c1-v1'),p=profile('c1'),capture=structuredClone(p.captures[0]);
  for(let i=0;i<63;i++){
    const c=structuredClone(capture);c.reference.id='e'+String(i).padStart(2,'0');p.captures.push(c);r.evidenceReferences.push(c.reference);
  }
  r.evidenceReferences.sort((a,b)=>Buffer.compare(Buffer.from(a.id),Buffer.from(b.id)));
  assert.equal(validateReport(seal(r),p),r.contentDigest);
  const extra=structuredClone(capture);extra.reference.id='z-extra';p.captures.push(extra);r.evidenceReferences.push(extra.reference);seal(r);
  assert.throws(()=>validateReport(r,p));
});
test('captured excerpt hashing preserves exact decomposed Unicode bytes',()=>{
  const r=report('c1-v1'),p=profile('c1'),raw=Buffer.from('a\u030a');
  r.evidenceReferences[0].excerptDigest=hash(raw);p.captures[0].reference=structuredClone(r.evidenceReferences[0]);p.captures[0].bytesBase64=raw.toString('base64');
  assert.equal(validateReport(seal(r),p),r.contentDigest);
  p.captures[0].bytesBase64=Buffer.from(raw.toString().normalize('NFC')).toString('base64');assert.throws(()=>validateReport(r,p));
});
test('self-consistent retokening cannot invent earlier durable summary lineage',()=>{
  const p=structuredClone(packet),h=p.histories[0];
  h.context.selection.summaryRevision='1';
  for(const field of ['rendererRequest','response']){
    const value=JSON.parse(h[field].canonicalUtf8);value.selection.summaryRevision='1';const raw=canonical(value);
    h[field]={canonicalUtf8:raw,utf8Bytes:Buffer.byteLength(raw),sha256:hash(raw)};
  }
  assert.throws(()=>verifyR2(p));
});

for(const field of ['acceptedHead','evaluationHead'])test(`a self-consistent substituted ${field} cannot replace its retained source object`,()=>{
  const p=structuredClone(packet),value=JSON.parse(p[field].canonicalUtf8);
  if(field==='acceptedHead')value.resultDigest='0'.repeat(64);else value.evaluationRevision=2;
  const raw=canonical(value);p[field]={canonicalUtf8:raw,utf8Bytes:Buffer.byteLength(raw),sha256:hash(raw)};
  assert.throws(()=>verifyR2(p));
});

for(const field of ['stored-size','workspace','bare-report-hash'])test(`report EvidenceSource rejects coherent ${field} substitution`,()=>{
  const entry=packet.evidenceSources.find(s=>s.reportEntryId==='c1-v1'),source=JSON.parse(entry.canonicalUtf8),r=report('c1-v1');
  if(field==='stored-size'){const {contentDigest,...input}=r;source.sizeBytes=Buffer.byteLength(canonical(input));}
  if(field==='workspace')source.reportIdentity.workspaceId='foreign';
  const sourceKey=field==='bare-report-hash'?r.contentDigest:domain('orkworks.assignment-evidence-source.v1',source);
  assert.throws(()=>validateReportEvidenceSource(source,r,profile('c1'),sourceKey));
});
test('report EvidenceSource consumes the complete stored descriptor and its own domain, not report hash input bytes',()=>{
  for(const entry of packet.evidenceSources){
    const source=JSON.parse(entry.canonicalUtf8),r=report(entry.reportEntryId);
    assert.equal(validateReportEvidenceSource(source,r,profile(entry.reportEntryId.split('-')[0]),entry.sourceKey),entry.sourceKey);
  }
});

test('the shared eight-version stream budget cannot be reset and exact retries consume no version',()=>{
  let state=initial();let last;
  for(let version=2;version<=8;version++){
    const r=request('c1-v2');r.report.reportVersion=version;
    r.expectedRunVersion=state.runVersion;r.expectedReportHead=state.heads.find(h=>h.stream.taskId==='research-c1').head;seal(r.report);
    last=r;state=applyCaptured(state,r,profile('c1')).state;
  }
  const before=structuredClone(state);assert.equal(state.heads.find(h=>h.stream.taskId==='research-c1').head.reportVersion,8);
  assert.equal(applyCaptured(state,last,profile('c1')).kind,'retry');assert.deepEqual(state,before);
  const ninth=structuredClone(last);ninth.report.reportVersion=9;ninth.expectedRunVersion=state.runVersion;ninth.expectedReportHead=state.heads.find(h=>h.stream.taskId==='research-c1').head;seal(ninth.report);
  assert.throws(()=>applyCaptured(state,ninth,profile('c1')));assert.deepEqual(state,before);
  const reset=request('c1-v1');reset.report.reportId='new-report-id';seal(reset.report);
  assert.throws(()=>applyCaptured(state,reset,profile('c1')));assert.deepEqual(state,before);
});

for(const [stamp,valid] of [['2016-12-31T23:59:60Z',true],['2015-06-30T23:59:60.1+00:00',true],
  ['2026-01-31T23:59:60Z',false],['2016-12-30T23:59:60Z',false],['2030-12-31T23:59:60Z',false]])test(`pinned UTC leap-second occurrence ${stamp}`,()=>{
    const [r,p]=urlCase(undefined,stamp);
    if(valid)assert.equal(validateReport(r,p),r.contentDigest);else assert.throws(()=>validateReport(r,p));
});
