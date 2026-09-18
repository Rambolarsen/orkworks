import test from "node:test";
import assert from "node:assert/strict";

import { createBackendRestorationCoordinator } from "../electron/backendRestoration.ts";
import { createSidecarLifecycle, type SidecarLifecycle, type SidecarState } from "../electron/sidecarLifecycle.ts";

type Listener = (...args: any[]) => void;

class FakeStream {
  private readonly listeners = new Map<string, Listener[]>();

  on(event: "data", listener: Listener): this {
    this.listeners.set(event, [...(this.listeners.get(event) ?? []), listener]);
    return this;
  }

  emit(data: string): void {
    for (const listener of this.listeners.get("data") ?? []) listener(Buffer.from(data));
  }
}

class FakeProcess {
  readonly stdout = new FakeStream();
  private readonly listeners = new Map<string, Listener[]>();
  killed = false;
  killCount = 0;

  on(event: "exit" | "error", listener: Listener): this {
    this.listeners.set(event, [...(this.listeners.get(event) ?? []), listener]);
    return this;
  }

  kill(): void {
    this.killed = true;
    this.killCount += 1;
  }

  exit(code: number | null): void {
    for (const listener of this.listeners.get("exit") ?? []) listener(code);
  }

  error(error: Error): void {
    for (const listener of this.listeners.get("error") ?? []) listener(error);
  }
}

class FakeTimers {
  private nextId = 1;
  private currentTimeMs = 0;
  private readonly timers = new Map<number, { callback: () => void; dueAtMs: number }>();
  readonly scheduledDelays: number[] = [];

  setTimeout = (callback: () => void, delayMs: number): number => {
    const id = this.nextId++;
    this.scheduledDelays.push(delayMs);
    this.timers.set(id, { callback, dueAtMs: this.currentTimeMs + delayMs });
    return id;
  };

  clearTimeout = (id: number): void => {
    this.timers.delete(id);
  };

  runNext(): void {
    const next = [...this.timers.entries()]
      .sort(([, left], [, right]) => left.dueAtMs - right.dueAtMs)
      .at(0);
    assert.ok(next, "expected a pending timer");
    this.timers.delete(next[0]);
    this.currentTimeMs = next[1].dueAtMs;
    next[1].callback();
  }

  advanceBy(delayMs: number): void {
    const targetTimeMs = this.currentTimeMs + delayMs;
    while (true) {
      const next = [...this.timers.entries()]
        .filter(([, timer]) => timer.dueAtMs <= targetTimeMs)
        .sort(([, left], [, right]) => left.dueAtMs - right.dueAtMs)
        .at(0);
      if (!next) break;
      this.timers.delete(next[0]);
      this.currentTimeMs = next[1].dueAtMs;
      next[1].callback();
    }
    this.currentTimeMs = targetTimeMs;
  }

  now = (): number => {
    return this.currentTimeMs;
  }

  get size(): number {
    return this.timers.size;
  }
}

function createHarness(
  spawn?: (cwd: string) => FakeProcess,
  options: { cleanupTimeoutMs?: number } = {},
) {
  const processes: FakeProcess[] = [];
  const spawnProcess = spawn ?? (() => {
    const process = new FakeProcess();
    processes.push(process);
    return process;
  });
  const timers = new FakeTimers();
  const states: SidecarState[] = [];
  const unavailable: string[] = [];
  const unexpectedExits: string[] = [];
  const ready: number[] = [];
  const lifecycle = createSidecarLifecycle({
    spawn: spawnProcess,
    setTimeout: timers.setTimeout,
    clearTimeout: timers.clearTimeout,
    now: timers.now,
    callbacks: {
      onReady: (port) => ready.push(port),
      onUnavailable: (message) => unavailable.push(message),
      onUnexpectedExit: (message) => unexpectedExits.push(message),
      onState: (state) => states.push(state),
    },
    readinessTimeoutMs: 10,
    cleanupTimeoutMs: options.cleanupTimeoutMs,
  });

  return { lifecycle, processes, timers, states, unavailable, unexpectedExits, ready };
}

test("rejects readiness when the process exits before publishing a port", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");

  processes[0].exit(1);

  await assert.rejects(readiness, /exited before readiness/i);
  assert.equal(lifecycle.getPort(), null);
});

test("stopAndWait resolves after the requested child exits", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("C:\\workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await readiness;
  const stopping = lifecycle.stopAndWait(1000);

  processes[0].exit(0);

  await assert.doesNotReject(stopping);
  assert.equal(timers.size, 0);
});

test("stopAndWait resolves after stopping before readiness", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("/workspace");
  const stopping = lifecycle.stopAndWait(1000);

  processes[0].exit(0);

  await assert.rejects(readiness, /stopped before readiness/i);
  await assert.doesNotReject(stopping);
  assert.equal(timers.size, 0);
});

test("stopAndWait rejects unbounded or negative timeouts", async () => {
  for (const timeoutMs of [-1, Number.POSITIVE_INFINITY, Number.NaN]) {
    const { lifecycle } = createHarness();

    await assert.rejects(lifecycle.stopAndWait(timeoutMs), /finite and non-negative/i);
  }
});

test("stopAndWait rejects on timeout without starting a replacement", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("C:\\workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await readiness;
  const stopping = lifecycle.stopAndWait(50);

  timers.advanceBy(50);

  await assert.rejects(stopping, /timed out/i);
  assert.equal(processes.length, 1);
  assert.equal(processes[0].killCount, 1);
  assert.equal(timers.size, 0);
});

test("starts the next generation only after the old process exits", async () => {
  const { lifecycle, processes } = createHarness();
  const first = lifecycle.start("/one");
  processes[0].stdout.emit("ORKWORKSD_PORT=4556\n");
  await first;
  const stopped = lifecycle.stop();
  processes[0].exit(0);
  await stopped;

  const second = lifecycle.start("/two");

  processes[1].stdout.emit("ORKWORKSD_PORT=4567\n");

  assert.equal(await second, 4567);
  assert.equal(lifecycle.getPort(), 4567);
});

test("stop resolves only after the owned process exits", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4567\n");
  await readiness;

  let stopped = false;
  const cleanup = lifecycle.stop().then(() => {
    stopped = true;
  });
  await Promise.resolve();
  assert.equal(stopped, false);

  processes[0].exit(0);
  await cleanup;
  assert.equal(stopped, true);
});

test("does not automatically replace an unexpectedly exited sidecar without ownership proof", async () => {
  const { lifecycle, processes, timers, states } = createHarness();
  const initial = lifecycle.start("/workspace");
  processes[0].exit(1);
  await assert.rejects(initial, /exited before readiness/i);

  assert.equal(timers.size, 0);
  assert.equal(processes.length, 1);
  assert.equal(states.at(-1), "failed");

  const retry = lifecycle.retry();
  await assert.rejects(retry, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_failed");
    return true;
  });
  assert.equal(processes.length, 1);
});

test("cleanup timeout rejects with a typed failure and blocks replacement while the old sidecar runs", async () => {
  const { lifecycle, processes, timers } = createHarness(undefined, { cleanupTimeoutMs: 5 });
  const readiness = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4567\n");
  await readiness;

  const cleanup = lifecycle.stop();
  timers.advanceBy(5);
  await assert.rejects(cleanup, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_timeout");
    return true;
  });

  const replacement = lifecycle.start("/next");
  assert.equal(processes.length, 1);
  await assert.rejects(replacement, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_timeout");
    return true;
  });
});

test("cleanup timeout remains sticky after process exit and blocks start and retry", async () => {
  const { lifecycle, processes, timers } = createHarness(undefined, { cleanupTimeoutMs: 5 });
  const readiness = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4567\n");
  await readiness;

  const cleanup = lifecycle.stop();
  timers.advanceBy(5);
  await assert.rejects(cleanup, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_timeout");
    return true;
  });

  processes[0].exit(0);

  const startResult = lifecycle.start("/replacement").then(
    () => null,
    (error: unknown) => error,
  );
  assert.equal(processes.length, 1, "a timed-out cleanup must remain a replacement tombstone after exit");
  const startError = await startResult;
  assert.equal((startError as { code?: string }).code, "cleanup_timeout");

  const retryResult = lifecycle.retry().then(
    () => null,
    (error: unknown) => error,
  );
  assert.equal(processes.length, 1, "retry must remain blocked until native ownership proof");
  const retryError = await retryResult;
  assert.equal((retryError as { code?: string }).code, "cleanup_timeout");
});

test("unexpected exit reports unresolved ownership and blocks replacement without an ownership receipt", async () => {
  const { lifecycle, processes, unavailable, unexpectedExits } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await readiness;

  processes[0].exit(9);

  assert.equal(lifecycle.getPort(), null);
  assert.match(unavailable.at(-1) ?? "", /exited/i);
  assert.match(unexpectedExits.at(-1) ?? "", /ownership.*unresolved/i);

  const blockedReplacement = lifecycle.start("/replacement");
  await assert.rejects(blockedReplacement, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_failed");
    assert.match((error as Error).message, /ownership.*unresolved/i);
    return true;
  });
  assert.equal(processes.length, 1, "an unresolved generation must not spawn a replacement");

  await assert.rejects(lifecycle.stop(), /ownership.*unresolved/i);
  const stillBlockedReplacement = lifecycle.retry();
  await assert.rejects(stillBlockedReplacement, (error: unknown) => {
    assert.equal((error as { code?: string }).code, "cleanup_failed");
    assert.match((error as Error).message, /ownership.*unresolved/i);
    return true;
  });
  assert.equal(processes.length, 1, "process exit is not a descendant ownership receipt");
});

test("an intentionally stopped generation can be replaced after its process exits", async () => {
  const { lifecycle, processes } = createHarness();
  const first = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await first;

  const stopped = lifecycle.stop();
  processes[0].exit(0);
  await stopped;

  const replacement = lifecycle.start("/replacement");
  processes[1].stdout.emit("ORKWORKSD_PORT=4555\n");
  assert.equal(await replacement, 4555);
});

test("rejects readiness when spawn emits an error", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].error(new Error("permission denied"));

  await assert.rejects(readiness, /permission denied/);
  assert.equal(lifecycle.getPort(), null);
  assert.equal(processes[0].killed, true);
});

test("turns a synchronous spawn failure into rejected readiness and allows explicit retry", async () => {
  let failFirstSpawn = true;
  const { lifecycle, processes, states, unavailable } = createHarness((cwd) => {
    if (failFirstSpawn) {
      failFirstSpawn = false;
      throw new Error(`spawn failed for ${cwd}`);
    }
    const process = new FakeProcess();
    processes.push(process);
    return process;
  });

  const first = lifecycle.start("/missing-sidecar-cwd");

  await assert.rejects(first, /spawn failed for \/missing-sidecar-cwd/);
  assert.deepEqual(states.slice(0, 2), ["starting", "failed"]);
  assert.deepEqual(unavailable, ["spawn failed for /missing-sidecar-cwd"]);

  const retry = lifecycle.retry();
  processes[0].stdout.emit("ORKWORKSD_PORT=7890\n");

  assert.equal(await retry, 7890);
  assert.equal(lifecycle.getPort(), 7890);
});

test("a synchronous explicit-retry failure rejects the wired restoration readiness", async () => {
  const timers = new FakeTimers();
  const failures: string[] = [];
  const restoration = createBackendRestorationCoordinator({
    setTimeout: timers.setTimeout,
    clearTimeout: timers.clearTimeout,
    onReady: () => {},
    onFailure: (error) => failures.push(error.message),
  });
  const lifecycle = createSidecarLifecycle({
    spawn: () => {
      throw new Error("spawn failed at /absolute/sidecar");
    },
    setTimeout: timers.setTimeout,
    clearTimeout: timers.clearTimeout,
    now: timers.now,
    callbacks: {
      onReady: () => {},
      onUnavailable: (message) => restoration.fail(new Error(message)),
      onState: (state) => {
        if (state === "starting") restoration.beginGeneration();
      },
    },
  });

  const initial = lifecycle.start("/workspace");
  const initialRestoration = restoration.getReadiness();
  await assert.rejects(initial, /spawn failed/);
  await assert.rejects(initialRestoration, /spawn failed/);

  const retry = lifecycle.retry();
  const retryRestoration = restoration.getReadiness();
  await assert.rejects(retry, /spawn failed/);
  await assert.rejects(retryRestoration, /spawn failed/);
  assert.deepEqual(failures, ["spawn failed at /absolute/sidecar", "spawn failed at /absolute/sidecar"]);
});

test("rejects readiness when publishing a port times out", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("/workspace");

  timers.runNext();

  await assert.rejects(readiness, /timed out/i);
  assert.equal(lifecycle.getPort(), null);
  assert.equal(processes[0].killed, true);
});

test("rejects announced ports outside the valid TCP range before readiness", async () => {
  for (const announcedPort of ["0", "65536", "-1"]) {
    const { lifecycle, processes } = createHarness();
    const readiness = lifecycle.start("/workspace");
    processes[0].stdout.emit(`ORKWORKSD_PORT=${announcedPort}\n`);

    await assert.rejects(readiness, /invalid port/i);
    assert.equal(lifecycle.getPort(), null);
    assert.equal(processes[0].killed, true);
  }
});

test("still discovers a valid port after noisy pre-readiness stdout", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");

  processes[0].stdout.emit("x".repeat(70 * 1024));
  processes[0].stdout.emit("ORKWORKSD_PORT=4321\n");

  assert.equal(await readiness, 4321);
});
