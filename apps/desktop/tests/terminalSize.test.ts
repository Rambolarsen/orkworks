import test from "node:test";
import assert from "node:assert/strict";

import { terminalPtySize } from "../src/terminalSize.ts";

test("keeps PTY width large enough for Claude Code TUI", () => {
  assert.deepEqual(terminalPtySize({ rows: 24, cols: 31 }), { rows: 24, cols: 80 });
});

test("keeps PTY height large enough for Claude Code TUI", () => {
  assert.deepEqual(terminalPtySize({ rows: 8, cols: 100 }), { rows: 24, cols: 100 });
});

test("preserves terminal sizes above the minimum", () => {
  assert.deepEqual(terminalPtySize({ rows: 30, cols: 120 }), { rows: 30, cols: 120 });
});
