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
}
export interface TaskmasterRunStatus {
  workspacePath: string | null; activeAttempt: TaskmasterRunAttempt | null; latestOutcome: TaskmasterRunOutcome | null;
}
export function formatTaskmasterRunStatus(status: TaskmasterRunStatus): string {
  const active = status.activeAttempt;
  if (active) return `${active.state === "queued" ? "Queued" : "Running"} ${active.trigger} analysis · ${active.provider} / ${active.model}`;
  const outcome = status.latestOutcome;
  if (!outcome) return "No analysis runs for this workspace.";
  const detail = outcome.state === "failed" && outcome.errorSummary ? `: ${outcome.errorSummary}` : "";
  return `${outcome.state[0].toUpperCase()}${outcome.state.slice(1)} ${outcome.trigger} analysis · ${outcome.provider} / ${outcome.model}${detail}`;
}
export function formatTaskmasterRunTimestamp(status: TaskmasterRunStatus): string {
  const active = status.activeAttempt;
  if (active) return active.startedAt ? `Started ${active.startedAt}` : `Queued ${active.queuedAt}`;
  return status.latestOutcome ? `Finished ${status.latestOutcome.completedAt}` : "";
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
