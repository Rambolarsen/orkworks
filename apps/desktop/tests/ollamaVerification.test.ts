import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("provider types include an Ollama verification response", () => {
  const source = readFileSync(new URL("../src/providerTypes.ts", import.meta.url), "utf8");
  assert.match(source, /export interface OllamaVerificationResponse/);
  assert.match(source, /normalizedBaseUrl/);
  assert.match(source, /reasonCode/);
  assert.match(source, /excludedModels/);
});

test("Electron model suggestions use sidecar discovery without Peon verification", () => {
  const main = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
  assert.match(main, /settings\/providers\/\$\{encodeURIComponent\(providerId\)\}\/models/);
  assert.doesNotMatch(main, /peonTransaction\.discover\(/);
  assert.doesNotMatch(main, /settings\/peon\/provider\/verify.*providerId/);
});

test("SettingsModal verifies and invalidates provider-first drafts", () => {
  const source = readFileSync(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
  assert.match(source, /verifyPeonSelection/);
  assert.match(source, /getAppliedPeonProvider/);
  assert.match(source, /setPeonVerification\(null\)/);
  assert.match(source, /testAndApplyPeonProvider/);
  assert.match(source, /savePeonSelection/);
});
