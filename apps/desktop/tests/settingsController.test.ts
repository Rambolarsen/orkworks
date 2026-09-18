import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { mergeIntegrationOperationFailures } from "../src/settingsController.ts";
import type { ActiveHarnessIntegrationResult } from "../src/harnessIntegrationPresentation.ts";
import {
  createPeonSelectionTransaction,
  normalizePeonSelectionInput,
  type PeonSelectionTransport,
} from "../electron/peonSelectionTransaction.ts";

const mainSource = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
const preloadSource = readFileSync(new URL("../electron/preload.ts", import.meta.url), "utf8");
const rendererTypes = readFileSync(new URL("../src/orkworksWindow.d.ts", import.meta.url), "utf8");
const settingsControllerSource = readFileSync(new URL("../src/settingsController.ts", import.meta.url), "utf8");

test("Peon bridge keeps Apply separate from durable Save and exposes applied identity", () => {
  assert.match(mainSource, /ipcMain\.handle\("verify-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("test-and-apply-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("get-applied-peon-provider"/);
  assert.match(mainSource, /ipcMain\.handle\("save-peon-selection"/);
  assert.match(mainSource, /peonSelectionMatchesAppliedState/);

  const applyStart = mainSource.indexOf('ipcMain.handle("test-and-apply-peon-provider"');
  const saveStart = mainSource.indexOf('ipcMain.handle("save-peon-selection"');
  assert.notEqual(applyStart, -1);
  assert.notEqual(saveStart, -1);
  assert.doesNotMatch(mainSource.slice(applyStart, saveStart), /writeSettings\(/);
  assert.match(mainSource.slice(saveStart), /writeSettings\(/);

  assert.match(preloadSource, /verifyPeonProvider:/);
  assert.match(preloadSource, /testAndApplyPeonProvider:/);
  assert.match(preloadSource, /getAppliedPeonProvider:/);
  assert.match(preloadSource, /savePeonSelection:/);
  assert.match(rendererTypes, /verifyPeonProvider:/);
  assert.match(rendererTypes, /testAndApplyPeonProvider:/);
  assert.match(rendererTypes, /getAppliedPeonProvider:/);
  assert.match(rendererTypes, /savePeonSelection:/);
  assert.ok((mainSource.match(/async function parsePeonError/g)?.length ?? 0) >= 1);

  for (const route of [
    'ipcMain.handle("save-hotkeys"',
    'ipcMain.handle("save-retention"',
    'ipcMain.handle("save-debug-settings"',
    'ipcMain.handle("save-provider-settings"',
  ]) {
    const start = mainSource.indexOf(route);
    const end = mainSource.indexOf("ipcMain.handle(", start + route.length);
    assert.match(mainSource.slice(start, end === -1 ? undefined : end), /writeSettings\(/);
  }

  const savePeonStart = mainSource.indexOf('ipcMain.handle("save-peon-selection"');
  assert.match(mainSource.slice(savePeonStart), /writeSettings\(/);
});

function transactionHarness(overrides: Partial<PeonSelectionTransport> = {}) {
  const calls: string[] = [];
  const transport: PeonSelectionTransport = {
    discover: async () => ["llama3"],
    verify: async ({ provider, ollamaBaseUrl, generation }) => {
      calls.push(`verify:${generation}`);
      return {
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["gpt-5"],
        ollamaBaseUrl: provider === "ollama" ? ollamaBaseUrl ?? "http://127.0.0.1:11434" : null,
        generation,
      };
    },
    apply: async ({ selection, generation }) => {
      calls.push(`apply:${generation}`);
      return {
        provider: selection.provider,
        model: selection.model,
        ollamaBaseUrl: selection.provider === "ollama" ? selection.ollamaBaseUrl ?? null : null,
        appliedAt: "now",
        connectionRevision: 1,
      };
    },
    getApplied: async () => ({ provider: "copilot", model: "gpt-5", ollamaBaseUrl: null, appliedAt: "now", connectionRevision: 1 }),
    ...overrides,
  };
  return { calls, transaction: createPeonSelectionTransaction(transport) };
}

test("Peon transaction requires a successful matching Apply before Save", async () => {
  const { calls, transaction } = transactionHarness();
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  let persisted = 0;
  assert.deepEqual(
    await transaction.save(selection, async () => { persisted += 1; }),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
  assert.equal(persisted, 0);

  await transaction.verify(selection.provider);
  await transaction.apply(selection);
  assert.deepEqual(await transaction.save(selection, async () => { persisted += 1; }), { ok: true });
  assert.equal(persisted, 1);
  assert.deepEqual(calls, ["verify:1", "apply:1"]);
});

test("persisted Peon synchronization passes the already-known ready port", async () => {
  const ports: unknown[] = [];
  const { transaction } = transactionHarness({
    verify: async ({ provider, ollamaBaseUrl, generation, readyPort }) => {
      ports.push(readyPort);
      return {
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["gpt-5"],
        ollamaBaseUrl: null,
        generation,
      };
    },
    apply: async ({ selection, readyPort }) => {
      ports.push(readyPort);
      return {
        provider: selection.provider,
        model: selection.model,
        ollamaBaseUrl: null,
        appliedAt: "now",
        connectionRevision: 1,
      };
    },
  });

  await transaction.syncPersistedSelection({ provider: "copilot", model: "gpt-5" }, undefined, 43123);
  assert.deepEqual(ports, [43123, 43123]);
});

test("Peon Save rejects matching sidecar state without a local successful Apply", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    getApplied: async () => ({
      provider: selection.provider,
      model: selection.model,
      ollamaBaseUrl: null,
      appliedAt: "now",
      connectionRevision: 1,
    }),
  });

  await transaction.verify(selection.provider);
  assert.deepEqual(
    await transaction.save(selection, async () => {}),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
});

test("Peon Save rejects an Apply from an older generation even when sidecar identity matches", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    getApplied: async () => ({
      provider: selection.provider,
      model: selection.model,
      ollamaBaseUrl: null,
      appliedAt: "now",
      connectionRevision: 1,
    }),
  });

  await transaction.verify(selection.provider);
  await transaction.apply(selection);
  await transaction.verify(selection.provider);
  assert.deepEqual(
    await transaction.save(selection, async () => {}),
    { ok: false, error: "Save requires a matching successful Apply." },
  );
});

test("Peon discovery does not mutate or supersede the Apply transaction", async () => {
  let resolveOld!: (value: Awaited<ReturnType<PeonSelectionTransport["verify"]>>) => void;
  const { transaction } = transactionHarness({
    verify: ({ provider, generation }) => provider === "copilot"
      ? new Promise((resolve) => { resolveOld = resolve; })
      : Promise.resolve({
        ok: true,
        provider,
        capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
        models: ["llama3"],
        ollamaBaseUrl: "http://custom-ollama:11434",
        generation,
      }),
  });
  const oldVerification = transaction.verify("copilot");
  const discovery = await transaction.discover("ollama", "http://custom-ollama:11434");
  assert.deepEqual(discovery, ["llama3"]);
  resolveOld({
    ok: true,
    provider: "copilot",
    capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
    models: ["gpt-5"],
    ollamaBaseUrl: null,
    generation: 1,
  });
  await oldVerification;
});

test("compatibility model discovery uses sidecar discovery without the transaction coordinator", () => {
  assert.doesNotMatch(mainSource, /providerModelDiscoveryGeneration/);
  const discoveryStart = mainSource.indexOf('ipcMain.handle("get-provider-models"');
  assert.notEqual(discoveryStart, -1);
  const discoveryRoute = mainSource.slice(discoveryStart);
  assert.match(discoveryRoute, /settings\/providers\/\$\{encodeURIComponent\(providerId\)\}\/models/);
  assert.doesNotMatch(discoveryRoute, /peonTransaction\.discover\(/);
  assert.doesNotMatch(discoveryRoute, /settings\/peon\/provider\/verify/);
});

test("provider model cache is keyed and invalidated by the Ollama base URL", () => {
  assert.match(mainSource, /providerModelCacheKey/);
  assert.match(mainSource, /providerModelCacheKey\(providerId, ollamaBaseUrl\)/);
  assert.match(mainSource, /previousOllamaBaseUrl/);
  assert.match(mainSource, /providerModels\.delete\(providerModelCacheKey\("ollama", previousOllamaBaseUrl\)\)/);
});

test("Peon selection input resolves one persisted custom Ollama URL", () => {
  assert.deepEqual(
    normalizePeonSelectionInput(
      { provider: "ollama", model: "llama3.2:3b" },
      "https://ollama.example.test:11434",
    ),
    {
      provider: "ollama",
      model: "llama3.2:3b",
      ollamaBaseUrl: "https://ollama.example.test:11434",
    },
  );
});

test("persisted Peon synchronization fails instead of allowing readiness to continue", async () => {
  const selection = { provider: "copilot" as const, model: "gpt-5" };
  const { transaction } = transactionHarness({
    apply: async () => { throw new Error("inference unavailable"); },
  });
  await assert.rejects(transaction.syncPersistedSelection(selection), /inference unavailable/);
});

function failedIntegration(message: string): ActiveHarnessIntegrationResult {
  return {
    operation: "install",
    outcome: "failed",
    registration: "absent",
    activation: "unknown",
    coverage: "full",
    diagnosticCode: "mutation_failed",
    message,
  };
}

test("mergeIntegrationOperationFailures clears a prior failure when Codex repair succeeds so status diagnostics can surface", () => {
  const current = {
    codex: failedIntegration("Approve the hook in Codex."),
    opencode: failedIntegration("permission denied"),
  };

  for (const operation of ["install", "repair"] as const) {
    assert.deepEqual(
      mergeIntegrationOperationFailures(current, {
        codex: {
          operation,
          outcome: "succeeded",
          registration: "installed",
          activation: "needs_trust",
          coverage: "full",
          diagnosticCode: "needs_trust",
          message: "Approve the hook in Codex.",
        },
      }),
      {
        opencode: failedIntegration("permission denied"),
      },
      `expected ${operation} success to clear the old Codex failure cache`,
    );
  }
});

test("mergeIntegrationOperationFailures records new failures without clearing unrelated warnings", () => {
  const current = {
    codex: failedIntegration("Approve the hook in Codex."),
  };

  assert.deepEqual(
    mergeIntegrationOperationFailures(current, {
      claude: failedIntegration("workspace config is read-only"),
    }),
    {
      codex: failedIntegration("Approve the hook in Codex."),
      claude: failedIntegration("workspace config is read-only"),
    },
  );
});

test("mergeIntegrationOperationFailures ignores stale workspace results when preserving warnings", () => {
  const current = {
    codex: failedIntegration("Approve the hook in Codex."),
  };

  assert.deepEqual(
    mergeIntegrationOperationFailures(current, {
      codex: {
        operation: "repair",
        outcome: "stale_workspace",
        registration: "installed",
        activation: "active",
        coverage: "full",
        diagnosticCode: "stale_workspace",
        message: "Workspace changed while saving coding tools.",
      },
    }),
    current,
  );
});

test("settingsController removes the obsolete modal-wide commit path", () => {
  assert.doesNotMatch(settingsControllerSource, /SettingsCommitResult/);
  assert.doesNotMatch(settingsControllerSource, /\bcommit\(\): Promise/);
  assert.doesNotMatch(settingsControllerSource, /saveHotkeys:/);
  assert.doesNotMatch(settingsControllerSource, /saveRetention:/);
  assert.doesNotMatch(settingsControllerSource, /saveDebugSettings:/);
  assert.doesNotMatch(settingsControllerSource, /saveProviderSettings:/);
});

