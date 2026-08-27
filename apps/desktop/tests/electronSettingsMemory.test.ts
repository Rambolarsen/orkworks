import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import * as settingsMemory from "../electron/settingsMemory.ts";

import {
  DEFAULT_HOTKEYS,
  DEFAULT_SETTINGS,
  normalizeProviderSettings,
  peonSelectionMatchesAppliedState,
  readSettings,
  savePeonSelection,
  settingsWithPeonSelection,
  settingsPath,
  settingsWithHotkeys,
  validateHotkeys,
  writeSettings,
} from "../electron/settingsMemory.ts";
import type { PeonAppliedState, PeonSelection, ProviderSettings } from "../electron/providerTypes.ts";

test("settings memory returns defaults when settings.json is missing", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const settings = readSettings(dir);
    assert.equal(settings.version, 1);
    assert.deepEqual(settings.hotkeys, DEFAULT_HOTKEYS);
    assert.equal(settings.debug.showSessionIds, false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Peon Apply preparation changes no durable settings and v2 save changes only selection", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const base = readSettings(dir);
    const selection: PeonSelection = { provider: "copilot", model: " gpt-5 " };

    const next = settingsWithPeonSelection(base, selection);

    assert.deepEqual(base.providers.peonSelection, null);
    assert.deepEqual(next.providers.peonSelection, { provider: "copilot", model: "gpt-5" });
    assert.equal(next.providers.peonModel, null);
    assert.deepEqual(next.providers.providers, base.providers.providers);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Peon selection save retains the previous file when persistence fails", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const before = { ...DEFAULT_SETTINGS, providers: { ...DEFAULT_SETTINGS.providers } };
    writeSettings(dir, before);
    const previousFile = readFileSync(settingsPath(dir), "utf8");

    assert.throws(
      () => savePeonSelection(dir, { provider: "copilot", model: "gpt-5" }, () => {
        throw new Error("disk full");
      }),
      /disk full/,
    );
    assert.equal(readFileSync(settingsPath(dir), "utf8"), previousFile);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Peon applied identity matching includes the Ollama URL", () => {
  const selection: PeonSelection = {
    provider: "ollama",
    model: "llama3.2:3b",
    ollamaBaseUrl: "https://localhost:11434",
  };
  const applied: PeonAppliedState = {
    provider: "ollama",
    model: "llama3.2:3b",
    ollamaBaseUrl: "https://localhost:11434",
    appliedAt: "2026-08-27T10:00:00Z",
    connectionRevision: 1,
  };

  assert.equal(peonSelectionMatchesAppliedState(selection, applied), true);
  assert.equal(
    peonSelectionMatchesAppliedState(selection, { ...applied, ollamaBaseUrl: "http://localhost:11434" }),
    false,
  );
  assert.equal(
    peonSelectionMatchesAppliedState({ ...selection, model: "other-model" }, applied),
    false,
  );
});

test("settings memory falls back to defaults when settings.json is corrupt", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(settingsPath(dir), "{not json");
    const settings = readSettings(dir);
    assert.deepEqual(settings.hotkeys, DEFAULT_HOTKEYS);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory returns fresh defaults when settings.json is missing or corrupt", () => {
  const missingDir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  const corruptDir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  const originalNewSession = DEFAULT_HOTKEYS.newSession;
  try {
    const missingSettings = readSettings(missingDir);
    missingSettings.hotkeys.newSession = "CmdOrCtrl+Alt+N";

    assert.equal(readSettings(missingDir).hotkeys.newSession, originalNewSession);
    assert.equal(DEFAULT_HOTKEYS.newSession, originalNewSession);

    writeFileSync(settingsPath(corruptDir), "{not json");
    const corruptSettings = readSettings(corruptDir);
    corruptSettings.hotkeys.newSession = "CmdOrCtrl+Alt+N";

    assert.equal(readSettings(corruptDir).hotkeys.newSession, originalNewSession);
    assert.equal(DEFAULT_HOTKEYS.newSession, originalNewSession);
  } finally {
    DEFAULT_HOTKEYS.newSession = originalNewSession;
    rmSync(missingDir, { recursive: true, force: true });
    rmSync(corruptDir, { recursive: true, force: true });
  }
});

test("default settings hotkeys are isolated from default hotkeys and returned settings", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  const originalDefaultHotkeys = { ...DEFAULT_HOTKEYS };
  const originalDefaultSettingsHotkeys = { ...DEFAULT_SETTINGS.hotkeys };
  try {
    assert.notEqual(DEFAULT_SETTINGS.hotkeys, DEFAULT_HOTKEYS);

    const settings = readSettings(dir);
    settings.hotkeys.newSession = "CmdOrCtrl+Alt+N";

    assert.deepEqual(DEFAULT_HOTKEYS, originalDefaultHotkeys);
    assert.deepEqual(DEFAULT_SETTINGS.hotkeys, originalDefaultSettingsHotkeys);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory merges partial persisted hotkeys with defaults", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({ version: 1, hotkeys: { newSession: "CmdOrCtrl+Alt+N" } }),
    );

    const settings = readSettings(dir);

    assert.equal(settings.hotkeys.newSession, "CmdOrCtrl+Alt+N");
    assert.equal(settings.hotkeys.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel);
    assert.equal(settings.hotkeys.resetLayout, null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory falls back invalid persisted hotkeys to defaults", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        hotkeys: {
          newSession: "NotAKey",
          toggleDetailPanel: "CmdOrCtrl+Alt+D",
        },
      }),
    );

    const settings = readSettings(dir);

    assert.equal(settings.hotkeys.newSession, DEFAULT_HOTKEYS.newSession);
    assert.equal(settings.hotkeys.toggleDetailPanel, "CmdOrCtrl+Alt+D");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory falls back duplicate persisted hotkeys to defaults", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        hotkeys: {
          newSession: "CmdOrCtrl+Shift+S",
          toggleSessionsPanel: "Shift+CmdOrCtrl+S",
          toggleDetailPanel: "CmdOrCtrl+Alt+D",
        },
      }),
    );

    const settings = readSettings(dir);

    assert.equal(settings.hotkeys.newSession, DEFAULT_HOTKEYS.newSession);
    assert.equal(settings.hotkeys.toggleSessionsPanel, DEFAULT_HOTKEYS.toggleSessionsPanel);
    assert.equal(settings.hotkeys.toggleDetailPanel, "CmdOrCtrl+Alt+D");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory writes canonical settings JSON", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeSettings(dir, {
      version: 1,
      hotkeys: {
        ...DEFAULT_HOTKEYS,
        newSession: "CmdOrCtrl+Alt+N",
      },
    });

    const raw = readFileSync(settingsPath(dir), "utf8");
    assert.equal(raw.endsWith("\n"), true);
    assert.equal(JSON.parse(raw).hotkeys.newSession, "CmdOrCtrl+Alt+N");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory preserves future top-level settings sections", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        hotkeys: { newSession: "CmdOrCtrl+Alt+N" },
        ui: { theme: "sepia", density: "compact" },
      }),
    );

    const settings = readSettings(dir);
    assert.deepEqual(settings.ui, { theme: "sepia", density: "compact" });

    writeSettings(dir, {
      ...settings,
      hotkeys: {
        ...settings.hotkeys,
        toggleTerminalPanel: "CmdOrCtrl+Alt+T",
      },
    });

    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.deepEqual(persisted.ui, { theme: "sepia", density: "compact" });
    assert.equal(persisted.hotkeys.newSession, "CmdOrCtrl+Alt+N");
    assert.equal(persisted.hotkeys.toggleTerminalPanel, "CmdOrCtrl+Alt+T");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory normalizes debug settings and preserves persisted showSessionIds", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        debug: { showSessionIds: true },
      }),
    );

    const settings = readSettings(dir);

    assert.deepEqual(settings.debug, { showSessionIds: true, rendererHealthLogMs: 0 });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settingsWithHotkeys returns one canonical settings object for validation menu and disk", () => {
  const baseSettings = {
    version: 1 as const,
    hotkeys: DEFAULT_HOTKEYS,
    ui: { theme: "sepia" },
  };

  const nextSettings = settingsWithHotkeys(baseSettings, {
    ...DEFAULT_HOTKEYS,
    newSession: "  CmdOrCtrl+Alt+N  ",
    resetLayout: {},
  });

  assert.equal(nextSettings.hotkeys.newSession, "CmdOrCtrl+Alt+N");
  assert.equal(nextSettings.hotkeys.resetLayout, null);
  assert.deepEqual(nextSettings.ui, { theme: "sepia" });
  assert.deepEqual(validateHotkeys(nextSettings.hotkeys), { ok: true, errors: {} });
});

test("settingsWithHotkeys preserves invalid save payloads for validation", () => {
  const nextSettings = settingsWithHotkeys(DEFAULT_SETTINGS, {
    ...DEFAULT_HOTKEYS,
    newSession: "N",
    toggleDetailPanel: "",
  });

  assert.equal(nextSettings.hotkeys.newSession, "N");
  assert.equal(nextSettings.hotkeys.toggleDetailPanel, "");

  const result = validateHotkeys(nextSettings.hotkeys);
  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.newSession, ["Shortcut must include a modifier."]);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut is required."]);
});

test("settingsWithHotkeys preserves duplicate save payloads for validation", () => {
  const nextSettings = settingsWithHotkeys(DEFAULT_SETTINGS, {
    ...DEFAULT_HOTKEYS,
    toggleSessionsPanel: "  CmdOrCtrl+N  ",
  });

  assert.equal(nextSettings.hotkeys.toggleSessionsPanel, "CmdOrCtrl+N");

  const result = validateHotkeys(nextSettings.hotkeys);
  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleSessionsPanel, ["Duplicate shortcut also used by New Session."]);
});

test("validateHotkeys rejects duplicates", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    toggleSessionsPanel: DEFAULT_HOTKEYS.newSession,
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleSessionsPanel, ["Duplicate shortcut also used by New Session."]);
});

test("validateHotkeys rejects duplicates with reordered modifiers", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    newSession: "Shift+CmdOrCtrl+S",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleSessionsPanel, ["Duplicate shortcut also used by New Session."]);
});

test("validateHotkeys rejects duplicate canonical modifiers", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    newSession: "CmdOrCtrl+CmdOrCtrl+Shift+S",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.newSession, ['Shortcut contains duplicate modifier "CmdOrCtrl".']);
});

test("validateHotkeys rejects invalid syntax and required empty values", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    newSession: "",
    toggleDetailPanel: "CmdOrCtrl+",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.newSession, ["Shortcut is required."]);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut must include a non-modifier key."]);
});

test("validateHotkeys rejects malformed separator syntax", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    toggleDetailPanel: "CmdOrCtrl++N",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut has invalid separator syntax."]);
});

test("validateHotkeys rejects trailing separator syntax", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    toggleDetailPanel: "CmdOrCtrl+N+",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut has invalid separator syntax."]);
});

test("validateHotkeys rejects unmodified ordinary keys", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    newSession: "N",
    toggleDetailPanel: "A",
  });

  assert.equal(result.ok, false);
  assert.deepEqual(result.errors.newSession, ["Shortcut must include a modifier."]);
  assert.deepEqual(result.errors.toggleDetailPanel, ["Shortcut must include a modifier."]);
});

test("validateHotkeys allows optional resetLayout to be unset", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    resetLayout: null,
  });

  assert.deepEqual(result, { ok: true, errors: {} });
});

test("settings memory seeds default provider settings", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const settings = readSettings(dir);
    assert.deepEqual(settings.providers, {
      version: 2,
      revision: 0,
      peonSelection: null,
      peonModel: null,
      ollamaBaseUrl: "http://127.0.0.1:11434",
      providers: [
        { id: "opencode", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null, model: null },
        { id: "claude-code", enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null, model: null },
        { id: "codex", enabled: true, fallbackOrder: 2, defaultState: "unknown", overrideState: null, model: null },
        { id: "aider", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null, model: null },
        { id: "copilot", enabled: true, fallbackOrder: 4, defaultState: "unknown", overrideState: null, model: null },
        { id: "ollama", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null, model: null },
      ],
    } satisfies ProviderSettings);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("startup migration canonicalizes legacy Copilot and removes retired Gemini", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 1,
          revision: 7,
          peonModel: null,
          ollamaBaseUrl: "http://127.0.0.1:11434",
          providers: [
            { id: "gemini", enabled: true, fallbackOrder: 3, defaultState: "unknown", overrideState: null },
            { id: "gh-copilot", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null },
          ],
        },
      }),
    );

    const loadSettingsForStartup = (settingsMemory as typeof settingsMemory & {
      loadSettingsForStartup?: (userDataPath: string) => typeof DEFAULT_SETTINGS;
    }).loadSettingsForStartup;
    assert.equal(typeof loadSettingsForStartup, "function");
    const settings = loadSettingsForStartup!(dir);

    assert.equal(settings.providers.revision, 7);
    assert.equal(settings.providers.providers.some((provider) => provider.id === "gemini"), false);
    assert.equal(settings.providers.providers.some((provider) => provider.id === "gh-copilot"), false);
    assert.equal(settings.providers.providers.find((provider) => provider.id === "copilot")?.enabled, true);
    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.equal(persisted.providers.providers.some((provider: { id: string }) => provider.id === "gh-copilot"), false);
    assert.equal(persisted.providers.providers.some((provider: { id: string }) => provider.id === "gemini"), false);
    assert.equal(persisted.providers.providers.find((provider: { id: string }) => provider.id === "copilot")?.enabled, true);
    const firstPersistedValue = readFileSync(settingsPath(dir), "utf8");
    loadSettingsForStartup!(dir);
    assert.equal(readFileSync(settingsPath(dir), "utf8"), firstPersistedValue);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("startup migration preserves canonical Copilot when legacy duplicate exists", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 1,
          revision: 7,
          peonModel: null,
          ollamaBaseUrl: "http://127.0.0.1:11434",
          providers: [
            { id: "copilot", enabled: false, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
            { id: "gh-copilot", enabled: true, fallbackOrder: 5, defaultState: "unknown", overrideState: null },
          ],
        },
      }),
    );

    const loadSettingsForStartup = (settingsMemory as typeof settingsMemory & {
      loadSettingsForStartup?: (userDataPath: string) => typeof DEFAULT_SETTINGS;
    }).loadSettingsForStartup;
    const settings = loadSettingsForStartup!(dir);

    assert.deepEqual(
      settings.providers.providers.filter((provider) => provider.id === "copilot"),
      [{ id: "copilot", model: null, enabled: false, fallbackOrder: 2, defaultState: "unknown", overrideState: null }],
    );
    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.equal(persisted.providers.providers.filter((provider: { id: string }) => provider.id === "copilot").length, 1);
    assert.equal(persisted.providers.providers.some((provider: { id: string }) => provider.id === "gh-copilot"), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("startup migration removes a user-modified Gemini entry", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const raw = JSON.stringify({
      version: 1,
      providers: {
        version: 1,
        revision: 7,
        peonModel: null,
        ollamaBaseUrl: "http://127.0.0.1:11434",
        providers: [{ id: "gemini", enabled: true, fallbackOrder: 8, defaultState: "unknown", overrideState: null }],
      },
    });
    writeFileSync(settingsPath(dir), raw);

    const loadSettingsForStartup = (settingsMemory as typeof settingsMemory & {
      loadSettingsForStartup?: (userDataPath: string) => typeof DEFAULT_SETTINGS;
    }).loadSettingsForStartup;
    assert.equal(typeof loadSettingsForStartup, "function");
    const settings = loadSettingsForStartup!(dir);

    assert.equal(settings.providers.providers.some((provider) => provider.id === "gemini"), false);
    assert.notEqual(readFileSync(settingsPath(dir), "utf8"), raw);
    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.equal(persisted.providers.providers.some((provider: { id: string }) => provider.id === "gemini"), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("startup migration continues with repaired settings when persistence fails", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 1,
          revision: 7,
          peonModel: null,
          ollamaBaseUrl: "http://127.0.0.1:11434",
          providers: [{ id: "gemini", enabled: true, fallbackOrder: 8, defaultState: "unknown", overrideState: null }],
        },
      }),
    );
    let writes = 0;
    const loadSettingsForStartup = settingsMemory.loadSettingsForStartup as unknown as (
      userDataPath: string,
      persist: (path: string, settings: typeof DEFAULT_SETTINGS) => void,
    ) => typeof DEFAULT_SETTINGS;

    const settings = loadSettingsForStartup(dir, () => {
      writes += 1;
      throw new Error("disk full");
    });

    assert.equal(writes, 1);
    assert.equal(settings.providers.providers.some((provider) => provider.id === "gemini"), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory normalizes malformed provider payloads", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 99,
          revision: 4.7,
          providers: [
            { id: "claude-code", enabled: "yes", fallbackOrder: -10, defaultState: "bad", overrideState: "capped" },
            { id: "unknown-provider", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
          ],
        },
      }),
    );

    const settings = readSettings(dir);
    assert.equal(settings.providers.version, 2);
    assert.equal(settings.providers.revision, 4);
    assert.deepEqual(settings.providers.providers.map((entry) => entry.id), ["claude-code", "opencode", "codex", "aider", "copilot", "ollama"]);
    assert.equal(settings.providers.providers[0].enabled, true);
    assert.equal(settings.providers.providers[0].fallbackOrder, 0);
    assert.equal(settings.providers.providers[0].defaultState, "unknown");
    assert.equal(settings.providers.providers[0].overrideState, "capped");
    assert.equal(settings.providers.providers[0].model, null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory normalizes persisted provider models", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          providers: [
            { id: "claude-code", enabled: false, fallbackOrder: 4, defaultState: "healthy", overrideState: "capped", model: "  llama3  " },
            { id: "codex", model: "   " },
            { id: "aider", model: 42 },
          ],
        },
      }),
    );

    const settings = readSettings(dir);

    assert.deepEqual(settings.providers.providers.find((entry) => entry.id === "claude-code"), {
      id: "claude-code",
      enabled: false,
      fallbackOrder: 3,
      defaultState: "healthy",
      overrideState: "capped",
      model: "llama3",
    });
    assert.deepEqual(settings.providers.providers.find((entry) => entry.id === "codex"), {
      id: "codex",
      enabled: true,
      fallbackOrder: 1,
      defaultState: "unknown",
      overrideState: null,
      model: null,
    });
    assert.deepEqual(settings.providers.providers.find((entry) => entry.id === "aider"), {
      id: "aider",
      enabled: true,
      fallbackOrder: 2,
      defaultState: "unknown",
      overrideState: null,
      model: null,
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory preserves normalized provider model through save then read", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeSettings(dir, {
      ...DEFAULT_SETTINGS,
      providers: {
        ...DEFAULT_SETTINGS.providers,
        providers: DEFAULT_SETTINGS.providers.providers.map((entry) =>
          entry.id === "ollama" ? { ...entry, model: "  llama3  " } : entry,
        ),
      },
    });

    const settings = readSettings(dir);

    assert.deepEqual(settings.providers.providers.find((entry) => entry.id === "ollama"), {
      id: "ollama",
      enabled: true,
      fallbackOrder: 5,
      defaultState: "unknown",
      overrideState: null,
      model: "llama3",
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory keeps legacy top-level peonModel migration separate from provider models", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          peonModel: "legacy-model",
          providers: [{ id: "opencode", model: "provider-model" }],
        },
      }),
    );

    const settings = readSettings(dir);

    assert.equal(settings.providers.peonModel, "legacy-model");
    assert.equal(settings.providers.providers.find((entry) => entry.id === "opencode")?.model, "provider-model");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("provider settings migrate one unambiguous provider model to peonSelection", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 1,
          revision: 9,
          peonModel: null,
          ollamaBaseUrl: " http://127.0.0.1:11434/ ",
          providers: [
            { id: "copilot", model: "  gpt-5  ", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
            { id: "ollama", model: null, enabled: true, fallbackOrder: 1, defaultState: "unknown", overrideState: null },
          ],
        },
      }),
    );

    const settings = settingsMemory.loadSettingsForStartup!(dir);

    assert.deepEqual(settings.providers.peonSelection, {
      provider: "copilot",
      model: "gpt-5",
    });
    assert.equal(settings.providers.version, 2);
    assert.equal(settings.providers.revision, 9);
    assert.equal(settings.providers.ollamaBaseUrl, "http://127.0.0.1:11434");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("provider settings migration is idempotent and does not increment revision", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(
      settingsPath(dir),
      JSON.stringify({
        version: 1,
        providers: {
          version: 1,
          revision: 4,
          providers: [{ id: "ollama", model: " llama3.2:3b ", enabled: true, fallbackOrder: 0, defaultState: "unknown", overrideState: null }],
        },
      }),
    );

    settingsMemory.loadSettingsForStartup!(dir);
    const first = readFileSync(settingsPath(dir), "utf8");
    settingsMemory.loadSettingsForStartup!(dir);

    assert.equal(readFileSync(settingsPath(dir), "utf8"), first);
    assert.equal(JSON.parse(first).providers.revision, 4);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("startup migration writes normalized v2 selections only once and preserves provider fields", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeFileSync(settingsPath(dir), JSON.stringify({
      version: 1,
      providers: {
        version: 2,
        revision: 11,
        peonSelection: { provider: "copilot", model: "  gpt-5  " },
        ollamaBaseUrl: " http://127.0.0.1:11434/ ",
        providers: [{
          id: "copilot", model: "  gpt-5  ", enabled: false, fallbackOrder: 4,
          defaultState: "capped", overrideState: "degraded",
        }, {
          id: "ollama", model: null, enabled: true, fallbackOrder: 5,
          defaultState: "healthy", overrideState: "capped",
        }],
      },
    }));
    let writes = 0;
    const persist = (path: string, settings: typeof DEFAULT_SETTINGS) => {
      writes += 1;
      writeSettings(path, settings);
    };
    const first = settingsMemory.loadSettingsForStartup!(dir, persist);
    assert.equal(writes, 1);
    assert.deepEqual(first.providers.peonSelection, { provider: "copilot", model: "gpt-5" });
    assert.deepEqual(first.providers.providers.find(({ id }) => id === "copilot"), {
      id: "copilot", model: "gpt-5", enabled: false, fallbackOrder: 4,
      defaultState: "capped", overrideState: "degraded",
    });
    assert.deepEqual(first.providers.providers.find(({ id }) => id === "ollama"), {
      id: "ollama", model: null, enabled: true, fallbackOrder: 5,
      defaultState: "healthy", overrideState: "capped",
    });
    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.deepEqual(persisted.providers.providers.find(({ id }: { id: string }) => id === "ollama"), {
      id: "ollama", model: null, enabled: true, fallbackOrder: 5,
      defaultState: "healthy", overrideState: "capped",
    });
    settingsMemory.loadSettingsForStartup!(dir, persist);
    assert.equal(writes, 1);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Electron Ollama URL validation rejects paths, queries, and fragments like Rust", () => {
  const invalid = [
    "http://127.0.0.1:11434/api",
    "http://127.0.0.1:11434?x=1",
    "http://127.0.0.1:11434#fragment",
  ];
  for (const value of invalid) {
    const settings = normalizeProviderSettings({ version: 2, peonSelection: { provider: "ollama", model: "llama", ollamaBaseUrl: value } });
    assert.equal(settings.peonSelection, null, value);
  }
  const valid = normalizeProviderSettings({ version: 2, peonSelection: { provider: "ollama", model: "llama", ollamaBaseUrl: " HTTPS://LOCALHOST:11434/ " } });
  assert.deepEqual(valid.peonSelection, { provider: "ollama", model: "llama", ollamaBaseUrl: "https://localhost:11434" });
  const multipleTrailingSlashes = normalizeProviderSettings({ version: 2, peonSelection: { provider: "ollama", model: "llama", ollamaBaseUrl: "http://localhost:11434//" } });
  assert.deepEqual(multipleTrailingSlashes.peonSelection, { provider: "ollama", model: "llama", ollamaBaseUrl: "http://localhost:11434" });
});

test("provider settings do not migrate multiple provider models or a global-only model", () => {
  const multiple = normalizeProviderSettings({
    version: 1,
    revision: 2,
    peonModel: null,
    providers: [
      { id: "copilot", model: "one", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
      { id: "ollama", model: "two", enabled: true, fallbackOrder: 1, defaultState: "healthy", overrideState: null },
    ],
  });
  const globalOnly = normalizeProviderSettings({
    version: 1,
    revision: 3,
    peonModel: "global",
    providers: [{ id: "copilot", model: null, enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null }],
  });

  assert.equal(multiple.peonSelection, null);
  assert.equal(globalOnly.peonSelection, null);
});

test("provider settings ignore invalid and retired provider models during migration", () => {
  const settings = normalizeProviderSettings({
    version: 1,
    revision: 1,
    providers: [
      { id: "gemini", model: "retired", enabled: true, fallbackOrder: 0, defaultState: "healthy", overrideState: null },
      { id: "not-a-provider", model: "invalid", enabled: true, fallbackOrder: 1, defaultState: "healthy", overrideState: null },
    ],
  });

  assert.equal(settings.peonSelection, null);
  assert.equal(settings.providers.some(({ id }) => id === "gemini"), false);
  assert.equal(settings.providers.some(({ id }) => id === "not-a-provider"), false);
});

test("settings memory preserves provider revisions and canonical fallback order on write", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeSettings(dir, {
      ...DEFAULT_SETTINGS,
      providers: {
        version: 1,
        revision: 7,
        peonModel: "sonnet",
        providers: [
          { id: "claude-code", enabled: true, fallbackOrder: 9, defaultState: "healthy", overrideState: null, model: null },
          { id: "opencode", enabled: false, fallbackOrder: 2, defaultState: "capped", overrideState: null, model: null },
        ],
      },
    });

    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.equal(persisted.providers.revision, 7);
    assert.deepEqual(
      persisted.providers.providers.map((entry: { id: string; fallbackOrder: number }) => [entry.id, entry.fallbackOrder]),
      [["codex", 0], ["opencode", 1], ["aider", 2], ["copilot", 3], ["ollama", 4], ["claude-code", 5]],
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
