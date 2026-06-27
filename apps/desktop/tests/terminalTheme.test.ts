import test from "node:test";
import assert from "node:assert/strict";

import { orkworksTerminalTheme } from "../src/terminalTheme.ts";

test("defines a colored ANSI palette for xterm", () => {
  assert.equal(orkworksTerminalTheme.background, "#0c0d10");
  assert.equal(orkworksTerminalTheme.red, "#ff6b63");
  assert.equal(orkworksTerminalTheme.green, "#66e08a");
  assert.equal(orkworksTerminalTheme.yellow, "#f3d35a");
  assert.equal(orkworksTerminalTheme.brightBlue, "#66b3ff");
});

test("uses the ork-lime cursor on the graphite base", () => {
  assert.equal(orkworksTerminalTheme.cursor, "#9dc520");
  assert.equal(orkworksTerminalTheme.cursorAccent, "#0c0d10");
});
