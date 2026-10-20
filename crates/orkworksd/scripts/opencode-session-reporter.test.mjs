import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
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

async function withReporter(run) {
  const posts = [];
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    posts.push({
      path: request.url,
      authorization: request.headers.authorization,
      body: JSON.parse(Buffer.concat(chunks).toString()),
    });
    response.writeHead(204).end();
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
