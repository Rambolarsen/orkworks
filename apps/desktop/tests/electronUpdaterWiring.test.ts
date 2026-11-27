import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import test from "node:test";
import type { AppUpdater, UpdateDownloadedEvent } from "electron-updater";
import {
  createElectronUpdateEngine,
  registerUpdateIpc,
  type UpdateIpcSender,
} from "../electron/electronUpdateEngine.ts";
import { DEFAULT_SETTINGS } from "../electron/settingsMemory.ts";
import type { SidecarProcess } from "../electron/sidecarLifecycle.ts";
import { createUpdateService, type UpdateEngineEvent } from "../electron/updateService.ts";
import { createWorkspaceRuntime } from "../electron/workspaceRuntime.ts";

const main = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");
const localRequire = createRequire(import.meta.url);

// Execute the pinned dependency with only OS/process boundaries substituted.
function loadPinnedModule(id: string, injected: Record<string, unknown>) {
  const filename = localRequire.resolve(id);
  const moduleRequire = createRequire(filename);
  const module = { exports: {} as any };
  new Function("require", "module", "exports", readFileSync(filename, "utf8"))(
    (name: string) => name in injected ? injected[name] : moduleRequire(name), module, module.exports,
  );
  return module.exports;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => { resolve = settle; });
  return { promise, resolve };
}

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

function pinnedNsisFixture() {
  const effects: string[] = [];
  const { BaseUpdater } = loadPinnedModule("electron-updater/out/BaseUpdater.js", {
    electron: { autoUpdater: new EventEmitter() },
  });
  const { NsisUpdater } = loadPinnedModule("electron-updater/out/NsisUpdater.js", {
    "./BaseUpdater": { BaseUpdater },
  });
  const updater = new NsisUpdater(null, {
    version: "1.0.0", name: "OrkWorks", isPackaged: true,
    quit() { effects.push("quit"); },
  });
  updater.logger = { info() {}, warn() {}, error() {} };
  // Test-only cache/spawn boundary; exercise pinned NSIS/BaseUpdater control flow
  // without writing an executable, spawning it, or quitting the test process.
  updater.downloadedUpdateHelper = { file: "C:/cache/update.exe", downloadedFileInfo: {} };
  updater.spawnLog = async () => { effects.push("spawn"); throw new Error("asynchronous installer failure"); };
  updater.on("error", () => effects.push("error"));
  return { updater, effects };
}

function adapterFixture(platform: NodeJS.Platform = "win32", configure: (updater: any) => void = () => {}) {
  let intercept: ((details: { url: string }, callback: (result: { cancel?: boolean }) => void) => void) | undefined;
  class Updater extends EventEmitter {
    autoDownload = true;
    autoInstallOnAppQuit = true;
    allowDowngrade = true;
    allowPrerelease = false;
    disableWebInstaller = false;
    channel = "latest";
    requestHeaders: Record<string, string> | null = null;
    feedConfig: unknown = null;
    info = {
      version: "1.0.1", tag: "v1.0.1",
      path: "OrkWorks.exe", sha512: Buffer.alloc(64, 1).toString("base64"),
      files: [{ url: "OrkWorks.exe", sha512: Buffer.alloc(64, 1).toString("base64") }],
      releaseDate: "2026-09-17T00:00:00Z",
    };
    metadataFile: string | null = null;
    signatureResult: string | null = null;
    signatureWarning: string | null = null;
    logger = { info() {}, warn(_message: string) {}, error() {} };
    verifyOnDownload = true;
    publishers = ["OrkWorks publisher"];
    signatureCalls = 0;
    checks = 0;
    installs = 0;
    netSession = { webRequest: { onBeforeRequest: (_filter: unknown, listener: typeof intercept) => { intercept = listener; } } };
    metadataRequest(url: string) {
      let cancelled = false;
      intercept?.({ url }, ({ cancel }) => { cancelled = cancel === true; });
      if (cancelled) throw new Error("Metadata request blocked");
    }
    setFeedURL(config: unknown) { this.feedConfig = config; }
    verifyUpdateCodeSignature = async (_publishers: string[], _file: string) => {
      this.signatureCalls++;
      if (this.signatureWarning) this.logger.warn(this.signatureWarning);
      return this.signatureResult;
    };
    async checkForUpdates() {
      this.checks++;
      const url = `https://github.com/Rambolarsen/orkworks/releases/download/${this.info.tag}/${this.metadataFile ?? `${this.channel}.yml`}`;
      this.metadataRequest(url);
      this.emit("update-available", this.info);
      return { isUpdateAvailable: true, updateInfo: this.info };
    }
    async downloadUpdate() {
      if (this.verifyOnDownload) {
        const failure = await this.verifyUpdateCodeSignature(this.publishers, "C:/cache/update.exe");
        if (failure) throw new Error(failure);
      }
      const event: UpdateDownloadedEvent = { ...this.info, downloadedFile: "C:/cache/update.exe" };
      this.emit("update-downloaded", event);
      return ["C:/cache/update.exe"];
    }
    quitAndInstall() { this.installs++; }
  }
  const updater = new Updater();
  configure(updater);
  const adapter = createElectronUpdateEngine(updater as unknown as AppUpdater, { platform });
  return { updater, ...adapter };
}

async function downloadedAdapter(overrides: (updater: any) => void = () => {}) {
  const fixture = adapterFixture("win32", overrides);
  const effects: string[] = [];
  const service = createUpdateService({
    isPackaged: true, platform: "win32", currentVersion: "1.0.0",
    createEngine: () => fixture.engine, verifyCandidate: fixture.verifyCandidate,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => { effects.push("query"); return []; },
    confirmInstall: async () => { effects.push("confirm"); return true; },
    stopSidecar: async () => { effects.push("stop"); },
    restartSidecar: async () => { effects.push("restart"); },
  });
  await service.check();
  await service.download();
  return { ...fixture, service, effects };
}

test("engine configures safe defaults and fixes the GitHub provider", () => {
  const { updater } = adapterFixture("win32");
  assert.equal(updater.autoDownload, false);
  assert.equal(updater.autoInstallOnAppQuit, false);
  assert.equal(updater.allowDowngrade, false);
  assert.equal(updater.disableWebInstaller, true);
  assert.deepEqual(updater.requestHeaders, { "Cache-Control": "no-cache" });
  assert.deepEqual(updater.feedConfig, { provider: "github", owner: "Rambolarsen", repo: "orkworks" });
});

test("public downloaded event preserves release identity through fresh metadata verification", async () => {
  const { service, verifyCandidate, updater } = await downloadedAdapter();
  const status = service.getStatus();
  assert.equal(status.state, "downloaded");
  assert.ok("candidate" in status);
  assert.equal(await verifyCandidate(status.candidate), true);
  assert.equal(updater.checks, 2);
});

test("Windows verification rechecks current metadata and blocks a changed payload", async () => {
  const { updater, service, verifyCandidate, effects } = await downloadedAdapter();
  const status = service.getStatus();
  assert.equal(status.state, "downloaded");
  assert.ok("candidate" in status);
  updater.info = { ...updater.info, files: [{ url: "OrkWorks.exe", sha512: Buffer.alloc(64, 2).toString("base64") }] };
  assert.equal(await verifyCandidate(status.candidate), false);
  assert.equal(updater.checks, 2);
  assert.deepEqual(effects, []);
  assert.equal(updater.installs, 0);
});

test("Windows verification requires the real public signature verifier to complete", async () => {
  const { service, verifyCandidate, effects, updater } = await downloadedAdapter((updater) => { updater.verifyOnDownload = false; });
  const status = service.getStatus();
  assert.equal(status.state, "downloaded");
  assert.ok("candidate" in status);
  assert.equal(await verifyCandidate(status.candidate), false);
  assert.equal(updater.checks, 1);
  assert.deepEqual(effects, []);
  assert.equal(updater.installs, 0);
});

test("an empty publisher set cannot establish Windows verification", async () => {
  const { service, effects, updater } = await downloadedAdapter((updater) => { updater.publishers = []; });
  const status = service.getStatus();
  assert.equal(status.state, "error");
  assert.ok("message" in status);
  assert.match(status.message, /expected publisher/i);
  assert.equal(updater.signatureCalls, 0);
  assert.deepEqual(effects, []);
  assert.equal(updater.installs, 0);
});

test("signature rejection never shuts down or installs", async () => {
  const { service, effects, updater } = await downloadedAdapter((updater) => { updater.signatureResult = "invalid signature"; });
  const status = service.getStatus();
  assert.equal(status.state, "error");
  assert.ok("message" in status);
  assert.match(status.message, /invalid signature/);
  assert.deepEqual(effects, []);
  assert.equal(updater.installs, 0);
});

test("Windows verifier returning null after a warning cannot prove signature verification", { timeout: 2000 }, async () => {
  const { service, effects, updater } = await downloadedAdapter((updater) => {
    updater.signatureWarning = "Ignoring signature validation due to unsupported powershell version";
  });
  const status = service.getStatus();
  assert.equal(status.state, "error");
  assert.ok("message" in status);
  assert.match(status.message, /verification could not be established/i);
  assert.deepEqual(effects, []);
  assert.equal(updater.installs, 0);
});

for (const outcome of ["matching", "mismatch", "skipped", "missing-path"] as const) {
  test(`pinned Windows publisher verifier: ${outcome} SimpleName fixture`, async () => {
    const fixture = await downloadedAdapter((updater) => {
      const { verifySignature } = loadPinnedModule("electron-updater/out/windowsExecutableCodeSignatureVerifier.js", {
        os: { release: () => "10.0.22631" },
        child_process: {
          execFile(...args: any[]) {
            const callback = args.at(-1);
            queueMicrotask(() => callback(outcome === "skipped" ? new Error("PowerShell unavailable") : null,
              JSON.stringify({ Status: 0, Path: outcome === "missing-path" ? undefined : "C:/cache/update.exe",
                SignerCertificate: { Subject: `CN=${outcome === "mismatch" ? "Other publisher" : "OrkWorks publisher"}, O=OrkWorks` } }), ""));
          },
          execFileSync() { throw new Error("ConvertTo-Json unavailable"); },
        },
      });
      updater.verifyUpdateCodeSignature = (publishers: string[], file: string) => verifySignature(publishers, file, updater.logger);
    });
    const status = fixture.service.getStatus();
    if (outcome === "matching") {
      assert.equal(status.state, "downloaded");
      assert.ok("candidate" in status);
      assert.equal(await fixture.verifyCandidate(status.candidate), true);
    } else {
      assert.equal(status.state, "error");
      assert.ok("message" in status);
      assert.match(status.message, /signature|publisher/i);
    }
    assert.deepEqual(fixture.effects, []);
    assert.equal(fixture.updater.installs, 0);
  });
}

test("pinned NSIS schedules quit after async failure and ignores the first direct retry", async () => {
  const { updater, effects } = pinnedNsisFixture();
  updater.quitAndInstall();
  await settle();
  assert.deepEqual(effects, ["spawn", "error", "quit"]);
  updater.quitAndInstall();
  await settle();
  assert.deepEqual(effects, ["spawn", "error", "quit"], "no supported success/latch-reset handshake precedes retry");
});

test("Windows install fails closed before pinned NSIS can arm quit or an install latch", { timeout: 2000 }, async () => {
  const native = pinnedNsisFixture();
  const { updater, service, effects } = await downloadedAdapter((updater) => {
    updater.quitAndInstall = () => { updater.installs++; native.updater.quitAndInstall(); };
  });
  const installing = service.requestInstall();
  assert.strictEqual(service.requestInstall(), installing);
  await settle();
  assert.deepEqual(effects, [], "blocked before session query, confirmation, shutdown, or recovery");
  assert.deepEqual(native.effects, []);
  const failed = await installing;
  assert.equal(failed.state, "error");
  assert.match(failed.message, /Windows.*unavailable.*manually/i);
  const retry = service.requestInstall();
  assert.notStrictEqual(retry, installing);
  assert.equal((await retry).state, "error");
  assert.equal((await service.check()).state, "available", "no pending install promise blocks checks");
  assert.equal(updater.checks, 2, "blocked install must not run metadata revalidation");
  assert.equal(updater.installs, 0);
  assert.deepEqual(effects, []);
  assert.deepEqual(native.effects, []);
  assert.equal(updater.listenerCount("error"), 0);
});

test("direct adapter install also fails closed without touching native updater listeners", { timeout: 2000 }, async () => {
  const { updater, engine } = adapterFixture("win32");
  const installing = engine.quitAndInstall();
  const rejected = assert.rejects(installing, /Windows.*unavailable.*manually/i);
  await settle();
  assert.equal(updater.installs, 0);
  await rejected;
  await assert.rejects(engine.quitAndInstall(), /unavailable/i);
  assert.equal(updater.listenerCount("error"), 0);
});

test("nightly adapter blocks stable metadata fallback before accepting a result", async () => {
  const { updater, engine } = adapterFixture("win32");
  engine.channel = "nightly";
  updater.info.version = "1.0.1-nightly.20260917.123.1";
  updater.info.tag = `v${updater.info.version}`;
  updater.metadataFile = "latest.yml";
  await assert.rejects(engine.checkForUpdates(1), /metadata|channel/i);
});

test("adapter rejects mismatched tags and non-nightly versions labeled nightly", async () => {
  for (const [version, tag] of [
    ["1.0.1", "v1.0.1-nightly.1"],
    ["1.0.1-beta.1", "v1.0.1-beta.1"],
    ["1.0.1-nightly.2", "v1.0.1-nightly.1"],
  ]) {
    const { updater, engine } = adapterFixture("win32");
    engine.channel = "nightly";
    updater.info.version = version;
    updater.info.tag = tag;
    const events: UpdateEngineEvent[] = [];
    engine.onEvent((event) => events.push(event));
    await engine.checkForUpdates(1).catch(() => {});
    assert.equal(events.some((event) => event.type === "update-available"), false);
    assert.equal(events.some((event) => event.type === "error"), true);
  }
});

test("pinned GitHub provider cannot fetch stable fallback when nightly metadata is missing", async () => {
  const { GitHubProvider } = await import("electron-updater/out/providers/GitHubProvider.js");
  const updaterRequire = createRequire(localRequire.resolve("electron-updater/package.json"));
  const { SemVer } = updaterRequire("semver");
  for (const metadata of [null, "version: 1.0.1-nightly.2\ntag: v1.0.1-nightly.2\nfiles: []"]) {
    const { updater, engine } = adapterFixture("win32");
    engine.channel = "nightly";
    engine.allowPrerelease = true;
    const networkRequests: string[] = [];
    const provider = new GitHubProvider(
      { provider: "github", owner: "Rambolarsen", repo: "orkworks", channel: "nightly" },
      Object.assign(updater, { currentVersion: new SemVer("1.0.0-nightly.1"), fullChangelog: false }),
      { platform: "win32", isUseMultipleRangeRequest: false, executor: {
        async request(options: { path: string }) {
          updater.metadataRequest(`https://github.com${options.path}`);
          networkRequests.push(options.path);
          if (options.path.endsWith(".atom")) return '<feed><entry><title>nightly</title><link href="https://github.com/Rambolarsen/orkworks/releases/tag/v1.0.1-nightly.1"/><content>notes</content></entry></feed>';
          if (options.path.endsWith("/nightly.yml") && metadata !== null) return metadata;
          throw new Error("404 missing nightly metadata");
        },
      } } as any,
    );
    updater.checkForUpdates = async () => {
      const info = await provider.getLatestVersion();
      updater.emit("update-available", info);
      return { isUpdateAvailable: true, updateInfo: info } as any;
    };
    const events: UpdateEngineEvent[] = [];
    engine.onEvent((event) => events.push(event));
    await engine.checkForUpdates(1).catch(() => {});
    assert.ok(networkRequests.some((url) => url.endsWith("/nightly.yml")));
    assert.equal(networkRequests.some((url) => url.endsWith("/latest.yml")), false);
    assert.equal(events.some((event) => event.type === "update-available"), false);
    assert.equal(events.some((event) => event.type === "error"), true);
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
    intercept: ((details: { url: string }, callback: (result: unknown) => void) => void) | undefined;
    netSession = { webRequest: { onBeforeRequest: (_filter: unknown, listener: typeof this.intercept) => { this.intercept = listener; } } };
    requestMetadata(tag: string) {
      this.intercept?.({ url: `https://github.com/Rambolarsen/orkworks/releases/download/${tag}/latest.yml` }, () => {});
    }
    setFeedURL() {}
    checkForUpdates() {
      const check = deferred<unknown>();
      this.checks.push(check);
      return check.promise;
    }
    downloadUpdate() { return Promise.resolve([]); }
    quitAndInstall() {}
  }

  const updater = new FakeUpdater();
  const { engine } = createElectronUpdateEngine(updater as unknown as AppUpdater, { platform: "win32" });
  const events: UpdateEngineEvent[] = [];
  engine.onEvent((event) => events.push(event));
  const firstInfo = {
    version: "1.0.1",
    tag: "v1.0.1",
    files: [{ url: "OrkWorks.exe", sha512: Buffer.alloc(64, 1).toString("base64") }],
    releaseDate: "2026-09-16T00:00:00Z",
    releaseNotes: null,
  };
  const secondInfo = { ...firstInfo, version: "1.0.2", tag: "v1.0.2", files: [{ url: "OrkWorks.exe", sha512: Buffer.alloc(64, 2).toString("base64") }] };

  const first = engine.checkForUpdates(1);
  await settle();
  const delayedFirstListener = updater.listeners("update-available")[0] as (info: typeof firstInfo) => void;
  updater.requestMetadata(firstInfo.tag);
  updater.emit("update-available", firstInfo);
  const second = engine.checkForUpdates(2);
  await settle();
  assert.equal(updater.checks.length, 1, "the singleton updater must not run overlapping operations");
  updater.checks[0].resolve({ isUpdateAvailable: true, updateInfo: firstInfo });
  await first;
  await settle();
  assert.equal(updater.checks.length, 2);
  updater.requestMetadata(secondInfo.tag);
  const eventCountBeforeDelay = events.length;
  delayedFirstListener(firstInfo);
  assert.equal(events.length, eventCountBeforeDelay, "the completed operation's listener must be inert");

  updater.emit("update-available", secondInfo);
  updater.checks[1].resolve({ isUpdateAvailable: true, updateInfo: secondInfo });
  await second;
  assert.deepEqual(
    events.filter((event): event is Extract<UpdateEngineEvent, { type: "update-available" }> => event.type === "update-available")
      .map(({ operationId, candidate }) => ({ operationId, version: candidate.identity.version })),
    [
      { operationId: 1, version: "1.0.1" },
      { operationId: 2, version: "1.0.2" },
    ],
  );
});

test("registerUpdateIpc exposes only the narrow IPC contract and replaces per-sender subscriptions", async () => {
  const handles = new Map<string, (...args: unknown[]) => unknown>();
  const ipcListeners = new Map<string, (event: { sender: UpdateIpcSender }) => void>();
  const ipc = {
    handle(channel: string, listener: (...args: unknown[]) => unknown) {
      handles.set(channel, listener);
    },
    on(channel: string, listener: (event: { sender: UpdateIpcSender }) => void) {
      ipcListeners.set(channel, listener);
    },
  };
  const subscribers = new Set<(status: unknown) => void>();
  const calls: string[] = [];
  const service = {
    getStatus: () => { calls.push("getStatus"); return { state: "never-checked" }; },
    check: async () => { calls.push("check"); return { state: "up-to-date" }; },
    download: async () => { calls.push("download"); return { state: "downloaded" }; },
    requestInstall: async () => { calls.push("requestInstall"); return { state: "error" }; },
    subscribe(listener: (status: unknown) => void) {
      subscribers.add(listener);
      listener({ state: "never-checked" });
      return () => { subscribers.delete(listener); };
    },
  };
  class FakeSender extends EventEmitter implements UpdateIpcSender {
    id = 42;
    deliveries: unknown[] = [];
    send(channel: string, status: unknown) { this.deliveries.push({ channel, status }); }
  }

  registerUpdateIpc(service as any, ipc);
  assert.deepEqual(
    [...handles.keys()].sort(),
    ["check-for-updates", "download-update", "get-update-status", "request-update-install"],
  );
  assert.deepEqual([...ipcListeners.keys()], ["subscribe-update-status"]);

  handles.get("get-update-status")?.();
  await handles.get("check-for-updates")?.();
  await handles.get("download-update")?.();
  await handles.get("request-update-install")?.();
  assert.deepEqual(calls, ["getStatus", "check", "download", "requestInstall"]);

  const subscribe = ipcListeners.get("subscribe-update-status")!;
  const sender = new FakeSender();
  subscribe({ sender });
  assert.equal(subscribers.size, 1);
  sender.deliveries.length = 0;
  subscribe({ sender });
  assert.equal(subscribers.size, 1, "resubscribe must replace the previous forwarder");

  sender.deliveries.length = 0;
  for (const listener of subscribers) listener({ state: "available" });
  assert.deepEqual(sender.deliveries, [{ channel: "update-status", status: { state: "available" } }]);
  sender.emit("destroyed");
  assert.equal(subscribers.size, 0);
});

test("macOS adapter expects -mac.yml metadata and .zip payload and fails closed on install", async () => {
  const { updater, engine } = adapterFixture("darwin", (u) => {
    u.metadataFile = "latest-mac.yml";
    u.info = {
      version: "1.0.1",
      tag: "v1.0.1",
      path: "OrkWorks.zip",
      sha512: Buffer.alloc(64, 3).toString("base64"),
      files: [{ url: "OrkWorks.zip", sha512: Buffer.alloc(64, 3).toString("base64") }],
      releaseDate: "2026-09-17T00:00:00Z",
    };
  });
  const events: UpdateEngineEvent[] = [];
  engine.onEvent((event) => events.push(event));
  await engine.checkForUpdates(1);
  assert.equal(events.some((event) => event.type === "update-available"), true);
  assert.match(engine.installationUnavailableReason ?? "", /unavailable.*manually/i);
  await assert.rejects(engine.quitAndInstall(), /unavailable.*manually/i);
  assert.equal(updater.installs, 0);
});

test("WorkspaceRuntime serializes update install and restarts the sidecar in the active or fallback cwd", async () => {
  const spawnedCwds: string[] = [];
  const procs: Array<EventEmitter & SidecarProcess & { emitReady(port: number): void; emitExit(): void }> = [];
  let failNextStart = false;

  const runtime = createWorkspaceRuntime({
    spawnSidecar: (cwd) => {
      spawnedCwds.push(cwd);
      const proc = Object.assign(new EventEmitter(), {
        stdout: new EventEmitter(),
        stderr: new EventEmitter(),
        pid: 9000 + procs.length,
        killed: false,
        kill() {
          proc.killed = true;
          queueMicrotask(() => proc.emit("exit", 0, null));
          return true;
        },
        emitReady(port: number) {
          proc.stdout.emit("data", Buffer.from(`ORKWORKSD_PORT=${port}\n`));
        },
        emitExit() {
          proc.emit("exit", 1, null);
        },
      });
      procs.push(proc);
      if (failNextStart) {
        queueMicrotask(() => proc.emitExit());
      }
      return proc;
    },
    sidecarSend: async (_port, endpoint) => {
      if (endpoint === "workspace") {
        return new Response(JSON.stringify({
          path: "/ws-update",
          workspaceIdentity: "/ws-update",
          repo_root: null,
          branch: null,
          dirty: false,
          lastActiveSessionId: null,
          activeHarnessIds: [],
          activeHarnessRevision: 0,
        }), { status: 200 });
      }
      if (endpoint === "providers") return new Response(JSON.stringify({ providers: [] }), { status: 200 });
      return new Response(null, { status: 204 });
    },
    fetch: async () => new Response(JSON.stringify({ ok: true }), { status: 200 }),
    validateDestination: (candidate) => candidate,
    canonicalizePath: (candidate) => candidate,
    rememberWorkspacePath: () => ({ diagnostic: null }),
    forgetWorkspacePath: async () => ({ historyDiagnostic: null }),
    getSettings: () => structuredClone(DEFAULT_SETTINGS),
    writeSettings: () => {},
    enqueueSettingsWrite: (operation) => Promise.resolve(operation()),
    clearProviderModelsCache: () => {},
    initialHistoryDiagnostic: null,
    logger: { log() {}, warn() {}, error() {} },
  });

  // Open a workspace, then run an update: runUpdate closes the active workspace
  // before invoking the callback, so restartSidecarAfterUpdate falls back to home.
  const opening = runtime.openWorkspace("/ws-update");
  await settle();
  procs[0].emitReady(4100);
  assert.equal((await opening).ok, true);

  await runtime.runUpdate(async (currentPath) => {
    assert.equal(currentPath, "/ws-update");
    await runtime.stopSidecarForUpdate(10_000);
    const restarting = runtime.restartSidecarAfterUpdate("/fallback-home");
    await settle();
    procs[1].emitReady(4101);
    await restarting;
  });
  assert.deepEqual(spawnedCwds, ["/ws-update", "/fallback-home"]);

  // Failed restart surfaces the explicit recovery error.
  failNextStart = true;
  await runtime.runUpdate(async () => {
    await runtime.stopSidecarForUpdate(10_000);
    await assert.rejects(
      runtime.restartSidecarAfterUpdate("/fallback-home"),
      /Restart OrkWorks to recover/,
    );
  });
  runtime.dispose();
});

test("main gates updater construction on app.isPackaged and before-quit does not invoke quitAndInstall", () => {
  assert.match(main, /if \(app\.isPackaged\)[\s\S]*createElectronUpdateEngine[\s\S]*createUpdateService[\s\S]*registerUpdateIpc\(updateService, ipcMain\)/);
  assert.match(main, /stopSidecarForUpdate\(10_000\)/);
  assert.match(main, /restartSidecarAfterUpdate\(/);
  assert.match(main, /runtime\.runUpdate\(/);
  const cleanup = main.slice(main.indexOf("function killSidecar"));
  assert.doesNotMatch(cleanup, /quitAndInstall/);
});
