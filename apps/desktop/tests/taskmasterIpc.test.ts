import test from 'node:test';
import assert from 'node:assert/strict';
import { installTaskmasterIpc, normalizeRecommendationAcceptOptions } from '../electron/taskmasterIpc.ts';

function fixture(response: unknown = { protocolVersion: 3 }, afterFetch: () => void = () => {}) {
  const handlers = new Map<string, (...args: any[]) => Promise<unknown>>();
  const calls: { url: string; init?: RequestInit }[] = [];
  const abort = new AbortController();
  let workspacePath = '/repo';
  installTaskmasterIpc({
    handle: (name, handler) => handlers.set(name, handler),
    withReady: async operation => operation(1234, 'test-authority', abort.signal),
    workspacePath: () => workspacePath,
    getGeneration: () => 1,
    getReadiness: async () => 1234,
    getToken: () => 'test-authority',
    setKnowledgeUpdatesEnabled: () => {},
    getKnowledgeUpdateStatus: () => ({ version: '1.0', lastSuccessfulUpdate: null, lastError: null }),
    refreshKnowledge: () => {},
    enqueueSettingsWrite: async (operation) => operation(),
    confirmInferenceApproval: async () => true,
    fetcher: async (url, init) => {
      calls.push({ url, init });
      afterFetch();
      return response instanceof Response ? response : new Response(JSON.stringify(response));
    },
  });
  return {
    handlers,
    calls,
    abort,
    select: (path: string) => { workspacePath = path; },
    invoke: async (name: string, ...args: unknown[]) => handlers.get(name)!(undefined, ...args),
  };
}

test('consolidated Taskmaster IPC owns advice protocol v3, route and workspace fences, with no legacy channels', async () => {
  const f = fixture();
  await f.invoke('get-advice-improvement-status', '/repo');
  assert.equal(f.calls[0].url, 'http://127.0.0.1:1234/taskmaster/improvement');
  assert.equal(new Headers(f.calls[0].init?.headers).get('X-OrkWorks-Advice-Protocol'), '3');
  assert.equal(new Headers(f.calls[0].init?.headers).get('x-orkworks-open-plan-token'), 'test-authority');
  assert.equal(f.calls[0].init?.method, 'GET');
  await f.invoke('get-advice-recommendation', '/repo', 'http://127.0.0.1:1234', 'rec/a');
  assert.equal(f.calls[1].url, 'http://127.0.0.1:1234/taskmaster/recommendations/rec%2Fa');
  await f.invoke('accept-advice-recommendation', '/repo', 'rec-1', { sessionId: 'session-1' });
  assert.equal(f.calls[2].init?.method, 'GET');
  assert.equal(f.calls[3].url, 'http://127.0.0.1:1234/taskmaster/recommendations/rec-1/accept');
  assert.deepEqual(JSON.parse(f.calls[3].init?.body as string), { sessionId: 'session-1' });
  await f.invoke('accept-advice-recommendation', '/repo', 'cleanup-1', {});
  assert.deepEqual(JSON.parse(f.calls[5].init?.body as string), {});
  const dispatched = f.calls.length;
  await assert.rejects(f.invoke('get-advice-recommendations', '/repo', 'https://outside.invalid'), /Workspace changed/);
  f.select('/other');
  await assert.rejects(f.invoke('request-advice-improvement', '/repo'), /Workspace changed/);
  await assert.rejects(f.invoke('accept-advice-recommendation', '/other', 'rec-1', { sessionId: '' }), /Invalid recommendation handoff session/);
  assert.equal(f.calls.length, dispatched);
  for (const legacyChannel of [
    'analyze-taskmaster',
    'dismiss-taskmaster-recommendation',
    'accept-taskmaster-recommendation',
    'get-readiness-status',
    'get-improvement-status',
    'request-improvement',
    'cancel-improvement',
    'assess-repository-readiness',
    'assess-and-suggest',
    'get-taskmaster-run-status',
    'assess-taskmaster-workflow',
  ]) {
    assert.equal(f.handlers.has(legacyChannel), false, `legacy channel ${legacyChannel} must not be registered`);
  }
  f.abort.abort();
  assert.equal(f.calls[0].init?.signal?.aborted, true);
});

test('consolidated IPC refuses old-server output without retrying or falling back', async () => {
  const f = fixture({ available: true });
  await assert.rejects(f.invoke('get-advice-improvement-status', '/repo'), /protocol upgrade/i);
  assert.equal(f.calls.length, 1);
});

test('a workspace switch during negotiation refuses mutation before POST', async () => {
  let select = () => {};
  const f = fixture({ protocolVersion: 3 }, () => select());
  select = () => f.select('/other');
  await assert.rejects(f.invoke('request-advice-improvement', '/repo'), /Workspace changed/);
  assert.deepEqual(f.calls.map(call => call.init?.method), ['GET']);
});

test('consolidated IPC preserves fixed HTTP failure status through Electron serialization', async () => {
  const f = fixture(new Response(null, { status: 404 }));
  assert.deepEqual(
    await f.invoke('get-advice-recommendation', '/repo', 'http://127.0.0.1:1234', 'missing'),
    { protocolVersion: 3, requestError: { status: 404 } },
  );
});

test('consolidated IPC owns settings, model refresh, and inference trust handlers with generation fencing', async () => {
  const revision = { documentRevision: 'a'.repeat(64), generation: '1', digest: 'b'.repeat(64) };
  const adapter = {
    id: 'custom',
    name: 'Custom',
    state: 'approval_required',
    resolvedPath: '/tools/custom',
    revision,
    definition: { kind: 'command', command: 'custom', args: [], input: 'stdin', output: 'result-json-v1', timeoutSecs: 60 },
  };
  let confirmations = 0;
  let onFetch = () => {};
  const handlers = new Map<string, (...args: any[]) => Promise<unknown>>();
  const calls: { url: string; init?: RequestInit }[] = [];
  let generation = 1;
  let knowledgeEnabled: boolean | null = null;
  let knowledgeRefreshes = 0;
  let writesQueued = 0;
  installTaskmasterIpc({
    handle: (name, handler) => handlers.set(name, handler),
    withReady: async operation => operation(1234, 'test-authority', new AbortController().signal),
    workspacePath: () => '/repo',
    getGeneration: () => generation,
    getReadiness: async () => 1234,
    getToken: () => 'test-authority',
    setKnowledgeUpdatesEnabled: (enabled) => { knowledgeEnabled = enabled; },
    getKnowledgeUpdateStatus: () => ({ version: '1.0', lastSuccessfulUpdate: null, lastError: null }),
    refreshKnowledge: () => { knowledgeRefreshes += 1; },
    enqueueSettingsWrite: async (operation) => { writesQueued += 1; return operation(); },
    confirmInferenceApproval: async () => { confirmations += 1; return true; },
    fetcher: async (url, init) => {
      calls.push({ url, init });
      onFetch();
      if (url.endsWith('/settings/taskmaster/inference')) {
        return new Response(JSON.stringify(init?.method === 'POST' ? { ok: true } : { adapters: [adapter] }));
      }
      return new Response(JSON.stringify({ settings: { automaticKnowledgeUpdates: false }, models: ['gpt-5'] }));
    },
  });
  const invoke = async (name: string, ...args: unknown[]) => handlers.get(name)!(undefined, ...args);

  const loaded = await invoke('get-taskmaster-settings') as Record<string, unknown>;
  assert.equal(knowledgeEnabled, false);
  assert.deepEqual(loaded.knowledgeUpdate, { version: '1.0', lastSuccessfulUpdate: null, lastError: null });

  await invoke('save-taskmaster-settings', { enabled: true });
  assert.equal(writesQueued, 1);
  assert.equal(knowledgeRefreshes, 1);

  const models = await invoke('refresh-taskmaster-models', 'codex');
  assert.deepEqual(models, [{ id: 'gpt-5', displayName: 'gpt-5' }]);
  await assert.rejects(invoke('refresh-taskmaster-models', 'custom'), /unavailable for this provider/);
  await assert.rejects(invoke('refresh-taskmaster-models', 'ollama', 42), /Invalid Ollama URL/);

  const adapters = await invoke('get-inference-trust') as Array<{ id: string }>;
  assert.equal(adapters[0].id, 'custom');
  assert.equal(await invoke('approve-inference-adapter', { harnessId: 'custom', expectedRevision: revision }), true);
  assert.equal(confirmations, 1);
  assert.equal(writesQueued, 2);
  await invoke('revoke-inference-adapter', { harnessId: 'custom', expectedRevision: { ...revision, digest: null } });
  assert.equal(writesQueued, 3);
  assert.equal(JSON.parse(calls.at(-1)!.init!.body as string).action, 'revoke');

  // Generation changes during in-flight settings/models/inference requests fail closed.
  knowledgeEnabled = null;
  onFetch = () => { generation += 1; };
  await assert.rejects(invoke('get-taskmaster-settings'), /Workspace changed; reopen Recommendations settings/);
  assert.equal(knowledgeEnabled, null);
  await assert.rejects(invoke('save-taskmaster-settings', { enabled: true }), /Workspace changed; reopen Recommendations settings/);
  assert.equal(knowledgeRefreshes, 1);
  await assert.rejects(invoke('refresh-taskmaster-models', 'codex'), /Workspace changed; retry model discovery/);
  await assert.rejects(invoke('get-inference-trust'), /Sidecar changed/);
});

test('consolidated IPC validates cancel run ID and dismissal reason bounds before dispatch', async () => {
  const f = fixture();
  await assert.rejects(f.invoke('cancel-advice-improvement', '/repo', '../bad'), /Invalid improvement run ID/);
  await assert.rejects(f.invoke('dismiss-advice-recommendation', '/repo', 'rec-1', 'x'.repeat(501)), /Invalid dismissal reason/);
  assert.equal(f.calls.length, 0);
});

test('normalizeRecommendationAcceptOptions accepts cleanup payloads and validates optional fields', () => {
  assert.deepEqual(normalizeRecommendationAcceptOptions({}), {});
  assert.deepEqual(
    normalizeRecommendationAcceptOptions({
      sessionId: 's-1',
      prompt: 'Apply fix',
      packetRevision: 2,
      evidenceFingerprint: 'a'.repeat(64),
      idempotencyKey: 'key-1',
    }),
    {
      sessionId: 's-1',
      prompt: 'Apply fix',
      packetRevision: 2,
      evidenceFingerprint: 'a'.repeat(64),
      idempotencyKey: 'key-1',
    },
  );
  assert.throws(() => normalizeRecommendationAcceptOptions(null), /Invalid recommendation handoff/);
  assert.throws(() => normalizeRecommendationAcceptOptions({ sessionId: '' }), /Invalid recommendation handoff session/);
  assert.throws(() => normalizeRecommendationAcceptOptions({ packetRevision: 0 }), /Invalid completion packet revision/);
  assert.throws(() => normalizeRecommendationAcceptOptions({ evidenceFingerprint: 'bad' }), /Invalid completion packet evidence fingerprint/);
  assert.throws(() => normalizeRecommendationAcceptOptions({ idempotencyKey: 'has space' }), /Invalid completion packet idempotency key/);
});
