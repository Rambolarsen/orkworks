import type { ActiveHarnessIntegrationResult } from "./harnessIntegrationPresentation.ts";

export function mergeIntegrationOperationFailures(
  current: Record<string, ActiveHarnessIntegrationResult>,
  results: Record<string, ActiveHarnessIntegrationResult>,
): Record<string, ActiveHarnessIntegrationResult> {
  const next = { ...current };
  for (const [resultKey, result] of Object.entries(results)) {
    // Keep accepting the pre-grouped shape here while older renderer callers
    // drain during the IPC contract rollout. New grouped results always carry
    // consumerHarnessIds, so one result is still projected to every row.
    const consumerHarnessIds = result.consumerHarnessIds ?? [resultKey];
    for (const harnessId of consumerHarnessIds) {
      if (result.outcome === "failed") {
        next[harnessId] = result;
        continue;
      }
      if (clearsIntegrationOperationFailure(result)) {
        delete next[harnessId];
      }
    }
  }
  return next;
}

function clearsIntegrationOperationFailure(result: ActiveHarnessIntegrationResult): boolean {
  // "unsupported" also clears: if the tool fell out of eligibility, the last
  // operation's "action required" failure is no longer actionable.
  return result.outcome === "succeeded" || result.outcome === "unsupported";
}
