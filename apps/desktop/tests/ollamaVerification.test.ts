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

test("preload and window typing expose verifyOllama", () => {
  const preload = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
  const types = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
  assert.match(preload, /verifyOllama:\s*\(baseUrl: string\)/);
  assert.match(types, /verifyOllama:\s*\(baseUrl: string\)\s*=>\s*Promise<OllamaVerificationResponse>/);
});
