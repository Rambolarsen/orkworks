export type WorkspaceInstanceState = "picker" | "opening" | "ready" | "closing" | "unresolved";

export type WorkspaceSwitchFailureCode =
  | "invalid_destination"
  | "cleanup_failed"
  | "cleanup_timeout"
  | "destination_conflict"
  | "readiness_failed"
  | "restoration_failed"
  | "quit_failed";

export class WorkspaceSwitchError extends Error {
  readonly code: WorkspaceSwitchFailureCode;

  constructor(code: WorkspaceSwitchFailureCode, message: string) {
    super(message);
    this.name = "WorkspaceSwitchError";
    this.code = code;
  }
}

export interface WorkspaceRuntime<TWorkspace> {
  workspace: TWorkspace;
  port: number;
}

export interface WorkspaceSwitchFailure {
  code: WorkspaceSwitchFailureCode;
  message: string;
}

export type WorkspaceSwitchEvent<TWorkspace, THistoryDiagnostic = unknown> =
  | { state: "picker"; generation: number; failure?: WorkspaceSwitchFailure }
  | { state: "opening"; generation: number; path: string }
  | { state: "closing"; generation: number; path: string }
  | { state: "ready"; generation: number; path: string; workspace: TWorkspace; port: number; historyDiagnostic: THistoryDiagnostic | null }
  | { state: "unresolved"; generation: number; failure: WorkspaceSwitchFailure };

export type WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic = unknown> =
  | {
    ok: true;
    state: "ready";
    generation: number;
    path: string;
    workspace: TWorkspace;
    port: number;
    historyDiagnostic: THistoryDiagnostic | null;
  }
  | {
    ok: false;
    state: WorkspaceInstanceState;
    generation: number;
    failure: WorkspaceSwitchFailure;
  };

export interface WorkspaceSwitchCoordinatorOptions<TWorkspace, THistoryDiagnostic = unknown> {
  initialWorkspacePath: string | null;
  validateDestination(path: string): string | null;
  setWorkspacePath(path: string | null): void;
  closeCurrentRuntime(): Promise<void>;
  startRuntime(path: string, generation: number): Promise<WorkspaceRuntime<TWorkspace>>;
  cleanupAttemptedRuntime(): Promise<void>;
  rememberWorkspace(path: string, workspace: TWorkspace): THistoryDiagnostic | null | Promise<THistoryDiagnostic | null>;
  publish(event: WorkspaceSwitchEvent<TWorkspace, THistoryDiagnostic>): void;
  onQuit?(): void | Promise<void>;
}

export interface WorkspaceSwitchCoordinator<TWorkspace, THistoryDiagnostic = unknown> {
  getState(): WorkspaceInstanceState;
  isCurrentGeneration(generation: number): boolean;
  getCurrentWorkspacePath(): string | null;
  markUnresolved(failure: WorkspaceSwitchFailure): void;
  switchWorkspace(path: string): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic>>;
  pickWorkspace(select: () => Promise<string | null>): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic> | null>;
  retry(): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic>>;
  close(): Promise<{ ok: true; state: "picker"; generation: number } | { ok: false; state: "unresolved"; generation: number; failure: WorkspaceSwitchFailure }>;
  quit(): Promise<{ ok: true; state: "picker"; generation: number } | { ok: false; state: "unresolved"; generation: number; failure: WorkspaceSwitchFailure }>;
}

function errorFrom(value: unknown): Error {
  return value instanceof Error ? value : new Error("Workspace switch failed");
}

function failureFrom(value: unknown, fallbackCode: WorkspaceSwitchFailureCode): WorkspaceSwitchFailure {
  if (value instanceof WorkspaceSwitchError) return { code: value.code, message: value.message };
  const error = errorFrom(value);
  if ((error as { code?: unknown }).code === "cleanup_timeout") {
    return { code: "cleanup_timeout", message: error.message };
  }
  return { code: fallbackCode, message: error.message };
}

export function createWorkspaceSwitchCoordinator<TWorkspace, THistoryDiagnostic = unknown>(
  options: WorkspaceSwitchCoordinatorOptions<TWorkspace, THistoryDiagnostic>,
): WorkspaceSwitchCoordinator<TWorkspace, THistoryDiagnostic> {
  let state: WorkspaceInstanceState = options.initialWorkspacePath ? "ready" : "picker";
  let currentWorkspacePath = options.initialWorkspacePath;
  let lastDestination = options.initialWorkspacePath;
  let attemptedRuntimeCleanupFailure: WorkspaceSwitchFailure | null = null;
  let generation = 0;
  let operationTail: Promise<void> = Promise.resolve();

  function publish(event: WorkspaceSwitchEvent<TWorkspace, THistoryDiagnostic>): void {
    state = event.state;
    options.publish(event);
  }

  function enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = operationTail.then(operation, operation);
    operationTail = result.then(() => undefined, () => undefined);
    return result;
  }

  async function closeCurrent(nextGeneration: number): Promise<{ ok: true } | { ok: false; failure: WorkspaceSwitchFailure }> {
    if (!currentWorkspacePath) return { ok: true };

    const path = currentWorkspacePath;
    publish({ state: "closing", generation: nextGeneration, path });
    try {
      await options.closeCurrentRuntime();
    } catch (error: unknown) {
      const failure = failureFrom(error, "cleanup_failed");
      publish({ state: "unresolved", generation: nextGeneration, failure });
      return { ok: false, failure };
    }

    currentWorkspacePath = null;
    options.setWorkspacePath(null);
    publish({ state: "picker", generation: nextGeneration });
    return { ok: true };
  }

  async function switchWorkspaceInternal(path: string): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic>> {
    if (attemptedRuntimeCleanupFailure) {
      return {
        ok: false,
        state: "unresolved",
        generation,
        failure: attemptedRuntimeCleanupFailure,
      };
    }

    const validatedPath = options.validateDestination(path);
    if (!validatedPath) {
      const failure: WorkspaceSwitchFailure = {
        code: "invalid_destination",
        message: "The selected workspace is not an accessible directory.",
      };
      return {
        ok: false,
        state,
        generation,
        failure,
      };
    }

    const nextGeneration = ++generation;
    lastDestination = validatedPath;
    const closed = await closeCurrent(nextGeneration);
    if (!closed.ok) {
      return { ok: false, state: "unresolved", generation: nextGeneration, failure: closed.failure };
    }

    publish({ state: "opening", generation: nextGeneration, path: validatedPath });
    let runtime: WorkspaceRuntime<TWorkspace>;
    try {
      runtime = await options.startRuntime(validatedPath, nextGeneration);
    } catch (error: unknown) {
      const failure = failureFrom(error, "restoration_failed");
      try {
        await options.cleanupAttemptedRuntime();
      } catch (cleanupError: unknown) {
        const cleanupFailure = failureFrom(cleanupError, "cleanup_failed");
        attemptedRuntimeCleanupFailure = cleanupFailure;
        publish({ state: "unresolved", generation: nextGeneration, failure: cleanupFailure });
        return { ok: false, state: "unresolved", generation: nextGeneration, failure: cleanupFailure };
      }
      if (attemptedRuntimeCleanupFailure) {
        publish({ state: "unresolved", generation, failure: attemptedRuntimeCleanupFailure });
        return { ok: false, state: "unresolved", generation, failure: attemptedRuntimeCleanupFailure };
      }
      publish({ state: "picker", generation: nextGeneration, failure });
      return { ok: false, state: "picker", generation: nextGeneration, failure };
    }

    if (nextGeneration !== generation) {
      const failure: WorkspaceSwitchFailure = {
        code: "restoration_failed",
        message: "Workspace opening was superseded before readiness was published.",
      };
      try {
        await options.cleanupAttemptedRuntime();
      } catch (cleanupError: unknown) {
        const cleanupFailure = failureFrom(cleanupError, "cleanup_failed");
        attemptedRuntimeCleanupFailure = cleanupFailure;
        publish({ state: "unresolved", generation: nextGeneration, failure: cleanupFailure });
        return { ok: false, state: "unresolved", generation: nextGeneration, failure: cleanupFailure };
      }
      if (attemptedRuntimeCleanupFailure) {
        publish({ state: "unresolved", generation, failure: attemptedRuntimeCleanupFailure });
        return { ok: false, state: "unresolved", generation, failure: attemptedRuntimeCleanupFailure };
      }
      return { ok: false, state: "picker", generation: nextGeneration, failure };
    }

    let historyDiagnostic: THistoryDiagnostic | null = null;
    try {
      historyDiagnostic = await options.rememberWorkspace(validatedPath, runtime.workspace);
    } catch {
      historyDiagnostic = null;
    }
    currentWorkspacePath = validatedPath;
    options.setWorkspacePath(validatedPath);
    publish({
      state: "ready",
      generation: nextGeneration,
      path: validatedPath,
      workspace: runtime.workspace,
      port: runtime.port,
      historyDiagnostic,
    });
    return {
      ok: true,
      state: "ready",
      generation: nextGeneration,
      path: validatedPath,
      workspace: runtime.workspace,
      port: runtime.port,
      historyDiagnostic,
    };
  }

  async function closeInternal(): Promise<{ ok: true; state: "picker"; generation: number } | { ok: false; state: "unresolved"; generation: number; failure: WorkspaceSwitchFailure }> {
    const nextGeneration = ++generation;
    if (attemptedRuntimeCleanupFailure) {
      publish({ state: "unresolved", generation: nextGeneration, failure: attemptedRuntimeCleanupFailure });
      return { ok: false, state: "unresolved", generation: nextGeneration, failure: attemptedRuntimeCleanupFailure };
    }
    if (!currentWorkspacePath) {
      publish({ state: "picker", generation: nextGeneration });
      return { ok: true, state: "picker", generation: nextGeneration };
    }

    const closed = await closeCurrent(nextGeneration);
    if (!closed.ok) return { ok: false, state: "unresolved", generation: nextGeneration, failure: closed.failure };
    return { ok: true, state: "picker", generation: nextGeneration };
  }

  return {
    getState(): WorkspaceInstanceState {
      return state;
    },

    isCurrentGeneration(candidate: number): boolean {
      return candidate === generation;
    },

    markUnresolved(failure: WorkspaceSwitchFailure): void {
      generation += 1;
      attemptedRuntimeCleanupFailure = failure;
      publish({ state: "unresolved", generation, failure });
    },

    getCurrentWorkspacePath(): string | null {
      return currentWorkspacePath;
    },

    switchWorkspace(path: string): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic>> {
      return enqueue(() => switchWorkspaceInternal(path));
    },

    pickWorkspace(select: () => Promise<string | null>): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic> | null> {
      return enqueue(async () => {
        const path = await select();
        return path === null ? null : switchWorkspaceInternal(path);
      });
    },

    retry(): Promise<WorkspaceSwitchResult<TWorkspace, THistoryDiagnostic>> {
      return enqueue(async () => {
        if (attemptedRuntimeCleanupFailure) {
          try {
            await options.cleanupAttemptedRuntime();
            attemptedRuntimeCleanupFailure = null;
            publish({ state: "picker", generation });
          } catch (error: unknown) {
            const failure = failureFrom(error, "cleanup_failed");
            attemptedRuntimeCleanupFailure = failure;
            publish({ state: "unresolved", generation, failure });
            return { ok: false, state: "unresolved", generation, failure };
          }
        }
        if (!lastDestination) {
          const failure: WorkspaceSwitchFailure = {
            code: "invalid_destination",
            message: "Choose a workspace before retrying.",
          };
          return { ok: false, state: "picker", generation, failure };
        }
        return switchWorkspaceInternal(lastDestination);
      });
    },

    close(): ReturnType<WorkspaceSwitchCoordinator<TWorkspace, THistoryDiagnostic>["close"]> {
      return enqueue(closeInternal);
    },

    quit(): ReturnType<WorkspaceSwitchCoordinator<TWorkspace, THistoryDiagnostic>["quit"]> {
      return enqueue(async () => {
        const result = await closeInternal();
        if (!result.ok) return result;
        try {
          await options.onQuit?.();
        } catch (error: unknown) {
          const failure = failureFrom(error, "quit_failed");
          publish({ state: "unresolved", generation: result.generation, failure });
          return { ok: false, state: "unresolved", generation: result.generation, failure };
        }
        return result;
      });
    },
  };
}
