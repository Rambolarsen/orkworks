import test from "node:test";
import assert from "node:assert/strict";

import { DEFAULT_SETTINGS } from "../electron/settingsMemory.ts";
import { buildMenuTemplate, findMenuItem } from "../electron/menuTemplate.ts";

test("menu template uses accelerators from settings", () => {
  const template = buildMenuTemplate({
    openExternal: () => {},
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

test("shell menu entries keep their existing identities and act as destinations instead of panel checkboxes", () => {
  const template = buildMenuTemplate({
    openExternal: () => {},
    appName: "OrkWorks",
    platform: "darwin",
    settings: DEFAULT_SETTINGS,
    sendCommand: () => {},
  });

  assert.equal(findMenuItem(template, "capacity"), undefined);
  for (const id of ["sessions", "detail", "terminal", "recommendations"]) {
    const item = findMenuItem(template, id);
    assert.ok(item, `${id} remains available`);
    assert.notEqual(item.type, "checkbox");
    assert.equal(item.id, id);
  }
});

test("menu template omits optional reset layout accelerator when unset", () => {
  const template = buildMenuTemplate({
    openExternal: () => {},
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
    openExternal: () => {},
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
    openExternal: () => {},
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


test("Review menu routes to the validated shell destination without adding an accelerator", () => {
  const commands: Array<{ action: string; panelId?: string }> = [];
  const template = buildMenuTemplate({ appName: "OrkWorks", platform: "darwin", settings: DEFAULT_SETTINGS, openExternal: () => {},
    sendCommand: command => commands.push(command) });
  const review = findMenuItem(template, "review");
  assert.ok(review);
  assert.equal(review.accelerator, undefined);
  review.click?.({} as never, {} as never, {} as never);
  assert.deepEqual(commands, [{ action: "focus", panelId: "review" }]);
});

for (const capturing of [false, true]) {
  test(`Learn More opens the public site only outside hotkey capture (${capturing})`, () => {
    const opened: string[] = [];
    const commands: unknown[] = [];
    const template = buildMenuTemplate({ appName: "OrkWorks", platform: "darwin", settings: DEFAULT_SETTINGS,
      isHotkeyCaptureActive: () => capturing, openExternal: url => opened.push(url),
      sendCommand: command => commands.push(command) });
    const help = template.find(item => item.role === "help" || item.label === "Help");
    const learnMore = (help?.submenu as Array<{ label?: string; click?: (...args: unknown[]) => void }>).find(item => item.label === "Learn More");
    assert.ok(learnMore);
    learnMore.click?.();
    assert.deepEqual(opened, capturing ? [] : ["https://rambolarsen.github.io/orkworks/"]);
    assert.deepEqual(commands, []);
  });
}
