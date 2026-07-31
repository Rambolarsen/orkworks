import test from "node:test";
import assert from "node:assert/strict";
import { configureExternalLinks } from "../electron/externalLinks.ts";

test("opens web URLs externally and blocks Electron navigation", async () => {
  let popup!: (details: { url: string }) => { action: "deny" };
  let navigate!: (event: { preventDefault(): void }, url: string) => void;
  const opened: string[] = [];
  configureExternalLinks({
    setWindowOpenHandler(next) { popup = next as typeof popup; },
    on(event, next) { assert.equal(event, "will-navigate"); navigate = next as typeof navigate; },
  } as never, async (url) => { opened.push(url); });

  assert.deepEqual(popup({ url: "https://example.test/docs" }), { action: "deny" });
  const prevented: string[] = [];
  navigate({ preventDefault() { prevented.push("yes"); } }, "http://example.test/");
  navigate({ preventDefault() { prevented.push("yes"); } }, "file:///private/secret");
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(prevented, ["yes", "yes"]);
  assert.deepEqual(opened, ["https://example.test/docs", "http://example.test/"]);
});

test("allows same-origin Vite reloads to remain in Electron", () => {
  let navigate!: (event: { preventDefault(): void }, url: string) => void;
  const opened: string[] = [];
  configureExternalLinks({
    setWindowOpenHandler() {},
    on(event, next) { assert.equal(event, "will-navigate"); navigate = next as typeof navigate; },
  } as never, (url) => { opened.push(url); }, "http://localhost:5173/");

  const prevented: string[] = [];
  navigate({ preventDefault() { prevented.push("yes"); } }, "http://localhost:5173/");
  assert.deepEqual(prevented, []);
  assert.deepEqual(opened, []);
});

test("denies malformed popup URLs without delegating them", () => {
  let popup!: (details: { url: string }) => { action: "deny" };
  const opened: string[] = [];
  configureExternalLinks({
    setWindowOpenHandler(next) { popup = next as typeof popup; },
    on() {},
  } as never, (url) => { opened.push(url); });

  assert.deepEqual(popup({ url: "https://" }), { action: "deny" });
  assert.deepEqual(opened, []);
});

test("logs rejected external-link handoffs without an unhandled rejection", async () => {
  let popup!: (details: { url: string }) => { action: "deny" };
  const failure = new Error("shell unavailable");
  const logged: unknown[][] = [];
  const unhandled: unknown[] = [];
  const originalError = console.error;
  const onUnhandled = (reason: unknown) => { unhandled.push(reason); };
  console.error = (...args: unknown[]) => { logged.push(args); };
  process.once("unhandledRejection", onUnhandled);

  try {
    configureExternalLinks({
      setWindowOpenHandler(next) { popup = next as typeof popup; },
      on() {},
    } as never, async () => { throw failure; });

    assert.deepEqual(popup({ url: "https://example.test/docs" }), { action: "deny" });
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(logged, [["[main] couldn't open external link", failure]]);
    assert.deepEqual(unhandled, []);
  } finally {
    console.error = originalError;
    process.removeListener("unhandledRejection", onUnhandled);
  }
});
