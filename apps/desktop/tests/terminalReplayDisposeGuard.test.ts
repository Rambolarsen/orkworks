import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(
  new URL("../src/terminalStore.ts", import.meta.url),
  "utf8",
);

test("ws.onclose replay-fetch guards handle.disposed before writing replay", () => {
  const block = source.match(/getTerminalOutput\(baseUrl, id\)\.then\(\([\s\S]*?\}\)\.catch\(/)?.[0]
    ?? "";
  assert.match(
    block,
    /handle\.disposed/,
    "the post-fetch path must check handle.disposed before writing replay into the terminal",
  );
});