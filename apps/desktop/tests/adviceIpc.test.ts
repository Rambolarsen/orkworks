import test from 'node:test';
import assert from 'node:assert/strict';
import { installAdviceIpc } from '../electron/adviceIpc.ts';

function fixture(response: unknown = {protocolVersion: 3}, afterFetch: () => void = () => {}) {
  const handlers = new Map<string, (...args: any[]) => Promise<unknown>>();
  const calls: {url: string; init?: RequestInit}[] = [];
  const abort = new AbortController();
  let workspacePath = '/repo';
  installAdviceIpc({
    handle: (name, handler) => handlers.set(name, handler),
    withReady: async operation => operation(1234, 'test-authority', abort.signal),
    workspacePath: () => workspacePath,
    normalizeAccept: value => {
      if (!value || typeof value !== 'object' || typeof (value as any).sessionId !== 'string') throw Error('Invalid handoff');
      return value as Record<string, unknown>;
    },
    fetcher: async (url, init) => { calls.push({url, init}); afterFetch(); return response instanceof Response ? response : new Response(JSON.stringify(response)); },
  });
  return {handlers, calls, abort, select: (path: string) => {workspacePath=path;},
    invoke: async (name: string, ...args: unknown[]) => handlers.get(name)!(undefined, ...args)};
}

test('new advice IPC owns protocol, route and selected-workspace fences', async () => {
  const f=fixture();
  await f.invoke('get-advice-improvement-status', '/repo');
  assert.equal(f.calls[0].url,'http://127.0.0.1:1234/taskmaster/improvement');
  assert.equal(new Headers(f.calls[0].init?.headers).get('X-OrkWorks-Advice-Protocol'),'3');
  assert.equal(new Headers(f.calls[0].init?.headers).get('x-orkworks-open-plan-token'),'test-authority');
  assert.equal(f.calls[0].init?.method,'GET');
  await f.invoke('get-advice-recommendation','/repo','http://127.0.0.1:1234','rec/a');
  assert.equal(f.calls[1].url,'http://127.0.0.1:1234/taskmaster/recommendations/rec%2Fa');
  await f.invoke('accept-advice-recommendation','/repo','rec-1',{sessionId:'session-1'});
  assert.equal(f.calls[2].init?.method,'GET');
  assert.equal(f.calls[3].url,'http://127.0.0.1:1234/taskmaster/recommendations/rec-1/accept');
  assert.deepEqual(JSON.parse(f.calls[3].init?.body as string),{sessionId:'session-1'});
  const dispatched=f.calls.length;
  await assert.rejects(f.invoke('get-advice-recommendations','/repo','https://outside.invalid'),/Workspace changed/);
  f.select('/other');
  await assert.rejects(f.invoke('request-advice-improvement','/repo'),/Workspace changed/);
  await assert.rejects(f.invoke('accept-advice-recommendation','/other','rec-1',{}),/Invalid handoff/);
  assert.equal(f.calls.length,dispatched);
  assert.equal(f.handlers.has('get-improvement-status'),false,'legacy channel remains separately owned');
  f.abort.abort();
  assert.equal(f.calls[0].init?.signal?.aborted,true);
});

test('new IPC refuses old-server output without retrying or falling back', async () => {
  const f=fixture({available:true});
  await assert.rejects(f.invoke('get-advice-improvement-status','/repo'),/protocol upgrade/i);
  assert.equal(f.calls.length,1);
});

test('a workspace switch during negotiation refuses mutation before POST', async () => {
  let select = () => {};
  const f=fixture({protocolVersion:3},()=>select());
  select=()=>f.select('/other');
  await assert.rejects(f.invoke('request-advice-improvement','/repo'),/Workspace changed/);
  assert.deepEqual(f.calls.map(call=>call.init?.method),['GET']);
});

test('new IPC preserves fixed HTTP failure status through Electron serialization', async () => {
  const f=fixture(new Response(null,{status:404}));
  assert.deepEqual(await f.invoke('get-advice-recommendation','/repo','http://127.0.0.1:1234','missing'),{protocolVersion:3,requestError:{status:404}});
});
