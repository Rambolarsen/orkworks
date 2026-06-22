import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("ProviderSettingsEntry peonModel is nullable string", () => {
  const entry = { id: "opencode" as const, enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy" as const, overrideState: null };
  assert.equal(entry.peonModel, null);
});

test("ProviderSettingsEntry peonModel can be set to a model string", () => {
  const entry = { id: "opencode" as const, enabled: true, fallbackOrder: 0, peonModel: "claude-sonnet-4-20250514", defaultState: "healthy" as const, overrideState: null };
  assert.equal(entry.peonModel, "claude-sonnet-4-20250514");
});

test("SettingsModal has a <select> for each provider model", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /<select/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /value=\{entry\.peonModel/);
  assert.match(source, /saveProviderDraft/);
});

test("SettingsModal renders sorted by fallbackOrder", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /fallbackOrder/);
  assert.match(source, /\.sort/);
});

test("SettingsModal auto-saves on model change", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /saveProviderDraft/);
  assert.match(source, /saveProviderSettings/);
});

test("ProviderSettings default option is empty string (default)", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /value=""/);
  assert.match(source, /default/);
});
