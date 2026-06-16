import test from "node:test";
import assert from "node:assert/strict";

import { terminalLaunchInput } from "../src/terminalLaunch.ts";

test("builds terminal input for Claude Code", () => {
  assert.equal(terminalLaunchInput("claude-code"), "claude\n");
});
