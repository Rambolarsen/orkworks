import {
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
  onSessions?: (sessions: readonly SessionInfo[]) => void;
  onActiveSession?: (id: string | null) => void;
  onError?: (error: ControllerError) => void;
}

export interface WorkspaceSessionController {
  setAdmissionEnabled(enabled: boolean): void;
  isAdmissionEnabled(): boolean;
  setPollingEnabled(enabled: boolean): void;
  openWorkspace(path: string): Promise<void>;
  adoptRestoredWorkspace(workspace: WorkspaceInfo | null): Promise<void>;
  refreshSessions(): Promise<readonly SessionInfo[] | null>;
  createSession(options: CreateSessionOptions): Promise<void>;
  resumeSession(id: string): Promise<void>;
  submitActiveSession(id: string): Promise<boolean>;
  selectSession(id: string): boolean;
  deleteSession(id: string, forget: boolean): Promise<void>;
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
      reportedErrors.clear();
      return next;
    } catch {
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
      pollingEpoch += 1;
      stopPolling?.();
      stopPolling = null;
    }
  }

  async function openWorkspace(path: string): Promise<void> {
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const info = await deps.setWorkspace(baseUrl, path);
      if (!isCurrent(token)) return;
      options.onWorkspace?.(info);
      activeSessionId = null;
      options.onActiveSession?.(null);
      publishSessions([]);
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
    const token = ++foregroundGeneration;
    pendingCreateIds = new Set();
    reportedErrors.clear();
    activeSessionId = null;
    options.onActiveSession?.(null);
    publishSessions([]);
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

  async function createSession(optionsForCreate: CreateSessionOptions): Promise<void> {
    const admissionToken = requireAdmission();
    const token = ++foregroundGeneration;
    try {
      const baseUrl = await deps.getBackendUrl();
      const created = await deps.createSession(baseUrl, optionsForCreate);
      if (!isCurrent(token) || !isCurrentAdmission(admissionToken)) return;
      pendingCreateIds = trackPendingCreate(pendingCreateIds, created.id);
      const [next, nextLastResortAt] = mergeSessionsById(sessions, [...sessions, created], lastResortAt, new Date());
      lastResortAt = nextLastResortAt;
      publishSessions(next);
      activeSessionId = created.id;
      options.onActiveSession?.(created.id);
    } catch {
      if (isCurrent(token)) publishError("create", "Couldn't start a new session.");
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

  async function submitActiveSession(id: string): Promise<boolean> {
    const admissionToken = requireAdmission();
    const baseUrl = await deps.getBackendUrl();
    if (!isCurrentAdmission(admissionToken)) return false;
    await deps.setActiveWorkspaceSession(baseUrl, id);
    return true;
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
        options.onActiveSession?.(null);
      }
      await refreshSessions();
    } catch {
      if (isCurrent(token)) publishError(forget ? "forget" : "delete", forget ? "Couldn't delete session." : "Couldn't end session.");
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
    setPollingEnabled,
    openWorkspace,
    adoptRestoredWorkspace,
    refreshSessions,
    createSession,
    resumeSession,
    submitActiveSession,
    selectSession,
    deleteSession,
    dispose,
  };
}
