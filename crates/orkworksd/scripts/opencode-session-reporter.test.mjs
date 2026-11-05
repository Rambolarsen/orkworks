import assert from 'node:assert/strict';
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer } from 'node:http';
import { after, test } from 'node:test';

// OpenCode 2 plugin shape: the file default-exports a definition whose
// `setup(ctx)` consumes `ctx.event.subscribe({ signal })`. The fake context
// below backs that subscription with a push queue. Because the plugin
// processes events strictly sequentially (`await handleEvent` before the
// next pull), each push resolves exactly when the generator *pulls* the
// item — so following every real event with a sentinel the reporter ignores
// gives a deterministic barrier: the sentinel's pull proves the real event
// was fully processed (including its awaited POSTs). No timing heuristics.
const source = await readFile(new URL('./opencode-session-reporter.js', import.meta.url), 'utf8');
const reporter = (await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`)).default;

const originalEnv = Object.fromEntries(
  ['ORKWORKS_PORT', 'ORKWORKS_SESSION_ID', 'ORKWORKS_REPORT_TOKEN'].map((key) => [key, process.env[key]]),
);
after(() => {
  for (const [key, value] of Object.entries(originalEnv)) {
    if (value === undefined) delete process.env[key];
    else process.env[key] = value;
  }
});

const DONE = Symbol('done');
const SENTINEL = { type: 'unrelated.noise', data: {} };

function makeSubscription() {
  const queue = [];
  let notify = null;
  const abort = () => {
    queue.push({ event: DONE, resolve: () => {} });
    notify?.();
  };
  async function* generator() {
    while (true) {
      if (!queue.length) await new Promise((resolve) => { notify = resolve; });
      notify = null;
      const next = queue.shift();
      next.resolve();
      if (next.event === DONE) {
        terminated = true;
        return;
      }
      yield next.event;
    }
  }
  let terminated = false;
  const push = (event) => new Promise((resolve) => {
    // After termination nothing pulls the queue; resolve immediately.
    if (terminated) {
      resolve();
      return;
    }
    queue.push({ event, resolve });
    notify?.();
  });
  return {
    push,
    abort,
    get terminated() {
      return terminated;
    },
    generator,
  };
}

async function withReporter(run, identityStatuses = []) {
  const posts = [];
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    posts.push({
      path: request.url,
      authorization: request.headers.authorization,
      body: JSON.parse(Buffer.concat(chunks).toString()),
    });
    const status = request.url.endsWith('/harness-session') ? (identityStatuses.shift() ?? 204) : 204;
    response.writeHead(status).end();
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  process.env.ORKWORKS_PORT = String(server.address().port);
  process.env.ORKWORKS_SESSION_ID = 'ork_1';
  process.env.ORKWORKS_REPORT_TOKEN = 'test-token';
  const subscription = makeSubscription();
  // Deterministic event barrier: resolve a push when the item is pulled,
  // then push an ignored sentinel and wait for ITS pull to prove the real
  // event finished processing.
  const push = async (event) => {
    await subscription.push(event);
    await subscription.push(SENTINEL);
  };
  let cleanup = null;
  try {
    cleanup = await reporter.setup({
      event: {
        subscribe: ({ signal } = {}) => {
          signal?.addEventListener('abort', () => subscription.abort());
          return subscription.generator();
        },
      },
    });
    await run({ push, posts, cleanup, subscription });
  } finally {
    subscription.abort();
    cleanup?.();
    await new Promise((resolve) => server.close(resolve));
  }
}

const data = (extra = {}) => ({ sessionID: 'ses_1', ...extra });
const created = (id = 'ses_1') => ({ type: 'session.created', data: { info: { id } } });
const sessionEvent = (type, extra = {}) => ({ type, data: data(extra) });
// attention() returns the body objects; harnessPosts() returns the raw posts.
const attention = (posts) => posts.filter((post) => post.path === '/sessions/ork_1/attention').map((post) => post.body);
const harnessPosts = (posts) => posts.filter((post) => post.path === '/sessions/ork_1/harness-session');

function withClock(nowValues, run) {
  const prior = Object.getOwnPropertyDescriptor(globalThis, 'performance');
  let index = 0;
  Object.defineProperty(globalThis, 'performance', {
    configurable: true,
    value: { timeOrigin: 1_758_888_000_000, now: () => nowValues[Math.min(index++, nowValues.length - 1)] },
  });
  return Promise.resolve().then(run).finally(() => Object.defineProperty(globalThis, 'performance', prior));
}

function micros(iso) {
  assert.match(iso, /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}Z$/);
  return Date.parse(iso.replace(/(\.\d{3})\d{3}Z$/, '$1Z')) * 1000 + Number(iso.slice(-4, -1));
}

test('captured session reports initial idle, busy work, and an explicit question', async () => {
  await withReporter(async ({ push, posts }) => {
    // V2 run mode emitted no session.created: the first session-scoped
    // event establishes the capture, whose attention POST carries the
    // normalized `session.capture` name; a step start immediately reports
    // working after the capture's initial idle.
    await push(sessionEvent('session.step.started', { assistantMessageID: 'msg_1' }));
    await push(sessionEvent('question.asked', { id: 'que_1', questions: [] }));
    const reports = attention(posts);
    assert.deepEqual(reports.map((report) => report.status), ['idle', 'working', 'waiting_for_input']);
    assert.deepEqual(reports.map((report) => report.event), ['session.capture', 'session.step.started', 'question.asked']);
    assert.ok(reports.every((report) => report.source === 'opencode_hook'));
    assert.ok(posts.every((post) => post.path !== '/sessions/ork_1/attention' || post.authorization === 'Bearer test-token'));
  });
});

test('the capture-time identity POST is unauthenticated', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    const identity = harnessPosts(posts);
    assert.equal(identity.length, 1);
    assert.equal(identity[0].authorization, undefined);
    assert.equal(identity[0].body.source, 'opencode_hook');
  });
});

// OpenCode 2.2.5 Form events: creation nests the session and request ID
// under data.form; replies/cancellation carry data.id and data.sessionID.
const formCreated = (id = 'frm_1', sessionID = 'ses_1') => ({
  type: 'form.created',
  data: { form: { id, sessionID, title: 'Confirm task', fields: [
    { key: 'answer', type: 'boolean', title: 'Proceed?', required: true },
  ] } },
});

for (const resolution of ['form.replied', 'form.cancelled']) {
  test(`${resolution} resolves an explicit form and restores working`, async () => {
    await withReporter(async ({ push, posts }) => {
      await push(sessionEvent('session.step.started'));
      await push(formCreated());
      await push(sessionEvent(resolution, { id: 'frm_1', answer: { answer: true } }));
      assert.deepEqual(attention(posts).map(({ status, event }) => [status, event]), [
        ['idle', 'session.capture'], ['working', 'session.step.started'],
        ['waiting_for_input', 'form.created'], ['working', resolution],
      ]);
    });
  });
}

test('forms remain independent of overlapping questions and permissions through turn end', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    await push(formCreated('same'));
    await push(formCreated('frm_2'));
    await push(sessionEvent('question.asked', { id: 'same' }));
    await push(sessionEvent('permission.asked', { id: 'same' }));
    await push(sessionEvent('session.execution.succeeded'));
    await push(sessionEvent('form.replied', { id: 'same' }));
    await push(sessionEvent('question.replied', { requestID: 'same' }));
    await push(sessionEvent('permission.replied', { requestID: 'same' }));
    assert.equal(attention(posts).at(-1).status, 'waiting_for_input');
    await push(sessionEvent('form.cancelled', { id: 'frm_2' }));
    assert.deepEqual(attention(posts).map(({ status }) => status), [
      'idle', 'working', 'waiting_for_input', 'waiting_for_input', 'waiting_for_input', 'idle',
    ]);
  });
});

test('malformed and foreign forms cannot open or resolve a captured prompt', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    for (const event of [
      { type: 'form.created', data: {} },
      { type: 'form.created', data: { form: null } },
      formCreated('', 'ses_1'), formCreated(1, 'ses_1'), formCreated('frm_foreign', 'ses_2'),
      // A misleading outer ID must not override the form owner.
      { ...formCreated('frm_foreign', 'ses_2'), data: { ...formCreated('frm_foreign', 'ses_2').data, sessionID: 'ses_1' } },
    ]) await push(event);
    assert.equal(attention(posts).at(-1).status, 'working');
    await push(formCreated());
    for (const event of [
      sessionEvent('form.replied', { id: 'frm_1', sessionID: 'ses_2' }),
      sessionEvent('form.cancelled', { id: '' }),
      sessionEvent('form.replied', { id: 1 }),
      sessionEvent('form.replied', { id: 'unknown' }),
      sessionEvent('form.replied', { requestID: 'frm_1' }),
      sessionEvent('question.replied', { requestID: 'frm_1' }),
      formCreated(),
    ]) await push(event);
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'working', 'waiting_for_input']);
    await push(sessionEvent('form.replied', { id: 'frm_1' }));
    await push(sessionEvent('form.replied', { id: 'frm_1' }));
    assert.equal(attention(posts).at(-1).status, 'working');
    assert.equal(attention(posts).length, 4);
  });
});

test('session.created still re-captures when OpenCode emits it', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    await push(sessionEvent('session.step.started'));
    const reports = attention(posts);
    assert.deepEqual(reports.map((report) => report.status), ['idle', 'working']);
    assert.deepEqual(reports.map((report) => report.event), ['session.capture', 'session.step.started']);
  });
});

test('permission and question resolutions restore the latest turn state', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    await push(sessionEvent('session.step.started'));
    await push(sessionEvent('permission.asked', { id: 'p1' }));
    await push(sessionEvent('session.execution.succeeded'));
    await push(sessionEvent('permission.replied', { requestID: 'p1' }));
    await push(sessionEvent('question.asked', { id: 'q1' }));
    await push(sessionEvent('question.rejected', { requestID: 'q1' }));
    assert.deepEqual(attention(posts).map(({ status, event, message }) => ({ status, event, message })), [
      { status: 'idle', event: 'session.capture', message: undefined },
      { status: 'working', event: 'session.step.started', message: undefined },
      { status: 'waiting_for_input', event: 'permission.asked', message: 'OpenCode is asking for a permission decision' },
      // The turn-end event is deduped while the permission decision is
      // still pending: the effective attention stays waiting_for_input.
      { status: 'idle', event: 'permission.replied', message: undefined },
      { status: 'waiting_for_input', event: 'question.asked', message: 'OpenCode is asking for an answer' },
      { status: 'idle', event: 'question.rejected', message: undefined },
    ]);
  });
});

test('overlapping requests keep waiting until each type is resolved', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    for (const event of [
      sessionEvent('permission.asked', { id: 'same' }),
      sessionEvent('permission.asked', { id: 'p2' }),
      sessionEvent('question.asked', { id: 'same' }),
      sessionEvent('session.step.started'),
      sessionEvent('permission.replied', { requestID: 'same' }),
      sessionEvent('permission.replied', { requestID: 'p2' }),
      sessionEvent('question.replied', { requestID: 'same' }),
    ]) await push(event);
    assert.deepEqual(attention(posts).map(({ status, message, event }) => [status, message, event]), [
      ['idle', undefined, 'session.capture'],
      ['waiting_for_input', 'OpenCode is asking for a permission decision', 'permission.asked'],
      ['waiting_for_input', 'OpenCode needs an answer or permission decision', 'question.asked'],
      ['waiting_for_input', 'OpenCode is asking for an answer', 'permission.replied'],
      ['working', undefined, 'question.replied'],
    ]);
  });
});

test('malformed, foreign, unknown, and duplicate events cannot change attention', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    await push(created());
    for (const event of [
      sessionEvent('permission.asked', { id: '' }),
      sessionEvent('question.asked', { id: 1 }),
      sessionEvent('permission.asked', { id: 'foreign', sessionID: 'ses_2' }),
      sessionEvent('session.step.started', { sessionID: 'ses_2' }),
      { type: 'permission.asked' },
      { type: 'permission.asked', properties: { id: 'v1-shape' } },
      sessionEvent('question.replied', { requestID: 'unknown' }),
      { type: 'session.created', data: { info: {} } },
    ]) await push(event);
    await push(sessionEvent('question.asked', { id: 'q1' }));
    for (const event of [
      sessionEvent('question.asked', { id: 'q1' }),
      sessionEvent('permission.replied', { requestID: 'q1' }),
      sessionEvent('question.replied', { requestID: '' }),
      sessionEvent('question.replied', { requestID: 1 }),
      sessionEvent('question.replied', { requestID: 'q1', sessionID: 'ses_2' }),
    ]) await push(event);
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input']);
    assert.equal(harnessPosts(posts).length, 1);
    await push(sessionEvent('question.replied', { requestID: 'q1' }));
    await push(sessionEvent('question.replied', { requestID: 'q1' }));
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input', 'idle']);
  });
});

test('a new captured session clears old pending requests and old sessions cannot steer it', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    await push(sessionEvent('permission.asked', { id: 'p1' }));
    await push(created('ses_2'));
    await push(sessionEvent('permission.replied', { requestID: 'p1' }));
    await push(sessionEvent('session.step.started', { sessionID: 'ses_2' }));
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input', 'idle', 'working']);
    assert.deepEqual(harnessPosts(posts).map(({ body }) => body.harnessSessionId), ['ses_1', 'ses_2']);
  });
});

test('teardown events of a dead session cannot bind the capture', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.deleted', { sessionID: 'ses_stale' }));
    await push(sessionEvent('session.error', { sessionID: 'ses_stale' }));
    assert.equal(harnessPosts(posts).length, 0);
    assert.equal(attention(posts).length, 0);
    // The first lifecycle-relevant session event still captures.
    await push(sessionEvent('session.step.started', { sessionID: 'ses_1' }));
    assert.deepEqual(harnessPosts(posts).map(({ body }) => body.harnessSessionId), ['ses_1']);
  });
});

test('an interrupted turn ends the turn, but a shutdown interruption does not', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(created());
    await push(sessionEvent('session.step.started'));
    // User cancel: V2 projects a non-shutdown interruption to idle.
    await push(sessionEvent('session.execution.interrupted', { reason: 'user' }));
    assert.deepEqual(attention(posts).map(({ status, event }) => [status, event]), [
      ['idle', 'session.capture'],
      ['working', 'session.step.started'],
      ['idle', 'session.execution.interrupted'],
    ]);
    // A shutdown interruption keeps the execution claim; the resumed drain
    // continues the turn, so the reporter neither changes nor posts state.
    const before = attention(posts).length;
    await push(sessionEvent('session.step.started'));
    await push(sessionEvent('session.execution.interrupted', { reason: 'shutdown' }));
    assert.equal(attention(posts).length, before + 1); // only the step.start
    assert.equal(attention(posts).at(-1).status, 'working');
  });
});

test('without a report token, the reporter omits claimed hook attention', async () => {
  await withReporter(async ({ push, posts }) => {
    delete process.env.ORKWORKS_REPORT_TOKEN;
    await push(created());
    await push(sessionEvent('permission.asked', { id: 'p1' }));
    assert.equal(attention(posts).length, 0);
  });
});

test('microsecond timestamps advance across same-millisecond events and a backward clock step', async () => {
  await withClock([0.456, 0.456, -1], () => withReporter(async ({ push, posts }) => {
    await push(created());
    await push(sessionEvent('session.step.started'));
    await push(sessionEvent('question.asked', { id: 'q1' }));
    const times = attention(posts).map(({ observedAt }) => micros(observedAt));
    assert.equal(times.length, 3);
    assert.ok(times[0] > 1_758_888_000_000_100, 'prompt report is newer than accepted input within the millisecond');
    assert.deepEqual(times.slice(1).map((value, index) => value - times[index]), [1, 1]);
  }));
});

test('a failed POST still consumes a timestamp before the next report', async () => {
  await withClock([0.456, 0.456, 0.456], () => withReporter(async ({ push, posts }) => {
    await push(created());
    const originalFetch = globalThis.fetch;
    let failedAt;
    globalThis.fetch = async (url, init) => {
      if (!failedAt && url.endsWith('/attention')) {
        failedAt = JSON.parse(init.body).observedAt;
        throw new Error('simulated transport failure');
      }
      return originalFetch(url, init);
    };
    try {
      await push(sessionEvent('session.step.started'));
      await push(sessionEvent('question.asked', { id: 'q1' }));
    } finally {
      globalThis.fetch = originalFetch;
    }
    assert.ok(failedAt);
    assert.ok(micros(attention(posts).at(-1).observedAt) > micros(failedAt));
  }));
});

test('the setup cleanup aborts the subscription', async () => {
  await withReporter(async ({ push, posts, cleanup, subscription }) => {
    await push(created());
    const before = posts.length;
    assert.equal(typeof cleanup, 'function');
    await cleanup();
    // The generator resumes asynchronously after the abort; give it a tick.
    await new Promise((resolve) => setTimeout(resolve, 10));
    assert.ok(subscription.terminated, 'the subscription generator ended after cleanup');
    // Events pushed after cleanup must produce no further POSTs.
    await new Promise((resolve) => setTimeout(resolve, 50));
    await push(sessionEvent('permission.asked', { id: 'late' }));
    assert.equal(posts.length, before);
  });
});

// Note: the V1 suite had an "overlapping POST completion" test that called
// the handler concurrently to prove the monotonic-timestamp guard under
// interleaving. The V2 plugin processes its event subscription strictly
// sequentially (`await handleEvent` before the next pull), so that
// interleaving is impossible by construction; monotonicity across
// same-millisecond and backward-stepping clocks is covered above.


test('identity delivery retries a transient failure before publishing attention', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    assert.equal(harnessPosts(posts).length, 2);
    assert.deepEqual(posts.slice(0, 2).map(p => p.path), [
      '/sessions/ork_1/harness-session', '/sessions/ork_1/harness-session',
    ]);
    assert.equal(attention(posts).at(-1).status, 'working');
  }, [503, 204]);
});

test('failed identity stays pending and a later event can recover it', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    assert.equal(harnessPosts(posts).length, 3);
    assert.equal(attention(posts).length, 0);
    await push(sessionEvent('session.execution.succeeded'));
    assert.equal(harnessPosts(posts).length, 4);
    assert.equal(attention(posts).at(-1).status, 'idle');
  }, [503, 503, 503, 204]);
});

test('identity rejection never publishes attention or retries another native ID', async () => {
  await withReporter(async ({ push, posts }) => {
    await push(sessionEvent('session.step.started'));
    assert.equal(harnessPosts(posts).length, 1);
    assert.equal(attention(posts).length, 0);
    await push({ type: 'session.step.started', data: { sessionID: 'ses_foreign' } });
    assert.equal(harnessPosts(posts).length, 1);
  }, [400]);
});

const scopeKeys = ['ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL', 'ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID',
  'ORKWORKS_WORKTREE_INSTRUCTION_HELPER'];
const helperSource = await readFile(new URL('./session-metadata.py', import.meta.url));
async function withContext(run, overrides = {}, options = {}) {
  const directory = await mkdtemp(join(tmpdir(), 'orkworks-context-test-'));
  const helper = join(directory, 'session-metadata.py');
  await writeFile(helper, helperSource);
  const keys = ['PATH', ...scopeKeys, 'ORKWORKS_PORT', 'ORKWORKS_SESSION_ID', 'ORKWORKS_REPORT_TOKEN'];
  const old = Object.fromEntries(keys.map(key => [key, process.env[key]]));
  Object.assign(process.env, {ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL: '1',
    ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID: 'ses_root', ORKWORKS_WORKTREE_INSTRUCTION_HELPER: helper,
    ORKWORKS_PORT: '1', ORKWORKS_SESSION_ID: 'fixture-session', ORKWORKS_REPORT_TOKEN: 'fixture-only'});
  Object.assign(process.env, overrides);
  const subscription = makeSubscription();
  let hook;
  let calls = 0;
  let get = async ({sessionID}) => ({id:sessionID});
  const cleanup = await reporter.setup({event:{subscribe:({signal}) => {
    signal.addEventListener('abort', subscription.abort); return subscription.generator();
  }}, session:{hook:async (name, callback) => {if(options.rejectHook) throw new Error('hook unavailable'); if(name === 'context') hook = callback;},
    get: args => {calls++; return get(args);}}});
  const event = (sessionID = 'ses_root') => ({sessionID, system:[], tools:{keep:true}, options:{keep:true}});
  try {
    await run({event, context: e => hook?.(e), helper, cleanup,
      get calls(){return calls;}, setGet: value => {get=value;}, push: async e => {
        await subscription.push(e); await subscription.push(SENTINEL);
      }});
  } finally {
    cleanup(); subscription.abort();
    for(const [key,value] of Object.entries(old)) {
      if(value === undefined) delete process.env[key]; else process.env[key]=value;
    }
    await rm(directory,{recursive:true,force:true});
  }
}

test('launch-selected root gets actual narrow helper text without metadata side effects', async () => {
  await withContext(async ({event, context}) => {
    const e=event(); await context(e);
    assert.equal(e.system.length,1);
    assert.match(e.system[0].text,/worktree/);
    assert.doesNotMatch(e.system[0].text,/session-metadata\.py (?:bootstrap|get|patch)/);
    assert.deepEqual(e.tools,{keep:true}); assert.deepEqual(e.options,{keep:true});
  });
});

test('child and peer context never trigger native lookups or instructions', async () => {
  await withContext(async x => {
    for(const id of ['ses_child','ses_peer']) {const e=x.event(id); await x.context(e); assert.deepEqual(e.system,[]);}
    assert.equal(x.calls,0);
  });
});

test('read-back mismatch, parent, and malformed response refuse root instructions', async () => {
  await withContext(async x => {
    for(const info of [null,{}, {id:'ses_other'}, {id:'ses_root',parentID:'ses_parent'}, {id:'ses_root',parentID:42}]) {
      x.setGet(async () => info); const e=x.event(); await x.context(e); assert.deepEqual(e.system,[]);
    }
  });
});

test('helper replaced after setup refuses delivery', async () => {
  await withContext(async x => {
    await writeFile(x.helper,'raise RuntimeError("must not execute foreign helper")');
    const e=x.event(); await x.context(e); assert.deepEqual(e.system,[]);
  });
});

test('cleanup prevents an outstanding native lookup from injecting later', async () => {
  await withContext(async x => {
    let resolve; x.setGet(() => new Promise(r => {resolve=r;}));
    const e=x.event(); const pending=x.context(e); await Promise.resolve();
    assert.equal(x.calls,1); x.cleanup(); resolve({id:'ses_root'}); await pending;
    assert.deepEqual(e.system,[]);
  });
});

test('unresolved native lookup is bounded and suppresses overlapping lookups', async () => {
  await withContext(async x => {
    let resolve; x.setGet(() => new Promise(r => {resolve=r;}));
    const e=x.event(); const pending=x.context(e); await Promise.resolve();
    const second=x.event(); await x.context(second); assert.equal(x.calls,1);
    await pending; assert.deepEqual(e.system,[]);
    await x.context(x.event()); assert.equal(x.calls,1);
    resolve({id:'ses_root'}); await Promise.resolve();
  });
});

test('missing or unsupported instruction environment yields no context', async () => {
  for(const overrides of [
    {ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL:''}, {ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL:'2'},
    {ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID:'bad\nID'}, {ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID:'ses_root\n'}, {ORKWORKS_WORKTREE_INSTRUCTION_HELPER:'relative.py'},
    {ORKWORKS_REPORT_TOKEN:''},
  ]) await withContext(async x => {const e=x.event(overrides.ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID || 'ses_root'); await x.context(e); assert.deepEqual(e.system,[]); assert.equal(x.calls,0);},overrides);
});


test('legacy capture ordering cannot change the selected context recipient', async () => {
  const previous = globalThis.fetch;
  const posts=[];
  globalThis.fetch=async (url, options) => {posts.push({url,options}); return {ok:true,status:204};};
  try {
    await withContext(async x => {
      await x.push({type:'session.step.started',data:{sessionID:'ses_child'}});
      await x.push(created('ses_peer'));
      const before=posts.length;
      const root=x.event(); await x.context(root); assert.equal(root.system.length,1);
      for(const id of ['ses_child','ses_peer']) {const e=x.event(id); await x.context(e); assert.deepEqual(e.system,[]);}
      assert.equal(posts.length,before);
    });
  } finally {globalThis.fetch=previous;}
});

test('native lookup rejection cannot reject model context dispatch', async () => {
  await withContext(async x => {
    x.setGet(async () => {throw new Error('unavailable');});
    const e=x.event(); await x.context(e); assert.deepEqual(e.system,[]);
  });
});

test('helper process malformed output, large output and timeout emit no context', async () => {
  const directory=await mkdtemp(join(tmpdir(),'orkworks-python-fixture-'));
  try {
    for(const script of [
      '#!/bin/sh\nprintf \'{"additionalContext":"wrong format"}\'\n',
      '#!/bin/sh\n/usr/bin/yes x\n',
      '#!/bin/sh\n/bin/sleep 3\n',
    ]) {
      await writeFile(join(directory,'python3'),script,{mode:0o700});
      await withContext(async x => {const e=x.event(); await x.context(e); assert.deepEqual(e.system,[]);}, {PATH:directory});
    }
  } finally {await rm(directory,{recursive:true,force:true});}
});

test('context hook registration failure preserves reporter setup and cleanup', async () => {
  await withContext(async x => {
    const e=x.event(); await x.context(e); assert.deepEqual(e.system,[]); x.cleanup();
  },{}, {rejectHook:true});
});
