import test from "node:test";
import assert from "node:assert/strict";

import { DEFAULT_SETTINGS } from "../electron/settingsMemory.ts";
import { buildMenuTemplate, findMenuItem } from "../electron/menuTemplate.ts";

test("menu template uses accelerators from settings", () => {
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "darwin",
    settings: {
      version: 1,
      hotkeys: {
        ...DEFAULT_SETTINGS.hotkeys,
        newSession: "CmdOrCtrl+Alt+N",
        toggleTerminalPanel: "CmdOrCtrl+Alt+T",
        resetLayout: "CmdOrCtrl+Alt+Backspace",
      },
    },
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "new-session")?.accelerator, "CmdOrCtrl+Alt+N");
  assert.equal(findMenuItem(template, "terminal")?.accelerator, "CmdOrCtrl+Alt+T");
  assert.equal(findMenuItem(template, "reset-layout")?.accelerator, "CmdOrCtrl+Alt+Backspace");
});

test("menu template omits optional reset layout accelerator when unset", () => {
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "darwin",
    settings: DEFAULT_SETTINGS,
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "reset-layout")?.accelerator, undefined);
});

test("menu command handlers are suppressible during hotkey capture", () => {
  const commands: Array<{ action: string; panelId?: string }> = [];
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "linux",
    settings: DEFAULT_SETTINGS,
    isHotkeyCaptureActive: () => true,
    sendCommand: (command) => commands.push(command),
  });

  findMenuItem(template, "new-session")?.click?.({} as never, {} as never, {} as never);
  findMenuItem(template, "sessions")?.click?.({} as never, {} as never, {} as never);

  assert.deepEqual(commands, []);
});

test("menu template removes accelerators and native roles during hotkey capture", () => {
  const template = buildMenuTemplate({
    appName: "OrkWorks",
    platform: "darwin",
    settings: {
      version: 1,
      hotkeys: {
        ...DEFAULT_SETTINGS.hotkeys,
        resetLayout: "CmdOrCtrl+Alt+Backspace",
      },
    },
    isHotkeyCaptureActive: () => true,
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "new-session")?.accelerator, undefined);
  assert.equal(findMenuItem(template, "sessions")?.accelerator, undefined);
  assert.equal(findMenuItem(template, "reset-layout")?.accelerator, undefined);
  assert.equal(findMenuItem(template, "sessions")?.enabled, false);
  assert.equal(findMenuItem(template, "detail")?.enabled, false);

  const serializedTemplate = JSON.stringify(template);
  assert.equal(serializedTemplate.includes('"role"'), false);
});
