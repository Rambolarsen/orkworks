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
    providers: [
      { id: "opencode", enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy", overrideState: null },
      { id: "claude-code", enabled: true, fallbackOrder: 1, peonModel: null, defaultState: "unknown", overrideState: null },
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
        peonModel: null,
        runtime: { fallbackStep: 1, lastErrorSummary: "usage limit reached", resetHint: "resets in 2h" },
      },
      {
        id: "claude-code",
        label: "Claude Code",
        enabled: true,
        fallbackOrder: 1,
        effectiveState: "healthy",
        peonModel: null,
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
  const model = buildProviderViewModel(sampleSettings(), sampleRuntime({ appliedRevision: 1 }), "claude-code");
  assert.deepEqual(model.rows.map((row) => row.id), ["opencode", "claude-code"]);
  assert.equal(model.isStale, true);
  assert.equal(model.summary.currentProviderLabel, "Claude Code");
});

test("CapacityPanel renders Providers labels and runtime details", () => {
  const source = readFileSync(new URL("../src/components/CapacityPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /Providers/);
  assert.match(source, /Default/);
  assert.match(source, /Override/);
  assert.match(source, /Effective/);
  assert.match(source, /lastErrorSummary/);
  assert.match(source, /label/);
});
