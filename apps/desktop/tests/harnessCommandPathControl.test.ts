import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function source(path: string): string {
  return readFileSync(new URL(path, import.meta.url), "utf8");
}

test("HarnessCommandPathControl preserves absolute-path validation and immediate Save or Clear IPC calls", () => {
  const text = source("../src/components/HarnessCommandPathControl.tsx");

  assert.match(text, /function looksAbsolute/);
  assert.match(text, /setHarnessCommandOverride\(harnessId,\s*customPathDraft\.trim\(\)\)/);
  assert.match(text, /clearHarnessCommandOverride\(harnessId\)/);
  assert.match(text, /!looksAbsolute\(customPathDraft\.trim\(\)\)/);
});

test("HarnessCommandPathControl refreshes status through the parent callback and keeps path errors local", () => {
  const text = source("../src/components/HarnessCommandPathControl.tsx");

  assert.match(text, /onChanged\?\.\(harnessId\)/);
  assert.doesNotMatch(text, /getHarnessIntegrationStatus\(harnessId\)/);
  assert.match(text, /settings-config-status/);
  assert.match(text, /Couldn't set the custom path\./);
  assert.match(text, /Couldn't clear the custom path\./);
});

test("HarnessCommandPathControl disables edits during busy states and keeps Clear stateful", () => {
  const text = source("../src/components/HarnessCommandPathControl.tsx");

  assert.match(text, /disabled\?: boolean/);
  assert.match(text, /disabled=\{disabled \|\| customPathBusy\}/);
  assert.match(text, /setCustomPathActive\(true\)/);
  assert.match(text, /setCustomPathActive\(false\)/);
  assert.match(text, /setCustomPathDraft\(\"\"\)/);
});

test("custom harness paths use a revision-aware complete-definition update", () => {
  const text = source("../src/components/HarnessCommandPathControl.tsx");

  assert.match(text, /harness\?\.origin === \"custom\"/);
  assert.match(text, /saveHarnessConfiguration/);
  assert.match(text, /mode:\s*\"custom\"/);
  assert.match(text, /expectedRevision/);
  assert.match(text, /stripDerivedHarnessFields/);
  assert.match(text, /launch:\s*\{ \.\.\.launch, command:/);
  assert.match(text, /!isCustom/);
});
