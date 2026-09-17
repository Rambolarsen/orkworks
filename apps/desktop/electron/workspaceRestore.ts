import type { BackendLifecycleWorkspace } from "./backendLifecycleEvent";

export type WorkspaceRestoreResult =
  | { ok: true; workspace: BackendLifecycleWorkspace }
  | { ok: false; status: number; removeFromHistory: boolean };

export async function parseWorkspaceRestoreResponse(
  response: Response,
): Promise<WorkspaceRestoreResult> {
  if (!response.ok) {
    if (response.status >= 400 && response.status < 500) {
      return {
        ok: false,
        status: response.status,
        // Only a genuine missing workspace is safe to remove. Lease and
        // accessibility conflicts must remain available for a later retry.
        removeFromHistory: response.status === 404,
      };
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
    ok: true,
    workspace: {
      path: rawWorkspace.path ?? "",
      repo_root: rawWorkspace.repo_root ?? null,
      branch: rawWorkspace.branch ?? null,
      dirty: rawWorkspace.dirty ?? null,
      lastActiveSessionId: rawWorkspace.lastActiveSessionId ?? null,
      activeHarnessIds: rawWorkspace.activeHarnessIds ?? [],
      activeHarnessRevision: restoredActiveHarnessRevision,
    },
  };
}
