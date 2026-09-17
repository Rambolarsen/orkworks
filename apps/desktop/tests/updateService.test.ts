import assert from "node:assert/strict";
import test from "node:test";
import {
  createUpdateService,
  type UpdateCandidate,
  type UpdateEngine,
  type UpdateEngineEvent,
} from "../electron/updateService.ts";

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

type EngineFixture = UpdateEngine & {
  events: Array<(event: UpdateEngineEvent) => void>;
  calls: string[];
  checkOperationIds: number[];
  downloadOperationIds: number[];
};

function engineFixture(): EngineFixture {
  const events: Array<(event: UpdateEngineEvent) => void> = [];
  const calls: string[] = [];
  const checkOperationIds: number[] = [];
  const downloadOperationIds: number[] = [];
  return {
    autoDownload: true,
    autoInstallOnAppQuit: true,
    allowDowngrade: true,
    allowPrerelease: false,
    channel: "latest",
    events,
    calls,
    checkOperationIds,
    downloadOperationIds,
    onEvent(listener) { events.push(listener); return () => events.splice(events.indexOf(listener), 1); },
    async checkForUpdates(operationId) { calls.push("check"); checkOperationIds.push(operationId); },
    async downloadUpdate(operationId) { calls.push("download"); downloadOperationIds.push(operationId); },
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
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  await checking;
  const downloading = service.download();
  engine.events[0]({
    type: "download-progress",
    operationId: engine.downloadOperationIds[0],
    percent: 50,
    transferred: 1,
    total: 2,
  });
  await downloading;
  unsubscribe();
  assert.equal(seen[0], 0);
  assert.deepEqual(seen.slice(1), [1, 2, 3, 4]);
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
  for (const currentVersion of [
    "0.2.0-beta.1",
    "0.2.0-nightly.preview",
    "not-semver",
    "01.2.3",
    "1.2.3+foo..bar",
    "1.2.3-nightly.01",
  ]) {
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

test("nightly builds ignore stable candidates", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const checking = service.check();
  const before = service.getStatus();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate({ channel: "latest", version: "0.2.0", tag: "v0.2.0" }),
  });
  assert.strictEqual(service.getStatus(), before);
  await checking;
});

test("duplicate checks and downloads share the active operation promise", async () => {
  const engine = engineFixture();
  const check = deferred();
  const download = deferred();
  engine.checkForUpdates = (operationId) => {
    engine.calls.push("check");
    engine.checkOperationIds.push(operationId);
    return check.promise;
  };
  engine.downloadUpdate = (operationId) => {
    engine.calls.push("download");
    engine.downloadOperationIds.push(operationId);
    return download.promise;
  };
  const service = packagedService(engine);

  const firstCheck = service.check();
  assert.strictEqual(service.check(), firstCheck);
  assert.deepEqual(engine.calls, ["check"]);
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  check.resolve();
  await firstCheck;

  const firstDownload = service.download();
  assert.strictEqual(service.download(), firstDownload);
  assert.deepEqual(engine.calls, ["check", "download"]);
  engine.events[0]({
    type: "update-downloaded",
    operationId: engine.downloadOperationIds[0],
    candidate: candidate(),
  });
  download.resolve();
  await firstDownload;
});

test("duplicate install requests coalesce without invoking the installer", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  await checking;
  const downloading = service.download();
  engine.events[0]({
    type: "update-downloaded",
    operationId: engine.downloadOperationIds[0],
    candidate: candidate(),
  });
  await downloading;
  const callsBeforeInstall = [...engine.calls];

  const firstInstall = service.requestInstall();
  assert.strictEqual(service.requestInstall(), firstInstall);
  assert.equal((await firstInstall).state, "downloaded");
  assert.deepEqual(engine.calls, callsBeforeInstall);
});

test("progress from an older download cannot overwrite a newer check", async () => {
  const engine = engineFixture();
  const download = deferred();
  engine.downloadUpdate = (operationId) => {
    engine.downloadOperationIds.push(operationId);
    return download.promise;
  };
  const service = packagedService(engine);
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  await checking;
  const pendingDownload = service.download();
  const staleDownloadOperationId = engine.downloadOperationIds[0];

  await service.check();
  const checked = service.getStatus();
  assert.equal(checked.state, "up-to-date");
  engine.events[0]({
    type: "download-progress",
    operationId: staleDownloadOperationId,
    percent: 75,
    transferred: 3,
    total: 4,
  });
  assert.strictEqual(service.getStatus(), checked);

  download.resolve();
  await pendingDownload;
});

test("a stale update-available event cannot overwrite a newer check", async () => {
  const engine = engineFixture();
  const checks = [deferred(), deferred()];
  engine.checkForUpdates = (operationId) => {
    engine.checkOperationIds.push(operationId);
    return checks[engine.checkOperationIds.length - 1].promise;
  };
  const service = packagedService(engine);

  const first = service.check();
  const staleOperationId = engine.checkOperationIds[0];
  checks[0].resolve();
  await first;

  const second = service.check();
  const currentOperationId = engine.checkOperationIds[1];
  const checking = service.getStatus();
  engine.events[0]({ type: "update-available", operationId: staleOperationId, candidate: candidate() });
  assert.strictEqual(service.getStatus(), checking);

  engine.events[0]({ type: "update-available", operationId: currentOperationId, candidate: candidate() });
  assert.equal(service.getStatus().state, "available");
  checks[1].resolve();
  await second;
});

test("downloaded events require the active operation and exact candidate identity", async () => {
  const engine = engineFixture();
  const downloads = [deferred(), deferred()];
  engine.downloadUpdate = (operationId) => {
    engine.downloadOperationIds.push(operationId);
    return downloads[engine.downloadOperationIds.length - 1].promise;
  };
  const service = packagedService(engine);
  const expectedCandidate = candidate();
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: expectedCandidate,
  });
  await checking;

  const first = service.download();
  const staleOperationId = engine.downloadOperationIds[0];
  downloads[0].resolve();
  await first;
  const second = service.download();
  const currentOperationId = engine.downloadOperationIds[1];
  const downloading = service.getStatus();

  engine.events[0]({ type: "update-downloaded", operationId: staleOperationId, candidate: expectedCandidate });
  assert.strictEqual(service.getStatus(), downloading);
  for (const overrides of [
    { channel: "latest" as const },
    { version: "0.2.0-nightly.20260918.1" },
    { tag: "v0.2.0-nightly.20260918.1" },
    { metadataUrl: "https://example.invalid/latest.yml" },
    { metadataDigest: "sha256:other" },
    { payloadDigest: "sha512:other" },
  ]) {
    engine.events[0]({
      type: "update-downloaded",
      operationId: currentOperationId,
      candidate: candidate(overrides),
    });
    assert.strictEqual(service.getStatus(), downloading);
  }

  engine.events[0]({ type: "update-downloaded", operationId: currentOperationId, candidate: expectedCandidate });
  assert.equal(service.getStatus().state, "downloaded");
  downloads[1].resolve();
  await second;
});

test("reentrant subscribers share the already-installed check and download promises", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  let reenteredCheck: Promise<unknown> | undefined;
  let didReenterCheck = false;
  const unsubscribeCheck = service.subscribe((status) => {
    if (status.state === "checking" && !didReenterCheck) {
      didReenterCheck = true;
      reenteredCheck = service.check();
    }
  });

  const checking = service.check();
  assert.strictEqual(reenteredCheck, checking);
  assert.deepEqual(engine.calls, ["check"]);
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  await checking;
  unsubscribeCheck();

  let reenteredDownload: Promise<unknown> | undefined;
  let didReenterDownload = false;
  const unsubscribeDownload = service.subscribe((status) => {
    if (status.state === "downloading" && !didReenterDownload) {
      didReenterDownload = true;
      reenteredDownload = service.download();
    }
  });
  const downloading = service.download();
  assert.strictEqual(reenteredDownload, downloading);
  assert.deepEqual(engine.calls, ["check", "download"]);
  await downloading;
  unsubscribeDownload();
});

test("a synchronous check throw clears the shared promise for retry", async () => {
  const engine = engineFixture();
  engine.checkForUpdates = () => {
    engine.calls.push("check");
    throw new Error("synchronous check failure");
  };
  const service = packagedService(engine);

  assert.equal((await service.check()).state, "error");
  assert.equal((await service.check()).state, "error");
  assert.deepEqual(engine.calls, ["check", "check"]);
});

test("a synchronous download throw clears the shared promise for retry", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds[0],
    candidate: candidate(),
  });
  await checking;
  engine.downloadUpdate = () => {
    engine.calls.push("download");
    throw new Error("synchronous download failure");
  };

  assert.equal((await service.download()).state, "error");
  assert.equal((await service.download()).state, "error");
  assert.deepEqual(engine.calls, ["check", "download", "download"]);
});

test("updater errors are retryable", async () => {
  const engine = engineFixture();
  engine.checkForUpdates = async (operationId) => {
    engine.calls.push("check");
    engine.checkOperationIds.push(operationId);
    engine.events[0]({ type: "error", operation: "check", operationId, message: "offline" });
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
