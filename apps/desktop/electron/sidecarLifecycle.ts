export type SidecarState = "starting" | "ready" | "failed" | "retrying" | "exhausted";

export type SidecarCleanupFailureCode = "cleanup_timeout" | "cleanup_failed";

export class SidecarCleanupError extends Error {
  readonly code: SidecarCleanupFailureCode;

  constructor(code: SidecarCleanupFailureCode, message: string) {
    super(message);
    this.name = "SidecarCleanupError";
    this.code = code;
  }
}

type Listener = (...args: any[]) => void;

export interface SidecarProcess {
  stdout?: { on(event: "data", listener: Listener): unknown };
  on(event: "exit" | "error", listener: Listener): unknown;
  kill(): void;
}

export interface SidecarLifecycle {
  start(cwd: string): Promise<number>;
  stop(): Promise<void>;
  stopAndWait(timeoutMs: number): Promise<void>;
  retry(): Promise<number>;
  getPort(): number | null;
  dispose(): void;
}

export interface SidecarLifecycleOptions {
  spawn(cwd: string): SidecarProcess;
  setTimeout(callback: () => void, delayMs: number): unknown;
  clearTimeout(timer: unknown): void;
  now(): number;
  callbacks: {
    onReady(port: number): void;
    onUnavailable(message: string): void;
    onUnexpectedExit?(message: string): void;
    onState(state: SidecarState): void;
  };
  readinessTimeoutMs?: number;
  cleanupTimeoutMs?: number;
}

interface Generation {
  readonly id: number;
  process: SidecarProcess | null;
  readonly readiness: Promise<number>;
  resolve(port: number): void;
  reject(error: Error): void;
  readinessTimer: unknown;
  ready: boolean;
  failed: boolean;
  exited: boolean;
  processError: Error | null;
  killRequested: boolean;
  stopWait: Promise<void> | null;
  stdout: string;
  cleanup: Promise<void>;
  resolveCleanup(): void;
  rejectCleanup(error: SidecarCleanupError): void;
  cleanupSettled: boolean;
  cleanupFailure: SidecarCleanupError | null;
  cleanupTimer: unknown;
  stopping: boolean;
}

const DEFAULT_READINESS_TIMEOUT_MS = 10_000;
const DEFAULT_CLEANUP_TIMEOUT_MS = 2_000;
const MAX_READINESS_OUTPUT_LENGTH = 64 * 1024;

export function createSidecarLifecycle(options: SidecarLifecycleOptions): SidecarLifecycle {
  let generation = 0;
  let current: Generation | null = null;
  let port: number | null = null;
  let lastCwd: string | null = null;
  let disposed = false;
  let stoppingGeneration: Generation | null = null;

  const readinessTimeoutMs = options.readinessTimeoutMs ?? DEFAULT_READINESS_TIMEOUT_MS;
  const cleanupTimeoutMs = options.cleanupTimeoutMs ?? DEFAULT_CLEANUP_TIMEOUT_MS;

  function setState(next: SidecarState): void {
    options.callbacks.onState(next);
  }

  function isCurrent(candidate: Generation): boolean {
    return !disposed && current?.id === candidate.id && !candidate.stopping;
  }

  function clearTimer(timer: unknown): void {
    if (timer !== null) options.clearTimeout(timer);
  }

  function settleCleanup(candidate: Generation, error?: SidecarCleanupError): void {
    if (candidate.cleanupSettled) return;
    candidate.cleanupSettled = true;
    candidate.cleanupFailure = error ?? null;
    clearTimer(candidate.cleanupTimer);
    candidate.cleanupTimer = null;
    if (error) candidate.rejectCleanup(error);
    else candidate.resolveCleanup();
  }

  function terminate(candidate: Generation): void {
    if (candidate.exited || candidate.killRequested || !candidate.process) return;
    candidate.killRequested = true;
    try {
      candidate.process.kill();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "Sidecar cleanup failed";
      settleCleanup(candidate, new SidecarCleanupError("cleanup_failed", message));
    }
  }

  function errorFrom(value: unknown): Error {
    return value instanceof Error ? value : new Error("Sidecar launch failed");
  }

  function stopCurrent(message: string, timeoutMs = cleanupTimeoutMs): Promise<void> {
    const previous = current;
    if (!previous) return Promise.resolve();

    // A process exit is not an ownership receipt: descendants may still be
    // running after the sidecar object has exited. With no generation-bound
    // native ownership proof available yet, preserve the unresolved failure
    // and fail closed for every replacement path.
    if (previous.cleanupSettled && previous.cleanupFailure) {
      return previous.cleanup;
    }

    port = null;
    clearTimer(previous.readinessTimer);
    if (!previous.ready && !previous.failed) {
      previous.failed = true;
      previous.reject(new Error(message));
    }
    previous.stopping = true;
    terminate(previous);
    if (!previous.process || previous.exited) {
      settleCleanup(previous);
      current = null;
      return Promise.resolve();
    }
    if (previous.cleanupSettled) return previous.cleanup;
    previous.cleanupTimer = options.setTimeout(() => {
      settleCleanup(previous, new SidecarCleanupError("cleanup_timeout", "Sidecar cleanup timed out"));
    }, timeoutMs);
    return previous.cleanup;
  }

  function fail(candidate: Generation, error: Error): void {
    if (!isCurrent(candidate) || candidate.failed) return;

    candidate.failed = true;
    port = null;
    clearTimer(candidate.readinessTimer);
    setState("failed");
    options.callbacks.onUnavailable(error.message);
    if (!candidate.ready) candidate.reject(error);
    terminate(candidate);
  }

  function ready(candidate: Generation, nextPort: number): void {
    if (!isCurrent(candidate) || candidate.failed || candidate.ready) return;

    candidate.ready = true;
    port = nextPort;
    clearTimer(candidate.readinessTimer);
    setState("ready");
    candidate.resolve(nextPort);
    options.callbacks.onReady(nextPort);
  }

  function launch(cwd: string): Promise<number> {
    if (disposed) return Promise.reject(new Error("Sidecar lifecycle has been disposed"));
    if (stoppingGeneration?.process && !stoppingGeneration.exited) {
      return Promise.reject(new Error("Sidecar exit has not been confirmed. Restart OrkWorks to recover."));
    }

    generation += 1;
    const id = generation;
    setState("starting");

    let resolve!: (port: number) => void;
    let reject!: (error: Error) => void;
    let resolveCleanup!: () => void;
    let rejectCleanup!: (error: SidecarCleanupError) => void;
    const readiness = new Promise<number>((resolvePromise, rejectPromise) => {
      resolve = resolvePromise;
      reject = rejectPromise;
    });
    const cleanup = new Promise<void>((resolvePromise, rejectPromise) => {
      resolveCleanup = resolvePromise;
      rejectCleanup = rejectPromise;
    });
    void cleanup.catch(() => {});
    const candidate: Generation = {
      id,
      process: null,
      readiness,
      resolve,
      reject,
      readinessTimer: null,
      ready: false,
      failed: false,
      exited: false,
      processError: null,
      killRequested: false,
      stopWait: null,
      stdout: "",
      cleanup,
      resolveCleanup,
      rejectCleanup,
      cleanupSettled: false,
      cleanupFailure: null,
      cleanupTimer: null,
      stopping: false,
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
        candidate.stdout = (candidate.stdout + data.toString()).slice(-MAX_READINESS_OUTPUT_LENGTH);
        const match = candidate.stdout.match(/ORKWORKSD_PORT=(-?\d+)/);
        if (match) {
          const announcedPort = Number.parseInt(match[1], 10);
          if (announcedPort >= 1 && announcedPort <= 65_535) {
            ready(candidate, announcedPort);
          } else {
            fail(candidate, new Error("invalid port announcement"));
          }
        }
      });
      candidate.process.on("error", (error: Error) => {
        candidate.processError = errorFrom(error);
        fail(candidate, candidate.processError);
      });
      candidate.process.on("exit", (code: number | null) => {
        candidate.exited = true;
        const message = candidate.ready
          ? `Sidecar exited with code ${code ?? "unknown"}`
          : `Sidecar exited before readiness with code ${code ?? "unknown"}`;
        if (!candidate.stopping) {
          const unresolved = new SidecarCleanupError(
            "cleanup_failed",
            `${message}; runtime ownership is unresolved`,
          );
          fail(candidate, unresolved);
          settleCleanup(candidate, unresolved);
          options.callbacks.onUnexpectedExit?.(unresolved.message);
          return;
        }
        settleCleanup(candidate);
        if (current?.id === candidate.id && !candidate.cleanupFailure) current = null;
      });
    } catch (error) {
      fail(candidate, errorFrom(error));
    }

    return readiness;
  }

  return {
    start(cwd: string): Promise<number> {
      lastCwd = cwd;
      const previous = current;
      const cleanup = stopCurrent("Sidecar stopped before readiness");
      if (!previous) return launch(cwd);
      if (previous.cleanupSettled) {
        return previous.cleanupFailure ? Promise.reject(previous.cleanupFailure) : launch(cwd);
      }
      return cleanup.then(() => launch(cwd));
    },

    stop(): Promise<void> {
      generation += 1;
      return stopCurrent("Sidecar stopped before readiness");
    },

    stopAndWait(timeoutMs: number): Promise<void> {
      if (!Number.isFinite(timeoutMs) || timeoutMs < 0) {
        return Promise.reject(new RangeError("timeoutMs must be finite and non-negative"));
      }

      const previous = current ?? stoppingGeneration;
      if (!previous) {
        generation += 1;
        return Promise.resolve();
      }
      if (previous.stopWait) return previous.stopWait;

      stoppingGeneration = previous;
      generation += 1;
      previous.stopWait = stopCurrent("Sidecar stopped before readiness", timeoutMs);
      return previous.stopWait;
    },

    retry(): Promise<number> {
      if (!lastCwd) return Promise.reject(new Error("No sidecar working directory is available for retry"));
      const previous = current;
      const cleanup = stopCurrent("Sidecar stopped before readiness");
      if (!previous) return launch(lastCwd);
      if (previous.cleanupSettled) {
        if (previous.cleanupFailure) return Promise.reject(previous.cleanupFailure);
        return launch(lastCwd);
      }
      return cleanup.then(() => launch(lastCwd!));
    },

    getPort(): number | null {
      return port;
    },

    dispose(): void {
      if (disposed) return;
      disposed = true;
      void stopCurrent("Sidecar lifecycle has been disposed").catch(() => {});
    },
  };
}
