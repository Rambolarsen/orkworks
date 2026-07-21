import test from "node:test";
import assert from "node:assert/strict";

import {
  startSessionPolling,
  type PollScheduler,
} from "../src/sessionPolling.ts";

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

test("background session polls wait for the previous refresh to settle", async () => {
  const first = deferred();
  const scheduled: Array<() => void> = [];
  let refreshes = 0;
  const scheduler: PollScheduler = {
    set(callback, delayMs) {
      assert.equal(delayMs, 2_000);
      scheduled.push(callback);
      return callback;
    },
    clear() {},
  };

  const stop = startSessionPolling(async () => {
    refreshes += 1;
    await first.promise;
  }, 2_000, scheduler);

  await flushMicrotasks();
  assert.equal(refreshes, 1);
  assert.equal(scheduled.length, 0);

  first.resolve();
  await flushMicrotasks();
  assert.equal(scheduled.length, 1);

  scheduled.shift()!();
  await flushMicrotasks();
  assert.equal(refreshes, 2);
  stop();
});

test("stopping an unresolved poll prevents it from scheduling again", async () => {
  const first = deferred();
  const scheduled: Array<() => void> = [];
  const scheduler: PollScheduler = {
    set(callback) {
      scheduled.push(callback);
      return callback;
    },
    clear() {},
  };

  const stop = startSessionPolling(() => first.promise, 2_000, scheduler);
  stop();
  first.resolve();
  await flushMicrotasks();

  assert.equal(scheduled.length, 0);
});
