export type WorkspaceLifecycleFailureCode =
  | "invalid_destination"
  | "cleanup_failed"
  | "cleanup_timeout"
  | "destination_conflict"
  | "readiness_failed"
  | "restoration_failed"
  | "quit_failed";

export type WorkspaceLifecycleFailure = {
  code: WorkspaceLifecycleFailureCode;
  message: string;
};

export type BackendLifecycleEvent =
  | { state: "picker"; failure?: WorkspaceLifecycleFailure }
  | { state: "opening" | "closing" }
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: BackendLifecycleWorkspace | null; historyDiagnostic: WorkspaceHistoryDiagnostic | null }
  | { state: "unresolved"; failure: WorkspaceLifecycleFailure }
  | { state: "failed" | "exhausted"; message: string };

export type BackendRetryResult =
  | { ok: true; state: "ready" }
  | { ok: false; state: "picker" | "unresolved"; failure: WorkspaceLifecycleFailure };

export interface BackendLifecycleWorkspace {
  path: string;
  repo_root: string | null;
  branch: string | null;
  dirty: boolean | null;
  lastActiveSessionId: string | null;
  activeHarnessIds: string[];
  activeHarnessRevision: number;
}

export interface WorkspaceHistoryDiagnostic {
  code: "corrupt_history" | "history_lock_timeout" | "history_write_failed" | "pin_limit_reached";
  message: string;
}

export interface InitialWorkspaceSnapshot {
  workspace: BackendLifecycleWorkspace | null;
  historyDiagnostic: WorkspaceHistoryDiagnostic | null;
}

function hasExactKeys(value: object, expected: readonly string[]): boolean {
  const keys = Reflect.ownKeys(value);
  return keys.length === expected.length && expected.every((key) => keys.includes(key));
}

function canonicalizeWorkspace(value: unknown): BackendLifecycleWorkspace | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  if (!hasExactKeys(value, [
    "path",
    "repo_root",
    "branch",
    "dirty",
    "lastActiveSessionId",
    "activeHarnessIds",
    "activeHarnessRevision",
  ])) return null;

  const workspace = value as Record<string, unknown>;
  return typeof workspace.path === "string"
    && (typeof workspace.repo_root === "string" || workspace.repo_root === null)
    && (typeof workspace.branch === "string" || workspace.branch === null)
    && (typeof workspace.dirty === "boolean" || workspace.dirty === null)
    && (typeof workspace.lastActiveSessionId === "string" || workspace.lastActiveSessionId === null)
    && Array.isArray(workspace.activeHarnessIds)
    && workspace.activeHarnessIds.every((id) => typeof id === "string")
    && typeof workspace.activeHarnessRevision === "number"
    && Number.isSafeInteger(workspace.activeHarnessRevision)
    && workspace.activeHarnessRevision >= 0
    ? {
      path: workspace.path,
      repo_root: workspace.repo_root,
      branch: workspace.branch,
      dirty: workspace.dirty,
      lastActiveSessionId: workspace.lastActiveSessionId,
      activeHarnessIds: [...workspace.activeHarnessIds],
      activeHarnessRevision: workspace.activeHarnessRevision,
    }
    : null;
}

function canonicalizeHistoryDiagnostic(value: unknown): WorkspaceHistoryDiagnostic | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  if (!hasExactKeys(value, ["code", "message"])) return null;
  const diagnostic = value as Record<string, unknown>;
  return (diagnostic.code === "corrupt_history"
    || diagnostic.code === "history_lock_timeout"
    || diagnostic.code === "history_write_failed"
    || diagnostic.code === "pin_limit_reached")
    && typeof diagnostic.message === "string"
    ? { code: diagnostic.code, message: diagnostic.message }
    : null;
}

export function canonicalizeBackendLifecycleEvent(data: unknown): BackendLifecycleEvent | null {
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;

  try {
    const event = data as Record<string, unknown>;
    const state = event.state;
    if (state === "picker") {
      if (hasExactKeys(data, ["state"])) return { state };
      const rawFailure = event.failure;
      const failure = canonicalizeWorkspaceLifecycleFailure(rawFailure);
      return hasExactKeys(data, ["state", "failure"]) && failure !== null
        ? { state, failure }
        : null;
    }
    if (state === "opening" || state === "closing" || state === "starting" || state === "retrying") {
      return hasExactKeys(data, ["state"]) ? { state } : null;
    }
    if (state === "ready") {
      const port = event.port;
      const rawWorkspace = event.workspace;
      const workspace = rawWorkspace === null ? null : canonicalizeWorkspace(rawWorkspace);
      const rawHistoryDiagnostic = event.historyDiagnostic;
      const historyDiagnostic = rawHistoryDiagnostic === null
        ? null
        : canonicalizeHistoryDiagnostic(rawHistoryDiagnostic);
      return hasExactKeys(data, ["state", "port", "workspace", "historyDiagnostic"])
        && typeof port === "number"
        && Number.isInteger(port)
        && port >= 1
        && port <= 65_535
        && (rawWorkspace === null || workspace !== null)
        && (rawHistoryDiagnostic === null || historyDiagnostic !== null)
        ? { state: "ready", port, workspace, historyDiagnostic }
        : null;
    }
    if (state === "failed" || state === "exhausted") {
      const message = event.message;
      return hasExactKeys(data, ["state", "message"]) && typeof message === "string"
        ? { state, message }
        : null;
    }
    if (state === "unresolved") {
      const failure = canonicalizeWorkspaceLifecycleFailure(event.failure);
      return hasExactKeys(data, ["state", "failure"]) && failure !== null
        ? { state, failure }
        : null;
    }
  } catch {
    return null;
  }

  return null;
}

function canonicalizeWorkspaceLifecycleFailure(value: unknown): WorkspaceLifecycleFailure | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  if (!hasExactKeys(value, ["code", "message"])) return null;
  const failure = value as Record<string, unknown>;
  return (failure.code === "invalid_destination"
    || failure.code === "cleanup_failed"
    || failure.code === "cleanup_timeout"
    || failure.code === "destination_conflict"
    || failure.code === "readiness_failed"
    || failure.code === "restoration_failed"
    || failure.code === "quit_failed")
    && typeof failure.message === "string"
    ? { code: failure.code, message: failure.message }
    : null;
}

export function subscribeBackendLifecycle(
  registerLive: (listener: (data: unknown) => void) => () => void,
  loadSnapshot: () => Promise<unknown>,
  callback: (event: BackendLifecycleEvent) => void,
): () => void {
  let active = true;
  let receivedLiveEvent = false;
  const unregisterLive = registerLive((data) => {
    if (!active) return;
    const event = canonicalizeBackendLifecycleEvent(data);
    if (!event) return;
    receivedLiveEvent = true;
    callback(event);
  });

  void Promise.resolve()
    .then(loadSnapshot)
    .then((data) => {
      if (!active || receivedLiveEvent) return;
      const event = canonicalizeBackendLifecycleEvent(data);
      if (event) callback(event);
    })
    .catch(() => {
      // A live event can still arrive after snapshot retrieval fails.
    });

  return () => {
    if (!active) return;
    active = false;
    unregisterLive();
  };
}
