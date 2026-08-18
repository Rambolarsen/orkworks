import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(
  new URL("../src/terminalStore.ts", import.meta.url),
  "utf8",
);

test("terminal-unavailable does not mark the terminal ended", () => {
  const branch = source.match(
    /case "terminal-unavailable":([\s\S]*?)break;/,
  )?.[1] ?? "";
  assert.doesNotMatch(branch, /handle\.ended = true/);
});

test("terminal-unavailable marks the handle unavailable, so later effects don't re-enable a dead terminal's stdin", () => {
  const branch = source.match(
    /case "terminal-unavailable":([\s\S]*?)break;/,
  )?.[1] ?? "";
  assert.match(branch, /handle\.unavailable = true/);
});

test("socket close without typed terminal end does not mark ended", () => {
  const onclose = source.match(
    /ws\.onclose = \(\) => \{([\s\S]*?)\n  \};/,
  )?.[1] ?? "";
  assert.doesNotMatch(onclose, /handle\.ended = true/);
});

test("socket close marks the handle unavailable, so reattaching after an unexpected drop doesn't re-enable stdin on a dead transport", () => {
  const onclose = source.match(
    /ws\.onclose = \(\) => \{([\s\S]*?)\n  \};/,
  )?.[1] ?? "";
  assert.match(onclose, /handle\.unavailable = true/);
});
