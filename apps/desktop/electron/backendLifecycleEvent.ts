export type BackendLifecycleEvent =
  | { state: "starting" | "retrying" }
  | { state: "ready"; port: number; workspace: BackendLifecycleWorkspace }
  | { state: "failed" | "exhausted"; message: string };

export interface BackendLifecycleWorkspace {
  path: string;
  repo_root: string | null;
  branch: string | null;
  dirty: boolean | null;
  lastActiveSessionId: string | null;
  activeHarnessIds: string[];
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
  ])) return null;

  const workspace = value as Record<string, unknown>;
  return typeof workspace.path === "string"
    && (typeof workspace.repo_root === "string" || workspace.repo_root === null)
    && (typeof workspace.branch === "string" || workspace.branch === null)
    && (typeof workspace.dirty === "boolean" || workspace.dirty === null)
    && (typeof workspace.lastActiveSessionId === "string" || workspace.lastActiveSessionId === null)
    && Array.isArray(workspace.activeHarnessIds)
    && workspace.activeHarnessIds.every((id) => typeof id === "string")
    ? {
      path: workspace.path,
      repo_root: workspace.repo_root,
      branch: workspace.branch,
      dirty: workspace.dirty,
      lastActiveSessionId: workspace.lastActiveSessionId,
      activeHarnessIds: [...workspace.activeHarnessIds],
    }
    : null;
}

export function canonicalizeBackendLifecycleEvent(data: unknown): BackendLifecycleEvent | null {
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;

  try {
    const event = data as Record<string, unknown>;
    const state = event.state;
    if (state === "starting" || state === "retrying") {
      return hasExactKeys(data, ["state"]) ? { state } : null;
    }
    if (state === "ready") {
      const port = event.port;
      const workspace = canonicalizeWorkspace(event.workspace);
      return hasExactKeys(data, ["state", "port", "workspace"])
        && typeof port === "number"
        && Number.isInteger(port)
        && port >= 1
        && port <= 65_535
        && workspace !== null
        ? { state: "ready", port, workspace }
        : null;
    }
    if (state === "failed" || state === "exhausted") {
      const message = event.message;
      return hasExactKeys(data, ["state", "message"]) && typeof message === "string"
        ? { state, message }
        : null;
    }
  } catch {
    return null;
  }

  return null;
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
