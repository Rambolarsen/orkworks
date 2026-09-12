import type { InferenceDefinition } from "./inferenceDefinition.ts";
// Deliberately independent of Electron's declarations across the process boundary.
export interface InferenceTrustRevision { documentRevision: string; generation: string; digest: string | null }
export interface InferenceTrustRequest { harnessId: string; expectedRevision: InferenceTrustRevision }
export interface InferenceAdapterView {
  id: string; name: string; state: "approved" | "approval_required" | "unavailable";
  definition: InferenceDefinition; resolvedPath: string | null; revision: InferenceTrustRevision;
}
export function inferenceTrustActions(view: InferenceAdapterView, busy: boolean) {
  return { canApprove: !busy && view.state === "approval_required" && !!view.resolvedPath && !!view.revision.digest, canRevoke: !busy };
}
