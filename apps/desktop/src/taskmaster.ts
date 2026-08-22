import type { Impact, TargetSurface, WorkflowObservationEvidence, WorkflowRecommendation } from "./api.ts";

export function formatImpact(impact: Impact): string {
  return impact[0].toUpperCase() + impact.slice(1);
}

export function formatTargetSurface(surface: TargetSurface): string {
  return surface[0].toUpperCase() + surface.slice(1);
}

export function formatRecurrence(recommendation: WorkflowRecommendation): string {
  const count = recommendation.workflowImprovement.recurrenceCount;
  const sessions = recommendation.workflowImprovement.affectedSessionIds.length;
  return `${count} occurrence${count === 1 ? "" : "s"} across ${sessions} session${sessions === 1 ? "" : "s"}`;
}

export function sortedEvidence(
  evidence: WorkflowObservationEvidence[],
): WorkflowObservationEvidence[] {
  return [...evidence].sort((left, right) => left.sequence - right.sequence);
}
