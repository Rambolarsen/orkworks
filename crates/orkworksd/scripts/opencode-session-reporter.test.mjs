import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { after, test } from 'node:test';

const source = await readFile(new URL('./opencode-session-reporter.js', import.meta.url), 'utf8');
const { OrkWorksSessionReporter } = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

const originalEnv = Object.fromEntries(
  ['ORKWORKS_PORT', 'ORKWORKS_SESSION_ID', 'ORKWORKS_REPORT_TOKEN'].map((key) => [key, process.env[key]]),
);
after(() => {
  for (const [key, value] of Object.entries(originalEnv)) {
    if (value === undefined) delete process.env[key];
    else process.env[key] = value;
  }
});

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
  try {
    await run(await OrkWorksSessionReporter(), posts);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

const created = (id = 'ses_1') => ({ type: 'session.created', properties: { info: { id } } });
const signal = (type, properties = {}) => ({ type, properties: { sessionID: 'ses_1', ...properties } });
const attention = (posts) => posts.filter((post) => post.path === '/sessions/ork_1/attention').map((post) => post.body);

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
  await withReporter(async (reporter, posts) => {
    const events = [
      { type: 'session.created', properties: { info: { id: 'ses_1' } } },
      { type: 'session.status', properties: { sessionID: 'ses_1', status: { type: 'busy' } } },
      { type: 'question.asked', properties: { id: 'que_1', sessionID: 'ses_1', questions: [] } },
    ];
    for (const event of events) await reporter.event({ event });
    const reports = posts.filter((post) => post.path === '/sessions/ork_1/attention');
    assert.deepEqual(reports.map((post) => post.body.status), ['idle', 'working', 'waiting_for_input']);
    assert.deepEqual(reports.map((post) => post.body.event), events.map((event) => event.type));
    assert.ok(reports.every((post) => post.body.source === 'opencode_hook'));
    assert.ok(reports.every((post) => post.authorization === 'Bearer test-token'));
  });
});

test('permission and question resolutions restore the latest turn state', async () => {
  await withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    await reporter.event({ event: signal('session.status', { status: { type: 'busy' } }) });
    await reporter.event({ event: signal('permission.asked', { id: 'p1' }) });
    await reporter.event({ event: signal('session.idle') });
    await reporter.event({ event: signal('permission.replied', { requestID: 'p1' }) });
    await reporter.event({ event: signal('question.asked', { id: 'q1' }) });
    await reporter.event({ event: signal('question.rejected', { requestID: 'q1' }) });
    assert.deepEqual(attention(posts).map(({ status, event, message }) => ({ status, event, message })), [
      { status: 'idle', event: 'session.created', message: undefined },
      { status: 'working', event: 'session.status', message: undefined },
      { status: 'waiting_for_input', event: 'permission.asked', message: 'OpenCode is asking for a permission decision' },
      { status: 'idle', event: 'permission.replied', message: undefined },
      { status: 'waiting_for_input', event: 'question.asked', message: 'OpenCode is asking for an answer' },
      { status: 'idle', event: 'question.rejected', message: undefined },
    ]);
  });
});

test('overlapping requests keep waiting until each type is resolved', async () => {
  await withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    for (const event of [
      signal('permission.asked', { id: 'same' }),
      signal('permission.asked', { id: 'p2' }),
      signal('question.asked', { id: 'same' }),
      signal('session.status', { status: { type: 'busy' } }),
      signal('permission.replied', { requestID: 'same' }),
      signal('permission.replied', { requestID: 'p2' }),
      signal('question.replied', { requestID: 'same' }),
    ]) await reporter.event({ event });
    assert.deepEqual(attention(posts).map(({ status, message, event }) => [status, message, event]), [
      ['idle', undefined, 'session.created'],
      ['waiting_for_input', 'OpenCode is asking for a permission decision', 'permission.asked'],
      ['waiting_for_input', 'OpenCode needs an answer or permission decision', 'question.asked'],
      ['waiting_for_input', 'OpenCode is asking for an answer', 'permission.replied'],
      ['working', undefined, 'question.replied'],
    ]);
  });
});

test('malformed, foreign, unknown, and duplicate events cannot change attention', async () => {
  await withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    await reporter.event({ event: created() });
    for (const event of [
      signal('permission.asked', { id: '' }),
      signal('question.asked', { id: 1 }),
      signal('permission.asked', { id: 'foreign', sessionID: 'ses_2' }),
      signal('session.status', { status: { type: 'busy' }, sessionID: 'ses_2' }),
      signal('session.status', { status: { type: 'idle' } }),
      signal('question.replied', { requestID: 'unknown' }),
      { type: 'session.created', properties: { info: {} } },
    ]) await reporter.event({ event });
    await reporter.event({ event: signal('question.asked', { id: 'q1' }) });
    for (const event of [
      signal('question.asked', { id: 'q1' }),
      signal('permission.replied', { requestID: 'q1' }),
      signal('question.replied', { requestID: '' }),
      signal('question.replied', { requestID: 1 }),
      signal('question.replied', { requestID: 'q1', sessionID: 'ses_2' }),
    ]) await reporter.event({ event });
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input']);
    assert.equal(posts.filter(({ path }) => path === '/sessions/ork_1/harness-session').length, 1);
    await reporter.event({ event: signal('question.replied', { requestID: 'q1' }) });
    await reporter.event({ event: signal('question.replied', { requestID: 'q1' }) });
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input', 'idle']);
  });
});

test('a new captured session clears old pending requests and old sessions cannot steer it', async () => {
  await withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    await reporter.event({ event: signal('permission.asked', { id: 'p1' }) });
    await reporter.event({ event: created('ses_2') });
    await reporter.event({ event: signal('permission.replied', { requestID: 'p1' }) });
    await reporter.event({ event: signal('session.status', { sessionID: 'ses_2', status: { type: 'busy' } }) });
    assert.deepEqual(attention(posts).map(({ status }) => status), ['idle', 'waiting_for_input', 'idle', 'working']);
    assert.deepEqual(posts.filter(({ path }) => path === '/sessions/ork_1/harness-session').map(({ body }) => body.harnessSessionId), ['ses_1', 'ses_2']);
  });
});

test('without a report token, the reporter omits claimed hook attention', async () => {
  await withReporter(async (reporter, posts) => {
    delete process.env.ORKWORKS_REPORT_TOKEN;
    await reporter.event({ event: created() });
    await reporter.event({ event: signal('permission.asked', { id: 'p1' }) });
    assert.equal(attention(posts).length, 0);
  });
});

test('microsecond timestamps advance across same-millisecond events and a backward clock step', async () => {
  await withClock([0.456, 0.456, -1], () => withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    await reporter.event({ event: signal('session.status', { status: { type: 'busy' } }) });
    await reporter.event({ event: signal('question.asked', { id: 'q1' }) });
    const times = attention(posts).map(({ observedAt }) => micros(observedAt));
    assert.equal(times.length, 3);
    assert.ok(times[0] > 1_758_888_000_000_100, 'prompt report is newer than accepted input within the millisecond');
    assert.deepEqual(times.slice(1).map((value, index) => value - times[index]), [1, 1]);
  }));
});

test('a failed POST still consumes a timestamp before the next report', async () => {
  await withClock([0.456, 0.456, 0.456], () => withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
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
      await reporter.event({ event: signal('session.status', { status: { type: 'busy' } }) });
      await reporter.event({ event: signal('question.asked', { id: 'q1' }) });
    } finally {
      globalThis.fetch = originalFetch;
    }
    assert.ok(failedAt);
    assert.ok(micros(attention(posts).at(-1).observedAt) > micros(failedAt));
  }));
});

test('overlapping POST completion cannot reverse event timestamps', async () => {
  await withClock([0.456, 0.456, 0.456], () => withReporter(async (reporter, posts) => {
    await reporter.event({ event: created() });
    const originalFetch = globalThis.fetch;
    let releaseFirst;
    let firstBody;
    globalThis.fetch = (url, init) => {
      if (!firstBody && url.endsWith('/attention')) {
        firstBody = JSON.parse(init.body);
        return new Promise((resolve) => { releaseFirst = () => resolve({ ok: true }); });
      }
      return originalFetch(url, init);
    };
    try {
      const first = reporter.event({ event: signal('session.status', { status: { type: 'busy' } }) });
      const second = reporter.event({ event: signal('question.asked', { id: 'q1' }) });
      await second;
      releaseFirst();
      await first;
    } finally {
      globalThis.fetch = originalFetch;
    }
    assert.equal(firstBody.event, 'session.status');
    assert.equal(attention(posts).at(-1).event, 'question.asked');
    assert.ok(micros(attention(posts).at(-1).observedAt) > micros(firstBody.observedAt));
  }));
});
