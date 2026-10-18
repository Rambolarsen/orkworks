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
  type WorkspaceInfo,
} from "./api.ts";
import type { CreateSessionOptions } from "./harnessTypes.ts";
import { resolvePendingCreates, trackPendingCreate } from "./pendingCreate.ts";
import { startSessionPolling, type PollScheduler } from "./sessionPolling.ts";
import { mergeSessionsById } from "./sessionSort.ts";

export interface WorkspaceSessionControllerDeps {
  getBackendUrl: () => Promise<string>;
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
  resumeSession(id: string): Promise<void>;
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
  const deps = { ...defaultDeps, ...options.deps };
  let disposed = false;
  let admissionEnabled = options.initialAdmissionEnabled ?? true;
  let admissionGeneration = 0;
  let foregroundGeneration = 0;
  let pollingEpoch = 0;
  let workspaceGeneration = 0;
  const pendingActions = new Set<string>();
  let sessions: SessionInfo[] = [];
  let activeSessionId: string | null = null;
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

  async function refreshSessions(epoch?: number, allowDuringTransition = false): Promise<readonly SessionInfo[] | null> {
    const admissionToken = admissionGeneration;
    if (!allowDuringTransition && !admissionEnabled) return null;
    const token = foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const list = await deps.listSessions(baseUrl);
      if (!isCurrent(token)
        || (!allowDuringTransition && !isCurrentAdmission(admissionToken))
        || (epoch !== undefined && epoch !== pollingEpoch)) return null;

      deps.pruneTerminals(new Set(list.filter((session) => session.lifecycle !== "dead").map((session) => session.id)));
      const resolution = resolvePendingCreates(pendingCreateIds, list);
      pendingCreateIds = resolution.ids;
      if (resolution.erroredIds.length > 0) publishError("create", "Couldn't start a new session.");

      const [next, nextLastResortAt] = mergeSessionsById(sessions, list, lastResortAt, new Date());
      lastResortAt = nextLastResortAt;
      publishSessions(next);
      options.onSessionSourceStatus?.("ready");
      reportedErrors.clear();
      return next;
    } catch {
      if (isCurrent(token) && (allowDuringTransition || isCurrentAdmission(admissionToken))
        && (epoch === undefined || epoch === pollingEpoch)) options.onSessionSourceStatus?.("unavailable");
      return null;
    }
  }

  const scheduler = options.scheduler ?? (typeof window === "undefined"
    ? { set: () => 0, clear: () => {} }
    : undefined);
  let stopPolling: (() => void) | null = null;

  function setPollingEnabled(enabled: boolean): void {
    if (disposed) return;
    if (enabled) {
      if (!admissionEnabled) return;
      if (stopPolling === null) {
        const epoch = ++pollingEpoch;
        stopPolling = startSessionPolling(() => refreshSessions(epoch), options.pollDelayMs, scheduler);
      }
      return;
    }
    pollingEpoch += 1;
    stopPolling?.();
    stopPolling = null;
  }

  function setAdmissionEnabled(enabled: boolean): void {
    if (disposed || admissionEnabled === enabled) return;
    admissionEnabled = enabled;
    admissionGeneration += 1;
    foregroundGeneration += 1;
    if (!enabled) {
      options.onSessionSourceStatus?.("unavailable");
      pollingEpoch += 1;
      stopPolling?.();
      stopPolling = null;
    }
  }

  async function openWorkspace(path: string): Promise<void> {
    workspaceGeneration += 1;
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const info = await deps.setWorkspace(baseUrl, path);
      if (!isCurrent(token)) return;
      options.onWorkspace?.(info);
      activeSessionId = null;
      options.onActiveSession?.(null);
      publishSessions([]);
      options.onSessionSourceStatus?.("loading");
      const refreshed = await refreshSessions(undefined, true);
      if (!refreshed || !isCurrent(token)) return;
      const restored = info.lastActiveSessionId;
      const match = restored && sessions.find((session) => session.id === restored);
      if (match && match.lifecycle !== "dead") {
        activeSessionId = match.id;
        options.onActiveSession?.(match.id);
      }
    } catch {
      if (isCurrent(token)) publishError("workspace", "Couldn't open workspace.");
    }
  }

  async function adoptRestoredWorkspace(workspace: WorkspaceInfo | null): Promise<void> {
    workspaceGeneration += 1;
    const token = ++foregroundGeneration;
    pendingCreateIds = new Set();
    reportedErrors.clear();
    activeSessionId = null;
    options.onActiveSession?.(null);
    publishSessions([]);
    options.onSessionSourceStatus?.("loading");
    options.onWorkspace?.(workspace);
    if (!workspace || !isCurrent(token)) return;

    const refreshed = await refreshSessions(undefined, true);
    if (!refreshed || !isCurrent(token)) return;
    const restored = workspace.lastActiveSessionId;
    const match = restored && sessions.find((session) => session.id === restored);
    if (match && match.lifecycle !== "dead") {
      activeSessionId = match.id;
      options.onActiveSession?.(match.id);
    }
  }

  async function createSession(optionsForCreate: CreateSessionOptions, shouldSelect: () => boolean = () => true): Promise<boolean> {
    const admissionToken = requireAdmission();
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const created = await deps.createSession(baseUrl, optionsForCreate);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return false;
      pendingCreateIds = trackPendingCreate(pendingCreateIds, created.id);
      const [next, nextLastResortAt] = mergeSessionsById(sessions, [...sessions, created], lastResortAt, new Date());
      lastResortAt = nextLastResortAt;
      publishSessions(next);
      if (shouldSelect()) {
        activeSessionId = created.id;
        options.onActiveSession?.(created.id);
      }
      return true;
    } catch {
      if (isCurrent(token)) publishError("create", "Couldn't start a new session.");
      return false;
    }
  }

  async function resumeSession(id: string): Promise<void> {
    const admissionToken = requireAdmission();
    const token = ++foregroundGeneration;
    deps.disposeTerminal(id);
    try {
      const baseUrl = await deps.getBackendUrl();
      const resumed = await deps.resumeSession(baseUrl, id);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return;
      publishSessions(sessions.map((session) => session.id === id ? resumed : session));
      activeSessionId = resumed.id;
      options.onActiveSession?.(resumed.id);
    } catch {
      if (isCurrent(token)) publishError("resume", "Couldn't resume session.");
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
    activeSessionId = id;
    options.onActiveSession?.(id);
    return true;
  }

  async function deleteSession(id: string, forget: boolean): Promise<void> {
    const admissionToken = requireAdmission();
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
    pendingActions.add(pendingKey);
    foregroundGeneration += 1;
    const current = () => isCurrentAdmission(admissionToken) && workspaceToken === workspaceGeneration;
    const disposeDeletedRow = () => {
      deps.disposeTerminal(id);
      if (activeSessionId === id) {
        activeSessionId = null;
        options.onActiveSession?.(null, { deliberate: true });
      }
    };
    try {
      const baseUrl = await deps.getBackendUrl();
      if (!current()) return;
      await deps.sessionAction(baseUrl, id, action, actionToken);
      if (!current()) return;
      if (action === "delete") disposeDeletedRow();
      await refreshSessions();
    } catch (error) {
      if (current()) {
        const refreshed = await refreshSessions();
        if (current()) {
          if (action === "delete" && refreshed !== null && !refreshed.some(session => session.id === id)) disposeDeletedRow();
          throw error;
        }
      }
    } finally {
      pendingActions.delete(pendingKey);
    }
  }

  function dispose(): void {
    if (disposed) return;
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
