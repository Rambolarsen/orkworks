import assert from "node:assert/strict";
import test from "node:test";

import {
  createWorkspaceSwitchCoordinator,
  WorkspaceSwitchError,
  type WorkspaceInstanceState,
  type WorkspaceSwitchEvent,
} from "../electron/workspaceSwitchCoordinator.ts";
import { SidecarCleanupError } from "../electron/sidecarLifecycle.ts";

type Workspace = { path: string };

function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function createHarness(options: {
  start?: (path: string, generation: number) => Promise<{ workspace: Workspace; port: number }>;
  close?: () => Promise<void>;
  onCloseAdmission?: () => void;
  cleanupAttempted?: () => Promise<void>;
  remember?: (path: string, workspace: Workspace) => void;
  initialWorkspacePath?: string | null;
  onQuit?: () => void;
} = {}) {
  const actions: string[] = [];
  const events: Array<WorkspaceSwitchEvent<Workspace>> = [];
  let currentPath = options.initialWorkspacePath === undefined ? "/current" : options.initialWorkspacePath;
  const validPaths = new Set(["/current", "/next", "/third"]);

  const coordinator = createWorkspaceSwitchCoordinator<Workspace>({
    initialWorkspacePath: currentPath,
    validateDestination: (path) => validPaths.has(path) ? path : null,
    setWorkspacePath: (path) => {
      currentPath = path;
      actions.push(`path:${path ?? "none"}`);
    },
    closeCurrentRuntime: async () => {
      actions.push("close-current");
      await options.close?.();
    },
    onCloseAdmission: options.onCloseAdmission,
    startRuntime: options.start ?? (async (path) => {
      actions.push(`start:${path}`);
      return { workspace: { path }, port: 4000 };
    }),
    cleanupAttemptedRuntime: async () => {
      actions.push("cleanup-attempted");
      await options.cleanupAttempted?.();
    },
    rememberWorkspace: (path, workspace) => {
      actions.push(`history:${path}`);
      options.remember?.(path, workspace);
    },
    publish: (event) => events.push(event),
    onQuit: options.onQuit,
  });

  return { coordinator, actions, events, getCurrentPath: () => currentPath };
}

test("validation failure preserves the current workspace", async () => {
  const harness = createHarness();

  const result = await harness.coordinator.switchWorkspace("/missing");

  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.failure.code, "invalid_destination");
  assert.equal(result.state, "ready");
  assert.equal(harness.getCurrentPath(), "/current");
  assert.deepEqual(harness.actions, []);
  assert.deepEqual(harness.events, []);
});

test("cleanup failure produces unresolved without starting the destination", async () => {
  const harness = createHarness({
    close: async () => {
      throw new Error("owned runtime did not exit");
    },
  });

  const result = await harness.coordinator.switchWorkspace("/next");

  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.state, "unresolved");
  assert.equal(result.failure.code, "cleanup_failed");
  assert.deepEqual(harness.actions, ["close-current"]);
  assert.deepEqual(harness.events.map((event) => event.state), ["closing", "unresolved"]);
});

test("cleanup timeout remains unresolved and blocks destination startup until the old runtime exits", async () => {
  let oldRuntimeExited = false;
  const harness = createHarness({
    close: async () => {
      if (!oldRuntimeExited) {
        throw new SidecarCleanupError("cleanup_timeout", "Sidecar cleanup timed out");
      }
    },
  });

  const failed = await harness.coordinator.switchWorkspace("/next");

  assert.equal(failed.ok, false);
  if (failed.ok) return;
  assert.equal(failed.state, "unresolved");
  assert.equal(failed.failure.code, "cleanup_timeout");
  assert.deepEqual(harness.actions, ["close-current"]);

  const blockedRetry = await harness.coordinator.retry();

  assert.equal(blockedRetry.ok, false);
  if (blockedRetry.ok) return;
  assert.equal(blockedRetry.state, "unresolved");
  assert.equal(blockedRetry.failure.code, "cleanup_timeout");
  assert.equal(harness.actions.includes("start:/next"), false);

  oldRuntimeExited = true;
  const recovered = await harness.coordinator.retry();

  assert.equal(recovered.ok, true);
  assert.equal(harness.actions.at(-1), "path:/next");
  assert.equal(harness.coordinator.getState(), "ready");
});

test("close admission invalidates external guards before delayed cleanup resolves", async () => {
  const closeStarted = deferred<void>();
  const cleanup = deferred<void>();
  let backendGeneration = 0;
  const harness = createHarness({
    onCloseAdmission: () => {
      backendGeneration += 1;
    },
    close: async () => {
      closeStarted.resolve();
      await cleanup.promise;
    },
  });

  const oldGeneration = backendGeneration;
  const switching = harness.coordinator.switchWorkspace("/next");
  await closeStarted.promise;

  assert.notEqual(backendGeneration, oldGeneration);
  cleanup.resolve();
  await switching;
});

test("destination lease conflict enters picker after the old runtime is closed", async () => {
  const harness = createHarness({
    start: async () => {
      harness.actions.push("start:/next");
      throw new WorkspaceSwitchError("destination_conflict", "Workspace is already open");
    },
  });

  const result = await harness.coordinator.switchWorkspace("/next");

  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.state, "picker");
  assert.equal(result.failure.code, "destination_conflict");
  assert.equal(harness.getCurrentPath(), null);
  assert.deepEqual(harness.actions, ["close-current", "path:none", "start:/next", "cleanup-attempted"]);
  assert.deepEqual(harness.events.map((event) => event.state), ["closing", "picker", "opening", "picker"]);
});

test("readiness failure cleans only the attempted runtime", async () => {
  const harness = createHarness({
    start: async () => {
      harness.actions.push("start:/next");
      throw new WorkspaceSwitchError("readiness_failed", "Sidecar did not become ready");
    },
  });

  const result = await harness.coordinator.switchWorkspace("/next");

  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.failure.code, "readiness_failed");
  assert.deepEqual(harness.actions, ["close-current", "path:none", "start:/next", "cleanup-attempted"]);
  assert.equal(harness.actions.filter((action) => action === "close-current").length, 1);
  assert.equal(harness.actions.filter((action) => action === "cleanup-attempted").length, 1);
});

test("restoration failure enters picker without reopening the previous workspace", async () => {
  const harness = createHarness({
    start: async () => {
      harness.actions.push("start:/next");
      throw new WorkspaceSwitchError("restoration_failed", "Workspace restore was rejected");
    },
  });

  const result = await harness.coordinator.switchWorkspace("/next");

  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.state, "picker");
  assert.equal(harness.getCurrentPath(), null);
  assert.doesNotMatch(harness.actions.join(","), /start:\/current/);
  assert.deepEqual(harness.events.at(-1)?.state, "picker");
});

test("retry recovers through the serialized open path and publishes ready", async () => {
  let attempts = 0;
  const harness = createHarness({
    initialWorkspacePath: null,
    start: async (path) => {
      harness.actions.push(`start:${path}`);
      attempts += 1;
      if (attempts === 1) throw new WorkspaceSwitchError("readiness_failed", "Sidecar did not become ready");
      return { workspace: { path }, port: 4100 };
    },
  });

  const failed = await harness.coordinator.switchWorkspace("/next");
  assert.equal(failed.ok, false);
  assert.equal(harness.coordinator.getState(), "picker");

  const recovered = await harness.coordinator.retry();

  assert.equal(recovered.ok, true);
  if (!recovered.ok) return;
  assert.equal(recovered.state, "ready");
  assert.equal(recovered.port, 4100);
  assert.deepEqual(harness.events.map((event) => event.state), ["opening", "picker", "opening", "ready"]);
});

test("successful restoration records history before publishing ready", async () => {
  const harness = createHarness({ initialWorkspacePath: null });

  const result = await harness.coordinator.switchWorkspace("/next");

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.state, "ready");
  assert.equal(result.workspace.path, "/next");
  assert.deepEqual(harness.actions, ["start:/next", "history:/next", "path:/next"]);
  assert.deepEqual(harness.events.map((event) => event.state), ["opening", "ready"]);
  assert.equal(harness.events.at(-1)?.state, "ready");
});

test("a failure during awaited history persistence cannot publish a stale ready workspace", async () => {
  const history = deferred<null>();
  const historyStarted = deferred<void>();
  const harness = createHarness({
    initialWorkspacePath: null,
    remember: () => {
      historyStarted.resolve();
      return history.promise;
    },
  });

  const opening = harness.coordinator.switchWorkspace("/next");
  await historyStarted.promise;
  harness.coordinator.markUnresolved({
    code: "cleanup_failed",
    message: "The sidecar descendants could not be proven stopped.",
  });
  history.resolve(null);

  const result = await opening;
  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.equal(result.state, "unresolved");
  assert.equal(result.failure.code, "cleanup_failed");
  assert.equal(harness.getCurrentPath(), null);
  assert.equal(harness.actions.includes("path:/next"), false);
  assert.equal(harness.events.some((event) => event.state === "ready"), false);
});

test("repeated switch and quit requests are serialized", async () => {
  let releaseClose!: () => void;
  const closeGate = new Promise<void>((resolve) => {
    releaseClose = resolve;
  });
  const harness = createHarness({
    close: async () => {
      await closeGate;
    },
  });

  const first = harness.coordinator.switchWorkspace("/next");
  const second = harness.coordinator.switchWorkspace("/third");
  const quit = harness.coordinator.quit();

  await Promise.resolve();
  await Promise.resolve();
  assert.deepEqual(harness.actions, ["close-current"]);

  releaseClose();
  const [firstResult, secondResult, quitResult] = await Promise.all([first, second, quit]);

  assert.equal(firstResult.ok, true);
  assert.equal(secondResult.ok, true);
  assert.equal(quitResult.ok, true);
  assert.deepEqual(harness.actions, [
    "close-current",
    "path:none",
    "start:/next",
    "history:/next",
    "path:/next",
    "close-current",
    "path:none",
    "start:/third",
    "history:/third",
    "path:/third",
    "close-current",
    "path:none",
  ]);
  assert.deepEqual(harness.events.map((event) => event.state), [
    "closing", "picker", "opening", "ready",
    "closing", "picker", "opening", "ready",
    "closing", "picker",
  ] satisfies WorkspaceInstanceState[]);
});

test("update shutdown holds the coordinator queue until installation completes", async () => {
  const install = deferred<void>();
  const started = deferred<void>();
  const harness = createHarness();
  const updating = harness.coordinator.runUpdate(async (workspacePath) => {
    assert.equal(workspacePath, "/current");
    started.resolve();
    await install.promise;
  });
  const switching = harness.coordinator.switchWorkspace("/next");

  await started.promise;
  assert.deepEqual(harness.actions, ["close-current", "path:none"]);

  install.resolve();
  await updating;
  await switching;
  assert.equal(harness.getCurrentPath(), "/next");
});

test("failed update restores the workspace through the serialized open path", async () => {
  const harness = createHarness();

  await assert.rejects(
    harness.coordinator.runUpdate(async () => {
      throw new Error("installer failed");
    }),
    /installer failed/,
  );

  assert.equal(harness.getCurrentPath(), "/current");
  assert.equal(harness.coordinator.getState(), "ready");
  assert.deepEqual(harness.actions, [
    "close-current",
    "path:none",
    "start:/current",
    "history:/current",
    "path:/current",
  ]);
});

test("quit cleanup timeout remains unresolved and can be retried", async () => {
  let oldRuntimeExited = false;
  const harness = createHarness({
    close: async () => {
      if (!oldRuntimeExited) {
        throw new SidecarCleanupError("cleanup_timeout", "Sidecar cleanup timed out");
      }
    },
  });

  const failed = await harness.coordinator.quit();

  assert.equal(failed.ok, false);
  if (failed.ok) return;
  assert.equal(failed.state, "unresolved");
  assert.equal(failed.failure.code, "cleanup_timeout");
  assert.equal(harness.coordinator.getState(), "unresolved");
  assert.deepEqual(harness.actions, ["close-current"]);

  oldRuntimeExited = true;
  const recovered = await harness.coordinator.quit();

  assert.equal(recovered.ok, true);
  assert.equal(harness.coordinator.getState(), "picker");
  assert.deepEqual(harness.actions, ["close-current", "close-current", "path:none"]);
});

test("path-null attempted-runtime cleanup failure blocks quit until retry acknowledges cleanup", async () => {
  let cleanupAcknowledged = false;
  let startAttempts = 0;
  let quitCalls = 0;
  const harness = createHarness({
    initialWorkspacePath: null,
    start: async (path) => {
      harness.actions.push(`start:${path}`);
      startAttempts += 1;
      if (startAttempts === 1) {
        throw new WorkspaceSwitchError("readiness_failed", "Destination did not become ready");
      }
      return { workspace: { path }, port: 4200 };
    },
    cleanupAttempted: async () => {
      if (!cleanupAcknowledged) {
        throw new SidecarCleanupError("cleanup_timeout", "Attempted runtime cleanup timed out");
      }
    },
    onQuit: () => {
      quitCalls += 1;
    },
  });

  const failed = await harness.coordinator.switchWorkspace("/next");

  assert.equal(failed.ok, false);
  if (failed.ok) return;
  assert.equal(failed.state, "unresolved");
  assert.equal(harness.coordinator.getCurrentWorkspacePath(), null);

  const quit = await harness.coordinator.quit();

  assert.equal(quit.ok, false);
  if (quit.ok) return;
  assert.equal(quit.state, "unresolved");
  assert.equal(quit.failure.code, "cleanup_timeout");
  assert.equal(quitCalls, 0);

  const blockedSwitch = await harness.coordinator.switchWorkspace("/third");

  assert.equal(blockedSwitch.ok, false);
  if (blockedSwitch.ok) return;
  assert.equal(blockedSwitch.state, "unresolved");
  assert.equal(harness.actions.includes("start:/third"), false);

  cleanupAcknowledged = true;
  const recovered = await harness.coordinator.retry();

  assert.equal(recovered.ok, true);
  assert.equal(harness.actions.filter((action) => action === "cleanup-attempted").length, 2);
  assert.equal(harness.coordinator.getState(), "ready");
});
