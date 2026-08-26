import assert from "node:assert/strict";
import test from "node:test";

import * as backendLifecycleEvents from "../electron/backendLifecycleEvent.ts";

const { canonicalizeBackendLifecycleEvent } = backendLifecycleEvents;

test("canonicalizes valid lifecycle payloads into new trusted objects", () => {
  const input = { state: "ready", port: 65535 };
  const event = canonicalizeBackendLifecycleEvent(input);

  assert.deepEqual(event, { state: "ready", port: 65535 });
  assert.notEqual(event, input);
  assert.deepEqual(canonicalizeBackendLifecycleEvent({ state: "starting" }), { state: "starting" });
  assert.deepEqual(canonicalizeBackendLifecycleEvent({ state: "failed", message: "offline" }), {
    state: "failed",
    message: "offline",
  });
});

test("rejects extra properties and invalid ready ports", () => {
  assert.equal(canonicalizeBackendLifecycleEvent({
    state: "ready",
    port: 4444,
    token: "must-not-cross-preload",
    workspacePath: "/private/workspace",
  }), null);
  assert.equal(canonicalizeBackendLifecycleEvent({ state: "starting", processHandle: 123 }), null);
  assert.equal(canonicalizeBackendLifecycleEvent({ state: "failed", message: "offline", extra: true }), null);

  for (const port of [0, 65536, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY]) {
    assert.equal(canonicalizeBackendLifecycleEvent({ state: "ready", port }), null);
  }
});

test("snapshots lifecycle fields exactly once before forwarding them", () => {
  let messageReads = 0;
  const input = {
    state: "failed",
    get message() {
      messageReads += 1;
      return messageReads === 1 ? "offline" : 42;
    },
  };

  assert.deepEqual(canonicalizeBackendLifecycleEvent(input), {
    state: "failed",
    message: "offline",
  });
  assert.equal(messageReads, 1);
});

test("a late-subscriber snapshot reaches only that subscriber and loses to newer live state", async () => {
  const subscribeBackendLifecycle = (
    backendLifecycleEvents as typeof backendLifecycleEvents & {
      subscribeBackendLifecycle?: (
        registerLive: (listener: (data: unknown) => void) => () => void,
        loadSnapshot: () => Promise<unknown>,
        callback: (event: unknown) => void,
      ) => () => void;
    }
  ).subscribeBackendLifecycle;
  assert.equal(typeof subscribeBackendLifecycle, "function");

  const listeners = new Set<(data: unknown) => void>();
  const registerLive = (listener: (data: unknown) => void) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };
  const firstEvents: unknown[] = [];
  const secondEvents: unknown[] = [];
  const firstSnapshot = Promise.resolve({ state: "starting" });
  let resolveSecondSnapshot!: (value: unknown) => void;
  const secondSnapshot = new Promise<unknown>((resolve) => {
    resolveSecondSnapshot = resolve;
  });

  const unsubscribeFirst = subscribeBackendLifecycle!(registerLive, () => firstSnapshot, (event) => {
    firstEvents.push(event);
  });
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.deepEqual(firstEvents, [{ state: "starting" }]);

  const unsubscribeSecond = subscribeBackendLifecycle!(registerLive, () => secondSnapshot, (event) => {
    secondEvents.push(event);
  });
  for (const listener of listeners) listener({ state: "ready", port: 4321 });
  resolveSecondSnapshot({ state: "starting" });
  await new Promise<void>((resolve) => setImmediate(resolve));

  assert.deepEqual(firstEvents, [{ state: "starting" }, { state: "ready", port: 4321 }]);
  assert.deepEqual(secondEvents, [{ state: "ready", port: 4321 }]);

  unsubscribeFirst();
  unsubscribeSecond();
  assert.equal(listeners.size, 0);
});
