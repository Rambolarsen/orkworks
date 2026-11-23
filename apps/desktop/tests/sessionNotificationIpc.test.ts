import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { test } from "node:test";
import type { BrowserWindow, IpcMain, IpcMainInvokeEvent, NotificationConstructorOptions } from "electron";
import { installSessionNotificationIpc } from "../electron/sessionNotificationIpc.ts";

const alertId = "8bd295a3-f721-41f3-a35a-94cdaf5de67b";

function fixture() {
  const handlers = new Map<string, (event: IpcMainInvokeEvent, id: unknown) => unknown>();
  const ipc = { handle: (channel: string, handler: (event: IpcMainInvokeEvent, id: unknown) => unknown) => {
    handlers.set(channel, handler);
  } } as Pick<IpcMain, "handle">;
  const windowActions: string[] = [];
  const mainFrame = {};
  const webContents = { mainFrame };
  const window = {
    webContents,
    isDestroyed: () => false,
    isFocused: () => false,
    isMinimized: () => true,
    restore: () => windowActions.push("restore"),
    show: () => windowActions.push("show"),
    focus: () => windowActions.push("focus"),
  } as unknown as BrowserWindow;
  const event = { sender: webContents, senderFrame: mainFrame } as IpcMainInvokeEvent;
  const signal = Object.assign(new EventEmitter(), {
    show() {},
    close() { signal.emit("close"); },
  });
  const alerts: NotificationConstructorOptions[] = [];
  let enabled = true;
  const clear = installSessionNotificationIpc(() => window, () => enabled, {
    ipc,
    isSupported: () => true,
    create: (options) => {
      alerts.push(options);
      return signal;
    },
  });
  return {
    clear, signal, alerts, windowActions,
    disable() { enabled = false; },
    show: (sender = event) => handlers.get("show-session-attention-notification")!(sender, alertId),
  };
}

test("desktop alert activation restores and focuses the requesting window", async () => {
  const native = fixture();
  const result = native.show();
  assert.deepEqual(native.alerts, [{ title: "A session needs you", body: "Open OrkWorks to respond.", silent: true }]);
  native.signal.emit("click");
  assert.equal(await result, true);
  assert.deepEqual(native.windowActions, ["restore", "show", "focus"]);
});

test("closing alerts for renderer or workspace replacement makes later activation inert", async () => {
  const native = fixture();
  const result = native.show();
  native.clear();
  native.signal.emit("click");
  assert.equal(await result, false);
  assert.deepEqual(native.windowActions, []);
});

test("desktop alerts require opt-in and the main renderer frame", () => {
  const native = fixture();
  assert.equal(native.show({} as IpcMainInvokeEvent), false);
  native.disable();
  assert.equal(native.show(), false);
  assert.deepEqual(native.alerts, []);
});
