import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("ProviderSettings peonModel is nullable string", () => {
  const settings = { version: 1 as const, revision: 0, peonModel: null, providers: [] };
  assert.equal(settings.peonModel, null);
});

test("ProviderSettings peonModel can be set to a model string", () => {
  const settings = { version: 1 as const, revision: 0, peonModel: "deepseek-v4-pro", providers: [] };
  assert.equal(settings.peonModel, "deepseek-v4-pro");
});

test("SettingsModal has a peon model selector", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Peon model/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /savePeonModel/);
});

test("ProviderSettingsSection renders model provider stale revision state", () => {
  const source = readFileSync(new URL("../src/components/ProviderSettingsSection.tsx", import.meta.url), "utf8");
  assert.match(source, /Loading model provider settings/);
  assert.match(source, /Saved model provider settings revision/);
  assert.match(source, /isAppliedRevisionStale/);
});

test("SettingsModal auto-saves on model change", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /savePeonModel/);
  assert.match(source, /saveProviderSettings/);
});

test("SettingsModal renders verify affordance and status region for Ollama", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Verify Ollama/);
  assert.match(source, /role="status"/);
  assert.match(source, /window\.orkworks\.verifyOllama/);
});
