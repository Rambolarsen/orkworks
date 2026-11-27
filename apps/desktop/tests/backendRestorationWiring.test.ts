import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import test from "node:test";
import type { BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic } from "../electron/backendLifecycleEvent.ts";
import { DEFAULT_SETTINGS, type AppSettings } from "../electron/settingsMemory.ts";
import type { SidecarProcess } from "../electron/sidecarLifecycle.ts";
import {
  createWorkspaceRuntime,
  StaleBackendGenerationError,
  type WorkspaceRuntimeOptions,
} from "../electron/workspaceRuntime.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

class FakeSidecarProcess extends EventEmitter implements SidecarProcess {
  stdout = new EventEmitter();
  stderr = new EventEmitter();
  pid: number;
  killed = false;
  exited = false;
  killSignals: Array<NodeJS.Signals | number | undefined> = [];
  autoExitOnKill = true;

  constructor(pid: number) {
    super();
    this.pid = pid;
  }

  emitReady(port: number): void {
    this.stdout.emit("data", Buffer.from(`ORKWORKSD_PORT=${port}\n`));
  }

  emitExit(code: number | null = 0, signal: NodeJS.Signals | null = null): void {
    if (this.exited) return;
    this.exited = true;
    this.emit("exit", code, signal);
  }

  kill(signal?: NodeJS.Signals | number): boolean {
    this.killed = true;
    this.killSignals.push(signal);
    if (this.autoExitOnKill && !this.exited) {
      queueMicrotask(() => this.emitExit(0, typeof signal === "string" ? signal : null));
    }
    return true;
  }
}

function makeWorkspace(workspacePath: string, activeHarnessRevision = 1): BackendLifecycleWorkspace {
  return {
    path: workspacePath,
    workspaceIdentity: workspacePath,
    repo_root: null,
    branch: null,
    dirty: false,
    lastActiveSessionId: null,
    activeHarnessIds: [],
    activeHarnessRevision,
  };
}

interface FixtureOptions {
  queued?: Promise<void>;
  providerBody?: Promise<unknown>;
  pushResponse?: Promise<Response>;
  workspaceResponse?: (port: number, init?: { signal?: AbortSignal }) => Promise<Response>;
  forgetWorkspacePath?: WorkspaceRuntimeOptions["forgetWorkspacePath"];
  initialHistoryDiagnostic?: WorkspaceHistoryDiagnostic | null;
  restorationTimeoutMs?: number;
}

function createRuntimeFixture(options: FixtureOptions = {}) {
  const initialSettings = structuredClone(DEFAULT_SETTINGS);
  let currentSettings: AppSettings = structuredClone(DEFAULT_SETTINGS);
  const writes: AppSettings[] = [];
  const requests: string[] = [];
  const processes: FakeSidecarProcess[] = [];
  const tokens: string[] = [];
  let clears = 0;
  let nextPid = 5000;
  let tokenSeq = 0;

  const sidecarSend: NonNullable<WorkspaceRuntimeOptions["sidecarSend"]> = async (port, endpoint, init) => {
    if (endpoint === "workspace") {
      if (options.workspaceResponse) {
        return options.workspaceResponse(port, init);
      }
      const body = init?.jsonBody as { path?: string } | undefined;
      const wsPath = body?.path ?? "/workspace";
      return new Response(JSON.stringify(makeWorkspace(wsPath)), { status: 200 });
    }
    if (endpoint === "settings/retention") {
      return new Response(null, { status: 204 });
    }
    if (endpoint === "providers") {
      requests.push("catalog");
      return {
        ok: true,
        status: 200,
        json: async () => options.providerBody ?? {
          providers: [{ id: "replacement-provider", label: "Replacement" }],
        },
      } as unknown as Response;
    }
    return new Response(null, { status: 404 });
  };

  const fetchImpl: typeof fetch = async (_input, init) => {
    requests.push("push");
    if (options.pushResponse) {
      return options.pushResponse;
    }
    init?.signal?.throwIfAborted();
    return new Response(JSON.stringify({ ok: true }), { status: 200 });
  };

  const runtime = createWorkspaceRuntime({
    generateToken: () => {
      const token = `token-${++tokenSeq}`;
      tokens.push(token);
      return token;
    },
    spawnSidecar: () => {
      const proc = new FakeSidecarProcess(++nextPid);
      processes.push(proc);
      return proc;
    },
    sidecarSend,
    fetch: fetchImpl,
    validateDestination: (candidate) => candidate,
    canonicalizePath: (candidate) => candidate,
    rememberWorkspacePath: () => ({ diagnostic: null }),
    forgetWorkspacePath: options.forgetWorkspacePath ?? (async () => ({ historyDiagnostic: null, navigationDiagnostic: null })),
    getSettings: () => currentSettings,
    writeSettings: (next) => {
      writes.push(next);
      currentSettings = next;
    },
    enqueueSettingsWrite: async (operation) => {
      if (options.queued) await options.queued;
      return operation();
    },
    clearProviderModelsCache: () => {
      clears += 1;
    },
    initialHistoryDiagnostic: options.initialHistoryDiagnostic ?? null,
    timers: {
      setTimeout: (cb, ms) => setTimeout(cb, ms),
      clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
      now: () => Date.now(),
      ...(options.restorationTimeoutMs === undefined ? {} : { restorationTimeoutMs: options.restorationTimeoutMs }),
    },
    logger: { log() {}, warn() {}, error() {} },
  });

  async function openReady(wsPath: string, port: number): Promise<FakeSidecarProcess> {
    const startCount = processes.length;
    const opening = runtime.openWorkspace(wsPath);
    await settle();
    const proc = processes[startCount];
    assert.ok(proc, `expected spawned sidecar for ${wsPath}`);
    proc.emitReady(port);
    const result = await opening;
    assert.equal(result.ok, true);
    return proc;
  }

  return {
    runtime,
    processes,
    tokens,
    initialSettings,
    settings: () => currentSettings,
    writes,
    requests,
    clears: () => clears,
    openReady,
  };
}

test("cancelled provider replay waiting in the settings queue has no effects", async () => {
  const queued = deferred<void>();
  const fixture = createRuntimeFixture({ queued: queued.promise });

  const opening = fixture.runtime.openWorkspace("/ws-queued");
  await settle();
  fixture.processes[0].emitReady(4001);
  await settle();

  // Unexpected sidecar exit cancels restoration while provider replay waits in the settings queue.
  fixture.processes[0].emitExit(1);
  queued.resolve();
  await opening;
  await settle();

  assert.deepEqual(fixture.requests, []);
  assert.deepEqual(fixture.writes, []);
  assert.deepEqual(fixture.settings(), fixture.initialSettings);
  assert.equal(fixture.clears(), 0);
  fixture.runtime.dispose();
});

test("a stale provider catalog body cannot write settings or push them after cancellation", async () => {
  const body = deferred<unknown>();
  const fixture = createRuntimeFixture({ providerBody: body.promise });

  const opening = fixture.runtime.openWorkspace("/ws-stale-catalog");
  await settle();
  fixture.processes[0].emitReady(4001);
  await settle();

  assert.deepEqual(fixture.requests, ["catalog"]);
  fixture.processes[0].emitExit(1);
  body.resolve({ providers: [{ id: "old-workspace-provider", label: "Old workspace" }] });
  await opening;
  await settle();

  assert.deepEqual(fixture.requests, ["catalog"]);
  assert.deepEqual(fixture.writes, []);
  assert.deepEqual(fixture.settings(), fixture.initialSettings);
  assert.equal(fixture.clears(), 0);
  fixture.runtime.dispose();
});

test("current provider replay normalizes, persists, pushes settings, and clears model cache", async () => {
  const fixture = createRuntimeFixture();
  await fixture.openReady("/ws-current", 4002);
  // Secondary restoration steps (retention, provider replay) run asynchronously after workspace readiness.
  await settle();
  await settle();

  assert.equal(fixture.writes.length, 1);
  assert.deepEqual(fixture.requests, ["catalog", "push"]);
  assert.equal(fixture.clears(), 1);
  assert.equal(
    fixture.settings().providers.providers.some((provider) => provider.id === "replacement-provider"),
    true,
  );
  fixture.runtime.dispose();
});

test("a cancelled provider push cannot clear model state for a newer generation", async () => {
  const push = deferred<Response>();
  const fixture = createRuntimeFixture({ pushResponse: push.promise });

  const opening = fixture.runtime.openWorkspace("/ws-stale-push");
  await settle();
  fixture.processes[0].emitReady(4001);
  await settle();

  assert.deepEqual(fixture.requests, ["catalog", "push"]);
  fixture.processes[0].emitExit(1);
  push.resolve(new Response(JSON.stringify({ ok: true }), { status: 200 }));
  await opening;
  await settle();

  assert.equal(fixture.clears(), 0);
  fixture.runtime.dispose();
});

for (const outcome of ["success", "failure"] as const) {
  test(`rejected-workspace history cleanup ${outcome} cannot clear a replacement path or diagnostic`, async () => {
    const removal = deferred<void>();
    let workspaceCalls = 0;

    const fixture = createRuntimeFixture({
      restorationTimeoutMs: 25,
      workspaceResponse: async (_port) => {
        workspaceCalls += 1;
        if (workspaceCalls === 1) {
          return new Response(JSON.stringify({ error: "not found" }), { status: 404 });
        }
        return new Response(JSON.stringify(makeWorkspace("/replacement")), { status: 200 });
      },
      forgetWorkspacePath: async () => {
        await removal.promise;
        if (outcome === "failure") throw new Error("history failure");
        return {
          historyDiagnostic: { code: "history_write_failed", message: "old diagnostic" },
          navigationDiagnostic: null,
        };
      },
    });

    const firstOpen = fixture.runtime.openWorkspace("/old");
    await settle();
    fixture.processes[0].emitReady(4001);

    // Let restoration time out cleanly while forgetWorkspacePath is still pending.
    // The attempted sidecar is stopped cleanly (no sticky unresolved failure).
    const firstResult = await firstOpen;
    assert.equal(firstResult.ok, false);
    assert.equal(fixture.runtime.getLifecycle().state, "picker");

    // Open the replacement workspace before the stale forgetWorkspacePath settles.
    const secondOpen = fixture.runtime.openWorkspace("/replacement");
    await settle();
    fixture.processes[1].emitReady(4002);
    const secondResult = await secondOpen;
    assert.equal(secondResult.ok, true);
    assert.equal(fixture.runtime.getWorkspacePath(), "/replacement");

    removal.resolve();
    await settle();

    assert.equal(fixture.runtime.getWorkspacePath(), "/replacement");
    assert.equal(fixture.runtime.getInitialWorkspaceSnapshot().historyDiagnostic, null);
    fixture.runtime.dispose();
  });
}

test("backend replacement while awaiting readiness cannot dispatch an old runReady operation", async () => {
  const fixture = createRuntimeFixture();
  const proc = await fixture.openReady("/ws-a", 4001);

  let dispatched = false;
  const pending = fixture.runtime.runReady(async () => {
    dispatched = true;
    return "should-not-run";
  });

  // Invalidate generation synchronously before getReadiness() resolves on the microtask queue.
  proc.emitExit(1);

  await assert.rejects(pending, (error: unknown) =>
    error instanceof StaleBackendGenerationError && /workspace changed/i.test(error.message));
  assert.equal(dispatched, false);
  fixture.runtime.dispose();
});

test("backend replacement aborts old runReady operations and fences late results while new work succeeds", async () => {
  const fixture = createRuntimeFixture();
  await fixture.openReady("/ws-a", 4001);

  const oldResult = deferred<string>();
  let oldSignal: AbortSignal | undefined;
  const pending = fixture.runtime.runReady(async ({ port, token, signal }) => {
    assert.equal(port, 4001);
    assert.equal(token, "token-1");
    oldSignal = signal;
    return oldResult.promise;
  });
  await settle();
  assert.equal(oldSignal?.aborted, false);

  // Switch to a replacement workspace.
  const switching = fixture.runtime.openWorkspace("/ws-b");
  await settle();
  assert.equal(oldSignal?.aborted, true);
  assert.equal(fixture.processes.length, 2);
  fixture.processes[1].emitReady(4002);
  const switchResult = await switching;
  assert.equal(switchResult.ok, true);

  const current = await fixture.runtime.runReady(async ({ port, token, signal, workspacePath }) => {
    assert.equal(port, 4002);
    assert.equal(token, "token-2");
    assert.equal(workspacePath, "/ws-b");
    assert.equal(signal.aborted, false);
    return "current result";
  });

  oldResult.resolve("obsolete result");
  await assert.rejects(pending, (error: unknown) =>
    error instanceof StaleBackendGenerationError && /workspace changed/i.test(error.message));
  assert.equal(current, "current result");
  fixture.runtime.dispose();
});

test("runReady enforces expectedWorkspacePath, expectedBackendUrl, and custom stale messages", async () => {
  const fixture = createRuntimeFixture();
  await fixture.openReady("/ws-a", 4001);

  await assert.rejects(
    fixture.runtime.runReady(async () => "never", {
      expectedWorkspacePath: "/ws-other",
      staleMessage: "Custom stale message",
    }),
    (error: unknown) => error instanceof StaleBackendGenerationError && error.message === "Custom stale message",
  );

  await assert.rejects(
    fixture.runtime.runReady(async () => "never", {
      expectedWorkspacePath: "/ws-a",
      expectedBackendUrl: "http://127.0.0.1:9999",
    }),
    (error: unknown) => error instanceof StaleBackendGenerationError,
  );

  const ok = await fixture.runtime.runReady(
    async ({ url, workspacePath }) => ({ url, workspacePath }),
    { expectedWorkspacePath: "/ws-a", expectedBackendUrl: "http://127.0.0.1:4001" },
  );
  assert.deepEqual(ok, { url: "http://127.0.0.1:4001", workspacePath: "/ws-a" });
  fixture.runtime.dispose();
});
