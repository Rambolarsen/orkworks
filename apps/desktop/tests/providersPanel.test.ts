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

test("SettingsModal mounts a per-harness command path control for command-template tools", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /import HarnessCommandPathControl from "\.\/HarnessCommandPathControl"/);
  assert.match(source, /h\.launch\.kind === "command-template"/);
  assert.match(source, /<HarnessCommandPathControl/);
});

test("SettingsModal keeps integration participation capability-derived without gating command-path controls on hook support", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /integrationKeyForHarness/);
  assert.doesNotMatch(source, /h\.id === "codex"|h\.id === "aider"|h\.id === "claude-code"/);
  assert.doesNotMatch(source, /h\.integration !== null[\s\S]{0,160}<HarnessCommandPathControl/);
});

test("HarnessDetectionStatus supports parent-triggered refresh and accessible status text", () => {
  const source = readFileSync(new URL("../src/components/HarnessDetectionStatus.tsx", import.meta.url), "utf8");
  assert.match(source, /refreshGeneration/);
  assert.match(source, /Coding tool detection status/);
  assert.match(source, /aria-live="polite"/);
});

test("combined coding-tool saves can invalidate both header detection and the per-harness integration status map", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /Object\.values\(result\.integrations\)/);
  assert.match(settingsSource, /setIntegrationStatusGeneration\(\(current\) => current \+ 1\)/);
});

test("SettingsModal wires command-path mutations back into the shared detection refresh callback", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /<HarnessCommandPathControl[\s\S]*?onChanged=\{refreshDetection\}/);
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
  assert.match(source, /getGroupedHarnessIntegrationStatus/);
  assert.match(source, /statusDescription=/);
  assert.match(source, /statusGlyph=/);
  assert.match(source, /tooltip=/);
  assert.match(source, /visualState=/);
});

test("SettingsModal uses a stable integration status effect dependency instead of the integration harness array identity", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /const integrationHarnessStatusKey = toolHarnesses[\s\S]*?integrationKeyForHarness[\s\S]*?join\("\\0"\)/);
  assert.match(source, /\[\s*integrationHarnessStatusKey,\s*integrationStatusGeneration\s*\]/);
  assert.doesNotMatch(source, /\[\s*integrationHarnesses,\s*integrationStatusGeneration\s*\]/);
});

test("SettingsModal keeps the draft toggle position while a tools save is in progress", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /checked=\{activeDraft\.includes\(h\.id\)\}/);
  assert.match(source, /disabled=\{[^}]*tools[^}]*save[^}]*inProgress[^}]*\}/i);
  assert.match(source, /inProgress:\s*[^,\n]*tools[^,\n]*save[^,\n]*inProgress/i);
});

test("SettingsModal disables mounted command-path controls while the tools batch save is active", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /<HarnessCommandPathControl[\s\S]*?disabled=\{toolsSaveInProgress\}/);
  assert.doesNotMatch(settingsSource, /<HarnessIntegrationSection/);
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

test("SettingsModal title-bar close discards subsection drafts before the modal exits", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /function discardDraftsAndClose\(\)/);
  assert.match(source, /setDraft\(clone\(savedHotkeys\)\)/);
  assert.match(source, /setProviderDraft\(clone\(savedSettingsRef\.current\.providers\)\)/);
  assert.match(source, /setActiveDraft\(normalizeActiveHarnessIds\(harnesses,\s*activeHarnessIds\)\)/);
  assert.match(source, /setIntegrationStatuses\(\{\}\)/);
  assert.match(source, /setIntegrationOperationFailures\(\{\}\)/);
  assert.match(source, /setIntegrationStatusGeneration\(\(current\) => current \+ 1\)/);
  assert.match(source, /onClick=\{discardDraftsAndClose\}/);
});

test("SettingsModal guards late tool-save and status-refresh results with local generations", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /const modalLifecycleGeneration = useRef\(0\)/);
  assert.match(source, /const toolsSaveGeneration = useRef\(0\)/);
  assert.match(source, /const integrationStatusRequestGeneration = useRef\(0\)/);
  assert.match(source, /if \(requestGeneration !== toolsSaveGeneration\.current \|\| lifecycleGeneration !== modalLifecycleGeneration\.current\) return;/);
  assert.match(source, /if \(cancelled \|\| requestGeneration !== integrationStatusRequestGeneration\.current \|\| lifecycleGeneration !== modalLifecycleGeneration\.current\)/);
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

test("grouped integration status has an explicit preload and renderer contract", () => {
  const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
  assert.match(preloadSource, /getGroupedHarnessIntegrationStatus: \(adapterId: string, targetId: string\)/);
  assert.match(preloadSource, /get-grouped-harness-integration-status/);
  assert.match(rendererTypes, /getGroupedHarnessIntegrationStatus: \(adapterId: string, targetId: string\)/);
  assert.match(rendererTypes, /export type GroupedIntegrationStatus/);
});
