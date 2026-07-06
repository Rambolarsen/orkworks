import test from "node:test";
import assert from "node:assert/strict";

import { HARNESS_ICON_PATHS, harnessIconKey } from "../src/harnessIcons.ts";

// Sessions may carry either the display name from the harness registry
// ("Claude Code") or a harness id ("claude-code"); both must land on the
// same icon key so the row mark doesn't depend on which field was set.
test("harnessIconKey maps display names and ids to the same key", () => {
  assert.equal(harnessIconKey("Claude Code"), "claude code");
  assert.equal(harnessIconKey("claude-code"), "claude code");
  assert.equal(harnessIconKey("Gemini CLI"), "gemini cli");
  assert.equal(harnessIconKey("gemini_cli"), "gemini cli");
  assert.equal(harnessIconKey("OpenCode"), "opencode");
});

// Both the registry's display names AND its ids must land on a real mark,
// not the fallback prompt glyph — sessions carry the id ("gemini"), while
// design callers use the display name ("Gemini CLI").
test("every builtin harness display name and id resolves to a vendored mark", () => {
  const builtins: Array<[name: string, id: string]> = [
    ["Claude Code", "claude-code"],
    ["Codex", "codex"],
    ["OpenCode", "opencode"],
    ["Aider", "aider"],
    ["Gemini CLI", "gemini"],
  ];
  for (const [name, id] of builtins) {
    assert.ok(HARNESS_ICON_PATHS[harnessIconKey(name)], `missing icon path for name ${name}`);
    assert.ok(HARNESS_ICON_PATHS[harnessIconKey(id)], `missing icon path for id ${id}`);
  }
});

test("harnessIconKey leaves unknown tools intact for the fallback path", () => {
  assert.equal(harnessIconKey("Some Future Tool"), "some future tool");
});
