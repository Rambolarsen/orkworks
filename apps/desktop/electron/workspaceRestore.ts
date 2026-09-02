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
  const restoredActiveHarnessRevision = rawWorkspace.activeHarnessRevision;
  if (
    typeof restoredActiveHarnessRevision !== "number" ||
    !Number.isSafeInteger(restoredActiveHarnessRevision) ||
    restoredActiveHarnessRevision < 0
  ) {
    throw new Error("Workspace restoration returned an invalid active harness revision.");
  }
  return {
    path: rawWorkspace.path ?? "",
    repo_root: rawWorkspace.repo_root ?? null,
    branch: rawWorkspace.branch ?? null,
    dirty: rawWorkspace.dirty ?? null,
    lastActiveSessionId: rawWorkspace.lastActiveSessionId ?? null,
    activeHarnessIds: rawWorkspace.activeHarnessIds ?? [],
    activeHarnessRevision: restoredActiveHarnessRevision,
  };
}
