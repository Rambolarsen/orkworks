export type SidecarState = "starting" | "ready" | "failed" | "retrying" | "exhausted";

type Listener = (...args: any[]) => void;

export interface SidecarProcess {
  stdout?: { on(event: "data", listener: Listener): unknown };
  on(event: "exit" | "error", listener: Listener): unknown;
  kill(): void;
}

export interface SidecarLifecycle {
  start(cwd: string): Promise<number>;
  stop(): void;
  retry(): Promise<number>;
  getPort(): number | null;
  dispose(): void;
}

export interface SidecarLifecycleOptions {
  spawn(cwd: string): SidecarProcess;
  fetch: typeof globalThis.fetch;
  setTimeout(callback: () => void, delayMs: number): unknown;
  clearTimeout(timer: unknown): void;
  now(): number;
  callbacks: {
    onReady(port: number): void;
    onUnavailable(message: string): void;
    onState(state: SidecarState): void;
  };
  readinessTimeoutMs?: number;
  retryDelaysMs?: readonly number[];
  readyStabilityMs?: number;
}

interface Generation {
  readonly id: number;
  process: SidecarProcess | null;
  readonly readiness: Promise<number>;
  resolve(port: number): void;
  reject(error: Error): void;
  readinessTimer: unknown;
  stabilityTimer: unknown;
  ready: boolean;
  failed: boolean;
  exited: boolean;
  killRequested: boolean;
  readyAtMs: number | null;
  stdout: string;
}

const DEFAULT_READINESS_TIMEOUT_MS = 10_000;
const DEFAULT_RETRY_DELAYS_MS = [250, 1_000] as const;
const DEFAULT_READY_STABILITY_MS = 5_000;
const MAX_AUTOMATIC_ATTEMPTS = 3;

export function createSidecarLifecycle(options: SidecarLifecycleOptions): SidecarLifecycle {
  // Task 3 uses this dependency while wiring workspace restoration. Keeping it
  // in the injected contract lets this controller remain Electron-free.
  void options.fetch;

  let generation = 0;
  let current: Generation | null = null;
  let port: number | null = null;
  let attempts = 0;
  let lastCwd: string | null = null;
  let recoveryTimer: unknown = null;
  let disposed = false;

  const readinessTimeoutMs = options.readinessTimeoutMs ?? DEFAULT_READINESS_TIMEOUT_MS;
  const retryDelaysMs = options.retryDelaysMs ?? DEFAULT_RETRY_DELAYS_MS;
  const readyStabilityMs = options.readyStabilityMs ?? DEFAULT_READY_STABILITY_MS;

  function setState(next: SidecarState): void {
    options.callbacks.onState(next);
  }

  function isCurrent(candidate: Generation): boolean {
    return !disposed && current?.id === candidate.id;
  }

  function clearTimer(timer: unknown): void {
    if (timer !== null) options.clearTimeout(timer);
  }

  function cancelRecovery(): void {
    clearTimer(recoveryTimer);
    recoveryTimer = null;
  }

  function terminate(candidate: Generation): void {
    if (candidate.exited || candidate.killRequested || !candidate.process) return;
    candidate.killRequested = true;
    candidate.process.kill();
  }

  function errorFrom(value: unknown): Error {
    return value instanceof Error ? value : new Error("Sidecar launch failed");
  }

  function stopCurrent(message: string): void {
    const previous = current;
    if (!previous) return;

    current = null;
    port = null;
    clearTimer(previous.readinessTimer);
    clearTimer(previous.stabilityTimer);
    if (!previous.ready && !previous.failed) {
      previous.failed = true;
      previous.reject(new Error(message));
    }
    terminate(previous);
  }

  function scheduleRecovery(candidate: Generation): void {
    if (!isCurrent(candidate) || recoveryTimer !== null) return;
    if (attempts >= MAX_AUTOMATIC_ATTEMPTS) {
      setState("exhausted");
      return;
    }

    const delay = retryDelaysMs[Math.min(attempts - 1, retryDelaysMs.length - 1)] ?? 0;
    setState("retrying");
    recoveryTimer = options.setTimeout(() => {
      recoveryTimer = null;
      if (!isCurrent(candidate) || !lastCwd) return;
      void launch(lastCwd).catch(() => {
        // Automatic recovery failures are surfaced through callbacks and may
        // schedule the next bounded attempt; no caller owns this promise.
      });
    }, delay);
  }

  function fail(candidate: Generation, error: Error): void {
    if (!isCurrent(candidate) || candidate.failed) return;

    candidate.failed = true;
    port = null;
    clearTimer(candidate.readinessTimer);
    clearTimer(candidate.stabilityTimer);
    setState("failed");
    options.callbacks.onUnavailable(error.message);
    if (!candidate.ready) candidate.reject(error);
    terminate(candidate);
    scheduleRecovery(candidate);
  }

  function resetAttemptsAfterStability(candidate: Generation): void {
    if (!isCurrent(candidate) || !candidate.ready || candidate.failed || candidate.readyAtMs === null) return;
    const remainingMs = readyStabilityMs - (options.now() - candidate.readyAtMs);
    if (remainingMs > 0) {
      candidate.stabilityTimer = options.setTimeout(() => {
        resetAttemptsAfterStability(candidate);
      }, remainingMs);
      return;
    }
    attempts = 0;
  }

  function ready(candidate: Generation, nextPort: number): void {
    if (!isCurrent(candidate) || candidate.failed || candidate.ready) return;

    candidate.ready = true;
    port = nextPort;
    clearTimer(candidate.readinessTimer);
    candidate.readyAtMs = options.now();
    setState("ready");
    candidate.resolve(nextPort);
    options.callbacks.onReady(nextPort);
    candidate.stabilityTimer = options.setTimeout(() => {
      resetAttemptsAfterStability(candidate);
    }, readyStabilityMs);
  }

  function launch(cwd: string): Promise<number> {
    if (disposed) return Promise.reject(new Error("Sidecar lifecycle has been disposed"));

    attempts += 1;
    generation += 1;
    const id = generation;
    setState("starting");

    let resolve!: (port: number) => void;
    let reject!: (error: Error) => void;
    const readiness = new Promise<number>((resolvePromise, rejectPromise) => {
      resolve = resolvePromise;
      reject = rejectPromise;
    });
    const candidate: Generation = {
      id,
      process: null,
      readiness,
      resolve,
      reject,
      readinessTimer: null,
      stabilityTimer: null,
      ready: false,
      failed: false,
      exited: false,
      killRequested: false,
      readyAtMs: null,
      stdout: "",
    };
    current = candidate;
    port = null;

    try {
      candidate.process = options.spawn(cwd);
      candidate.readinessTimer = options.setTimeout(() => {
        fail(candidate, new Error("Sidecar readiness timed out"));
      }, readinessTimeoutMs);
      candidate.process.stdout?.on("data", (data: Buffer | string) => {
        if (!isCurrent(candidate) || candidate.failed) return;
        candidate.stdout += data.toString();
        const match = candidate.stdout.match(/ORKWORKSD_PORT=(\d+)/);
        if (match) ready(candidate, Number.parseInt(match[1], 10));
      });
      candidate.process.on("error", (error: Error) => {
        fail(candidate, error);
      });
      candidate.process.on("exit", (code: number | null) => {
        candidate.exited = true;
        const message = candidate.ready
          ? `Sidecar exited with code ${code ?? "unknown"}`
          : `Sidecar exited before readiness with code ${code ?? "unknown"}`;
        fail(candidate, new Error(message));
      });
    } catch (error) {
      fail(candidate, errorFrom(error));
    }

    return readiness;
  }

  return {
    start(cwd: string): Promise<number> {
      cancelRecovery();
      stopCurrent("Sidecar stopped before readiness");
      attempts = 0;
      lastCwd = cwd;
      return launch(cwd);
    },

    stop(): void {
      cancelRecovery();
      generation += 1;
      stopCurrent("Sidecar stopped before readiness");
    },

    retry(): Promise<number> {
      if (!lastCwd) return Promise.reject(new Error("No sidecar working directory is available for retry"));
      cancelRecovery();
      stopCurrent("Sidecar stopped before readiness");
      attempts = 0;
      return launch(lastCwd);
    },

    getPort(): number | null {
      return port;
    },

    dispose(): void {
      if (disposed) return;
      disposed = true;
      cancelRecovery();
      stopCurrent("Sidecar lifecycle has been disposed");
    },
  };
}
