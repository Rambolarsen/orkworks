import type { KnowledgeEvidence, RepositoryEvidence } from "./api.ts";

export type AnalysisContext = "session_observations" | "workflow_context" | "source_code";
export type TaskmasterProviderState = "ready" | "unsupported_capability" | "approval_required" | "execution_inactive" | "unavailable";
export interface TaskmasterProvider {
  id: string; label: string; state: TaskmasterProviderState;
  models: string[]; supportsReasoningEffort: boolean;
}
export function supportsTaskmasterAnalysis(provider: Pick<TaskmasterProvider, "id" | "state">): boolean {
  return provider.state === "ready" || provider.state === "approval_required" || provider.state === "execution_inactive";
}
export interface TaskmasterSelection {
  provider: string; model: string; reasoningEffort?: string; ollamaBaseUrl?: string;
}
export interface TaskmasterOverride {
  enabled?: boolean; selection?: TaskmasterSelection | null; contextLevel?: AnalysisContext;
  excludedPaths?: string[]; minIntervalMinutes?: number;
}
export interface TaskmasterSettings {
  enabled: boolean; selection: TaskmasterSelection | null; contextLevel: AnalysisContext;
  excludedPaths: string[]; dailyEvaluationLimit: number; minIntervalMinutes: number;
  automaticKnowledgeUpdates: boolean; workspaceOverrides: Record<string, TaskmasterOverride>;
}
export interface TaskmasterSettingsStatus {
  providers: TaskmasterProvider[];
  settings: TaskmasterSettings; effectiveSettings: TaskmasterSettings;
  remainingEvaluations: number; analysisStatus: string; knowledgeVersion: string | null;
  lastEvaluatedAt: string | null; workspacePath: string | null;
  knowledgeUpdate: { version: string | null; lastSuccessfulUpdate: string | null; lastError: string | null };
}
export type TaskmasterRunTrigger = "manual" | "background";
export type TaskmasterRunAttemptState = "queued" | "running";
export type TaskmasterRunOutcomeState = "succeeded" | "failed" | "interrupted";
export interface TaskmasterRunAttempt {
  id: number; state: TaskmasterRunAttemptState; queuedAt: string; startedAt: string | null;
  trigger: TaskmasterRunTrigger; provider: string; model: string;
}
export interface TaskmasterRunOutcome {
  state: TaskmasterRunOutcomeState; startedAt: string; completedAt: string;
  trigger: TaskmasterRunTrigger; provider: string; model: string; errorSummary: string | null;
  proposalCounts?: { created: number; suppressed: number };
}
export type TaskmasterAssessmentState = "idle" | "queued" | "running" | "succeeded" | "failed" | "interrupted" | "unavailable";
export interface AssessmentReport {
  id: string;
  workspacePath: string;
  observedAt: string;
  provider: string;
  model: string;
  bundleVersion: string;
  outcome: "proposed" | "no_proposal";
  summary: string;
  noProposalReason: string | null;
  unverifiedChecks: string[];
  repositoryEvidence: RepositoryEvidence[];
  knowledgeEvidence: KnowledgeEvidence[];
  recommendationId: string | null;
}
export interface TaskmasterAssessmentStatus {
  state: TaskmasterAssessmentState;
  report?: AssessmentReport | null;
  errorSummary?: string | null;
  reused: boolean;
}
export interface TaskmasterManualRequest {
  id: number;
  requestedAt: string;
  completedAt: string;
  disposition: "cached" | "skipped";
  reason: "unchanged_inputs" | "context_changed" | "unavailable";
}
export interface TaskmasterRunStatus {
  workspacePath: string | null; activeAttempt: TaskmasterRunAttempt | null; latestOutcome: TaskmasterRunOutcome | null;
  assessment?: TaskmasterAssessmentStatus;
  latestManualRequest?: TaskmasterManualRequest | null;
}
export function formatTaskmasterRunStatus(status: TaskmasterRunStatus): string {
  const active = status.activeAttempt;
  if (active) return `${active.state === "queued" ? "Queued" : "Running"} ${active.trigger} analysis · ${active.provider} / ${active.model}`;
  const outcome = status.latestOutcome;
  if (!outcome) return "No analysis runs for this workspace.";
  const detail = outcome.errorSummary ? `: ${outcome.errorSummary}` : "";
  return `${outcome.state[0].toUpperCase()}${outcome.state.slice(1)} ${outcome.trigger} analysis · ${outcome.provider} / ${outcome.model}${detail}`;
}
export function formatTaskmasterRunTimestamp(status: TaskmasterRunStatus): string {
  const active = status.activeAttempt;
  if (active) return active.startedAt ? `Started ${active.startedAt}` : `Queued ${active.queuedAt}`;
  return status.latestOutcome ? `Finished ${status.latestOutcome.completedAt}` : "";
}
export function taskmasterAnalysisBusy(status: TaskmasterRunStatus | null): boolean {
  return Boolean(status?.activeAttempt
    || status?.assessment?.state === "queued"
    || status?.assessment?.state === "running");
}
export function taskmasterAssessmentForWorkspace(
  status: TaskmasterRunStatus | null,
  workspacePath: string,
): TaskmasterAssessmentStatus | null {
  if (!status || status.workspacePath !== workspacePath) return null;
  const assessment = status.assessment;
  if (assessment?.report && assessment.report.workspacePath !== workspacePath) return null;
  return assessment ?? null;
}
export function formatTaskmasterAssessment(assessment: TaskmasterAssessmentStatus): string {
  if (assessment.state === "idle") return "No assessment has run for this workspace.";
  if (assessment.state === "queued") return "Workflow assessment queued.";
  if (assessment.state === "running") return "Assessing this workspace…";
  if (assessment.state === "unavailable") return assessment.errorSummary ?? "Workflow assessment is unavailable.";
  if (assessment.state === "failed" || assessment.state === "interrupted") {
    return assessment.errorSummary ?? `Workflow assessment ${assessment.state}.`;
  }
  const report = assessment.report;
  if (!report) return assessment.errorSummary ?? "Assessment report is unavailable.";
  const prefix = assessment.reused ? "Reused assessment" : "Assessment complete";
  const reason = report.outcome === "no_proposal" && report.noProposalReason ? ` · No proposal: ${report.noProposalReason}` : "";
  return `${prefix} · ${report.provider} / ${report.model} · ${report.summary}${reason}`;
}
export function formatTaskmasterManualRequest(request: TaskmasterManualRequest): string {
  if (request.disposition === "cached" && request.reason === "unchanged_inputs") {
    return "Analysis reused the cached result because its inputs were unchanged.";
  }
  if (request.reason === "context_changed") return "The analysis request was skipped because its context changed.";
  return "The analysis request was skipped because Taskmaster was unavailable.";
}
export function formatTaskmasterProposalCounts(
  outcome: TaskmasterRunOutcome | null,
): string {
  const counts = outcome?.proposalCounts;
  if (!counts) return "";
  if (counts.created === 0 && counts.suppressed === 0) return "No proposals were created.";
  const created = counts.created === 0 ? "No new proposals" : counts.created === 1 ? "1 proposal created" : `${counts.created} proposals created`;
  if (!counts.suppressed) return created;
  const suppressed = counts.suppressed === 1 ? "1 duplicate suppressed" : `${counts.suppressed} duplicates suppressed`;
  return `${created} · ${suppressed}`;
}
export function editTaskmasterScope(settings: TaskmasterSettings, workspace: string | null, patch: Partial<TaskmasterSettings> | null): TaskmasterSettings {
  if (!workspace) return { ...settings, ...patch };
  const workspaceOverrides = { ...settings.workspaceOverrides };
  if (patch === null) delete workspaceOverrides[workspace];
  else {
    const override = { ...workspaceOverrides[workspace] };
    for (const key of ["enabled", "selection", "contextLevel", "excludedPaths", "minIntervalMinutes"] as const) {
      if (key in patch) Object.assign(override, { [key]: patch[key] });
    }
    workspaceOverrides[workspace] = override;
  }
  return { ...settings, workspaceOverrides };
}
