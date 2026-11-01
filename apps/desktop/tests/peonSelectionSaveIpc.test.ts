import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";
import { createPeonSelectionTransaction, normalizePeonSelectionInput } from "../electron/peonSelectionTransaction.ts";
import { DEFAULT_HOTKEYS, readSettings, settingsWithPeonSelection, writeSettings } from "../electron/settingsMemory.ts";
import type { PeonSelectionSaveResult } from "../src/providerTypes.ts";

const main = readFileSync(new URL("../electron/main.ts", import.meta.url), "utf8");
function snippet(startMarker: string, endMarker: string): string {
  const start = main.indexOf(startMarker);
  const end = main.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `missing main-process code: ${startMarker}`);
  return main.slice(start, end);
}

// Execute the registered production handler, projection and settings queue.
// Only Electron registration and the sidecar transport are replaced.
function saveHarness(directory: string, persist = writeSettings) {
  const selection = { provider: "copilot", model: "gpt-5", reasoningEffort: "high" };
  const applied = { ...selection, ollamaBaseUrl: null, appliedAt: "now", connectionRevision: 1 };
  const transaction = createPeonSelectionTransaction({
    verify: async ({ provider, generation }) => ({
      ok: true, provider, generation, models: ["gpt-5"], ollamaBaseUrl: null,
      capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
    }),
    discover: async () => ["gpt-5"],
    apply: async () => applied,
    getApplied: async () => applied,
  });
  let handler!: (_event: unknown, value: unknown) => Promise<PeonSelectionSaveResult>;
  const injected = {
    ipcMain: { handle: (_channel: string, callback: typeof handler) => { handler = callback; } },
    app: { getPath: () => directory },
    DEFAULT_HOTKEYS, normalizePeonSelectionInput, readSettings, settingsWithPeonSelection,
    writeSettings: persist, peonTransaction: transaction,
    persistedOllamaBaseUrl: () => "http://127.0.0.1:11434",
  };
  const compiled = transpileModule(`
    let currentSettings = readSettings(app.getPath("userData"));
    let settingsWriteQueue = Promise.resolve();
    ${snippet("function rendererSettings(", "function providerModelCacheKey(")}
    ${snippet('  ipcMain.handle("save-peon-selection"', "  // Compatibility IPC")}
  `, { compilerOptions: { module: ModuleKind.CommonJS, target: ScriptTarget.ES2022 } }).outputText;
  new Function(...Object.keys(injected), compiled)(...Object.values(injected));
  return { transaction, selection, save: (value: unknown = selection) => handler(undefined, value) };
}

test("Peon save IPC projects persisted settings and default hotkeys after Apply", async () => {
  const directory = mkdtempSync(join(tmpdir(), "peon-save-ipc-"));
  try {
    const { transaction, selection, save } = saveHarness(directory);
    await transaction.verify(selection.provider);
    await transaction.apply(selection);
    const result = await save();
    assert.equal(result.ok, true);
    if (!result.ok) return;
    assert.deepEqual(result.settings.providers.peonSelection, selection);
    assert.equal(result.settings.providers.revision, 1);
    assert.equal(result.settings.defaultHotkeys.newSession, "CmdOrCtrl+N");
    assert.deepEqual(result.settings, { ...readSettings(directory), defaultHotkeys: DEFAULT_HOTKEYS });
    assert.equal(Object.hasOwn(result, "providerApplyStatus"), false);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("Peon save IPC returns transaction rejection without settings or persistence", async () => {
  const directory = mkdtempSync(join(tmpdir(), "peon-save-ipc-"));
  try {
    const before = readSettings(directory);
    const { save } = saveHarness(directory);
    assert.deepEqual(await save(), { ok: false, error: "Save requires a matching successful Apply." });
    assert.deepEqual(readSettings(directory), before);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("Peon save IPC preserves validation and persistence exceptions", async () => {
  const directory = mkdtempSync(join(tmpdir(), "peon-save-ipc-"));
  try {
    const { transaction, selection, save } = saveHarness(directory, () => { throw new Error("disk full"); });
    await assert.rejects(save({ provider: "copilot" }), /Invalid Peon provider selection/);
    await transaction.verify(selection.provider);
    await transaction.apply(selection);
    await assert.rejects(save(), /disk full/);
    assert.equal(readSettings(directory).providers.peonSelection, null);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
