import { startWorkspaceUpdates } from "./workspaceUpdates.ts";
import { publishWorkspaceUpdateSignal } from "./workspaceUpdateSignals.ts";
import { isHistoricalSession } from "./sessionRuntimeState.ts";
import {
  sessionAction as apiSessionAction,
  type SessionAction,
  createSession as apiCreateSession,
  deleteSession as apiDeleteSession,
  forgetSession as apiForgetSession,
  listSessions as apiListSessions,
  resumeSession as apiResumeSession,
  setActiveWorkspaceSession as apiSetActiveWorkspaceSession,
  setWorkspace as apiSetWorkspace,
  type SessionInfo,
  type ResumeStrategy,
  type WorkspaceInfo,
} from "./api.ts";
import type { CreateSessionOptions } from "./harnessTypes.ts";
import { resolvePendingCreates, trackPendingCreate } from "./pendingCreate.ts";
import { startSessionPolling, type PollScheduler } from "./sessionPolling.ts";
import { mergeSessionsById } from "./sessionSort.ts";

export interface WorkspaceSessionControllerDeps {
  getBackendUrl: () => Promise<string>;
  startUpdates?: typeof startWorkspaceUpdates;
  setWorkspace: typeof apiSetWorkspace;
  listSessions: typeof apiListSessions;
  createSession: typeof apiCreateSession;
  resumeSession: typeof apiResumeSession;
  deleteSession: typeof apiDeleteSession;
  forgetSession: typeof apiForgetSession;
  sessionAction: typeof apiSessionAction;
  setActiveWorkspaceSession: typeof apiSetActiveWorkspaceSession;
  pruneTerminals: (keepLiveSessionIds: ReadonlySet<string>) => void;
  disposeTerminal: (id: string) => void;
}

export interface ControllerError {
  key: string;
  message: string;
}

export interface WorkspaceSessionControllerOptions {
  deps?: Partial<WorkspaceSessionControllerDeps>;
  scheduler?: PollScheduler;
  pollDelayMs?: number;
  initialAdmissionEnabled?: boolean;
  onWorkspace?: (workspace: WorkspaceInfo | null) => void;
  onSessionSourceStatus?: (status: "loading" | "ready" | "unavailable") => void;
  onResumingSessions?: (ids: ReadonlySet<string>) => void;
  onSessions?: (sessions: readonly SessionInfo[]) => void;
  onActiveSession?: (id: string | null, cause?: { deliberate?: boolean }) => void;
  onError?: (error: ControllerError) => void;
}

export interface WorkspaceSessionController {
  setAdmissionEnabled(enabled: boolean): void;
  isAdmissionEnabled(): boolean;
  captureAdmission(): number | null;
  isAdmissionCurrent(token: number): boolean;
  setPollingEnabled(enabled: boolean): void;
  openWorkspace(path: string): Promise<void>;
  adoptRestoredWorkspace(workspace: WorkspaceInfo | null): Promise<void>;
  refreshSessions(): Promise<readonly SessionInfo[] | null>;
  createSession(options: CreateSessionOptions, shouldSelect?: () => boolean): Promise<boolean>;
  resumeSession(id: string, strategy?: ResumeStrategy): Promise<void>;
  submitActiveSession(id: string, admissionToken?: number | null): Promise<boolean>;
  selectSession(id: string): boolean;
  deleteSession(id: string, forget: boolean): Promise<void>;
  sessionAction(id: string, action: SessionAction, token: string): Promise<void>;
  dispose(): void;
}

const defaultDeps: WorkspaceSessionControllerDeps = {
  getBackendUrl: () => window.orkworks.getBackendUrl(),
  setWorkspace: apiSetWorkspace,
  listSessions: apiListSessions,
  createSession: apiCreateSession,
  resumeSession: apiResumeSession,
  deleteSession: apiDeleteSession,
  forgetSession: apiForgetSession,
  sessionAction: apiSessionAction,
  setActiveWorkspaceSession: apiSetActiveWorkspaceSession,
  pruneTerminals: () => {},
  disposeTerminal: () => {},
};

export function createWorkspaceSessionController(
  options: WorkspaceSessionControllerOptions = {},
): WorkspaceSessionController {
  const deps = { ...defaultDeps, startUpdates: typeof window !== "undefined" && typeof WebSocket !== "undefined" ? startWorkspaceUpdates : undefined, ...options.deps };
  let disposed = false;
  let admissionEnabled = options.initialAdmissionEnabled ?? true;
  let admissionGeneration = 0;
  let foregroundGeneration = 0;
  let pollingEpoch = 0;
  let sourceGeneration = 0;
  let refreshGeneration = 0;
  let synchronizationEnabled = false;
  let synchronizationEpoch = 0;
  let workspacePath: string | null = null;
  let stream: ReturnType<typeof startWorkspaceUpdates> | null = null;
  let streamHealthy = false;
  let workspaceGeneration = 0;
  const pendingResumes = new Map<string, symbol>();
  function publishResumes(): void { options.onResumingSessions?.(new Set(pendingResumes.keys())); }
  function clearResumes(): void {
    if (!pendingResumes.size) return;
    pendingResumes.clear();
    publishResumes();
  }
  const pendingActions = new Set<string>();
  const pendingSnapshotActions = new Map<string, (list: readonly SessionInfo[]) => void>();
  let sessions: SessionInfo[] = [];
  let activeSessionId: string | null = null;
  let pendingRestoration: { id: string | null; foreground: number } | null = null;
  let lastResortAt = new Date(0);
  let pendingCreateIds: ReadonlySet<string> = new Set();
  const reportedErrors = new Set<string>();

  const publishError = (key: string, message: string): void => {
    if (disposed || reportedErrors.has(key)) return;
    reportedErrors.add(key);
    options.onError?.({ key, message });
  };

  const isCurrent = (token: number): boolean => !disposed && token === foregroundGeneration;

  function requireAdmission(): number {
    if (disposed || !admissionEnabled) throw new Error("Workspace transition is in progress");
    return admissionGeneration;
  }

  function isCurrentAdmission(token: number): boolean {
    return !disposed && admissionEnabled && token === admissionGeneration;
  }

  const isAdmissionCurrent = isCurrentAdmission;

  const publishSessions = (next: SessionInfo[]): void => {
    sessions = next;
    options.onSessions?.(next);
  };

  function acceptSessionList(list: readonly SessionInfo[]): SessionInfo[] {
    deps.pruneTerminals(new Set(list.filter((session) => !isHistoricalSession(session)).map((session) => session.id)));
    const resolution = resolvePendingCreates(pendingCreateIds, list);
    pendingCreateIds = resolution.ids;
    if (resolution.erroredIds.length > 0) publishError("create", "Couldn't start a new session.");

    const [next, nextLastResortAt] = mergeSessionsById(sessions, list, lastResortAt, new Date());
    lastResortAt = nextLastResortAt;
    publishSessions(next);
    options.onSessionSourceStatus?.("ready");
    reportedErrors.clear();
    // Either REST or the stream can deliver the first authoritative list.
    // Consume restoration once, without replacing intervening user navigation.
    const restoration = pendingRestoration;
    pendingRestoration = null;
    if (restoration && isCurrent(restoration.foreground)) {
      const match = next.find(session => session.id === restoration.id);
      if (match && !isHistoricalSession(match)) {
        activeSessionId = match.id;
        options.onActiveSession?.(match.id);
      }
    }
    for (const [key, reconcile] of pendingSnapshotActions) {
      pendingSnapshotActions.delete(key);
      reconcile(next);
    }
    return next;
  }

  async function refreshSessions(epoch?: number, allowDuringTransition = false): Promise<readonly SessionInfo[] | null> {
    if (epoch === undefined && streamHealthy) stream?.resync();
    const admissionToken = admissionGeneration;
    if (!allowDuringTransition && !admissionEnabled) return null;
    const token = foregroundGeneration;
    const sourceToken = sourceGeneration;
    const request = ++refreshGeneration;
    const canPublish = () => request === refreshGeneration && sourceToken === sourceGeneration && isCurrent(token)
      && (allowDuringTransition || isCurrentAdmission(admissionToken))
      && (epoch === undefined || epoch === pollingEpoch);
    try {
      const baseUrl = await deps.getBackendUrl();
      const list = await deps.listSessions(baseUrl);
      if (!canPublish()) return null;

      return acceptSessionList(list);
    } catch {
      if (canPublish()) options.onSessionSourceStatus?.("unavailable");
      return null;
    }
  }

  const scheduler = options.scheduler ?? (typeof window === "undefined"
    ? { set: () => 0, clear: () => {} }
    : undefined);
  let stopPolling: (() => void) | null = null;

  function stopFallback(): void {
    pollingEpoch++; stopPolling?.(); stopPolling = null;
  }
  function startFallback(): void {
    if (!disposed && synchronizationEnabled && admissionEnabled && !streamHealthy && stopPolling === null) {
      const epoch = ++pollingEpoch;
      stopPolling = startSessionPolling(() => refreshSessions(epoch), options.pollDelayMs, scheduler);
    }
  }
  function setPollingEnabled(enabled: boolean): void {
    if (disposed) return;
    if (!enabled || !admissionEnabled) {
      synchronizationEnabled = false;
      synchronizationEpoch++; sourceGeneration++;
      streamHealthy = false;
      const old = stream; stream = null; old?.dispose();
      if (workspacePath) publishWorkspaceUpdateSignal(workspacePath, false);
      stopFallback();
      return;
    }
    synchronizationEnabled = true;
    startFallback();
    if (!stream && deps.startUpdates && workspacePath) {
      const path = workspacePath, epoch = synchronizationEpoch;
      const current = () => !disposed && synchronizationEnabled && epoch === synchronizationEpoch && workspacePath === path;
      stream = deps.startUpdates({
        workspacePath: path, getBackendUrl: deps.getBackendUrl,
        onHealthy(healthy) {
          if (!current()) return;
          streamHealthy = healthy;
          if (healthy) { sourceGeneration++; stopFallback(); } else startFallback();
          publishWorkspaceUpdateSignal(path, healthy);
        },
        onSessions(list) {
          if (!current() || !streamHealthy) return;
          // A newer pushed snapshot supersedes REST reads started even after
          // this socket became healthy, not only reads from fallback polling.
          sourceGeneration++;
          acceptSessionList(list);
        },
        onRecommendationsInvalidated() { if (current()) publishWorkspaceUpdateSignal(path); },
      });
    }
  }

  function setAdmissionEnabled(enabled: boolean): void {
    if (disposed || admissionEnabled === enabled) return;
    admissionEnabled = enabled;
    admissionGeneration += 1;
    foregroundGeneration += 1;
    if (!enabled) {
      clearResumes();
      pendingSnapshotActions.clear();
      options.onSessionSourceStatus?.("unavailable");
      setPollingEnabled(false);
    }
  }

  async function openWorkspace(path: string): Promise<void> {
    pendingRestoration = null;
    pendingSnapshotActions.clear();
    setPollingEnabled(false);
    workspacePath = path;
    clearResumes();
    workspaceGeneration += 1;
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const info = await deps.setWorkspace(baseUrl, path);
      if (!isCurrent(token)) return;
      pendingRestoration = { id: info.lastActiveSessionId ?? null, foreground: token };
      options.onWorkspace?.(info);
      activeSessionId = null;
      options.onActiveSession?.(null);
      publishSessions([]);
      options.onSessionSourceStatus?.("loading");
      await refreshSessions(undefined, true);
    } catch {
      if (isCurrent(token)) publishError("workspace", "Couldn't open workspace.");
    }
  }

  async function adoptRestoredWorkspace(workspace: WorkspaceInfo | null): Promise<void> {
    pendingSnapshotActions.clear();
    setPollingEnabled(false);
    workspacePath = workspace?.path ?? null;
    clearResumes();
    workspaceGeneration += 1;
    const token = ++foregroundGeneration;
    pendingCreateIds = new Set();
    pendingRestoration = workspace ? { id: workspace.lastActiveSessionId ?? null, foreground: token } : null;
    reportedErrors.clear();
    activeSessionId = null;
    options.onActiveSession?.(null);
    publishSessions([]);
    options.onSessionSourceStatus?.("loading");
    options.onWorkspace?.(workspace);
    if (!workspace || !isCurrent(token)) return;

    await refreshSessions(undefined, true);
  }

  async function createSession(optionsForCreate: CreateSessionOptions, shouldSelect: () => boolean = () => true): Promise<boolean> {
    const admissionToken = requireAdmission();
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const created = await deps.createSession(baseUrl, optionsForCreate);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return false;
      // A list started while creation was pending may not include this record.
      // Fence it before publishing so it cannot drop the card or prune its terminal.
      foregroundGeneration += 1;
      stream?.resync();
      pendingCreateIds = trackPendingCreate(pendingCreateIds, created.id);
      const [next, nextLastResortAt] = mergeSessionsById(sessions, [...sessions, created], lastResortAt, new Date());
      lastResortAt = nextLastResortAt;
      publishSessions(next);
      if (shouldSelect()) {
        activeSessionId = created.id;
        options.onActiveSession?.(created.id);
      }
      return true;
    } catch (error) {
      if (isCurrent(token)) publishError("create", error instanceof Error && optionsForCreate.initialImages?.length ? error.message : "Couldn't start a new session.");
      if (optionsForCreate.initialImages?.length) throw error;
      return false;
    }
  }

  async function resumeSession(id: string, strategy?: ResumeStrategy): Promise<void> {
    const admissionToken = requireAdmission();
    if (pendingResumes.has(id)) return;
    const operation = Symbol();
    pendingResumes.set(id, operation);
    publishResumes();
    pendingSnapshotActions.delete(`${workspaceGeneration}:${id}`);
    const token = ++foregroundGeneration;
    try {
      deps.disposeTerminal(id);
      const baseUrl = await deps.getBackendUrl();
      const resumed = await deps.resumeSession(baseUrl, id, strategy);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return;
      stream?.resync();
      publishSessions(sessions.map((session) => session.id === id ? resumed : session));
      activeSessionId = resumed.id;
      options.onActiveSession?.(resumed.id);
    } catch (error) {
      if (isCurrent(token)) publishError("resume", error instanceof Error ? error.message : "Couldn't resume session.");
    } finally {
      if (pendingResumes.get(id) === operation) {
        pendingResumes.delete(id);
        publishResumes();
      }
    }
  }

  async function submitActiveSession(id: string, expectedAdmissionToken?: number | null): Promise<boolean> {
    if (expectedAdmissionToken === null) return false;
    const admissionToken = expectedAdmissionToken ?? requireAdmission();
    const baseUrl = await deps.getBackendUrl();
    if (!isCurrentAdmission(admissionToken)) return false;
    await deps.setActiveWorkspaceSession(baseUrl, id);
    return isCurrentAdmission(admissionToken);
  }

  function selectSession(id: string): boolean {
    if (disposed || !admissionEnabled) return false;
    pendingRestoration = null;
    activeSessionId = id;
    options.onActiveSession?.(id);
    return true;
  }

  async function deleteSession(id: string, forget: boolean): Promise<void> {
    const admissionToken = requireAdmission();
    pendingSnapshotActions.delete(`${workspaceGeneration}:${id}`);
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      if (forget) await deps.forgetSession(baseUrl, id);
      else await deps.deleteSession(baseUrl, id);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return;
      deps.disposeTerminal(id);
      if (activeSessionId === id) {
        activeSessionId = null;
        options.onActiveSession?.(null, { deliberate: true });
      }
      await refreshSessions();
    } catch {
      if (isCurrent(token)) publishError(forget ? "forget" : "delete", forget ? "Couldn't delete session." : "Couldn't end session.");
    }
  }

  async function sessionAction(id: string, action: SessionAction, actionToken: string): Promise<void> {
    const admissionToken = requireAdmission();
    const workspaceToken = workspaceGeneration;
    const pendingKey = `${workspaceToken}:${id}`;
    if (pendingActions.has(pendingKey)) throw new Error("A session action is already running.");
    pendingSnapshotActions.delete(pendingKey);
    pendingActions.add(pendingKey);
    foregroundGeneration += 1;
    const current = () => isCurrentAdmission(admissionToken) && workspaceToken === workspaceGeneration;
    let targetDisposed = false;
    const disposeDeletedRow = () => {
      if (targetDisposed) return;
      targetDisposed = true;
      deps.disposeTerminal(id);
      if (activeSessionId === id) {
        activeSessionId = null;
        options.onActiveSession?.(null, { deliberate: true });
      }
    };
    const refreshAfterAction = () => {
      // Neither a poll nor a still-connecting socket captured before the
      // mutation settled may consume its post-action reconciliation.
      sourceGeneration++;
      // The stream may replace the REST refresh. Complete cleanup from the first
      // post-action authoritative list, including after a failed/partial mutation.
      pendingSnapshotActions.set(pendingKey, list => {
        if (current() && !list.some(session => session.id === id)) disposeDeletedRow();
      });
      stream?.resync();
      return refreshSessions();
    };
    try {
      const baseUrl = await deps.getBackendUrl();
      if (!current()) return;
      await deps.sessionAction(baseUrl, id, action, actionToken);
      if (!current()) return;
      if (action === "delete") disposeDeletedRow();
      const refreshed = await refreshAfterAction();
      if (current() && refreshed !== null && !refreshed.some(session => session.id === id)) disposeDeletedRow();
    } catch (error) {
      if (current()) {
        const refreshed = await refreshAfterAction();
        if (current()) {
          if (refreshed !== null && !refreshed.some(session => session.id === id)) disposeDeletedRow();
          throw error;
        }
      }
    } finally {
      pendingActions.delete(pendingKey);
    }
  }

  function dispose(): void {
    if (disposed) return;
    setPollingEnabled(false);
    pendingSnapshotActions.clear();
    clearResumes();
    disposed = true;
    foregroundGeneration += 1;
    pollingEpoch += 1;
    stopPolling?.();
    stopPolling = null;
  }

  return {
    setAdmissionEnabled,
    isAdmissionEnabled: () => admissionEnabled && !disposed,
    captureAdmission: () => admissionEnabled && !disposed ? admissionGeneration : null,
    isAdmissionCurrent,
    setPollingEnabled,
    openWorkspace,
    adoptRestoredWorkspace,
    refreshSessions,
    createSession,
    resumeSession,
    submitActiveSession,
    selectSession,
    deleteSession,
    sessionAction,
    dispose,
  };
}
