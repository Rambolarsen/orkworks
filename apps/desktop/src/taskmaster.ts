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

// Mirrors the Rust template in crates/orkworksd/src/taskmaster/mod.rs's
// build_fix_prompt — keep wording in sync between the two.
export function buildFixPromptDraft(recommendation: WorkflowRecommendation): string {
  const improvement = recommendation.workflowImprovement;
  const surface = improvement.targetSurface;
  return [
    `Implement the following workflow improvement so future sessions don't hit this recurring issue: ${improvement.proposedImprovement}`,
    "",
    `Target surface: ${surface} (edit the repository's ${surface} accordingly).`,
    "",
    `Why: ${recommendation.reason.join(" ")} Expected benefit: ${improvement.expectedBenefit}`,
    "",
    "Scope: only modify repository-level instructions, skills, tests, tooling, or documentation to address this recurring issue. " +
      "Do not resume, reopen, or modify any other session — this request applies only to the session you are currently running in.",
  ].join("\n");
}
