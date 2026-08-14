import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { MOUSE_TRACKING_RESET_SEQUENCE } from "../src/terminalProtocol.ts";

const source = readFileSync(
  new URL("../src/terminalStore.ts", import.meta.url),
  "utf8",
);

test("MOUSE_TRACKING_RESET_SEQUENCE disables the common DEC mouse-tracking modes", () => {
  for (const mode of ["1000", "1002", "1003", "1005", "1006", "1015", "1016"]) {
    assert.match(
      MOUSE_TRACKING_RESET_SEQUENCE,
      new RegExp(`\\x1b\\[\\?${mode}l`),
      `expected reset sequence to disable DEC private mode ${mode}`,
    );
  }
});

test('"ended" resets local xterm.js mouse-tracking state before disabling stdin', () => {
  const branch = source.match(/case "ended":([\s\S]*?)break;/)?.[1] ?? "";
  assert.match(branch, /term\.write\(MOUSE_TRACKING_RESET_SEQUENCE\)/);
});

test('"error" resets local xterm.js mouse-tracking state before disabling stdin', () => {
  const branch = source.match(/case "error":([\s\S]*?)break;/)?.[1] ?? "";
  assert.match(branch, /term\.write\(MOUSE_TRACKING_RESET_SEQUENCE\)/);
});
