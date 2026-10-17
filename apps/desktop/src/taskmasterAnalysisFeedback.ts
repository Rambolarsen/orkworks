import type { TaskmasterRunStatus } from "./taskmasterSettings.ts";
export type { TaskmasterRunStatus } from "./taskmasterSettings.ts";

// A request acknowledgement must not outlive a newer authoritative record.
// Compare identities rather than card counts: an empty result is still complete.
export function analysisStatusAdvanced(status: TaskmasterRunStatus | null, baseline: TaskmasterRunStatus): boolean {
  if (!status || status.workspacePath !== baseline.workspacePath) return false;
  return Boolean(status.activeAttempt && status.activeAttempt.id !== baseline.activeAttempt?.id)
    || Boolean(status.latestOutcome && status.latestOutcome.completedAt !== baseline.latestOutcome?.completedAt)
    || Boolean(status.latestManualRequest && status.latestManualRequest.id !== baseline.latestManualRequest?.id);
}

export function formatAnalysisDuration(startedAt: string, now: number): string | null {
  const start = Date.parse(startedAt);
  if (!Number.isFinite(start) || !Number.isFinite(now)) return null;
  const seconds = Math.max(0, Math.floor((now - start) / 1000));
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}
