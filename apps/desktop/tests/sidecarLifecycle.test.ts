import test from "node:test";
import assert from "node:assert/strict";

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

  on(event: "exit" | "error", listener: Listener): this {
    this.listeners.set(event, [...(this.listeners.get(event) ?? []), listener]);
    return this;
  }

  kill(): void {
    this.killed = true;
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

  setTimeout = (callback: () => void, delayMs: number): number => {
    const id = this.nextId++;
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

function createHarness() {
  const processes: FakeProcess[] = [];
  const timers = new FakeTimers();
  const states: SidecarState[] = [];
  const unavailable: string[] = [];
  const ready: number[] = [];
  const lifecycle = createSidecarLifecycle({
    spawn: () => {
      const process = new FakeProcess();
      processes.push(process);
      return process;
    },
    fetch: async () => new Response(),
    setTimeout: timers.setTimeout,
    clearTimeout: timers.clearTimeout,
    now: timers.now,
    callbacks: {
      onReady: (port) => ready.push(port),
      onUnavailable: (message) => unavailable.push(message),
      onState: (state) => states.push(state),
    },
    readinessTimeoutMs: 10,
    retryDelaysMs: [1, 2],
    readyStabilityMs: 5,
  });

  return { lifecycle, processes, timers, states, unavailable, ready };
}

test("rejects readiness when the process exits before publishing a port", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");

  processes[0].exit(1);

  await assert.rejects(readiness, /exited before readiness/i);
  assert.equal(lifecycle.getPort(), null);
});

test("ignores exit from an obsolete generation", async () => {
  const { lifecycle, processes } = createHarness();
  const first = lifecycle.start("/one");
  const second = lifecycle.start("/two");

  processes[1].stdout.emit("ORKWORKSD_PORT=4567\n");
  processes[0].exit(1);

  await assert.rejects(first, /stopped/i);
  assert.equal(await second, 4567);
  assert.equal(lifecycle.getPort(), 4567);
});

test("stops after three automatic attempts and permits explicit retry", async () => {
  const { lifecycle, processes, timers, states } = createHarness();
  const initial = lifecycle.start("/workspace");
  processes[0].exit(1);
  await assert.rejects(initial, /exited before readiness/i);

  timers.runNext();
  processes[1].exit(1);
  timers.runNext();
  processes[2].exit(1);

  assert.equal(processes.length, 3);
  assert.equal(timers.size, 0);
  assert.equal(states.at(-1), "exhausted");

  const retry = lifecycle.retry();
  processes[3].stdout.emit("ORKWORKSD_PORT=7890\n");

  assert.equal(await retry, 7890);
  assert.equal(lifecycle.getPort(), 7890);
});

test("notifies when a ready process exits", async () => {
  const { lifecycle, processes, unavailable } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await readiness;

  processes[0].exit(9);

  assert.equal(lifecycle.getPort(), null);
  assert.match(unavailable.at(-1) ?? "", /exited/i);
});

test("rejects readiness when spawn emits an error", async () => {
  const { lifecycle, processes } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].error(new Error("permission denied"));

  await assert.rejects(readiness, /permission denied/);
  assert.equal(lifecycle.getPort(), null);
  assert.equal(processes[0].killed, true);
});

test("rejects readiness when publishing a port times out", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("/workspace");

  timers.runNext();

  await assert.rejects(readiness, /timed out/i);
  assert.equal(lifecycle.getPort(), null);
  assert.equal(processes[0].killed, true);
});

test("does not reset automatic retries when a ready process fails before the stability window", async () => {
  const { lifecycle, processes, timers, states } = createHarness();
  const initial = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await initial;

  processes[0].exit(1);
  timers.advanceBy(1);
  processes[1].exit(1);
  timers.advanceBy(2);
  processes[2].exit(1);

  assert.equal(processes.length, 3);
  assert.equal(states.at(-1), "exhausted");
});

test("resets automatic retries only after the ready stability window expires", async () => {
  const { lifecycle, processes, timers, states } = createHarness();
  const initial = lifecycle.start("/workspace");
  processes[0].stdout.emit("ORKWORKSD_PORT=4444\n");
  await initial;

  timers.advanceBy(5);
  processes[0].exit(1);
  timers.advanceBy(1);
  processes[1].exit(1);
  timers.advanceBy(1);
  processes[2].exit(1);
  timers.advanceBy(2);
  processes[3].exit(1);

  assert.equal(processes.length, 4);
  assert.equal(states.at(-1), "exhausted");
});

test("creates only one automatic recovery sequence for repeated failure callbacks", async () => {
  const { lifecycle, processes, timers } = createHarness();
  const readiness = lifecycle.start("/workspace");
  processes[0].error(new Error("spawn failed"));
  processes[0].exit(1);
  await assert.rejects(readiness, /spawn failed/);

  assert.equal(timers.size, 1);
  timers.runNext();
  assert.equal(processes.length, 2);
});
