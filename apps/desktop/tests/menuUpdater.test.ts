import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { DEFAULT_SETTINGS } from "../electron/settingsMemory.ts";
import { buildMenuTemplate, findMenuItem } from "../electron/menuTemplate.ts";

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");

test("Help menu exposes the shared update check command", () => {
  const commands: Array<{ action: string; panelId?: string }> = [];
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "linux",
    settings: DEFAULT_SETTINGS,
    sendCommand: (command) => commands.push(command),
  });
  const item = findMenuItem(template, "check-for-updates");

  assert.equal(item?.label, "Check for updates");
  item?.click?.({} as never, {} as never, {} as never);
  assert.deepEqual(commands, [{ action: "check-for-updates" }]);
  assert.match(app, /check-for-updates[\s\S]*checkForUpdates/);
});

test("update checks are suppressed during hotkey capture", () => {
  const commands: Array<{ action: string; panelId?: string }> = [];
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "linux",
    settings: DEFAULT_SETTINGS,
    isHotkeyCaptureActive: () => true,
    sendCommand: (command) => commands.push(command),
  });
  const item = findMenuItem(template, "check-for-updates");

  assert.equal(item?.enabled, false);
  item?.click?.({} as never, {} as never, {} as never);
  assert.deepEqual(commands, []);
});
