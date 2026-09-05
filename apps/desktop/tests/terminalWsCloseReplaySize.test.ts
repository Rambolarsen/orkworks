import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(
  new URL("../src/terminalStore.ts", import.meta.url),
  "utf8",
);

test("ws.onclose in-place replay sizes the terminal to the recorded grid", () => {
  const block = source.match(/getTerminalOutput\(baseUrl, id\)\.then\(\([\s\S]*?\}\)\.catch\(/)?.[0]
    ?? "";
  assert.match(
    block,
    /recordedReplaySize\(payload\)/,
    "the replay path must consult the payload's recorded cols/rows",
  );
  assert.match(
    block,
    /resizeObserver\.disconnect\(\)/,
    "live fit-to-container resizing must stop before the recorded grid is applied",
  );
  assert.match(
    block,
    /term\.resize\(size\.cols, size\.rows\)/,
    "the terminal must be reshaped to the recorded grid before replay is written",
  );
});

test("ws.onclose in-place replay writes regardless of recorded-size presence", () => {
  const block = source.match(/getTerminalOutput\(baseUrl, id\)\.then\(\([\s\S]*?\}\)\.catch\(/)?.[0]
    ?? "";
  assert.match(
    block,
    /writeTerminalReplay\(term, payload\.lines\)/,
    "replay must still be written when the sidecar omitted cols/rows (legacy fallback)",
  );
});

test("ws.onclose in-place replay resizes to the recorded grid before writing", () => {
  const block = source.match(/getTerminalOutput\(baseUrl, id\)\.then\(\([\s\S]*?\}\)\.catch\(/)?.[0]
    ?? "";
  const resizeAt = block.indexOf("term.resize(size.cols, size.rows)");
  const writeAt = block.indexOf("writeTerminalReplay(term, payload.lines)");
  assert.ok(
    resizeAt !== -1 && writeAt !== -1 && resizeAt < writeAt,
    "replay text written before the recorded grid is applied would wrap at the old width and then reflow",
  );
});
