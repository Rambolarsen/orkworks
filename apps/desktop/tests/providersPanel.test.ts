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
  assert.match(source, /verifyPeonProvider/);
  assert.match(source, /testAndApplyPeonProvider/);
});

test("SettingsModal mounts a per-harness attention hook install affordance when enabled but not installed", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(source, /INTEGRATION_HARNESS_IDS/);
  assert.match(source, /h\.integration !== null/);
  assert.match(source, /<HarnessIntegrationSection/);
});

test("SettingsModal derives integration participation from each harness capability", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /h\.integration !== null/);
  assert.doesNotMatch(source, /h\.id === "codex"|h\.id === "aider"|h\.id === "claude-code"/);
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

test("HarnessIntegrationSection only claims 'active' for a backend-verified activation, not merely 'installed'", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  // Only Codex's fingerprint check ever resolves to activation "active";
  // other non-attention-signal harnesses (e.g. OpenCode) resolve to
  // "unknown" once installed and must keep the weaker, accurate "installed"
  // wording instead of falsely claiming a verified-active hook.
  assert.match(source, /activation === "active"[\s\S]{0,120}Session capture hook active/);
  assert.match(source, /Session capture hook installed/);
});

test("HarnessIntegrationSection suppresses install or reinstall actions when saved status is disabled", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(
    source,
    /const canInstallOrRepairIntegration =[\s\S]*?integrationStatus\.enabled[\s\S]*?registration === "absent"[\s\S]*?registration === "drifted"/,
  );
});

test("HarnessIntegrationSection keeps disabled OrkWorks-owned integrations on the uninstall cleanup path", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(
    source,
    /const canCleanupOwnedDisabledIntegration =[\s\S]*?!integrationStatus\.enabled[\s\S]*?ownership === "ork_works"[\s\S]*?registration !== "absent"/,
  );
  assert.match(source, /canCleanupOwnedDisabledIntegration \? \([\s\S]*?Disabled — remove the OrkWorks-owned/);
  assert.match(source, /Removing…/);
  assert.match(source, /Uninstall/);
});

test("HarnessDetectionStatus supports parent-triggered refresh and accessible status text", () => {
  const source = readFileSync(new URL("../src/components/HarnessDetectionStatus.tsx", import.meta.url), "utf8");
  assert.match(source, /refreshGeneration/);
  assert.match(source, /Coding tool detection status/);
  assert.match(source, /aria-live="polite"/);
});

test("combined coding-tool saves can invalidate both header detection and mounted integration rows", () => {
  const sectionSource = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(sectionSource, /refreshGeneration/);
  assert.match(sectionSource, /\[harnessId,\s*refreshGeneration\]/);

  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /Object\.keys\(result\.integrations\)/);
  assert.match(settingsSource, /<HarnessIntegrationSection[\s\S]*?refreshGeneration=\{detectionGenerations\[h\.id\] \?\? 0\}/);
});

test("HarnessIntegrationSection reports successful detection-changing mutations", () => {
  const source = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");
  assert.match(source, /onDetectionChanged/);
  assert.match(source, /onDetectionChanged\?\.\(harnessId\)/);

  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /<HarnessIntegrationSection[\s\S]*?onDetectionChanged=\{refreshDetection\}/);
});

test("SettingsModal renders verified model choices and manual override", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /peonVerification\?\.models/);
  assert.match(source, /Enter model manually/);
  assert.match(source, /Select a verified model/);
});

test("SettingsModal derives active coding tool toggle presentation from per-tool integration status", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /deriveIntegrationDisplayState/);
  assert.match(source, /getHarnessIntegrationStatus/);
  assert.match(source, /statusDescription=/);
  assert.match(source, /statusGlyph=/);
  assert.match(source, /tooltip=/);
  assert.match(source, /visualState=/);
});

test("SettingsModal uses a stable integration status effect dependency instead of the integration harness array identity", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /const integrationHarnessStatusKey = toolHarnesses[\s\S]*?filter\(\(h\) => h\.integration !== null\)[\s\S]*?map\(\(h\) => h\.id\)[\s\S]*?join\("\\0"\)/);
  assert.match(source, /\[\s*integrationHarnessStatusKey,\s*integrationStatusGeneration\s*\]/);
  assert.doesNotMatch(source, /\[\s*integrationHarnesses,\s*integrationStatusGeneration\s*\]/);
});

test("SettingsModal keeps the draft toggle position while a tools save is in progress", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /checked=\{activeDraft\.includes\(h\.id\)\}/);
  assert.match(source, /disabled=\{[^}]*tools[^}]*save[^}]*inProgress[^}]*\}/i);
  assert.match(source, /inProgress:\s*[^,\n]*tools[^,\n]*save[^,\n]*inProgress/i);
});

test("SettingsModal disables inline integration controls while the tools batch save is active", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  const integrationSource = readFileSync(new URL("../src/components/HarnessIntegrationSection.tsx", import.meta.url), "utf8");

  assert.match(settingsSource, /<HarnessIntegrationSection[\s\S]*?disabled=\{toolsSaveInProgress\}/);
  assert.match(integrationSource, /disabled\?: boolean/);
  assert.match(integrationSource, /disabled=\{disabled \|\| integrationBusy\}/);
  assert.match(integrationSource, /disabled=\{disabled \|\| customPathBusy/);
});

test("SettingsModal removes the modal-wide save footer and generic saveError path", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.doesNotMatch(source, /const \[saveError,\s*setSaveError\]/);
  assert.doesNotMatch(source, /const \[saving,\s*setSaving\]/);
  assert.doesNotMatch(source, /function save\(/);
  assert.doesNotMatch(source, /settings-save-error/);
  assert.doesNotMatch(source, /settings-modal-footer/);
  assert.match(source, /activeSection === "hotkeys"[\s\S]*Restore defaults/);
  assert.match(source, /activeSection === "hotkeys"[\s\S]*Cancel/);
  assert.match(source, /activeSection === "hotkeys"[\s\S]*Save/);
});

test("preload exposes the combined active-harness save IPC bridge", () => {
  const source = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  assert.match(
    source,
    /saveActiveHarnessesWithIntegrations: \(ids: string\[\]\): Promise<ActiveHarnessSaveResult> =>\s*ipcRenderer\.invoke\("save-active-harnesses-with-integrations", ids\)/,
  );
});

test("preload and orkworksWindow keep the combined active-harness save contract aligned", () => {
  const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  const windowSource = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");

  const preloadType = preloadSource.match(/type ActiveHarnessSaveResult = \{[\s\S]*?\n\};/);
  const windowType = windowSource.match(/export type ActiveHarnessSaveResult = \{[\s\S]*?\n\};/);

  assert.ok(preloadType, "expected ActiveHarnessSaveResult in preload.ts");
  assert.ok(windowType, "expected ActiveHarnessSaveResult in orkworksWindow.d.ts");

  const normalizeType = (value: string) =>
    value
      .replace(/^export\s+/m, "")
      .replace(/\s+/g, " ")
      .trim();

  assert.equal(normalizeType(preloadType[0]), normalizeType(windowType[0]));
  assert.match(windowSource, /saveActiveHarnessesWithIntegrations: \(ids: string\[\]\) => Promise<ActiveHarnessSaveResult>/);
});
