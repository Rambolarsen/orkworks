import assert from "node:assert/strict";
import test from "node:test";

import { attachTerminalAfterBackendReady } from "../src/terminalAttach.ts";

test("attaches after backend readiness resolves", async () => {
  const attached: string[] = [];
  const unavailable: string[] = [];

  await attachTerminalAfterBackendReady(
    async () => "http://127.0.0.1:4321",
    () => false,
    (baseUrl) => attached.push(baseUrl),
    () => unavailable.push("unavailable"),
  );

  assert.deepEqual(attached, ["http://127.0.0.1:4321"]);
  assert.deepEqual(unavailable, []);
});

test("transitions to unavailable when backend readiness rejects without rethrowing", async () => {
  const attached: string[] = [];
  const unavailable: string[] = [];

  await assert.doesNotReject(() => attachTerminalAfterBackendReady(
    async () => { throw new Error("private prompt payload"); },
    () => false,
    (baseUrl) => attached.push(baseUrl),
    () => unavailable.push("unavailable"),
  ));

  assert.deepEqual(attached, []);
  assert.deepEqual(unavailable, ["unavailable"]);
});

test("does not update terminal state after cancellation", async () => {
  let cancelled = false;
  let rejectBackend!: (error: Error) => void;
  const attached: string[] = [];
  const unavailable: string[] = [];

  const pending = attachTerminalAfterBackendReady(
    () => new Promise<string>((_resolve, reject) => { rejectBackend = reject; }),
    () => cancelled,
    (baseUrl) => attached.push(baseUrl),
    () => unavailable.push("unavailable"),
  );
  cancelled = true;
  rejectBackend(new Error("stale failure"));
  await pending;

  assert.deepEqual(attached, []);
  assert.deepEqual(unavailable, []);
});
