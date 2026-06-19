import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  DEFAULT_HOTKEYS,
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

test("validateHotkeys allows optional resetLayout to be unset", () => {
  const result = validateHotkeys({
    ...DEFAULT_HOTKEYS,
    resetLayout: null,
  });

  assert.deepEqual(result, { ok: true, errors: {} });
});
