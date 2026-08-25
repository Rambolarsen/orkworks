import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pushProviderSettings } from "../electron/providerSettingsSync.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";
import { updateProviderModel } from "../src/providerPresentation.ts";

function baseSettings(peonModel: ProviderSettings["peonModel"]): ProviderSettings {
  return {
    version: 1,
    revision: 1,
    peonModel,
    ollamaBaseUrl: "http://127.0.0.1:11434",
    providers: [
      { id: "copilot", model: null, enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
      { id: "ollama", model: null, enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    ],
  };
}

const okResponse = () =>
  new Response(JSON.stringify({ appliedRevision: 1, appliedAt: "now", lastApplyError: null }));

test("pushProviderSettings sends peonModel:null to the sidecar", async () => {
  const bodies: Record<string, unknown>[] = [];
  const fetchImpl = async (_url: string, init?: RequestInit) => {
    bodies.push(JSON.parse(String(init?.body)));
    return okResponse();
  };
  await pushProviderSettings("http://127.0.0.1:4444", baseSettings(null), fetchImpl);
  assert.equal(bodies[0]?.peonModel, null);
});

test("pushProviderSettings sends peonModel string to the sidecar", async () => {
  const bodies: Record<string, unknown>[] = [];
  const fetchImpl = async (_url: string, init?: RequestInit) => {
    bodies.push(JSON.parse(String(init?.body)));
    return okResponse();
  };
  await pushProviderSettings("http://127.0.0.1:4444", baseSettings("deepseek-v4-pro"), fetchImpl);
  assert.equal(bodies[0]?.peonModel, "deepseek-v4-pro");
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
  assert.match(source, /ollamaBaseUrl:\s*nextBaseUrl/);
});

test("SettingsModal renders verify affordance and status region for Ollama", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Verify Ollama/);
  assert.match(source, /role="status"/);
  assert.match(source, /window\.orkworks\.verifyOllama/);
});

test("updateProviderModel trims and updates only the selected provider override", () => {
  const settings = baseSettings("global-model");
  const next = updateProviderModel(settings, "ollama", "  llama3  ");

  assert.equal(next.peonModel, "global-model");
  assert.equal(next.providers.find((entry) => entry.id === "ollama")?.model, "llama3");
  assert.equal(next.providers.find((entry) => entry.id === "copilot")?.model, null);
  assert.notEqual(next, settings);
  assert.notEqual(next.providers, settings.providers);
});

test("updateProviderModel clears a provider override to null", () => {
  const settings = updateProviderModel(baseSettings(null), "ollama", "llama3");
  const cleared = updateProviderModel(settings, "ollama", "   ");

  assert.equal(cleared.providers.find((entry) => entry.id === "ollama")?.model, null);
});

test("ProviderSettingsSection wires provider-scoped model inputs and suggestions", () => {
  const source = readFileSync(new URL("../src/components/ProviderSettingsSection.tsx", import.meta.url), "utf8");
  assert.match(source, /providerModels/);
  assert.match(source, /onProviderModelChange/);
  assert.match(source, /datalist/);
  assert.match(source, /provider-model-suggestions/);
  assert.match(source, /Use default Peon model/);
});

test("SettingsModal keeps the global field as fallback and Ollama candidates provider-scoped", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Default Peon model/);
  assert.match(source, /onProviderModelChange/);
  assert.match(source, /updateProviderModel/);
  assert.match(source, /ollama.*model/si);
  assert.match(source, /does not pin.*Copilot.*Claude/si);
});
