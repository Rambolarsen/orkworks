import assert from "node:assert/strict";
import test from "node:test";
import { createUpdateService, type UpdateCandidate, type UpdateEngine } from "../electron/updateService.ts";

const candidate = (overrides: Partial<UpdateCandidate["identity"]> = {}): UpdateCandidate => ({
  identity: {
    channel: "nightly",
    version: "0.2.0-nightly.20260917.1",
    tag: "v0.2.0-nightly.20260917.1",
    metadataUrl: "https://github.com/Rambolarsen/orkworks/releases/download/v0.2.0-nightly.20260917.1/latest.yml",
    metadataDigest: "sha256:metadata-1",
    payloadDigest: "sha512:payload-1",
    ...overrides,
  },
  releaseNotes: "nightly notes",
  publishedAt: "2026-09-17T00:00:00Z",
});

function engineFixture(): UpdateEngine & { events: Array<(event: any) => void>; calls: string[] } {
  const events: Array<(event: any) => void> = [];
  const calls: string[] = [];
  return {
    autoDownload: true,
    autoInstallOnAppQuit: true,
    allowDowngrade: true,
    allowPrerelease: false,
    channel: "latest",
    events,
    calls,
    onEvent(listener) { events.push(listener); return () => events.splice(events.indexOf(listener), 1); },
    async checkForUpdates() { calls.push("check"); },
    async downloadUpdate() { calls.push("download"); },
    quitAndInstall() { calls.push("install"); },
  };
}

function packagedService(engine: UpdateEngine, currentVersion = "0.2.0-nightly.20260916.1") {
  return createUpdateService({
    isPackaged: true,
    currentVersion,
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
}

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve = () => undefined;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

test("development construction never creates an updater and is unavailable", async () => {
  let constructed = false;
  const service = createUpdateService({
    isPackaged: false,
    currentVersion: "0.2.0",
    createEngine: () => { constructed = true; throw new Error("must not construct"); },
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  assert.equal(constructed, false);
  assert.equal(service.getStatus().state, "unavailable");
  assert.equal((await service.check()).state, "unavailable");
});

test("nightly status is replayed first and then only increasing sequences are delivered", async () => {
  const engine = engineFixture();
  const service = createUpdateService({
    isPackaged: true,
    currentVersion: "0.2.0-nightly.20260916.1",
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  const seen: number[] = [];
  const unsubscribe = service.subscribe((status) => seen.push(status.sequence));
  engine.events[0]({ type: "update-available", candidate: candidate() });
  engine.events[0]({ type: "download-progress", percent: 50, transferred: 1, total: 2 });
  unsubscribe();
  assert.equal(seen[0], 0);
  assert.deepEqual(seen.slice(1), [1, 2]);
  assert.ok(seen.every((value, index) => index === 0 || value > seen[index - 1]));
});

test("channel selection admits exact nightly prereleases and never falls back to stable", () => {
  const engine = engineFixture();
  createUpdateService({
    isPackaged: true,
    currentVersion: "0.2.0-nightly.20260916.1",
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
  });
  assert.equal(engine.channel, "nightly");
  assert.equal(engine.allowPrerelease, true);
});

test("stable builds use latest with every automatic updater behavior disabled", () => {
  const engine = engineFixture();
  packagedService(engine, "0.2.0");
  assert.deepEqual(
    {
      autoDownload: engine.autoDownload,
      autoInstallOnAppQuit: engine.autoInstallOnAppQuit,
      allowDowngrade: engine.allowDowngrade,
      allowPrerelease: engine.allowPrerelease,
      channel: engine.channel,
    },
    {
      autoDownload: false,
      autoInstallOnAppQuit: false,
      allowDowngrade: false,
      allowPrerelease: false,
      channel: "latest",
    },
  );
});

test("malformed and other prerelease versions are unavailable without constructing an engine", () => {
  for (const currentVersion of ["0.2.0-beta.1", "0.2.0-nightly.preview", "not-semver"]) {
    let constructed = false;
    const service = createUpdateService({
      isPackaged: true,
      currentVersion,
      createEngine: () => { constructed = true; return engineFixture(); },
      now: () => "2026-09-17T00:00:00Z",
      querySessions: async () => [],
      verifyCandidate: async () => true,
      confirmInstall: async () => true,
      stopSidecar: async () => undefined,
      restartSidecar: async () => undefined,
    });
    assert.equal(constructed, false);
    assert.deepEqual(service.getStatus(), { state: "unavailable", reason: "unsupported-version", sequence: 0 });
  }
});

test("nightly builds ignore stable candidates", () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const before = service.getStatus();
  engine.events[0]({
    type: "update-available",
    candidate: candidate({ channel: "latest", version: "0.2.0", tag: "v0.2.0" }),
  });
  assert.strictEqual(service.getStatus(), before);
});

test("duplicate checks and downloads share the active operation promise", async () => {
  const engine = engineFixture();
  const check = deferred();
  const download = deferred();
  engine.checkForUpdates = () => { engine.calls.push("check"); return check.promise; };
  engine.downloadUpdate = () => { engine.calls.push("download"); return download.promise; };
  const service = packagedService(engine);

  const firstCheck = service.check();
  assert.strictEqual(service.check(), firstCheck);
  assert.deepEqual(engine.calls, ["check"]);
  engine.events[0]({ type: "update-available", candidate: candidate() });
  check.resolve();
  await firstCheck;

  const firstDownload = service.download();
  assert.strictEqual(service.download(), firstDownload);
  assert.deepEqual(engine.calls, ["check", "download"]);
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });
  download.resolve();
  await firstDownload;
});

test("duplicate install requests coalesce without invoking the installer", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  engine.events[0]({ type: "update-downloaded", candidate: candidate() });

  const firstInstall = service.requestInstall();
  assert.strictEqual(service.requestInstall(), firstInstall);
  assert.equal((await firstInstall).state, "downloaded");
  assert.deepEqual(engine.calls, []);
});

test("progress from an older download cannot overwrite a newer check", async () => {
  const engine = engineFixture();
  const download = deferred();
  engine.downloadUpdate = () => download.promise;
  const service = packagedService(engine);
  engine.events[0]({ type: "update-available", candidate: candidate() });
  const pendingDownload = service.download();

  await service.check();
  const checked = service.getStatus();
  assert.equal(checked.state, "up-to-date");
  engine.events[0]({ type: "download-progress", percent: 75, transferred: 3, total: 4 });
  assert.strictEqual(service.getStatus(), checked);

  download.resolve();
  await pendingDownload;
});

test("updater errors are retryable", async () => {
  const engine = engineFixture();
  engine.checkForUpdates = async () => {
    engine.calls.push("check");
    engine.events[0]({ type: "error", operation: "check", message: "offline" });
  };
  const service = packagedService(engine);

  const failed = await service.check();
  assert.deepEqual(failed, {
    state: "error",
    operation: "check",
    message: "offline",
    retryable: true,
    sequence: 2,
  });
  assert.equal((await service.check()).state, "error");
  assert.deepEqual(engine.calls, ["check", "check"]);
});

test("a completed check with no candidate is up to date", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const status = await service.check();
  assert.deepEqual(status, {
    state: "up-to-date",
    channel: "nightly",
    currentVersion: "0.2.0-nightly.20260916.1",
    checkedAt: "2026-09-17T00:00:00Z",
    sequence: 2,
  });
});
