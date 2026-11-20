import test from "node:test";
import assert from "node:assert/strict";

import { orkworksTerminalOptions, orkworksTerminalTheme } from "../src/terminalTheme.ts";

test("defines a soft, muted ANSI palette for xterm", () => {
  assert.equal(orkworksTerminalTheme.background, "#111319");
  assert.equal(orkworksTerminalTheme.foreground, "#c9ced6");
  assert.equal(orkworksTerminalTheme.red, "#e8837c");
  assert.equal(orkworksTerminalTheme.green, "#8fcf9c");
  assert.equal(orkworksTerminalTheme.yellow, "#e6cc82");
  assert.equal(orkworksTerminalTheme.blue, "#82aaf0");
});

test("keeps the ork-lime cursor on the lifted graphite base", () => {
  assert.equal(orkworksTerminalTheme.cursor, "#9dc520");
  assert.equal(orkworksTerminalTheme.cursorAccent, "#111319");
});

test("uses a modern mono stack with a thin bar cursor and airy lines", () => {
  assert.match(orkworksTerminalOptions.fontFamily, /^'SF Mono'/);
  assert.equal(orkworksTerminalOptions.fontSize, 13);
  assert.equal(orkworksTerminalOptions.lineHeight, 1.4);
  assert.equal(orkworksTerminalOptions.cursorStyle, "bar");
  assert.equal(orkworksTerminalOptions.theme, orkworksTerminalTheme);
});
