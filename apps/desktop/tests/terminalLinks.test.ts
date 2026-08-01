import test from "node:test";
import assert from "node:assert/strict";
import { terminalLinkHandler } from "../src/terminalLinks.ts";

test("forwards an activated terminal link to Electron", async () => {
  const opened: string[] = [];
  terminalLinkHandler(async (url) => { opened.push(url); }).activate(
    {} as MouseEvent,
    "https://example.test/docs",
    {} as never,
  );

  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(opened, ["https://example.test/docs"]);
});
