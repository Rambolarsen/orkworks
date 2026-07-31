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
