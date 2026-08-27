export interface BackendRestorationSteps<TWorkspace> {
  restoreWorkspace(signal: AbortSignal): Promise<TWorkspace | null>;
  applyRetentionSettings(signal: AbortSignal): Promise<void>;
  syncProviderSettings(signal: AbortSignal): Promise<void>;
}

export interface BackendRestorationCoordinator<TWorkspace> {
  beginGeneration(): void;
  restore(port: number, steps: BackendRestorationSteps<TWorkspace>): void;
  fail(error: Error): void;
  getReadiness(): Promise<number>;
  getRestoredWorkspace(): TWorkspace | null;
  dispose(): void;
}

interface RestorationGeneration<TWorkspace> {
  readonly id: number;
  readonly controller: AbortController;
  readonly readiness: Promise<number>;
  resolve(port: number): void;
  reject(error: Error): void;
  timer: unknown;
  settled: boolean;
  failure: Error | null;
  workspace: TWorkspace | null;
}

export interface BackendRestorationOptions<TWorkspace> {
  setTimeout(callback: () => void, delayMs: number): unknown;
  clearTimeout(timer: unknown): void;
  onReady(port: number, workspace: TWorkspace | null): void;
  onFailure(error: Error): void;
  timeoutMs?: number;
}

const DEFAULT_RESTORATION_TIMEOUT_MS = 10_000;

function errorFrom(value: unknown): Error {
  return value instanceof Error ? value : new Error("Backend restoration failed");
}

export function createBackendRestorationCoordinator<TWorkspace>(
  options: BackendRestorationOptions<TWorkspace>,
): BackendRestorationCoordinator<TWorkspace> {
  let nextGeneration = 0;
  let current: RestorationGeneration<TWorkspace> | null = null;
  let restoredWorkspace: TWorkspace | null = null;
  let disposed = false;
  const timeoutMs = options.timeoutMs ?? DEFAULT_RESTORATION_TIMEOUT_MS;

  function isCurrent(candidate: RestorationGeneration<TWorkspace>): boolean {
    return !disposed && current?.id === candidate.id;
  }

  function clearTimer(candidate: RestorationGeneration<TWorkspace>): void {
    if (candidate.timer !== null) {
      options.clearTimeout(candidate.timer);
      candidate.timer = null;
    }
  }

  function abort(candidate: RestorationGeneration<TWorkspace>, error: Error): void {
    clearTimer(candidate);
    candidate.failure = error;
    if (!candidate.controller.signal.aborted) candidate.controller.abort(error);
    if (!candidate.settled) {
      candidate.settled = true;
      candidate.reject(error);
    }
  }

  function failGeneration(candidate: RestorationGeneration<TWorkspace>, error: Error): void {
    if (!isCurrent(candidate) || candidate.failure) return;
    restoredWorkspace = null;
    abort(candidate, error);
    options.onFailure(error);
  }

  function assertCurrent(candidate: RestorationGeneration<TWorkspace>): boolean {
    return isCurrent(candidate) && !candidate.controller.signal.aborted && !candidate.failure;
  }

  return {
    beginGeneration(): void {
      if (disposed) throw new Error("Backend restoration coordinator has shut down");
      if (current) abort(current, new Error("Backend generation was replaced"));

      let resolve!: (port: number) => void;
      let reject!: (error: Error) => void;
      const readiness = new Promise<number>((resolvePromise, rejectPromise) => {
        resolve = resolvePromise;
        reject = rejectPromise;
      });
      void readiness.catch(() => {});

      current = {
        id: ++nextGeneration,
        controller: new AbortController(),
        readiness,
        resolve,
        reject,
        timer: null,
        settled: false,
        failure: null,
        workspace: null,
      };
      restoredWorkspace = null;
    },

    restore(port: number, steps: BackendRestorationSteps<TWorkspace>): void {
      const candidate = current;
      if (!candidate || candidate.failure || disposed) return;

      candidate.timer = options.setTimeout(() => {
        failGeneration(candidate, new Error("Backend restoration timed out"));
      }, timeoutMs);

      void (async () => {
        const workspace = await steps.restoreWorkspace(candidate.controller.signal);
        if (!assertCurrent(candidate)) return;
        await steps.applyRetentionSettings(candidate.controller.signal);
        if (!assertCurrent(candidate)) return;
        await steps.syncProviderSettings(candidate.controller.signal);
        if (!assertCurrent(candidate)) return;

        clearTimer(candidate);
        candidate.workspace = workspace;
        restoredWorkspace = workspace;
        candidate.settled = true;
        candidate.resolve(port);
        options.onReady(port, workspace);
      })().catch((error: unknown) => {
        if (!assertCurrent(candidate)) return;
        failGeneration(candidate, errorFrom(error));
      });
    },

    fail(error: Error): void {
      if (current) failGeneration(current, error);
    },

    getReadiness(): Promise<number> {
      if (!current) return Promise.reject(new Error("Backend generation has not started"));
      if (current.failure) return Promise.reject(current.failure);
      return current.readiness;
    },

    getRestoredWorkspace(): TWorkspace | null {
      return restoredWorkspace;
    },

    dispose(): void {
      if (disposed) return;
      if (current) abort(current, new Error("Backend restoration coordinator shut down"));
      current = null;
      restoredWorkspace = null;
      disposed = true;
    },
  };
}

export function switchWorkspaceBackend<TResult>(
  workspacePath: string,
  persist: (workspacePath: string) => void,
  startReplacement: (workspacePath: string) => TResult,
): TResult {
  persist(workspacePath);
  return startReplacement(workspacePath);
}

