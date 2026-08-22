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
  onWorkspace?: (workspace: WorkspaceInfo) => void;
  onSessions?: (sessions: readonly SessionInfo[]) => void;
  onActiveSession?: (id: string | null) => void;
  onError?: (error: ControllerError) => void;
}

export interface WorkspaceSessionController {
  openWorkspace(path: string): Promise<void>;
  refreshSessions(): Promise<boolean>;
  createSession(options: CreateSessionOptions): Promise<void>;
  resumeSession(id: string): Promise<void>;
  selectSession(id: string): void;
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
  let generation = 0;
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

  const isCurrent = (token: number): boolean => !disposed && token === generation;

  const publishSessions = (next: SessionInfo[]): void => {
    sessions = next;
    options.onSessions?.(next);
  };

  async function refreshSessions(): Promise<boolean> {
    const token = ++generation;
    try {
      const baseUrl = await deps.getBackendUrl();
      const list = await deps.listSessions(baseUrl);
      if (!isCurrent(token)) return false;

      deps.pruneTerminals(new Set(list.filter((session) => session.lifecycle !== "dead").map((session) => session.id)));
      const resolution = resolvePendingCreates(pendingCreateIds, list);
      pendingCreateIds = resolution.ids;
      if (resolution.erroredIds.length > 0) publishError("create", "Couldn't start a new session.");

      const [next, nextLastResortAt] = mergeSessionsById(sessions, list, lastResortAt, new Date());
      lastResortAt = nextLastResortAt;
      publishSessions(next);
      reportedErrors.clear();
      return true;
    } catch {
      return false;
    }
  }

  const scheduler = options.scheduler ?? (typeof window === "undefined"
    ? { set: () => 0, clear: () => {} }
    : undefined);
  const stopPolling = startSessionPolling(refreshSessions, options.pollDelayMs, scheduler);

  async function openWorkspace(path: string): Promise<void> {
    const token = ++generation;
    try {
      const baseUrl = await deps.getBackendUrl();
      const info = await deps.setWorkspace(baseUrl, path);
      if (!isCurrent(token)) return;
      options.onWorkspace?.(info);
      activeSessionId = null;
      options.onActiveSession?.(null);
      publishSessions([]);
      const refreshed = await refreshSessions();
      if (!refreshed || !isCurrent(generation)) return;
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

  async function createSession(optionsForCreate: CreateSessionOptions): Promise<void> {
    const token = ++generation;
    try {
      const baseUrl = await deps.getBackendUrl();
      const created = await deps.createSession(baseUrl, optionsForCreate);
      if (!isCurrent(token)) return;
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
    const token = ++generation;
    deps.disposeTerminal(id);
    try {
      const baseUrl = await deps.getBackendUrl();
      const resumed = await deps.resumeSession(baseUrl, id);
      if (!isCurrent(token)) return;
      publishSessions(sessions.map((session) => session.id === id ? resumed : session));
      activeSessionId = resumed.id;
      options.onActiveSession?.(resumed.id);
    } catch {
      if (isCurrent(token)) publishError("resume", "Couldn't resume session.");
    }
  }

  function selectSession(id: string): void {
    if (disposed) return;
    activeSessionId = id;
    options.onActiveSession?.(id);
  }

  async function deleteSession(id: string, forget: boolean): Promise<void> {
    const token = ++generation;
    try {
      const baseUrl = await deps.getBackendUrl();
      if (forget) await deps.forgetSession(baseUrl, id);
      else await deps.deleteSession(baseUrl, id);
      if (!isCurrent(token)) return;
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
    generation += 1;
    stopPolling();
  }

  return { openWorkspace, refreshSessions, createSession, resumeSession, selectSession, deleteSession, dispose };
}
