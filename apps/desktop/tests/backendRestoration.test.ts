import assert from "node:assert/strict";
import test from "node:test";

import {
  createBackendRestorationCoordinator,
  switchWorkspaceBackend,
} from "../electron/backendRestoration.ts";

class FakeTimers {
  private nextId = 1;
  private readonly timers = new Map<number, () => void>();

  setTimeout = (callback: () => void): number => {
    const id = this.nextId++;
    this.timers.set(id, callback);
    return id;
  };

  clearTimeout = (id: number): void => {
    this.timers.delete(id);
  };

  runNext(): void {
    const next = this.timers.entries().next().value as [number, () => void] | undefined;
    assert.ok(next, "expected a pending timer");
    this.timers.delete(next[0]);
    next[1]();
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function createHarness() {
  const timers = new FakeTimers();
  const ready: Array<{ port: number; workspace: unknown }> = [];
  const failed: string[] = [];
  const coordinator = createBackendRestorationCoordinator({
    setTimeout: timers.setTimeout,
    clearTimeout: timers.clearTimeout,
    timeoutMs: 10,
    onReady: (port, workspace) => ready.push({ port, workspace }),
    onFailure: (error) => failed.push(error.message),
  });
  return { coordinator, timers, ready, failed };
}

test("a replaced generation aborts and cannot publish after deferred restoration completes", async () => {
  const { coordinator, ready } = createHarness();
  const firstWorkspace = deferred<unknown>();
  let firstSignal: AbortSignal | null = null;

  coordinator.beginGeneration();
  const firstReadiness = coordinator.getReadiness();
  coordinator.restore(4001, {
    restoreWorkspace: (signal) => {
      firstSignal = signal;
      return firstWorkspace.promise;
    },
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {},
  });

  coordinator.beginGeneration();
  assert.equal(firstSignal?.aborted, true);
  await assert.rejects(firstReadiness, /replaced/i);

  const secondReadiness = coordinator.getReadiness();
  coordinator.restore(4002, {
    restoreWorkspace: async () => ({ path: "/second" }),
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {},
  });

  assert.equal(await secondReadiness, 4002);
  assert.deepEqual(coordinator.getRestoredWorkspace(), { path: "/second" });
  firstWorkspace.resolve({ path: "/stale" });
  await Promise.resolve();
  await Promise.resolve();

  assert.deepEqual(ready, [{ port: 4002, workspace: { path: "/second" } }]);
  assert.deepEqual(coordinator.getRestoredWorkspace(), { path: "/second" });
});

test("a restoration timeout aborts hung work, rejects readiness, and publishes failure", async () => {
  const { coordinator, timers, failed } = createHarness();
  let signal: AbortSignal | null = null;

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(5001, {
    restoreWorkspace: (nextSignal) => {
      signal = nextSignal;
      return new Promise((_resolve, reject) => {
        nextSignal.addEventListener("abort", () => reject(nextSignal.reason), { once: true });
      });
    },
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {},
  });

  timers.runNext();

  assert.equal(signal?.aborted, true);
  await assert.rejects(readiness, /restoration timed out/i);
  assert.deepEqual(failed, ["Backend restoration timed out"]);
});

test("a restoration timeout aborts a hung retention step and cannot publish ready", async () => {
  const { coordinator, timers, ready, failed } = createHarness();
  let retentionSignal: AbortSignal | null = null;

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(5003, {
    restoreWorkspace: async () => ({ path: "/workspace" }),
    applyRetentionSettings: (signal) => {
      retentionSignal = signal;
      return new Promise((_resolve, reject) => {
        signal.addEventListener("abort", () => reject(signal.reason), { once: true });
      });
    },
    syncProviderSettings: async () => {},
  });

  await Promise.resolve();
  timers.runNext();

  assert.equal(retentionSignal?.aborted, true);
  await assert.rejects(readiness, /restoration timed out/i);
  assert.deepEqual(ready, []);
  assert.deepEqual(failed, ["Backend restoration timed out"]);
  assert.equal(coordinator.getRestoredWorkspace(), null);
});

test("a restoration timeout aborts a hung provider sync and cannot publish ready", async () => {
  const { coordinator, timers, ready, failed } = createHarness();
  let providerSignal: AbortSignal | null = null;

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(5004, {
    restoreWorkspace: async () => ({ path: "/workspace" }),
    applyRetentionSettings: async () => {},
    syncProviderSettings: (signal) => {
      providerSignal = signal;
      return new Promise((_resolve, reject) => {
        signal.addEventListener("abort", () => reject(signal.reason), { once: true });
      });
    },
  });

  await Promise.resolve();
  await Promise.resolve();
  timers.runNext();

  assert.equal(providerSignal?.aborted, true);
  await assert.rejects(readiness, /restoration timed out/i);
  assert.deepEqual(ready, []);
  assert.deepEqual(failed, ["Backend restoration timed out"]);
  assert.equal(coordinator.getRestoredWorkspace(), null);
});

test("a provider sync failure rejects readiness before ready is published", async () => {
  const { coordinator, ready, failed } = createHarness();

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(5005, {
    restoreWorkspace: async () => ({ path: "/workspace" }),
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {
      throw new Error("provider settings unavailable");
    },
  });

  await assert.rejects(readiness, /provider settings unavailable/);
  assert.deepEqual(ready, []);
  assert.deepEqual(failed, ["provider settings unavailable"]);
});

test("a timed-out restoration cannot publish after work that ignored abort completes", async () => {
  const { coordinator, timers, ready } = createHarness();
  const workspace = deferred<unknown>();

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(5002, {
    restoreWorkspace: () => workspace.promise,
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {},
  });

  timers.runNext();
  await assert.rejects(readiness, /restoration timed out/i);
  workspace.resolve({ path: "/late" });
  await Promise.resolve();
  await Promise.resolve();

  assert.deepEqual(ready, []);
  assert.equal(coordinator.getRestoredWorkspace(), null);
});

test("explicit backend failure and shutdown abort their owned restoration work", async () => {
  const { coordinator } = createHarness();
  const signals: AbortSignal[] = [];
  const hungSteps = {
    restoreWorkspace: (signal: AbortSignal) => {
      signals.push(signal);
      return new Promise<unknown>(() => {});
    },
    applyRetentionSettings: async () => {},
    syncProviderSettings: async () => {},
  };

  coordinator.beginGeneration();
  const failedReadiness = coordinator.getReadiness();
  coordinator.restore(6001, hungSteps);
  coordinator.fail(new Error("sidecar exited"));

  assert.equal(signals[0].aborted, true);
  await assert.rejects(failedReadiness, /sidecar exited/i);

  coordinator.beginGeneration();
  const disposedReadiness = coordinator.getReadiness();
  coordinator.restore(6002, hungSteps);
  coordinator.dispose();

  assert.equal(signals[1].aborted, true);
  await assert.rejects(disposedReadiness, /shut down/i);
});

test("workspace persistence failure leaves the current backend untouched", () => {
  let started = false;

  assert.throws(
    () => switchWorkspaceBackend(
      "/replacement",
      () => {
        throw new Error("disk full");
      },
      () => {
        started = true;
        return "replacement readiness";
      },
    ),
    /disk full/,
  );

  assert.equal(started, false);
});

test("workspace persistence completes before replacement startup", () => {
  const calls: string[] = [];

  const result = switchWorkspaceBackend(
    "/replacement",
    () => calls.push("persist"),
    () => {
      calls.push("start");
      return "replacement readiness";
    },
  );

  assert.equal(result, "replacement readiness");
  assert.deepEqual(calls, ["persist", "start"]);
});

test("side steps start while workspace restoration is still in flight", async () => {
  const { coordinator, ready } = createHarness();
  const started: string[] = [];
  const workspace = deferred<unknown>();

  coordinator.beginGeneration();
  const readiness = coordinator.getReadiness();
  coordinator.restore(7001, {
    restoreWorkspace: () => {
      started.push("restoreWorkspace");
      return workspace.promise;
    },
    applyRetentionSettings: async () => {
      started.push("applyRetentionSettings");
    },
    syncProviderSettings: async () => {
      started.push("syncProviderSettings");
    },
  });

  await Promise.resolve();
  await Promise.resolve();

  assert.ok(
    started.includes("applyRetentionSettings"),
    "retention should start while workspace restoration is pending",
  );
  assert.ok(
    started.includes("syncProviderSettings"),
    "provider sync should start while workspace restoration is pending",
  );

  workspace.resolve({ path: "/workspace" });
  await Promise.resolve();
  await Promise.resolve();

  assert.deepEqual(await readiness, 7001);
  assert.deepEqual(coordinator.getRestoredWorkspace(), { path: "/workspace" });
  assert.deepEqual(ready, [{ port: 7001, workspace: { path: "/workspace" } }]);
});
