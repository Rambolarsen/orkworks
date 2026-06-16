import test from "node:test";
import assert from "node:assert/strict";

import { orkworksTerminalTheme } from "../src/terminalTheme.ts";

test("defines a colored ANSI palette for xterm", () => {
  assert.equal(orkworksTerminalTheme.background, "#1e1e1e");
  assert.equal(orkworksTerminalTheme.red, "#ff5f57");
  assert.equal(orkworksTerminalTheme.green, "#5af78e");
  assert.equal(orkworksTerminalTheme.yellow, "#f3f99d");
  assert.equal(orkworksTerminalTheme.brightBlue, "#57c7ff");
});
