import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  DEFAULT_HOTKEYS,
  DEFAULT_SETTINGS,
  readSettings,
  settingsPath,
  settingsWithHotkeys,
  validateHotkeys,
  writeSettings,
} from "../electron/settingsMemory.ts";
import type { ProviderSettings } from "../src/providerTypes.ts";

test("settings memory returns defaults when settings.json is missing", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    const settings = readSettings(dir);
    assert.equal(settings.version, 1);
    assert.deepEqual(settings.hotkeys, DEFAULT_HOTKEYS);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
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
      version: 1,
      revision: 0,
      providers: [
        {
          id: "opencode",
          enabled: true,
          fallbackOrder: 0,
          peonModel: null,
          defaultState: "healthy",
          overrideState: null,
        },
        {
          id: "claude-code",
          enabled: true,
          fallbackOrder: 1,
          peonModel: null,
          defaultState: "unknown",
          overrideState: null,
        },
      ],
    } satisfies ProviderSettings);
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
            { id: "claude-code", enabled: "yes", fallbackOrder: -10, peonModel: 42, defaultState: "bad", overrideState: "capped" },
            { id: "unknown-provider", enabled: true, fallbackOrder: 0, peonModel: null, defaultState: "healthy", overrideState: null },
          ],
        },
      }),
    );

    const settings = readSettings(dir);
    assert.equal(settings.providers.version, 1);
    assert.equal(settings.providers.revision, 4);
    assert.deepEqual(settings.providers.providers.map((entry) => entry.id), ["claude-code", "opencode"]);
    assert.equal(settings.providers.providers[0].enabled, true);
    assert.equal(settings.providers.providers[0].fallbackOrder, 0);
    assert.equal(settings.providers.providers[0].peonModel, null);
    assert.equal(settings.providers.providers[0].defaultState, "unknown");
    assert.equal(settings.providers.providers[0].overrideState, "capped");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("settings memory preserves provider revisions and canonical fallback order on write", () => {
  const dir = mkdtempSync(join(tmpdir(), "orkworks-settings-"));
  try {
    writeSettings(dir, {
      ...DEFAULT_SETTINGS,
      providers: {
        version: 1,
        revision: 7,
        providers: [
          { id: "claude-code", enabled: true, fallbackOrder: 9, peonModel: "sonnet", defaultState: "healthy", overrideState: null },
          { id: "opencode", enabled: false, fallbackOrder: 2, peonModel: null, defaultState: "capped", overrideState: null },
        ],
      },
    });

    const persisted = JSON.parse(readFileSync(settingsPath(dir), "utf8"));
    assert.equal(persisted.providers.revision, 7);
    assert.deepEqual(
      persisted.providers.providers.map((entry: { id: string; fallbackOrder: number }) => [entry.id, entry.fallbackOrder]),
      [["opencode", 0], ["claude-code", 1]],
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
