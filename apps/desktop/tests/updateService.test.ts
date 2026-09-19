import assert from "node:assert/strict";
import test from "node:test";
import {
  createUpdateService,
  type UpdateCandidate,
  type UpdateEngine,
  type UpdateEngineEvent,
  type UpdateService,
  type UpdateServiceDependencies,
} from "../electron/updateService.ts";

const candidate = (overrides: Partial<UpdateCandidate["identity"]> = {}): UpdateCandidate => ({
  identity: {
    channel: "nightly",
    version: "0.2.0-nightly.20260917.123.1",
    tag: "v0.2.0-nightly.20260917.123.1",
    metadataUrl: "https://github.com/Rambolarsen/orkworks/releases/download/v0.2.0-nightly.20260917.123.1/latest.yml",
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
    installationUnavailableReason: null,
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
    async quitAndInstall() { calls.push("install"); },
  };
}

function packagedService(
  engine: UpdateEngine,
  overrides: Partial<UpdateServiceDependencies> | string = {},
) {
  const dependencyOverrides = typeof overrides === "string"
    ? { currentVersion: overrides }
    : overrides;
  return createUpdateService({
    isPackaged: true,
    platform: "win32",
    currentVersion: dependencyOverrides.currentVersion ?? "0.2.0-nightly.20260916.123.1",
    createEngine: () => engine,
    now: () => "2026-09-17T00:00:00Z",
    querySessions: async () => [],
    verifyCandidate: async () => true,
    confirmInstall: async () => true,
    stopSidecar: async () => undefined,
    restartSidecar: async () => undefined,
    ...dependencyOverrides,
  });
}

async function prepareDownloaded(
  service: UpdateService,
  engine: EngineFixture,
  availableCandidate = candidate(),
  downloadedCandidate = candidate(),
): Promise<void> {
  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds.at(-1)!,
    candidate: availableCandidate,
  });
  await checking;
  const downloading = service.download();
  engine.events[0]({
    type: "update-downloaded",
    operationId: engine.downloadOperationIds.at(-1)!,
    candidate: downloadedCandidate,
  });
  await downloading;
}

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve = () => undefined;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

test("macOS installation is blocked before verification, sessions, confirmation, or shutdown", async () => {
  const engine = engineFixture();
  const sideEffects: string[] = [];
  const service = packagedService(engine, {
    platform: "darwin",
    verifyCandidate: async () => { sideEffects.push("verify"); return true; },
    querySessions: async () => { sideEffects.push("query"); return []; },
    confirmInstall: async () => { sideEffects.push("confirm"); return true; },
    stopSidecar: async () => { sideEffects.push("stop"); },
    restartSidecar: async () => { sideEffects.push("restart"); },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;
  for (let attempt = 0; attempt < 2; attempt++) {
    const status = await service.requestInstall();
    assert.equal(status.state, "error");
    assert.equal(status.retryable, true);
    assert.match(status.message, /macOS.*verification/i);
  }
  assert.deepEqual(sideEffects, []);
  assert.deepEqual(engine.calls, []);
});

test("asynchronous installer failure owns the transaction through exactly one recovery", async () => {
  const engine = engineFixture();
  let rejectInstall!: (error: Error) => void;
  engine.quitAndInstall = () => {
    engine.calls.push("install");
    return new Promise<void>((_resolve, reject) => { rejectInstall = reject; });
  };
  const recovery = deferred();
  let recoveries = 0;
  const service = packagedService(engine, {
    restartSidecar: () => { recoveries++; return recovery.promise; },
  });
  await prepareDownloaded(service, engine);
  const installing = service.requestInstall();
  await settle();
  assert.strictEqual(service.requestInstall(), installing);
  assert.strictEqual(service.check(), installing);
  rejectInstall(new Error("asynchronous installer failure"));
  await settle();
  assert.equal(recoveries, 1);
  assert.strictEqual(service.requestInstall(), installing);
  recovery.resolve();
  const status = await installing;
  assert.equal(status.state, "error");
  assert.match(status.message, /asynchronous installer failure/);
  assert.equal(recoveries, 1);
  assert.equal(engine.calls.filter((call) => call === "install").length, 1);
});

test("download during a refreshed check cannot steal candidate or operation ownership", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  await prepareDownloaded(service, engine);
  const refreshed = deferred();
  engine.checkForUpdates = (id) => { engine.checkOperationIds.push(id); return refreshed.promise; };
  const checking = service.check();
  const downloading = service.download();
  const next = candidate({ version: "0.2.0-nightly.20260918.124.1", tag: "v0.2.0-nightly.20260918.124.1" });
  engine.events[0]({ type: "update-available", operationId: engine.checkOperationIds.at(-1)!, candidate: next });
  refreshed.resolve();
  await Promise.all([checking, downloading]);
  assert.equal(service.getStatus().state, "available");
  assert.equal(service.getStatus().candidate.identity.version, next.identity.version);
  assert.equal(engine.calls.filter((call) => call === "download").length, 1);
});

test("a download that settles without a matching completion reports a retryable failure", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  await prepareDownloaded(service, engine);
  const status = await service.download();
  assert.equal(status.state, "error");
  assert.equal(status.operation, "download");
});

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

test("Linux packaged builds stay unavailable without constructing an updater", () => {
  let constructed = false;
  const service = createUpdateService({
    isPackaged: true,
    platform: "linux",
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
  assert.deepEqual(service.getStatus(), { state: "unavailable", reason: "unsupported-platform", sequence: 0 });
});

test("nightly status is replayed first and then only increasing sequences are delivered", async () => {
  const engine = engineFixture();
  const service = createUpdateService({
    isPackaged: true,
    platform: "win32",
    currentVersion: "0.2.0-nightly.20260916.123.1",
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
  assert.deepEqual(seen.slice(1), [1, 2, 3, 4, 5]);
  assert.ok(seen.every((value, index) => index === 0 || value > seen[index - 1]));
});

test("replayed available, downloaded, and error snapshots preserve the installed version", async () => {
  const engine = engineFixture();
  const service = packagedService(engine);
  const replay = () => {
    let snapshot = service.getStatus();
    const unsubscribe = service.subscribe((status) => { snapshot = status; });
    unsubscribe();
    return snapshot as typeof snapshot & { currentVersion?: string };
  };

  const checking = service.check();
  engine.events[0]({
    type: "update-available",
    operationId: engine.checkOperationIds.at(-1)!,
    candidate: candidate(),
  });
  await checking;
  assert.equal(replay().state, "available");
  assert.equal(replay().currentVersion, "0.2.0-nightly.20260916.123.1");

  const downloading = service.download();
  engine.events[0]({
    type: "update-downloaded",
    operationId: engine.downloadOperationIds.at(-1)!,
    candidate: candidate(),
  });
  await downloading;
  assert.equal(replay().state, "downloaded");
  assert.equal(replay().currentVersion, "0.2.0-nightly.20260916.123.1");

  engine.downloadUpdate = async () => { throw new Error("offline"); };
  await service.download();
  assert.equal(replay().state, "error");
  assert.equal(replay().currentVersion, "0.2.0-nightly.20260916.123.1");
});

test("channel selection admits exact nightly prereleases and never falls back to stable", () => {
  const engine = engineFixture();
  createUpdateService({
    isPackaged: true,
    platform: "win32",
    currentVersion: "0.2.0-nightly.20260916.123.1",
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
      platform: "win32",
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

test("noncanonical nightly identities are unavailable without constructing an engine", () => {
  for (const currentVersion of [
    "0.2.0-nightly.20260916.1",
    "0.2.0-nightly.20260916.123",
    "0.2.0-nightly.20260916.123.0",
    "0.2.0-nightly.20260916.123.100",
    "0.2.0-nightly.20261301.123.1",
  ]) {
    let constructed = false;
    const service = createUpdateService({
      isPackaged: true,
      platform: "win32",
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

test("nightly service rejects candidates with a false channel label or mismatched tag", async () => {
  for (const [version, tag] of [
    ["0.3.0", "v0.3.0"],
    ["0.3.0-beta.1", "v0.3.0-beta.1"],
    ["0.3.0-nightly.2", "v0.3.0-nightly.1"],
  ]) {
    const engine = engineFixture();
    engine.checkForUpdates = async (operationId) => {
      engine.events[0]({ type: "update-available", operationId, candidate: candidate({ version, tag }) });
    };
    const service = packagedService(engine);
    assert.equal((await service.check()).state, "error");
    await service.download();
    assert.deepEqual(engine.calls, []);
  }
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

test("duplicate install requests share one transaction and installer invocation", async () => {
  const engine = engineFixture();
  let finishVerification!: (verified: boolean) => void;
  const verification = new Promise<boolean>((resolve) => { finishVerification = resolve; });
  const service = packagedService(engine, {
    verifyCandidate: () => verification,
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const firstInstall = service.requestInstall();
  assert.strictEqual(service.requestInstall(), firstInstall);
  assert.deepEqual(engine.calls, []);
  finishVerification(true);
  assert.equal((await firstInstall).state, "installing");
  assert.deepEqual(engine.calls, ["install"]);
});

test("install verifies, queries sessions, confirms, waits, then installs", async () => {
  const order: string[] = [];
  const engine = engineFixture();
  const stopped = deferred();
  engine.quitAndInstall = () => { order.push("install"); };
  const service = packagedService(engine, {
    verifyCandidate: async () => { order.push("verify"); return true; },
    querySessions: async () => {
      order.push("sessions");
      return [{ lifecycle: "alive" }, { lifecycle: "ended" }];
    },
    confirmInstall: async ({ liveSessionCount }) => {
      order.push(`confirm:${liveSessionCount}`);
      return true;
    },
    stopSidecar: (timeoutMs) => {
      order.push(`stop:${timeoutMs}`);
      return stopped.promise;
    },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const installing = service.requestInstall();
  await settle();
  assert.deepEqual(order, ["verify", "sessions", "confirm:1", "stop:10000"]);

  stopped.resolve();
  const status = await installing;
  assert.deepEqual(order, ["verify", "sessions", "confirm:1", "stop:10000", "install"]);
  assert.equal(status.state, "installing");
});

test("invalid verification never queries sessions, stops the sidecar, or installs", async () => {
  const engine = engineFixture();
  let queriedSessions = false;
  let stopped = 0;
  let restarted = 0;
  const service = packagedService(engine, {
    verifyCandidate: async () => false,
    querySessions: async () => { queriedSessions = true; return []; },
    stopSidecar: async () => { stopped += 1; },
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const status = await service.requestInstall();

  assert.equal(status.state, "error");
  assert.equal(queriedSessions, false);
  assert.equal(stopped, 0);
  assert.equal(restarted, 0);
  assert.deepEqual(engine.calls, []);
});

test("cancelled confirmation has no shutdown or install side effect", async () => {
  const engine = engineFixture();
  let confirmed = false;
  let stopped = 0;
  let restarted = 0;
  const service = packagedService(engine, {
    confirmInstall: async () => { confirmed = true; return false; },
    stopSidecar: async () => { stopped += 1; },
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);
  const downloaded = service.getStatus();
  assert.equal(downloaded.state, "downloaded");
  engine.calls.length = 0;

  const status = await service.requestInstall();

  assert.equal(status.state, "downloaded");
  assert.strictEqual(status.candidate, downloaded.candidate);
  assert.equal(confirmed, true);
  assert.equal(stopped, 0);
  assert.equal(restarted, 0);
  assert.deepEqual(engine.calls, []);
});

test("confirmation rejection has no side effects and remains retryable", async () => {
  const engine = engineFixture();
  let confirmationCalls = 0;
  let verificationCalls = 0;
  let stopped = 0;
  let restarted = 0;
  const service = packagedService(engine, {
    verifyCandidate: async () => { verificationCalls += 1; return true; },
    confirmInstall: async () => {
      confirmationCalls += 1;
      if (confirmationCalls === 1) throw new Error("dialog unavailable");
      return true;
    },
    stopSidecar: async () => { stopped += 1; },
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const status = await service.requestInstall();

  assert.equal(status.state, "error");
  assert.equal(stopped, 0);
  assert.equal(restarted, 0);
  assert.deepEqual(engine.calls, []);

  const retried = await service.requestInstall();
  assert.equal(retried.state, "installing");
  assert.equal(verificationCalls, 2);
  assert.equal(confirmationCalls, 2);
  assert.equal(stopped, 1);
  assert.equal(restarted, 0);
  assert.deepEqual(engine.calls, ["install"]);
});

test("installer failure attempts sidecar recovery and reports restart guidance if recovery fails", async () => {
  const engine = engineFixture();
  engine.quitAndInstall = () => { throw new Error("installer failed"); };
  let restarted = 0;
  const service = packagedService(engine, {
    stopSidecar: async () => undefined,
    restartSidecar: async () => { restarted += 1; throw new Error("recovery failed"); },
  });
  await prepareDownloaded(service, engine);

  const status = await service.requestInstall();

  assert.equal(restarted, 1);
  assert.equal(status.state, "error");
  assert.match(status.message, /restart OrkWorks/i);
});

test("each candidate identity field is revalidated before verification or shutdown", async () => {
  const mismatches: Array<(value: UpdateCandidate) => void> = [
    (value) => { value.identity.channel = "latest"; },
    (value) => { value.identity.version = "0.2.0-nightly.20260918.124.1"; },
    (value) => { value.identity.tag = "v0.2.0-nightly.20260918.124.1"; },
    (value) => { value.identity.metadataUrl = "https://example.invalid/latest.yml"; },
    (value) => { value.identity.metadataDigest = "sha256:other"; },
    (value) => { value.identity.payloadDigest = "sha512:other"; },
  ];

  for (const mismatch of mismatches) {
    const engine = engineFixture();
    let verified = false;
    let stopped = false;
    const downloadedCandidate = candidate();
    const service = packagedService(engine, {
      verifyCandidate: async () => { verified = true; return true; },
      stopSidecar: async () => { stopped = true; },
    });
    await prepareDownloaded(service, engine, candidate(), downloadedCandidate);
    engine.calls.length = 0;
    mismatch(downloadedCandidate);

    const status = await service.requestInstall();

    assert.equal(status.state, "error");
    assert.strictEqual(await service.requestInstall(), status);
    assert.equal(verified, false);
    assert.equal(stopped, false);
    assert.deepEqual(engine.calls, []);
  }
});

test("verification errors never query sessions, stop the sidecar, or install", async () => {
  const engine = engineFixture();
  let queriedSessions = false;
  let stopped = 0;
  let restarted = 0;
  const service = packagedService(engine, {
    verifyCandidate: async () => { throw new Error("signature unavailable"); },
    querySessions: async () => { queriedSessions = true; return []; },
    stopSidecar: async () => { stopped += 1; },
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const status = await service.requestInstall();

  assert.equal(status.state, "error");
  assert.equal(queriedSessions, false);
  assert.equal(stopped, 0);
  assert.equal(restarted, 0);
  assert.deepEqual(engine.calls, []);
});

test("a synchronous verification throw preserves the shared install promise for reentrant subscribers", async () => {
  const engine = engineFixture();
  let verificationCalls = 0;
  const service = packagedService(engine, {
    verifyCandidate: () => {
      verificationCalls += 1;
      throw new Error("synchronous verification failure");
    },
  });
  await prepareDownloaded(service, engine);
  let reentered: Promise<unknown> | undefined;
  const unsubscribe = service.subscribe((status) => {
    if (status.state === "error" && status.operation === "install") {
      reentered = service.requestInstall();
    }
  });

  const installing = service.requestInstall();
  await installing;
  unsubscribe();

  assert.strictEqual(reentered, installing);
  assert.equal(verificationCalls, 1);
});

test("unknown session state is confirmed explicitly before installation", async () => {
  const engine = engineFixture();
  let confirmedSessionCount: number | null | undefined;
  const service = packagedService(engine, {
    querySessions: async () => { throw new Error("backend unavailable"); },
    confirmInstall: async ({ liveSessionCount }) => {
      confirmedSessionCount = liveSessionCount;
      return true;
    },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  await service.requestInstall();

  assert.equal(confirmedSessionCount, null);
  assert.deepEqual(engine.calls, ["install"]);
});

test("stale updater events cannot overwrite an in-flight install", async () => {
  const engine = engineFixture();
  const stopped = deferred();
  const service = packagedService(engine, {
    stopSidecar: () => stopped.promise,
  });
  await prepareDownloaded(service, engine);
  const staleOperationId = engine.downloadOperationIds.at(-1)!;
  engine.calls.length = 0;

  const installing = service.requestInstall();
  await settle();
  assert.equal(service.getStatus().state, "installing");
  engine.events[0]({
    type: "error",
    operation: "download",
    operationId: staleOperationId,
    message: "late error",
  });
  engine.events[0]({
    type: "update-downloaded",
    operationId: staleOperationId,
    candidate: candidate({ version: "0.2.0-nightly.20260918.124.1" }),
  });
  assert.equal(service.getStatus().state, "installing");

  stopped.resolve();
  await installing;
  assert.deepEqual(engine.calls, ["install"]);
});

test("shutdown failure skips installation and reports successful sidecar recovery", async () => {
  const engine = engineFixture();
  let restarted = 0;
  const service = packagedService(engine, {
    stopSidecar: async () => { throw new Error("shutdown timed out"); },
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);
  engine.calls.length = 0;

  const status = await service.requestInstall();

  assert.equal(restarted, 1);
  assert.equal(status.state, "error");
  assert.match(status.message, /backend was restarted/i);
  assert.doesNotMatch(status.message, /restart OrkWorks/i);
  assert.deepEqual(engine.calls, []);
});

test("installer failure reports successful sidecar recovery without restart guidance", async () => {
  const engine = engineFixture();
  engine.quitAndInstall = () => { throw new Error("installer failed"); };
  let restarted = 0;
  const service = packagedService(engine, {
    restartSidecar: async () => { restarted += 1; },
  });
  await prepareDownloaded(service, engine);

  const status = await service.requestInstall();

  assert.equal(restarted, 1);
  assert.equal(status.state, "error");
  assert.match(status.message, /backend was restarted/i);
  assert.doesNotMatch(status.message, /restart OrkWorks/i);
});

test("a retryable install error starts a new verification and install transaction", async () => {
  const engine = engineFixture();
  let installCalls = 0;
  engine.quitAndInstall = () => {
    installCalls += 1;
    if (installCalls === 1) throw new Error("installer failed");
  };
  let verificationCalls = 0;
  let restartCalls = 0;
  const service = packagedService(engine, {
    verifyCandidate: async () => { verificationCalls += 1; return true; },
    restartSidecar: async () => { restartCalls += 1; },
  });
  await prepareDownloaded(service, engine);

  const failed = await service.requestInstall();
  assert.equal(failed.state, "error");
  assert.equal(failed.operation, "install");

  const retried = await service.requestInstall();

  assert.equal(retried.state, "installing");
  assert.equal(verificationCalls, 2);
  assert.equal(installCalls, 2);
  assert.equal(restartCalls, 1);
});

test("install without a downloaded candidate has no verification, shutdown, or install side effect", async () => {
  const engine = engineFixture();
  let verified = false;
  let stopped = false;
  const service = packagedService(engine, {
    verifyCandidate: async () => { verified = true; return true; },
    stopSidecar: async () => { stopped = true; },
  });

  const status = await service.requestInstall();

  assert.equal(status.state, "never-checked");
  assert.equal(verified, false);
  assert.equal(stopped, false);
  assert.deepEqual(engine.calls, []);
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
    { version: "0.2.0-nightly.20260918.124.1" },
    { tag: "v0.2.0-nightly.20260918.124.1" },
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
    currentVersion: "0.2.0-nightly.20260916.123.1",
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
    currentVersion: "0.2.0-nightly.20260916.123.1",
    checkedAt: "2026-09-17T00:00:00Z",
    sequence: 2,
  });
});
