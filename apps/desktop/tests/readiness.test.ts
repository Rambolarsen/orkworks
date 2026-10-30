import assert from 'node:assert/strict';
import test from 'node:test';
import { compareReadiness, conceptIds, defaultBaseline, parseReadinessStatus } from '../src/readiness.ts';
import { taskmasterRequest } from '../electron/taskmasterSettings.ts';
const snapshot=(id:string,score:number,comparison='same',coverage=14)=>({id,startedAt:'2026-10-25T00:00:00.000Z',completedAt:'2026-10-25T00:00:00.000Z',redacted:false,data:{comparisonFingerprint:comparison,coverage,overallScore:score,concepts:[{conceptId:'verification',maturity:score/25,confidence:'medium'}]}} as any);
test('readiness deltas use displayed complete totals and comparable conditions',()=>{
 assert.deepEqual(compareReadiness(snapshot('after',75),snapshot('before',50)),{delta:25,reason:null});
 assert.equal(compareReadiness(snapshot('after',75,'new'),snapshot('before',50)).delta,null);
 assert.equal(compareReadiness(snapshot('after',75,'same',13),snapshot('before',50)).delta,null);
 assert.equal(defaultBaseline([snapshot('now',75),snapshot('other',50,'other'),snapshot('prior',50)])?.id,'prior');
});
test('readiness transport uses fixed authorized routes and rejects renderer request bodies',async()=>{
 const calls:any[]=[];const fetcher=async(url:string,init?:RequestInit)=>{calls.push({url,init});return new Response(JSON.stringify({outcome:'scheduled',runId:'one',reasonCode:null}),{status:200});};
 await taskmasterRequest(1234,'token','assess-readiness',undefined,fetcher);
 assert.equal(calls[0].url,'http://127.0.0.1:1234/taskmaster/assess-readiness');assert.equal(calls[0].init.method,'POST');assert.equal(calls[0].init.headers['x-orkworks-open-plan-token'],'token');
 await assert.rejects(taskmasterRequest(1234,'token','assess-readiness',{},fetcher));
});
test('readiness status rejects invalid history bounds and malformed source payloads',()=>{
 assert.throws(()=>parseReadinessStatus({workspacePath:'/repo',run:null,snapshots:Array(101).fill({}),freshness:'current',retentionLimit:100,reasonCode:null}));
 assert.throws(()=>parseReadinessStatus({workspacePath:'/repo',run:null,snapshots:[{id:'one',redacted:false,data:{overallScore:101}}],freshness:'current',retentionLimit:100,reasonCode:null}));
 assert.deepEqual(parseReadinessStatus({workspacePath:'/repo',run:null,snapshots:[],freshness:'none',retentionLimit:100,reasonCode:null}).snapshots,[]);
});

// Hand-checked complete result: fourteen maturity-3 rows total 42/56 = 75/100.
function validStatus() {
 const hash = 'a'.repeat(64);
 return {
  workspacePath: '/repo', run: null, freshness: 'current', retentionLimit: 100, reasonCode: null,
  snapshots: [{
   id: 'complete', startedAt: '2026-10-25T00:00:00Z', completedAt: '2026-10-25T00:01:00Z', redacted: false,
   data: {
    workspacePath: '/repo', inputFingerprint: hash, comparisonFingerprint: hash, rubricDigest: hash,
    git: { headOid: 'abc123', dirty: true }, inferenceIdentity: {}, knowledgeIdentity: {},
    summary: 'Observed repository readiness', coverage: 14, rawTotal: 42, overallScore: 75,
    concepts: conceptIds.map(conceptId => ({
     conceptId, maturity: 3 as number | null, confidence: 'medium' as string | null,
     repositoryFactHashes: [hash], rubricPageId: 'rubric', rationale: 'Supported by saved evidence',
     limits: [], nextTransition: 'Retain verification results', unassessedReason: null as string | null,
    })),
    repositoryEvidence: [{ factId: hash, path: 'README.md', sha256: hash, excerpt: 'Saved evidence', observedAt: '2026-10-25T00:00:00Z' }],
    knowledgeEvidence: [{ pageId: 'rubric', title: 'Readiness', status: 'stable', bundleVersion: 'brain-3', sha256: hash, excerpt: 'Rubric guidance' }],
   },
  }],
 };
}

test('readiness accepts complete scores and partial coverage without inventing an overall score', () => {
 const complete = validStatus();
 assert.equal(parseReadinessStatus(complete).snapshots[0].data?.overallScore, 75);
 const partial = validStatus();
 const data: any = partial.snapshots[0].data;
 Object.assign(data.concepts[0], { maturity: null, confidence: null, repositoryFactHashes: [], unassessedReason: 'No permitted evidence' });
 Object.assign(data, { coverage: 13, rawTotal: null, overallScore: null });
 assert.equal(parseReadinessStatus(partial).snapshots[0].data?.overallScore, null);
 data.overallScore = 75;
 assert.throws(() => parseReadinessStatus(partial), /Invalid readiness totals/);
});

test('readiness accepts redacted audit records only when source data is absent', () => {
 const value: any = validStatus();
 value.snapshots[0].redacted = true;
 assert.throws(() => parseReadinessStatus(value), /Redacted readiness contains source data/);
 value.snapshots[0].data = null;
 assert.equal(parseReadinessStatus(value).snapshots[0].data, null);
});

const malformedCases: [string, (value: any) => void, RegExp][] = [
 ['duplicate snapshot IDs', v => v.snapshots.push(structuredClone(v.snapshots[0])), /Invalid readiness snapshot/],
 ['missing concept', v => v.snapshots[0].data.concepts.pop(), /Invalid readiness details/],
 ['wrong concept order', v => v.snapshots[0].data.concepts.reverse(), /Invalid readiness row/],
 ['invalid fact reference', v => v.snapshots[0].data.concepts[0].repositoryFactHashes = ['bad'], /Invalid readiness row/],
 ['rating without evidence', v => v.snapshots[0].data.concepts[0].repositoryFactHashes = [], /Invalid readiness rating/],
 ['fractional maturity', v => v.snapshots[0].data.concepts[0].maturity = 2.5, /Invalid readiness rating/],
 ['unassessed without reason', v => Object.assign(v.snapshots[0].data.concepts[0], { maturity: null, confidence: null }), /Invalid unassessed row/],
 ['incorrect raw total', v => v.snapshots[0].data.rawTotal = 41, /Invalid readiness totals/],
 ['incorrect displayed score', v => v.snapshots[0].data.overallScore = 74, /Invalid readiness totals/],
 ['invalid repository evidence', v => v.snapshots[0].data.repositoryEvidence[0].sha256 = 'bad', /Invalid readiness evidence/],
 ['invalid knowledge evidence', v => v.snapshots[0].data.knowledgeEvidence[0].excerpt = 'x'.repeat(4001), /Invalid readiness knowledge/],
 ['invalid Git provenance', v => v.snapshots[0].data.git.dirty = 'unknown', /Invalid readiness provenance/],
 ['invalid run state', v => v.run = { id: 'run', state: 'unknown', phase: null }, /Invalid readiness progress/],
 ['invalid run phase', v => v.run = { id: 'run', state: 'running', phase: 'unknown' }, /Invalid readiness progress/],
];
for (const [name, mutate, error] of malformedCases) {
 test(`readiness rejects ${name}`, () => {
  const value = validStatus();
  mutate(value);
  assert.throws(() => parseReadinessStatus(value), error);
 });
}

test('combined assessment transport is an authorized empty-body POST',async()=>{
 const calls:{url:string;init?:RequestInit}[]=[];
 const fetcher=async(url:string,init?:RequestInit)=>{calls.push({url,init});return new Response('{}');};
 await taskmasterRequest(1234,'token','assess-and-suggest' as any,undefined,fetcher);
 assert.equal(calls[0].url,'http://127.0.0.1:1234/taskmaster/assess-and-suggest');
 assert.equal(calls[0].init?.method,'POST');
 await assert.rejects(taskmasterRequest(1234,'token','assess-and-suggest' as any,{},fetcher));
});


test('readiness admission metadata is validated independently at renderer and privileged transport', async () => {
 const valid: any = validStatus();
 valid.snapshots[0].data.versions = {collector:4,prompt:'taskmaster-readiness-v3',discoverySchema:2,finalSchema:1,scoring:1};
 valid.snapshots[0].data.inputManifest = {collector:{collectorVersion:4,purpose:'readiness',preferredFileIds:[0,1],requestedFileIds:[1],selectionOmissions:[{id:0,reason:'evidence_limit'}]}};
 assert.equal(parseReadinessStatus(valid).snapshots.length,1);
 await taskmasterRequest(1234,'token','readiness',undefined,async()=>new Response(JSON.stringify(valid)));
 for (const mutate of [
  (c:any)=>c.selectionOmissions[0].reason='unknown',
  (c:any)=>c.requestedFileIds=[0,1],
  (c:any)=>c.preferredFileIds=[0,0],
  (c:any)=>c.selectionOmissions[0].id=2,
 ]) {
  const malformed = structuredClone(valid); mutate(malformed.snapshots[0].data.inputManifest.collector);
  assert.throws(()=>parseReadinessStatus(malformed), /readiness.*admission/i);
  await assert.rejects(taskmasterRequest(1234,'token','readiness',undefined,async()=>new Response(JSON.stringify(malformed))), /readiness.*admission/i);
 }
});
