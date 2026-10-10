import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

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
  assert.match(source, /import HarnessCommandPathControl, \{ looksAbsolute \} from "\.\/HarnessCommandPathControl"/);
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
  assert.match(source, /tooltip=\{display\.tooltip\}/);
  assert.match(source, /visualState=\{display\.appearance\}/);
  assert.match(source, /<ToggleStatusText[^>]*description=\{display\.description\}[^>]*glyph=\{display\.glyph\}/);
});

test("SettingsModal uses a stable integration status effect dependency instead of the integration harness array identity", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /const integrationHarnessStatusKey = toolHarnesses[\s\S]*?integrationKeyForHarness[\s\S]*?join\("\\0"\)/);
  assert.match(source, /\[\s*integrationHarnessStatusKey,\s*integrationStatusGeneration\s*\]/);
  assert.doesNotMatch(source, /\[\s*integrationHarnesses,\s*integrationStatusGeneration\s*\]/);
});

test("SettingsModal keeps each tool's subsection (status, actions, custom-path control) collapsed behind a per-row disclosure by default", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /useState<Record<string,\s*boolean>>\(\{\}\)/);
  assert.match(settingsSource, /<div className="settings-config-item-subsection" hidden=\{!expanded\}>[\s\S]{0,1200}isCommandTemplate && \([\s\S]{0,200}<HarnessCommandPathControl/);
});

test("SettingsModal gates enabling a coding tool on live detection", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(settingsSource, /function handleToolToggle\(h: HarnessConfig\)[\s\S]*?if \(turningOn && !isHarnessDetected\(h\.id\)\) return;/);
  assert.match(settingsSource, /disabled=\{rowBusy\(h\.id\) \|\| \(!activeDraft\.includes\(h\.id\) && !isHarnessDetected\(h\.id\)\)\}/);
});

test("NewSessionDialog only offers detected coding tools and rechecks before starting", () => {
  const source = readFileSync(new URL("../src/components/NewSessionDialog.tsx", import.meta.url), "utf8");

  assert.match(source, /getHarnessDetectionStatus/);
  assert.match(source, /detectedSelectableHarnesses/);
  assert.match(source, /const freshStatus = await getHarnessDetectionStatus\(selectedHarness\);/);
  assert.match(source, /if \(freshStatus\.ok !== true \|\| !freshStatus\.status\.toolDetected\) return;/);
  assert.match(source, /const harnessesKey = useMemo/);
  assert.match(source, /\}, \[harnessesKey\]\);/);
  assert.match(source, /savedDraft && harnesses\.some\(\(harness\) => harness\.id === savedDraft\.harnessId/);
});

test("NewSessionDialog cancels an in-flight confirmation before it can start a session", () => {
  const source = readFileSync(new URL("../src/components/NewSessionDialog.tsx", import.meta.url), "utf8");

  assert.match(source, /const confirmationGeneration = useRef\(0\);/);
  assert.match(source, /const handleCancel = useCallback\(\(\) => \{\s*confirmationGeneration\.current \+= 1;\s*onCancel\(\);\s*\}, \[onCancel\]\);/);
  assert.match(source, /const generation = confirmationGeneration\.current;/);
  assert.match(source, /if \(generation !== confirmationGeneration\.current\) return;/);
  assert.match(source, /if \(e\.key === "Escape"\)[\s\S]{0,100}handleCancel\(\);/);
  assert.match(source, /finally \{\s*if \(generation === confirmationGeneration\.current\) setConfirmBusy\(false\);\s*\}/);
  assert.match(source, /onClick=\{handleCancel\}/);
});

test("coding-tool availability uses the selected harness executable, not a shared integration representative", () => {
  const source = readFileSync(new URL("../src/harnessDetection.ts", import.meta.url), "utf8");

  assert.match(source, /return window\.orkworks\.getHarnessIntegrationStatus\(harnessId\);/);
  assert.doesNotMatch(source, /getGroupedHarnessIntegrationStatus/);
});

test("SettingsModal invalidates detection after saving a harness definition", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");

  assert.match(source, /invalidateDetection\(result\.harness\.id\);\s*await onRefreshHarnesses\(\);/);
});

test("SettingsModal keeps the command-path control mounted while its subsection is collapsed instead of unmounting its draft state", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(settingsSource, /\{expanded && \(\s*<HarnessCommandPathControl/);
  assert.match(settingsSource, /hidden=\{!expanded\}/);
});

test("SettingsModal hidden subsections override the flex layout", () => {
  const css = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");
  assert.match(css, /\.settings-config-item-subsection\[hidden\]\s*\{\s*display:\s*none;/);
});

test("SettingsModal stops both click and keydown propagation from the toggle so activating it cannot also expand or collapse the row", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(
    settingsSource,
    /className="settings-config-item-header-actions"\s*\n\s*onClick=\{\(event\) => event\.stopPropagation\(\)\}\s*\n\s*onKeyDown=\{\(event\) => event\.stopPropagation\(\)\}/,
  );
});

test("SettingsModal surfaces a custom-path tell on collapsed rows via the exported looksAbsolute check", () => {
  const settingsSource = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(settingsSource, /import HarnessCommandPathControl, \{ looksAbsolute \} from "\.\/HarnessCommandPathControl"/);
  assert.match(settingsSource, /settings-config-custom-path-tell/);

  const controlSource = readFileSync(new URL("../src/components/HarnessCommandPathControl.tsx", import.meta.url), "utf8");
  assert.match(controlSource, /export function looksAbsolute/);
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
