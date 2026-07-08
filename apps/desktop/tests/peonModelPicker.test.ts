import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pushProviderSettings } from "../electron/providerSettingsSync.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";

function baseSettings(peonModel: ProviderSettings["peonModel"]): ProviderSettings {
  return { version: 1, revision: 1, peonModel, ollamaBaseUrl: "http://127.0.0.1:11434", providers: [] };
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
