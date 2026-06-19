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
  validateHotkeys,
  writeSettings,
} from "../electron/settingsMemory.ts";

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

test("validateHotkeys allows optional resetLayout to be unset", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    resetLayout: null,
  });

  assert.deepEqual(result, { ok: true, errors: {} });
});
