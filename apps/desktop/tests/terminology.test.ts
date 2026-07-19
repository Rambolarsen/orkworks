import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function source(path: string): string {
  return readFileSync(new URL(path, import.meta.url), "utf8");
}

test("NewSessionDialog labels the CLI selector as Coding tool", () => {
  const text = source("../src/components/NewSessionDialog.tsx");
  assert.match(text, />Coding tool</);
  assert.doesNotMatch(text, /htmlFor="nsd-harness">Provider</);
});

test("NewSessionDialog does not describe the initial prompt as sent to the provider", () => {
  const text = source("../src/components/NewSessionDialog.tsx");
  assert.match(text, /sent when the agent session starts/);
  assert.doesNotMatch(text, /sent to the provider on start/);
});

test("SessionDetailPanel distinguishes coding tool from model provider", () => {
  const text = source("../src/components/SessionDetailPanel.tsx");
  // Field labels are DetailField props now, not inline element text.
  assert.match(text, /label="Coding tool"/);
  assert.match(text, /label="Provider state"/);
  assert.match(text, /OrkWorks session ID/);
  assert.match(text, /Harness session ID/);
  assert.match(text, /Not captured/);
  assert.doesNotMatch(text, />Provider</);
  assert.doesNotMatch(text, /label="Provider"/);
  assert.doesNotMatch(text, />State</);
  assert.doesNotMatch(text, /label="State"/);
  // Model provider is demoted to a muted sub-line under the model name, not its own labeled row.
  assert.match(text, /providerContext\.modelProvider/);
  assert.match(text, /session-detail-value-sub/);
});

test("SessionDetailPanel gates the debug attention injection control behind showDebugMetadata", () => {
  const text = source("../src/components/SessionDetailPanel.tsx");
  assert.match(text, /onApplyDebugAttention/);
  assert.match(text, /debug-injection/);
  const debugBlockStart = text.indexOf("showDebugMetadata && (");
  const injectionControlIndex = text.indexOf("debug-injection");
  assert.notEqual(debugBlockStart, -1);
  assert.ok(
    injectionControlIndex > debugBlockStart,
    "the injection control must live inside the showDebugMetadata-gated block",
  );
});

test("api.ts defines applyDebugAttention against the debug-injection endpoint", () => {
  const text = source("../src/api.ts");
  assert.match(text, /export async function applyDebugAttention/);
  assert.match(text, /\/sessions\/\$\{id\}\/debug-injection/);
});

test("Settings provider copy refers to model providers", () => {
  const modal = source("../src/components/SettingsModal.tsx");
  const section = source("../src/components/ProviderSettingsSection.tsx");
  assert.match(modal, /Model providers/);
  assert.match(section, /model provider/);
});

test("No active coding tool prompt does not call tools providers", () => {
  const app = source("../src/App.tsx");
  assert.match(app, /No active coding tools/);
  assert.doesNotMatch(app, /No Active Providers/);
  assert.doesNotMatch(app, /No provider harnesses/);
});
