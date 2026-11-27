import { randomBytes } from "crypto";
import type { WorkspaceGuardSnapshot } from "./activeHarnessIntegration.ts";
import type {
  BackendLifecycleEvent,
  BackendLifecycleWorkspace,
  BackendRetryResult,
  InitialWorkspaceSnapshot,
  WorkspaceHistoryDiagnostic,
  WorkspaceLifecycleFailure,
} from "./backendLifecycleEvent.ts";
import {
  createBackendRestorationCoordinator,
  WorkspaceRestorationFailure,
} from "./backendRestoration.ts";
import { providerSettingsSyncError, pushProviderSettings } from "./providerSettingsSync.ts";
import type { ProviderDefinition } from "./providerTypes.ts";
import { DEFAULT_RETENTION, normalizeProviderSettings, type AppSettings, type RetentionSettings } from "./settingsMemory.ts";
import { sidecarAccessHeaders, sidecarSend, sidecarUrl } from "./sidecarClient.ts";
import { createSidecarLifecycle, type SidecarProcess, type SidecarState } from "./sidecarLifecycle.ts";
import {
  buildWorkspaceRestoreRequest,
  parseWorkspaceRestoreResponse,
  sameWorkspaceIdentity,
  workspaceHistoryPath,
} from "./workspaceRestore.ts";
import {
  createWorkspaceSwitchCoordinator,
  WorkspaceSwitchError,
  type WorkspaceSwitchEvent,
  type WorkspaceSwitchResult,
} from "./workspaceSwitchCoordinator.ts";

export const BACKEND_UNAVAILABLE_MESSAGE = "The OrkWorks sidecar is unavailable.";
export const STALE_BACKEND_GENERATION_MESSAGE =
  "The workspace changed before this request could run. Reload the current workspace and retry.";
export const WORKSPACE_TRANSITION_IN_PROGRESS_MESSAGE = "Workspace transition is in progress";

export class StaleBackendGenerationError extends Error {
  constructor(message = STALE_BACKEND_GENERATION_MESSAGE) {
    super(message);
    this.name = "StaleBackendGenerationError";
  }
}

export interface ReadyBackendContext {
  readonly port: number;
  readonly url: string;
  readonly token: string;
  readonly generation: number;
  readonly workspacePath: string;
  readonly workspaceIdentity: string;
  readonly activeHarnessRevision: number;
  readonly signal: AbortSignal;
  isCurrent(): boolean;
  assertCurrent(message?: string): void;
}

export interface RunReadyOptions {
  expectedWorkspacePath?: string;
  expectedWorkspaceIdentity?: string;
  expectedBackendUrl?: string;
  staleMessage?: string;
  notReadyMessage?: string;
}

export interface ReadyNavigationIdentity {
  readonly workspaceIdentity: string;
  readonly generation: number;
  isCurrent(): boolean;
}

export interface WorkspaceRuntimeTimers {
  setTimeout(callback: () => void, delayMs: number): unknown;
  clearTimeout(timer: unknown): void;
  now(): number;
  readinessTimeoutMs?: number;
  cleanupTimeoutMs?: number;
  restorationTimeoutMs?: number;
}

export interface WorkspaceRuntimeLogger {
  log(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
}

export interface WorkspaceRuntimeSideEffectsContext {
  readonly port: number;
  readonly generation: number;
  readonly workspace: BackendLifecycleWorkspace;
  readonly signal: AbortSignal;
  isCurrent(): boolean;
}

export interface WorkspaceRuntimeOptions {
  spawnSidecar(cwd: string, openPlanToken: string): SidecarProcess;
  generateToken?(): string;
  sidecarSend?: typeof sidecarSend;
  fetch?: typeof fetch;
  validateDestination(candidate: string): string | null;
  canonicalizePath(candidate: string): string | null;
  rememberWorkspacePath(canonicalPath: string): { diagnostic: WorkspaceHistoryDiagnostic | null };
  forgetWorkspacePath(
    rejectedPath: string,
  ): Promise<{ historyDiagnostic: WorkspaceHistoryDiagnostic | null; navigationDiagnostic?: string | null }>;
  getSettings(): AppSettings;
  writeSettings(next: AppSettings): void;
  getRetention?(): RetentionSettings;
  enqueueSettingsWrite<T>(operation: () => T | Promise<T>): Promise<T>;
  clearProviderModelsCache(): void;
  initialHistoryDiagnostic: WorkspaceHistoryDiagnostic | null;
  onLifecycleChange?(event: BackendLifecycleEvent): void;
  onReadySideEffects?(context: WorkspaceRuntimeSideEffectsContext): void;
  onQuit?(): void | Promise<void>;
  timers?: WorkspaceRuntimeTimers;
  logger?: WorkspaceRuntimeLogger;
}

export interface WorkspaceRuntime {
  openWorkspace(path: string): Promise<WorkspaceSwitchResult<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic>>;
  pickWorkspace(
    select: () => Promise<string | null>,
  ): Promise<WorkspaceSwitchResult<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic> | null>;
  retry(): Promise<BackendRetryResult>;
  close(): Promise<
    | { ok: true; state: "picker"; generation: number }
    | { ok: false; state: "unresolved"; generation: number; failure: WorkspaceLifecycleFailure }
  >;
  quit(): Promise<
    | { ok: true; state: "picker"; generation: number }
    | { ok: false; state: "unresolved"; generation: number; failure: WorkspaceLifecycleFailure }
  >;
  runUpdate<T>(operation: (workspacePath: string | null) => Promise<T>): Promise<T>;
  stopSidecarForUpdate(timeoutMs: number): Promise<void>;
  restartSidecarAfterUpdate(fallbackCwd: string): Promise<void>;
  dispose(): void;

  getLifecycle(): BackendLifecycleEvent;
  subscribeLifecycle(listener: (event: BackendLifecycleEvent) => void): () => void;
  getInitialWorkspaceSnapshot(): InitialWorkspaceSnapshot;
  getWorkspacePath(): string | null;
  getGeneration(): number;
  getWorkspaceRevision(): string;
  getActiveHarnessRevision(): number;
  setActiveHarnessRevision(expectedGeneration: number, expectedWorkspacePath: string | null, nextRevision: number): boolean;
  getWorkspaceGuard(): WorkspaceGuardSnapshot;
  getReadyNavigationIdentity(): ReadyNavigationIdentity;
  getReadiness(): Promise<number>;
  getBackendUrl(): Promise<string>;
  getOpenPlanToken(): string;

  runReady<T>(
    operation: (ctx: ReadyBackendContext) => Promise<T>,
    options?: RunReadyOptions,
  ): Promise<T>;
}

const DEFAULT_TIMERS: WorkspaceRuntimeTimers = {
  setTimeout: (callback, delayMs) => setTimeout(callback, delayMs),
  clearTimeout: (timer) => clearTimeout(timer as NodeJS.Timeout),
  now: () => Date.now(),
};

export function createWorkspaceRuntime(options: WorkspaceRuntimeOptions): WorkspaceRuntime {
  const send = options.sidecarSend ?? sidecarSend;
  const fetchImpl = options.fetch ?? fetch;
  const timers = options.timers ?? DEFAULT_TIMERS;
  const logger = options.logger ?? console;
  const generateToken = options.generateToken ?? (() => randomBytes(32).toString("hex"));

  let latestLifecycle: BackendLifecycleEvent = { state: "picker" };
  let workspacePath: string | null = null;
  let workspaceDisplayPath: string | null = null;
  let pendingWorkspaceDisplayPath: string | null = null;
  let currentHistoryDiagnostic: WorkspaceHistoryDiagnostic | null = options.initialHistoryDiagnostic;
  let openPlanToken = "";
  let generation = 0;
  let activeHarnessRevision = 0;
  let generationController = new AbortController();
  const requestControllers = new Set<AbortController>();
  const lifecycleListeners = new Set<(event: BackendLifecycleEvent) => void>();

  function logLifecycleFailure(scope: string, error: unknown): void {
    const detail = error instanceof Error ? error.stack ?? error.message : String(error);
    logger.error("[main] backend lifecycle failure", scope, detail);
  }

  function invalidateGeneration(): void {
    generation += 1;
    if (!generationController.signal.aborted) {
      generationController.abort(new StaleBackendGenerationError(STALE_BACKEND_GENERATION_MESSAGE));
    }
    generationController = new AbortController();
    for (const controller of requestControllers) {
      if (!controller.signal.aborted) {
        controller.abort(new StaleBackendGenerationError(STALE_BACKEND_GENERATION_MESSAGE));
      }
    }
    requestControllers.clear();
  }

  function publishLifecycle(event: BackendLifecycleEvent): void {
    latestLifecycle = event;
    options.onLifecycleChange?.(event);
    for (const listener of lifecycleListeners) {
      listener(event);
    }
  }

  function rememberRestoredWorkspace(workspace: BackendLifecycleWorkspace | null): WorkspaceHistoryDiagnostic | null {
    const restoredPath = workspaceHistoryPath(workspacePath, workspace);
    if (!restoredPath) return currentHistoryDiagnostic;
    const canonicalPath = options.canonicalizePath(restoredPath);
    if (!canonicalPath) {
      currentHistoryDiagnostic = {
        code: "history_write_failed",
        message: "Workspace history could not be saved; the ready workspace was kept.",
      };
      logger.warn("[main] workspace history was not updated", "workspace path could not be canonicalized");
      return currentHistoryDiagnostic;
    }
    try {
      const result = options.rememberWorkspacePath(canonicalPath);
      currentHistoryDiagnostic = result.diagnostic;
      if (currentHistoryDiagnostic) {
        logger.warn("[main] workspace history was not updated", currentHistoryDiagnostic.message);
      }
    } catch (error) {
      currentHistoryDiagnostic = {
        code: "history_write_failed",
        message: "Workspace history could not be saved; the ready workspace was kept.",
      };
      logger.warn("[main] workspace history was not updated", error instanceof Error ? error.message : "unknown error");
    }
    return currentHistoryDiagnostic;
  }

  async function restoreWorkspace(port: number, signal: AbortSignal): Promise<BackendLifecycleWorkspace | null> {
    if (!workspacePath) return null;

    const displayPath = workspaceDisplayPath ?? workspacePath;
    const response = await send(port, "workspace", {
      method: "POST",
      jsonBody: buildWorkspaceRestoreRequest(displayPath, workspacePath),
      signal,
    });
    const restoreResult = await parseWorkspaceRestoreResponse(response);
    signal.throwIfAborted();
    if (!restoreResult.ok) {
      const rejectedPath = workspacePath;
      logger.warn(`[main] remembered workspace path was rejected by the sidecar: ${rejectedPath}`);
      if (restoreResult.removeFromHistory) {
        try {
          const result = await options.forgetWorkspacePath(rejectedPath);
          signal.throwIfAborted();
          currentHistoryDiagnostic = result.historyDiagnostic;
          if (currentHistoryDiagnostic) {
            logger.warn("[main] rejected workspace could not be removed from history", currentHistoryDiagnostic.message);
          }
          if (result.navigationDiagnostic) {
            logger.warn("[main] rejected workspace navigation could not be removed", result.navigationDiagnostic);
          }
        } catch (error) {
          signal.throwIfAborted();
          logger.warn("[main] rejected workspace could not be removed from history", error instanceof Error ? error.message : "unknown error");
        }
      }
      workspacePath = null;
      if (restoreResult.failureCode === "destination_conflict") {
        throw new WorkspaceSwitchError("destination_conflict", "Workspace is already owned by another sidecar.");
      }
      throw new WorkspaceRestorationFailure(restoreResult.status);
    }
    return restoreResult.workspace;
  }

  async function applyRetentionSettings(port: number, signal: AbortSignal): Promise<void> {
    const retention = options.getRetention?.() ?? options.getSettings().retention ?? DEFAULT_RETENTION;
    try {
      const response = await send(port, "settings/retention", {
        method: "POST",
        jsonBody: retention,
        signal,
      });
      signal.throwIfAborted();
      if (!response.ok) {
        logger.warn(`[main] failed to restore retention settings: ${response.status}`);
      }
    } catch (error) {
      signal.throwIfAborted();
      logger.warn(`[main] failed to restore retention settings: ${error instanceof Error ? error.message : "unknown error"}`);
    }
  }

  async function syncSavedProviderSettings(port: number, signal: AbortSignal): Promise<void> {
    return options.enqueueSettingsWrite(async () => {
      signal.throwIfAborted();
      const settings = options.getSettings();
      const providerResponse = await send(port, "providers", { signal });
      if (!providerResponse.ok) throw new Error(`provider catalog sync failed: ${providerResponse.status}`);
      const providerBody = await providerResponse.json() as {
        providers?: unknown;
      };
      signal.throwIfAborted();
      if (!Array.isArray(providerBody.providers)) throw new Error("provider catalog sync returned malformed data");
      const definitions: ProviderDefinition[] = providerBody.providers.map((entry) => {
        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
          throw new Error("provider catalog sync returned malformed entry");
        }
        const raw = entry as Record<string, unknown>;
        if (typeof raw.id !== "string" || typeof raw.label !== "string") {
          throw new Error("provider catalog sync returned malformed identity");
        }
        return {
          id: raw.id,
          label: raw.label,
          ...(typeof raw.harnessId === "string" ? { harnessId: raw.harnessId } : {}),
          ...(raw.origin === "builtin" || raw.origin === "override" || raw.origin === "custom" || raw.origin === "standalone"
            ? { origin: raw.origin }
            : {}),
        };
      });
      const normalizedProviders = normalizeProviderSettings(settings.providers, definitions);
      const nextSettings: AppSettings = { ...settings, providers: normalizedProviders };
      if (JSON.stringify(settings.providers) !== JSON.stringify(normalizedProviders)) {
        options.writeSettings(nextSettings);
      }
      const abortableFetch: typeof fetch = (input, init) => {
        const headers = new Headers(init?.headers);
        for (const [name, value] of Object.entries(sidecarAccessHeaders())) headers.set(name, value);
        return fetchImpl(input, { ...init, headers, signal });
      };
      const result = await pushProviderSettings(
        sidecarUrl(port),
        normalizedProviders,
        abortableFetch,
      );
      signal.throwIfAborted();
      const syncError = providerSettingsSyncError(result);
      if (syncError) throw syncError;
      options.clearProviderModelsCache();
    });
  }

  const restoration = createBackendRestorationCoordinator<BackendLifecycleWorkspace>({
    setTimeout: (callback, delayMs) => timers.setTimeout(callback, delayMs),
    clearTimeout: (timer) => timers.clearTimeout(timer),
    ...(timers.restorationTimeoutMs === undefined ? {} : { timeoutMs: timers.restorationTimeoutMs }),
    onReady: (_port, workspace) => {
      activeHarnessRevision = workspace?.activeHarnessRevision ?? 0;
    },
    onFailure: (error) => {
      logLifecycleFailure("restoration", error);
      if (coordinator.getState() === "opening") return;
      publishLifecycle({ state: "failed", message: BACKEND_UNAVAILABLE_MESSAGE });
    },
    onStepFailure: (step, error) => {
      logLifecycleFailure(`restoration:${step}`, error);
    },
  });

  const sidecar = createSidecarLifecycle({
    spawn: (cwd) => {
      openPlanToken = generateToken();
      return options.spawnSidecar(cwd, openPlanToken);
    },
    setTimeout: (callback, delayMs) => timers.setTimeout(callback, delayMs),
    clearTimeout: (timer) => timers.clearTimeout(timer),
    now: () => timers.now(),
    ...(timers.readinessTimeoutMs === undefined ? {} : { readinessTimeoutMs: timers.readinessTimeoutMs }),
    ...(timers.cleanupTimeoutMs === undefined ? {} : { cleanupTimeoutMs: timers.cleanupTimeoutMs }),
    callbacks: {
      onReady: (port) => {
        logger.log(`[main] sidecar ready on port ${port}`);
        restoration.restore(port, {
          restoreWorkspace: (signal) => restoreWorkspace(port, signal),
          applyRetentionSettings: (signal) => applyRetentionSettings(port, signal),
          syncProviderSettings: (signal) => syncSavedProviderSettings(port, signal),
        });
      },
      onUnavailable: (message) => {
        logLifecycleFailure("sidecar", message);
        // A failed generation is reported to the coordinator; recovery stays
        // explicit through retry-backend so replacement remains serialized.
        restoration.fail(new Error(BACKEND_UNAVAILABLE_MESSAGE));
      },
      onUnexpectedExit: () => {
        invalidateGeneration();
        restoration.cancel(new Error(BACKEND_UNAVAILABLE_MESSAGE));
        coordinator.markUnresolved({
          code: "cleanup_failed",
          message: BACKEND_UNAVAILABLE_MESSAGE,
        });
      },
      onState: (state: SidecarState) => {
        if (state === "starting") {
          invalidateGeneration();
          restoration.beginGeneration();
        }
      },
    },
  });

  function publishSwitchEvent(event: WorkspaceSwitchEvent<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic>): void {
    if (event.state === "picker") {
      publishLifecycle(event.failure ? { state: "picker", failure: event.failure } : { state: "picker" });
    } else if (event.state === "opening" || event.state === "closing") {
      publishLifecycle({ state: event.state });
    } else if (event.state === "unresolved") {
      publishLifecycle({ state: "unresolved", failure: event.failure });
    } else {
      publishLifecycle({
        state: "ready",
        port: event.port,
        workspace: event.workspace,
        historyDiagnostic: event.historyDiagnostic,
      });
      const readyGeneration = generation;
      const readySignal = generationController.signal;
      options.onReadySideEffects?.({
        port: event.port,
        generation: readyGeneration,
        workspace: event.workspace,
        signal: readySignal,
        isCurrent: () => !readySignal.aborted
          && readyGeneration === generation
          && latestLifecycle.state === "ready",
      });
    }
  }

  const coordinator = createWorkspaceSwitchCoordinator<BackendLifecycleWorkspace, WorkspaceHistoryDiagnostic>({
    initialWorkspacePath: null,
    validateDestination: (candidate) => {
      const identity = options.validateDestination(candidate);
      if (identity) pendingWorkspaceDisplayPath = candidate;
      return identity;
    },
    setWorkspacePath: (nextPath) => {
      workspacePath = nextPath;
      if (nextPath) {
        workspaceDisplayPath = pendingWorkspaceDisplayPath ?? nextPath;
        pendingWorkspaceDisplayPath = null;
      } else {
        workspaceDisplayPath = null;
      }
    },
    onCloseAdmission: () => {
      invalidateGeneration();
    },
    closeCurrentRuntime: async () => {
      restoration.cancel(new Error("Workspace is closing"));
      await sidecar.stop();
    },
    startRuntime: async (nextPath, switchGeneration) => {
      workspacePath = nextPath;
      workspaceDisplayPath = pendingWorkspaceDisplayPath ?? nextPath;
      let lifecycleReadiness: Promise<number>;
      try {
        lifecycleReadiness = sidecar.start(nextPath);
        void lifecycleReadiness.catch(() => {});
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : "The sidecar did not become ready.";
        throw new WorkspaceSwitchError("readiness_failed", message);
      }
      const port = await lifecycleReadiness.catch((error: unknown) => {
        const message = error instanceof Error ? error.message : "The sidecar did not become ready.";
        throw new WorkspaceSwitchError("readiness_failed", message);
      });

      let restoredPort: number;
      try {
        restoredPort = await restoration.getReadiness();
      } catch (error: unknown) {
        if (error instanceof WorkspaceSwitchError) throw error;
        if (error instanceof WorkspaceRestorationFailure) {
          throw new WorkspaceSwitchError("restoration_failed", error.message);
        }
        const message = error instanceof Error ? error.message : "The destination workspace could not be restored.";
        throw new WorkspaceSwitchError("restoration_failed", message);
      }
      if (!coordinator.isCurrentGeneration(switchGeneration)) {
        throw new WorkspaceSwitchError("restoration_failed", "Workspace opening was superseded.");
      }
      const workspace = restoration.getRestoredWorkspace();
      if (!workspace) {
        throw new WorkspaceSwitchError("restoration_failed", "The destination workspace was not restored.");
      }
      return { port: restoredPort || port, workspace };
    },
    cleanupAttemptedRuntime: async () => {
      restoration.cancel(new Error("Attempted workspace runtime is being cleaned up"));
      workspacePath = null;
      workspaceDisplayPath = null;
      pendingWorkspaceDisplayPath = null;
      await sidecar.stop();
    },
    rememberWorkspace: (_path, workspace) => rememberRestoredWorkspace(workspace),
    publish: publishSwitchEvent,
    ...(options.onQuit ? { onQuit: options.onQuit } : {}),
  });

  function isCurrentReadyGeneration(
    capturedGeneration: number,
    port: number,
    runOptions: RunReadyOptions,
  ): boolean {
    if (capturedGeneration !== generation || latestLifecycle.state !== "ready") return false;
    if (runOptions.expectedWorkspacePath !== undefined && runOptions.expectedWorkspacePath !== workspacePath) {
      return false;
    }
    if (
      runOptions.expectedWorkspaceIdentity !== undefined
      && runOptions.expectedWorkspaceIdentity !== latestLifecycle.workspace?.workspaceIdentity
    ) {
      return false;
    }
    if (runOptions.expectedBackendUrl !== undefined && runOptions.expectedBackendUrl !== sidecarUrl(port)) {
      return false;
    }
    return true;
  }

  return {
    openWorkspace(path: string) {
      return coordinator.switchWorkspace(path);
    },

    pickWorkspace(select: () => Promise<string | null>) {
      return coordinator.pickWorkspace(select);
    },

    async retry(): Promise<BackendRetryResult> {
      const result = await coordinator.retry();
      if (result.ok) return { ok: true, state: "ready" };
      if (result.state === "unresolved") {
        return { ok: false, state: "unresolved", failure: result.failure };
      }
      if (result.failure.code === "invalid_destination") {
        publishLifecycle({ state: "picker" });
      }
      return { ok: false, state: "picker", failure: result.failure };
    },

    close() {
      return coordinator.close();
    },

    quit() {
      return coordinator.quit();
    },

    runUpdate<T>(operation: (currentPath: string | null) => Promise<T>): Promise<T> {
      return coordinator.runUpdate(operation);
    },

    stopSidecarForUpdate(timeoutMs: number): Promise<void> {
      return sidecar.stopAndWait(timeoutMs);
    },

    async restartSidecarAfterUpdate(fallbackCwd: string): Promise<void> {
      const restartCwd = workspacePath ?? fallbackCwd;
      try {
        const lifecycleReadiness = sidecar.start(restartCwd);
        await lifecycleReadiness;
        await restoration.getReadiness();
      } catch {
        throw new Error("The sidecar could not be restarted. Restart OrkWorks to recover.");
      }
    },

    dispose(): void {
      invalidateGeneration();
      restoration.dispose();
      sidecar.dispose();
    },

    getLifecycle(): BackendLifecycleEvent {
      return latestLifecycle;
    },

    subscribeLifecycle(listener: (event: BackendLifecycleEvent) => void): () => void {
      lifecycleListeners.add(listener);
      return () => {
        lifecycleListeners.delete(listener);
      };
    },

    getInitialWorkspaceSnapshot(): InitialWorkspaceSnapshot {
      // Workspace history is a picker hint, not proof that this process owns the
      // remembered directory after a restart. Native process-tree ownership is
      // not proven on every supported platform yet, so startup remains in the
      // picker until the user explicitly chooses a destination.
      return {
        workspace: null,
        historyDiagnostic: currentHistoryDiagnostic,
      };
    },

    getWorkspacePath(): string | null {
      return workspacePath;
    },

    getGeneration(): number {
      return generation;
    },

    getWorkspaceRevision(): string {
      return `${workspacePath}/${generation}`;
    },

    getActiveHarnessRevision(): number {
      return activeHarnessRevision;
    },

    setActiveHarnessRevision(
      expectedGeneration: number,
      expectedWorkspacePath: string | null,
      nextRevision: number,
    ): boolean {
      if (expectedGeneration !== generation || expectedWorkspacePath !== workspacePath) {
        return false;
      }
      activeHarnessRevision = nextRevision;
      return true;
    },

    getWorkspaceGuard(): WorkspaceGuardSnapshot {
      return {
        workspacePath,
        generation,
        activeHarnessRevision,
      };
    },

    getReadyNavigationIdentity(): ReadyNavigationIdentity {
      if (latestLifecycle.state !== "ready" || !latestLifecycle.workspace) {
        throw new Error("Workspace navigation is available only in a ready workspace.");
      }
      const workspaceIdentity = latestLifecycle.workspace.workspaceIdentity;
      if (
        !workspaceIdentity
        || !workspacePath
        || !sameWorkspaceIdentity(workspacePath, workspaceIdentity)
        || !sameWorkspaceIdentity(coordinator.getCurrentWorkspacePath() ?? "", workspaceIdentity)
      ) {
        throw new Error("Ready workspace identity is unavailable.");
      }
      const capturedGeneration = generation;
      return {
        workspaceIdentity,
        generation: capturedGeneration,
        isCurrent: () => latestLifecycle.state === "ready"
          && latestLifecycle.workspace?.workspaceIdentity === workspaceIdentity
          && !!workspacePath
          && sameWorkspaceIdentity(workspacePath, workspaceIdentity)
          && generation === capturedGeneration,
      };
    },

    getReadiness(): Promise<number> {
      return restoration.getReadiness();
    },

    async getBackendUrl(): Promise<string> {
      const port = await restoration.getReadiness();
      return sidecarUrl(port);
    },

    getOpenPlanToken(): string {
      return openPlanToken;
    },

    async runReady<T>(
      operation: (ctx: ReadyBackendContext) => Promise<T>,
      runOptions: RunReadyOptions = {},
    ): Promise<T> {
      const staleMessage = runOptions.staleMessage ?? STALE_BACKEND_GENERATION_MESSAGE;
      if (runOptions.expectedWorkspacePath !== undefined && runOptions.expectedWorkspacePath !== workspacePath) {
        throw new StaleBackendGenerationError(staleMessage);
      }
      if (latestLifecycle.state !== "ready") {
        throw new Error(runOptions.notReadyMessage ?? WORKSPACE_TRANSITION_IN_PROGRESS_MESSAGE);
      }
      const capturedGeneration = generation;
      const port = await restoration.getReadiness();
      const assertCurrent = (message = staleMessage): void => {
        if (!isCurrentReadyGeneration(capturedGeneration, port, runOptions)) {
          throw new StaleBackendGenerationError(message);
        }
      };
      assertCurrent();
      const token = openPlanToken;
      const readyWorkspace = latestLifecycle.state === "ready" ? latestLifecycle.workspace : null;
      const capturedWorkspacePath = workspacePath ?? "";
      const capturedWorkspaceIdentity = readyWorkspace?.workspaceIdentity ?? "";
      const capturedActiveHarnessRevision = activeHarnessRevision;
      const controller = new AbortController();
      requestControllers.add(controller);
      const isCurrent = (): boolean =>
        !controller.signal.aborted && isCurrentReadyGeneration(capturedGeneration, port, runOptions);
      try {
        const result = await operation({
          port,
          url: sidecarUrl(port),
          token,
          generation: capturedGeneration,
          workspacePath: capturedWorkspacePath,
          workspaceIdentity: capturedWorkspaceIdentity,
          activeHarnessRevision: capturedActiveHarnessRevision,
          signal: controller.signal,
          isCurrent,
          assertCurrent,
        });
        assertCurrent();
        return result;
      } finally {
        requestControllers.delete(controller);
      }
    },
  };
}
