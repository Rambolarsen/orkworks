import type { BackendLifecycleWorkspace } from "./backendLifecycleEvent";

export async function parseWorkspaceRestoreResponse(
  response: Response,
): Promise<BackendLifecycleWorkspace | null> {
  if (!response.ok) {
    if (response.status >= 400 && response.status < 500) {
      // A 4xx from a reachable sidecar means the remembered workspace path is
      // stale or invalid. That is a routine "no workspace open" case, not a
      // backend failure — it must not escalate into the failure lifecycle.
      return null;
    }
    throw new Error(`Workspace restoration failed: ${response.status}`);
  }
  const rawWorkspace = await response.json() as Partial<BackendLifecycleWorkspace>;
  return {
    path: rawWorkspace.path ?? "",
    repo_root: rawWorkspace.repo_root ?? null,
    branch: rawWorkspace.branch ?? null,
    dirty: rawWorkspace.dirty ?? null,
    lastActiveSessionId: rawWorkspace.lastActiveSessionId ?? null,
    activeHarnessIds: rawWorkspace.activeHarnessIds ?? [],
  };
}
