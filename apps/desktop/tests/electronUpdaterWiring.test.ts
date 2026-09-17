import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import test from "node:test";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";

const main = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");
const localRequire = createRequire(import.meta.url);

function loadMainSnippet<T>(
  startMarker: string,
  endMarker: string,
  exported: string,
  injected: Record<string, unknown> = {},
): T {
  const start = main.indexOf(startMarker);
  const end = main.indexOf(endMarker, start);
  assert.notEqual(start, -1, `missing start marker: ${startMarker}`);
  assert.notEqual(end, -1, `missing end marker: ${endMarker}`);

  const source = `${main.slice(start, end)}\nmodule.exports = ${exported};`;
  const compiled = transpileModule(source, {
    compilerOptions: { module: ModuleKind.CommonJS, target: ScriptTarget.ES2022 },
  }).outputText;
  const module = { exports: {} as T };
  const names = ["require", "module", "exports", "process", ...Object.keys(injected)];
  const values = [localRequire, module, module.exports, process, ...Object.values(injected)];
  new Function(...names, compiled)(...values);
  return module.exports;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => { resolve = settle; });
  return { promise, resolve };
}

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

test("main gates updater construction on app.isPackaged and keeps automatic install disabled", () => {
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*createUpdateService/);
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*registerUpdateIpc/);
  assert.match(main, /autoDownload:\s*false/);
  assert.match(main, /autoInstallOnAppQuit:\s*false/);
  assert.match(main, /allowDowngrade:\s*false/);
  assert.match(main, /stopAndWait\(10_000\)/);
});

test("main fixes the updater provider and maps public updater events", () => {
  assert.match(main, /provider:\s*"github"/);
  assert.match(main, /owner:\s*"Rambolarsen"/);
  assert.match(main, /repo:\s*"orkworks"/);
  for (const eventName of ["update-available", "update-not-available", "download-progress", "update-downloaded", "error"]) {
    assert.match(main, new RegExp(`autoUpdater\\.on\\(\\"${eventName}\\"`));
  }
});

test("a delayed event from an old updater operation cannot complete a newer operation", async () => {
  class FakeUpdater extends EventEmitter {
    autoDownload = true;
    autoInstallOnAppQuit = true;
    allowDowngrade = true;
    allowPrerelease = false;
    channel: string | null = null;
    checks: ReturnType<typeof deferred<unknown>>[] = [];

    setFeedURL() {}
    checkForUpdates() {
      const check = deferred<unknown>();
      this.checks.push(check);
      return check.promise;
    }
    downloadUpdate() { return Promise.resolve([]); }
    quitAndInstall() {}
  }

  const { createElectronUpdateEngine } = loadMainSnippet<{
    createElectronUpdateEngine(updater: FakeUpdater): {
      engine: {
        onEvent(listener: (event: Record<string, unknown>) => void): () => void;
        checkForUpdates(operationId: number): Promise<void>;
      };
    };
  }>(
    "function updateCandidate",
    "function registerUpdateIpc",
    "{ createElectronUpdateEngine }",
    { createHash },
  );
  const updater = new FakeUpdater();
  const { engine } = createElectronUpdateEngine(updater);
  const events: Record<string, unknown>[] = [];
  engine.onEvent((event) => events.push(event));
  const firstInfo = {
    version: "1.0.1",
    tag: "v1.0.1",
    files: [{ url: "OrkWorks.exe", sha512: "first" }],
    releaseDate: "2026-09-16T00:00:00Z",
    releaseNotes: null,
  };
  const secondInfo = { ...firstInfo, version: "1.0.2", tag: "v1.0.2", files: [{ url: "OrkWorks.exe", sha512: "second" }] };

  const first = engine.checkForUpdates(1);
  await settle();
  const delayedFirstListener = updater.listeners("update-available")[0] as (info: typeof firstInfo) => void;
  updater.emit("update-available", firstInfo);
  const second = engine.checkForUpdates(2);
  await settle();
  assert.equal(updater.checks.length, 1, "the singleton updater must not run overlapping operations");
  updater.checks[0].resolve({ isUpdateAvailable: true, updateInfo: firstInfo });
  await first;
  await settle();
  assert.equal(updater.checks.length, 2);
  const eventCountBeforeDelay = events.length;
  delayedFirstListener(firstInfo);
  assert.equal(events.length, eventCountBeforeDelay, "the completed operation's listener must be inert");

  updater.emit("update-available", secondInfo);
  updater.checks[1].resolve({ isUpdateAvailable: true, updateInfo: secondInfo });
  await second;
  assert.deepEqual(
    events.filter(({ type }) => type === "update-available").map(({ operationId, candidate }) => ({
      operationId,
      version: (candidate as { identity: { version: string } }).identity.version,
    })),
    [
      { operationId: 1, version: "1.0.1" },
      { operationId: 2, version: "1.0.2" },
    ],
  );
});

test("main registers only the narrow updater IPC contract", () => {
  for (const channel of ["get-update-status", "check-for-updates", "download-update", "request-update-install"]) {
    assert.match(main, new RegExp(`ipcMain\\.handle\\(\\"${channel}\\"`));
  }
  assert.match(main, /ipcMain\.on\("subscribe-update-status"/);
  assert.match(main, /updateSubscriptions\.get\(senderId\)\?\.\(\)/);
  assert.match(main, /event\.sender\.once\("destroyed"/);
});

test("resubscribing one renderer replaces its existing main-process subscription", () => {
  const ipcListeners = new Map<string, (event: { sender: FakeSender }) => void>();
  const ipcMain = {
    handle() {},
    on(channel: string, listener: (event: { sender: FakeSender }) => void) {
      ipcListeners.set(channel, listener);
    },
  };
  const subscribers = new Set<(status: { phase: string }) => void>();
  const service = {
    getStatus: () => ({ phase: "idle" }),
    check() {},
    download() {},
    requestInstall() {},
    subscribe(listener: (status: { phase: string }) => void) {
      subscribers.add(listener);
      listener({ phase: "idle" });
      return () => subscribers.delete(listener);
    },
  };
  class FakeSender extends EventEmitter {
    id = 42;
    deliveries: unknown[] = [];
    send(channel: string, status: unknown) { this.deliveries.push({ channel, status }); }
  }
  const { registerUpdateIpc } = loadMainSnippet<{
    registerUpdateIpc(service: typeof service): void;
  }>(
    "function registerUpdateIpc",
    "function rendererSettings",
    "{ registerUpdateIpc }",
    { ipcMain },
  );
  registerUpdateIpc(service);
  const subscribe = ipcListeners.get("subscribe-update-status");
  assert.ok(subscribe);
  const sender = new FakeSender();

  subscribe({ sender });
  assert.equal(subscribers.size, 1);
  sender.deliveries.length = 0;
  subscribe({ sender });
  assert.equal(subscribers.size, 1, "resubscribe must replace the previous forwarder");

  sender.deliveries.length = 0;
  for (const listener of subscribers) listener({ phase: "available" });
  assert.deepEqual(sender.deliveries, [{ channel: "update-status", status: { phase: "available" } }]);
  sender.emit("destroyed");
  assert.equal(subscribers.size, 0);
});

test("main queries current sessions and restarts through the existing lifecycle", () => {
  assert.match(main, /await restoration\.getReadiness\(\)[\s\S]*listSessions\(baseUrl\)/);
  assert.match(main, /lifecycle === "alive"/);
  assert.match(main, /sidecarLifecycle\.start\(restartCwd\)/);
  assert.match(main, /Restart OrkWorks to recover/);
});

test("ordinary before-quit cleanup does not invoke the installer", () => {
  const cleanup = main.slice(main.indexOf("function killSidecar"));
  assert.doesNotMatch(cleanup, /quitAndInstall/);
});
