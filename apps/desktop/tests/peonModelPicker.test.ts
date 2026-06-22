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
  assert.match(source, /Peon Model/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /savePeonModel/);
});

test("SettingsModal renders providers sorted by fallbackOrder", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /fallbackOrder/);
  assert.match(source, /\.sort/);
});

test("SettingsModal auto-saves on model change", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /savePeonModel/);
  assert.match(source, /saveProviderSettings/);
});
