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

function createHarness(options: {
  start?: (path: string, generation: number) => Promise<{ workspace: Workspace; port: number }>;
  close?: () => Promise<void>;
  cleanupAttempted?: () => Promise<void>;
  remember?: (path: string, workspace: Workspace) => void;
  initialWorkspacePath?: string | null;
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
