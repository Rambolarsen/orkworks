import test from "node:test";
import assert from "node:assert/strict";

import { pushProviderSettings } from "../electron/providerSettingsSync.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";

function sampleProviderSettings(revision: number): ProviderSettings {
  return {
    version: 1,
    revision,
    peonModel: null,
    ollamaBaseUrl: "http://127.0.0.1:11434",
    providers: [
      { id: "opencode", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
      { id: "claude-code", enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    ],
  };
}

test("pushProviderSettings posts saved settings to the sidecar and records success", async () => {
  const calls: Array<{ url: string; body: unknown }> = [];
  const fetchImpl = async (url: string, init?: RequestInit) => {
    calls.push({ url, body: JSON.parse(String(init?.body)) });
    return new Response(JSON.stringify({ appliedRevision: 3, appliedAt: "2026-06-21T10:00:00Z", lastApplyError: null }), { status: 200 });
  };

  const result = await pushProviderSettings("http://127.0.0.1:4444", sampleProviderSettings(3), fetchImpl);

  assert.equal(result.appliedRevision, 3);
  assert.equal(result.lastApplyError, null);
  assert.deepEqual(calls.map((call) => call.url), ["http://127.0.0.1:4444/settings/providers"]);
});

test("pushProviderSettings keeps the last error on non-fatal sidecar failures", async () => {
  const fetchImpl = async () => new Response("boom", { status: 500 });

  const result = await pushProviderSettings("http://127.0.0.1:4444", sampleProviderSettings(9), fetchImpl);

  assert.equal(result.appliedRevision, null);
  assert.match(result.lastApplyError ?? "", /500/);
});
