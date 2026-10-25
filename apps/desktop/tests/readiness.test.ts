import assert from 'node:assert/strict';
import test from 'node:test';
import { compareReadiness, defaultBaseline, parseReadinessStatus } from '../src/readiness.ts';
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
