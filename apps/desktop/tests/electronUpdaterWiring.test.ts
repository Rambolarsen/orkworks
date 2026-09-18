import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import test from "node:test";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";
import type { UpdateDownloadedEvent } from "electron-updater";
import { channelForVersion, createUpdateService } from "../electron/updateService.ts";

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
  const names = ["require", "module", "exports", "process", "channelForVersion", ...Object.keys(injected)];
  const values = [localRequire, module, module.exports, process, channelForVersion, ...Object.values(injected)];
  new Function(...names, compiled)(...values);
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

function adapterFixture(platform = "win32", configure: (updater: any) => void = () => {}) {
  const application = new EventEmitter();
  let intercept: ((details: { url: string }, callback: (result: { cancel?: boolean }) => void) => void) | undefined;
  class Updater extends EventEmitter {
    autoDownload = true;
    autoInstallOnAppQuit = true;
    allowDowngrade = true;
    allowPrerelease = false;
    channel = "latest";
    requestHeaders = null;
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
    setFeedURL() {}
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
  const createAdapter = loadMainSnippet<any>(
    "function updateCandidate", "function registerUpdateIpc", "createElectronUpdateEngine",
    { createHash, app: application, process: { platform } },
  );
  const adapter = createAdapter(updater);
  return { updater, application, ...adapter };
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
  const { updater, service, effects, application } = await downloadedAdapter((updater) => {
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
  assert.equal(application.listenerCount("quit"), 0);
  assert.equal(updater.listenerCount("error"), 0);
});

test("direct adapter install also fails closed without touching native updater or quit listeners", { timeout: 2000 }, async () => {
  const { updater, application, engine } = adapterFixture();
  const installing = engine.quitAndInstall();
  const rejected = assert.rejects(installing, /Windows.*unavailable.*manually/i);
  await settle();
  assert.equal(updater.installs, 0);
  await rejected;
  await assert.rejects(engine.quitAndInstall(), /unavailable/i);
  assert.equal(application.listenerCount("quit"), 0);
  assert.equal(updater.listenerCount("error"), 0);
});

test("nightly adapter blocks stable metadata fallback before accepting a result", async () => {
  const { updater, engine } = adapterFixture();
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
    const { updater, engine } = adapterFixture();
    engine.channel = "nightly";
    updater.info.version = version;
    updater.info.tag = tag;
    const events: any[] = [];
    engine.onEvent((event: any) => events.push(event));
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
    const { updater, engine } = adapterFixture();
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
    const events: any[] = [];
    engine.onEvent((event: any) => events.push(event));
    await engine.checkForUpdates(1).catch(() => {});
    assert.ok(networkRequests.some((url) => url.endsWith("/nightly.yml")));
    assert.equal(networkRequests.some((url) => url.endsWith("/latest.yml")), false);
    assert.equal(events.some((event) => event.type === "update-available"), false);
    assert.equal(events.some((event) => event.type === "error"), true);
  }
});

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

test("main queries current sessions and routes update recovery through the workspace coordinator", () => {
  assert.match(main, /await restoration\.getReadiness\(\)[\s\S]*listSessions\(baseUrl\)/);
  assert.match(main, /lifecycle === "alive"/);
  assert.match(main, /workspaceSwitchCoordinator\.runUpdate\(/);
  assert.match(main, /sidecarLifecycle\.start\(restartCwd\)/);
  assert.match(main, /Restart OrkWorks to recover/);
});

test("ordinary before-quit cleanup does not invoke the installer", () => {
  const cleanup = main.slice(main.indexOf("function killSidecar"));
  assert.doesNotMatch(cleanup, /quitAndInstall/);
});
