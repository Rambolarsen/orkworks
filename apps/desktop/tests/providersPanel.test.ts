import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { deriveEffectiveState, buildProviderViewModel } from "../src/providerPresentation.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";
import type { ProviderRuntimeResponse } from "../src/api.ts";

function sampleSettings(): ProviderSettings {
  return {
    version: 1,
    revision: 2,
    peonModel: null,
    providers: [
      { id: "opencode", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
      { id: "claude-code", enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
    ],
  };
}

function sampleRuntime(overrides: Partial<ProviderRuntimeResponse> = {}): ProviderRuntimeResponse {
  return {
    appliedRevision: 2,
    providers: [
      {
        id: "opencode",
        label: "OpenCode",
        enabled: true,
        fallbackOrder: 0,
        effectiveState: "capped",
        runtime: { fallbackStep: 1, lastErrorSummary: "usage limit reached", resetHint: "resets in 2h" },
      },
      {
        id: "claude-code",
        label: "Claude Code",
        enabled: true,
        fallbackOrder: 1,
        effectiveState: "healthy",
        runtime: { fallbackStep: 2, lastErrorSummary: null, resetHint: null },
      },
    ],
    ...overrides,
  };
}

test("deriveEffectiveState prefers disabled, then override, then default", () => {
  assert.equal(deriveEffectiveState({ enabled: false, defaultState: "healthy", overrideState: null }), "disabled");
  assert.equal(deriveEffectiveState({ enabled: true, defaultState: "healthy", overrideState: "capped" }), "capped");
  assert.equal(deriveEffectiveState({ enabled: true, defaultState: "degraded", overrideState: null }), "degraded");
});

test("buildProviderViewModel sorts by fallback order and marks stale applied revisions", () => {
  const model = buildProviderViewModel(sampleSettings(), sampleRuntime({ appliedRevision: 1 }));
  assert.deepEqual(model.rows.map((row) => row.id), ["opencode", "claude-code"]);
  assert.equal(model.isStale, true);
});

test("buildProviderViewModel preserves runtime checking_capacity state", () => {
  const runtime = sampleRuntime({
    providers: [
      {
        id: "opencode",
        label: "OpenCode",
        enabled: true,
        fallbackOrder: 0,
        effectiveState: "checking_capacity",
        runtime: { fallbackStep: 1, lastErrorSummary: "usage limit reached", resetHint: "resets soon" },
      },
      sampleRuntime().providers[1],
    ],
  });

  const model = buildProviderViewModel(sampleSettings(), runtime);
  assert.equal(model.rows[0].effectiveState, "checking_capacity");
});

test("SettingsModal renders a Model providers section", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Model providers/);
  assert.match(source, /providerDraft/);
  assert.match(source, /provider-model-select/);
  assert.match(source, /getProviderModels/);
});

test("SettingsModal offers a per-harness attention hook install affordance when enabled but not installed", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /getClaudeCodeHookStatus/);
  assert.match(source, /installClaudeCodeHook/);
  assert.match(source, /h\.id === "claude-code" && activeDraft\.includes\(h\.id\)/);
  assert.match(source, /Install attention hook/);
  assert.match(source, /window\.confirm/);
});

test("ProviderSettingsSection keeps model provider editing simplified", () => {
  const source = readFileSync(new URL("../src/components/ProviderSettingsSection.tsx", import.meta.url), "utf8");
  assert.match(source, /Loading model provider settings/);
  assert.match(source, /Saved model provider settings revision/);
  assert.match(source, /isAppliedRevisionStale/);
  assert.match(source, /providers-stale-banner/);
  assert.doesNotMatch(source, /Move up/);
  assert.doesNotMatch(source, /Clear override/);
  assert.doesNotMatch(source, /Last error/);
});

test("SettingsModal renders a visible candidate model list with a use action", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /Use this model/);
  assert.match(source, /ollama-candidate-list/);
  assert.match(source, /selected-model/);
});
