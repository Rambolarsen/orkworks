import test from 'node:test';
import assert from 'node:assert/strict';
import * as api from '../src/api.ts';
import { unverifiedRecommendation } from './fixtures/unverifiedAdvice.ts';
import { adviceIsSupported } from '../src/advice.ts';

test('history prose cannot lose its producer origin and become legacy display data', () => {
  assert.equal(adviceIsSupported({id:'snapshot',data:{inputManifest:{purpose:'brain-improvement-v3'}},redacted:false}),false);
  assert.equal(adviceIsSupported({id:'snapshot',data:{inputManifest:{purpose:'brain-improvement-v4'}},redacted:false}),false);
});

test('new advice API uses the fenced bridge and rejects unlabelled or unsupported responses', async t => {
  const originalWindow=globalThis.window, originalFetch=globalThis.fetch;
  const calls: unknown[][]=[];
  const item=unverifiedRecommendation();
  let response: any={protocolVersion:3,recommendations:[item],diagnostics:[]};
  (globalThis as any).window={orkworks:{getAdviceRecommendations:async (...args: unknown[])=>{calls.push(args);return response;}}};
  globalThis.fetch=async()=>{throw Error('Renderer HTTP fallback must not run');};
  t.after(()=>{globalThis.window=originalWindow;globalThis.fetch=originalFetch;});
  const get=(api as any).getAdviceRecommendations;
  assert.equal(typeof get,'function','new client entry is missing');
  assert.equal((await get('http://127.0.0.1:1234','/repo')).recommendations[0].summary,'Tests passed.');
  assert.deepEqual(calls[0],['/repo','http://127.0.0.1:1234']);
  response=structuredClone(response);response.recommendations[0].workspaceId='/other';
  await assert.rejects(get('http://127.0.0.1:1234','/repo'),/Workspace changed/);
  response={protocolVersion:3,recommendations:[unverifiedRecommendation()],diagnostics:[]};
  response=structuredClone(response);delete response.recommendations[0].verificationStatus;
  await assert.rejects(get('http://127.0.0.1:1234','/repo'),/Unsupported|unverified|protocol/i);
  response={recommendations:[],diagnostics:[]};
  await assert.rejects(get('http://127.0.0.1:1234','/repo'),/protocol upgrade/i);
  response={protocolVersion:3,requestError:{status:404}};
  await assert.rejects(get('http://127.0.0.1:1234','/repo'),error=>(error as {status?:number}).status===404);
  delete (window.orkworks as any).getAdviceRecommendations;
  await assert.rejects(get('http://127.0.0.1:1234','/repo'),/protocol upgrade/i);
});
