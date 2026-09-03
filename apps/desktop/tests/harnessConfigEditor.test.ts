import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parseHarnessDraft } from "../src/harnessTypes.ts";

function source(path: string): string {
  try {
    return readFileSync(new URL(path, import.meta.url), "utf8");
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code === "ENOENT") return "";
    throw error;
  }
}

function functionSource(text: string, name: string): string {
  const start = text.indexOf(`export async function ${name}`);
  if (start < 0) return "";
  const end = text.indexOf("\nexport ", start + 1);
  return text.slice(start, end < 0 ? text.length : end);
}

test("renderer harness snapshots preserve effective definitions, origins, overrides, profiles, and revisions", () => {
  const api = source("../src/api.ts");
  const types = source("../src/harnessTypes.ts");
  const app = source("../src/App.tsx");
  const typeContract = `${types}\n${api}`;

  assert.match(typeContract, /export interface HarnessConfigEntry/);
  assert.match(typeContract, /origin:\s*"builtin"\s*\|\s*"override"\s*\|\s*"custom"/);
  assert.match(typeContract, /storedOverride\??:/);
  assert.match(typeContract, /export interface HarnessCompatibilityMetadata/);
  assert.match(typeContract, /sessionSignals:/);
  assert.match(typeContract, /integration:/);
  assert.match(typeContract, /export interface HarnessListResponse[\s\S]*documentRevision[\s\S]*harnesses/);
  assert.match(api, /\.\.\.entry\.definition/);
  assert.match(api, /export async function listHarnesses/);

  assert.match(app, /const \[harnesses, setHarnesses\]/);
  assert.match(app, /list\.documentRevision/);
  assert.match(app, /setHarnesses\(list\.harnesses\)/);
});

test("renderer API exposes duplicate preview and revision-aware configuration mutations", () => {
  const api = source("../src/api.ts");
  const duplicate = functionSource(api, "duplicateHarness");
  const save = functionSource(api, "saveHarnessConfiguration");
  const removeProfile = functionSource(api, "removeHarnessProfile");
  const remove = functionSource(api, "deleteHarness");

  assert.match(api, /export async function duplicateHarness/);
  assert.match(duplicate, /\/harnesses\/\$\{[^}]*sourceId[^}]*\}\/duplicate/);
  assert.match(duplicate, /method:\s*"POST"/);
  assert.match(duplicate, /proposedId|proposed_id/);
  assert.match(duplicate, /proposedName|proposed_name/);
  assert.doesNotMatch(duplicate, /saveHarnessConfiguration|setHarnessCommandOverride|clearHarnessCommandOverride/);

  assert.match(api, /export async function saveHarnessConfiguration/);
  assert.match(save, /expectedRevision/);
  assert.match(save, /duplicateSourceId/);
  assert.match(save, /JSON\.stringify/);
  assert.match(save, /definition/);

  assert.match(api, /export async function removeHarnessProfile/);
  assert.match(removeProfile, /remove-profile/);
  assert.match(removeProfile, /expectedRevision/);

  assert.match(api, /export async function deleteHarness/);
  assert.match(remove, /method:\s*"DELETE"/);
  assert.match(remove, /expectedRevision/);
});

test("HarnessConfigEditor exposes the planned in-place editor API and override/custom modes", () => {
  const editor = source("../src/components/HarnessConfigEditor.tsx");

  assert.match(editor, /interface HarnessConfigEditorProps/);
  assert.match(editor, /mode\s*:/);
  assert.match(editor, /["']override["']/);
  assert.match(editor, /["']custom["']/);
  assert.match(editor, /draftText:\s*string/);
  assert.match(editor, /metadata/);
  assert.match(editor, /onCancel/);
  assert.match(editor, /onSaved/);
  assert.match(editor, /export default function HarnessConfigEditor/);
  assert.match(editor, /<textarea/);
  assert.match(editor, /onClick=\{[^}]*onCancel/);
  assert.match(editor, /Override JSON/);
  assert.match(editor, /Configuration JSON/);
});

test("Settings renders origin badges for built-ins, overrides, and custom harnesses", () => {
  const editor = source("../src/components/HarnessConfigEditor.tsx");
  const settings = source("../src/components/SettingsModal.tsx");

  assert.match(`${editor}\n${settings}`, /origin-badge/);
  assert.match(`${editor}\n${settings}`, /Built-in/);
  assert.match(`${editor}\n${settings}`, /Override/);
  assert.match(`${editor}\n${settings}`, /Custom/);
  assert.match(settings, /h\.origin/);
});

test("Settings starts a server duplicate preview without saving a record or installing a hook", () => {
  const api = source("../src/api.ts");
  const settings = source("../src/components/SettingsModal.tsx");
  const duplicateCall = settings.indexOf("duplicateHarness(");
  const duplicateRegion = duplicateCall < 0 ? "" : settings.slice(duplicateCall - 500, duplicateCall + 1200);

  assert.match(settings, /HarnessConfigEditor/);
  assert.match(settings, /duplicateHarness/);
  assert.match(settings, /Duplicate/);
  assert.match(api, /duplicateHarness/);
  assert.doesNotMatch(duplicateRegion, /saveHarnessConfiguration|saveActiveHarnessesWithIntegrations|setHarnessCommandOverride|installIntegration/);
});

test("the editor shows a read-only effective preview and compatibility-profile copy outside editable JSON", () => {
  const editor = source("../src/components/HarnessConfigEditor.tsx");
  const settings = source("../src/components/SettingsModal.tsx");
  const text = `${editor}\n${settings}`;

  assert.match(text, /Effective configuration/i);
  assert.match(text, /JSON\.stringify/);
  assert.match(text, /Compatibility profile/i);
  assert.match(text, /Derived integration/i);
  assert.match(text, /Derived session signals/i);
  assert.match(text, /read-only/i);
  assert.match(text, /This profile is code-owned/);
  assert.match(text, /command and harness settings remain independently editable/);
  assert.match(text, /This tool uses the Copilot integration/);
  assert.match(text, /shared hook remains installed while any active compatible tool uses it/);
});

test("the editor explains sparse overrides and independent custom copies", () => {
  const text = `${source("../src/components/HarnessConfigEditor.tsx")}\n${source("../src/components/SettingsModal.tsx")}`;

  assert.match(text, /Only these fields are customized/);
  assert.match(text, /Unspecified fields continue using the built-in defaults/);
  assert.match(text, /Future built-in improvements will apply automatically/);
  assert.match(text, /This is an independent copy/);
  assert.match(text, /Future changes to the source harness will not modify it/);
});

test("invalid JSON keeps the typed draft visible, reports line and column diagnostics, and blocks save", () => {
  const editor = source("../src/components/HarnessConfigEditor.tsx");

  assert.match(editor, /value=\{draft/);
  assert.match(editor, /setDraft\w*\([^\n]*target\.value/);
  assert.match(editor, /parseHarnessDraft\(draft/);
  assert.match(editor, /line/i);
  assert.match(editor, /column/i);
  assert.match(editor, /draft(?:Error|Diagnostic)|parseError|validationError|diagnostics/);
  assert.match(editor, /disabled=\{[^}]*?(?:draft|parse|validation|error)/i);
});

test("renderer validation rejects duplicate keys and unknown nested schema fields", () => {
  const duplicate = parseHarnessDraft('{"id":"tool","id":"other","name":"Tool","launch":{"kind":"platform-shell","login":true}}', "create");
  assert.ok(duplicate.diagnostics.some((diagnostic) => diagnostic.code === "duplicate_key"));
  assert.equal(duplicate.diagnostics.find((diagnostic) => diagnostic.code === "duplicate_key")?.path, "$");

  const unknown = parseHarnessDraft(JSON.stringify({
    id: "tool",
    name: "Tool",
    launch: { kind: "command-template", command: "tool", args: [], unknownField: true },
  }), "create");
  assert.ok(unknown.diagnostics.some((diagnostic) => diagnostic.code === "unknown_field" && diagnostic.path === "$.launch.unknownField"));
});

test("revision conflicts are surfaced as retryable configuration errors without replacing the draft", () => {
  const api = source("../src/api.ts");
  const editor = source("../src/components/HarnessConfigEditor.tsx");

  assert.match(api, /harness_config_revision_changed/);
  assert.match(api, /(?:response|resp)\.status\s*===\s*409|status:\s*409/);
  assert.match(editor, /409|harness_config_revision_changed|revision conflict/i);
  assert.match(editor, /reload|refresh|retry/i);
  assert.match(editor, /draftText/);
  assert.match(editor, /onSaved/);
});

test("custom deletion sends the document revision and explains active-workspace rejection", () => {
  const api = source("../src/api.ts");
  const settings = source("../src/components/SettingsModal.tsx");
  const editor = source("../src/components/HarnessConfigEditor.tsx");
  const contract = `${api}\n${settings}\n${editor}`;

  assert.match(api, /deleteHarness/);
  assert.match(api, /expectedRevision/);
  assert.match(settings, /activeHarnessIds/);
  assert.match(editor, /deleteHarness/);
  assert.match(contract, /active_harness_delete_forbidden/);
  assert.match(contract, /disable.*save|save.*disable/i);
});

test("Settings keeps toggles, detection, command path, integration, confirmation, and active-tool Save controls", () => {
  const settings = source("../src/components/SettingsModal.tsx");

  assert.match(settings, /<Toggle/);
  assert.match(settings, /<HarnessDetectionStatus/);
  assert.match(settings, /<HarnessCommandPathControl/);
  assert.match(settings, /getGroupedHarnessIntegrationStatus/);
  assert.match(settings, /saveActiveHarnessesHandler/);
  assert.match(settings, /onSaveActiveHarnesses/);
  assert.match(settings, /settings-config-footer/);
});
