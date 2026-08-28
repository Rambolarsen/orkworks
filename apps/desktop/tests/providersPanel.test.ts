import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  deriveEffectiveState,
  buildProviderViewModel,
  synchronizeProviderModelDrafts,
} from "../src/providerPresentation.ts";
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

test("synchronizeProviderModelDrafts preserves one provider draft across another provider commit", () => {
  const previousProviders = sampleSettings().providers;
  const previousDrafts = { opencode: "typed-but-unblurred", "claude-code": "" };
  const nextProviders = previousProviders.map((entry) =>
    entry.id === "claude-code" ? { ...entry, model: "claude-model" } : entry,
  );

  assert.deepEqual(
    synchronizeProviderModelDrafts(previousDrafts, previousProviders, nextProviders),
    { opencode: "typed-but-unblurred", "claude-code": "claude-model" },
  );
});

test("synchronizeProviderModelDrafts syncs committed changes and provider-list changes", () => {
  const previousProviders = sampleSettings().providers;
  const previousDrafts = { opencode: "", "claude-code": "edited" };
  const nextProviders = [
    { ...previousProviders[0]!, model: "new-default" },
    { id: "copilot", model: "copilot-model", enabled: true, fallbackOrder: 2, defaultState: "healthy" as const, overrideState: null },
  ];

  assert.deepEqual(
    synchronizeProviderModelDrafts(previousDrafts, previousProviders, nextProviders),
    { opencode: "new-default", copilot: "copilot-model" },
  );
});

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

test("SettingsModal mounts a per-harness attention hook install affordance when enabled but not installed", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /INTEGRATION_HARNESS_IDS\.includes\(h\.id\) && activeDraft\.includes\(h\.id\)/);
  assert.match(source, /<HarnessIntegrationSection/);
});

test("SettingsModal exposes Codex's hook integration through the same Settings path", () => {
  // A working backend integration is unreachable if this allowlist omits
  // the harness id — HarnessIntegrationSection never mounts for it.
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  const match = source.match(/const INTEGRATION_HARNESS_IDS = (\[[^\]]*\]);/);
  assert.ok(match, "expected to find the INTEGRATION_HARNESS_IDS declaration");
  const ids = JSON.parse(match[1].replace(/'/g, '"'));
  assert.ok(ids.includes("codex"), `expected "codex" in ${match[1]}`);
});

test("HarnessIntegrationSection offers the attention hook install affordance", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(source, /getHarnessIntegrationStatus/);
  assert.match(source, /installHarnessIntegration/);
  assert.match(source, /uninstallHarnessIntegration/);
  assert.match(source, /Install attention hook/);
  assert.match(source, /Attention hooks installed/);
  assert.match(source, /begins a tool action/);
});

test("HarnessIntegrationSection distinguishes non-attention integrations and unapproved installs", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  // Codex's SessionStart hook only reports a session ID (ADR 0034) — the
  // "attention hook"/"waits for input" copy must not be the only wording,
  // and "installed" must not read as "active" while activation is
  // needs_trust (Codex requires a one-time in-tool /hooks approval).
  assert.match(source, /isAttentionSignal/);
  assert.match(source, /Session capture hook active/);
  assert.match(source, /needs_trust/);
  assert.match(source, /approve the hook inside/);
});

test("HarnessDetectionStatus supports parent-triggered refresh and accessible status text", () => {
  const source = readFileSync(new URL("../src/components/HarnessDetectionStatus.tsx", import.meta.url), "utf8");
  assert.match(source, /refreshGeneration/);
  assert.match(source, /Coding tool detection status/);
  assert.match(source, /aria-live="polite"/);
});

test("HarnessIntegrationSection reports successful detection-changing mutations", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(source, /onDetectionChanged/);
  assert.match(source, /onDetectionChanged\?\.\(harnessId\)/);

  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /<HarnessIntegrationSection[\s\S]*?onDetectionChanged=\{refreshDetection\}/);
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
