import type { HarnessConfig } from "./harnessTypes.ts";
import type { ProviderEffectiveState } from "./providerTypes.ts";

export type MemoryState = "live" | "remembered" | "resumable" | "unsupported";
export type ResumeStrategy = "exact" | "latest_cwd" | "latest_repo" | "none";
export type SessionConnectivity = "online" | "offline";
export type TerminalOutcome = "ended" | "killed" | "error";
export type WorkPhase = "ideation" | "implementation" | "review" | "debugging" | "unknown";
export type LifecyclePhase = "creating" | "active" | "ending" | "ended";
export type SessionLifecycle = "creating" | "alive" | "stopping" | "dead";
export type SessionAttention = "working" | "idle" | "needs_you" | "blocked" | "failed" | "capped";
export type PeonSchedulerState = "idle" | "candidate" | "in_flight" | "completed" | "failed";

export interface PeonDiagnostics {
  schedulerState: PeonSchedulerState;
  reason: string | null;
  lastAttemptAt: string | null;
  lastSuccessfulInferenceAt: string | null;
  providerId: string | null;
  providerModel: string | null;
  fallbackStep: number | null;
  attemptCount: number | null;
  errorSummary: string | null;
  observationCount: number | null;
}

/** Lifecycle phase with the migration fallback for payloads that predate `lifecyclePhase`. */
export function effectiveLifecyclePhase(
  status: string,
  lifecyclePhase: LifecyclePhase | undefined,
): LifecyclePhase {
  if (lifecyclePhase) return lifecyclePhase;
  if (status === "creating") return "creating";
  return status === "running" ? "active" : "ended";
}

export interface ResumeMemory {
  state: "available" | "unavailable";
  preferredStrategy: ResumeStrategy;
  harnessSessionId?: string;
  latestFallback: boolean;
  lastSeenAt?: string;
}

export interface ResumeOption {
  strategy: ResumeStrategy;
  label: string;
  available: boolean;
  preferred: boolean;
  reason?: string;
}

export interface SessionInfo {
  id: string;
  label: string;
  harnessId?: string;
  modelProviderId?: string;
  modelId?: string;
  provider?: string;
  providerModel?: string;
  providerState?: ProviderEffectiveState;
  harness?: string;
  model?: string;
  workPhase?: WorkPhase;
  lifecyclePhase?: LifecyclePhase;
  lifecycle?: SessionLifecycle;
  attention?: SessionAttention;
  status: string;
  connectivity?: SessionConnectivity;
  terminalOutcome?: TerminalOutcome;
  cwd: string;
  created_at: string;
  lastActivityAt?: string;
  lastOutputAt?: string;
  finalObservedStatus?: string | null;
  observedStatus?: string;
  summary?: string;
  nextAction?: string;
  needsUserInput?: boolean;
  detectedQuestion?: string;
  suggestedOptions?: string[];
  blockerDescription?: string;
  failedCommand?: string;
  failedTest?: string;
  capacityHints?: string[];
  atUsageLimit?: boolean;
  capacityCheckPending?: boolean;
  usageLimitResetHint?: string;
  peonLastInference?: string;
  peonDiagnostics?: PeonDiagnostics | null;
  metadataSource?: string;
  metadataConfidence?: number;
  repoRoot?: string;
  branch?: string;
  dirty?: boolean;
  changedFiles?: number;
  isWorktree?: boolean;
  conflictWarning?: string;
  recommendation?: string;
  memoryState: MemoryState;
  resumeStrategy: ResumeStrategy;
  resume?: ResumeMemory;
  resumeOptions?: ResumeOption[];
  resumedFrom?: string;
  hasOpenablePlan?: boolean;
}

export async function createSession(
  baseUrl: string,
  opts?: { harnessId?: string; model?: string; initialPrompt?: string },
): Promise<SessionInfo> {
  const resp = await fetch(`${baseUrl}/sessions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(opts ?? {}),
  });
  if (!resp.ok) throw new Error(`create session failed: ${resp.status}`);
  return resp.json();
}

export interface HarnessConfigEntry extends HarnessConfig {
  origin: "builtin" | "override" | "custom";
  profile: string | null;
  compatibility: {
    profile: string | null;
    sessionSignals: unknown;
    integration: unknown;
  };
  storedOverride?: unknown;
}

export interface HarnessListResponse {
  documentRevision: string | null;
  harnesses: HarnessConfigEntry[];
}

export async function listHarnesses(baseUrl: string): Promise<HarnessListResponse> {
  const resp = await fetch(`${baseUrl}/harnesses`);
  if (!resp.ok) throw new Error(`list harnesses failed: ${resp.status}`);
  const body = await resp.json() as {
    documentRevision?: unknown;
    harnesses?: Array<{
      definition?: HarnessConfig;
      origin?: HarnessConfigEntry["origin"];
      storedOverride?: unknown;
      compatibility?: HarnessConfigEntry["compatibility"];
    }>;
  };
  if (!Array.isArray(body?.harnesses)) throw new Error("list harnesses failed: malformed response");
  if (body.documentRevision !== null && typeof body.documentRevision !== "string" && body.documentRevision !== undefined) {
    throw new Error("list harnesses failed: malformed revision");
  }
  const harnesses = body.harnesses.map((entry) => {
    if (!entry.definition || !entry.origin || !entry.compatibility) {
      throw new Error("list harnesses failed: malformed response");
    }
    return {
      ...entry.definition,
      origin: entry.origin,
      profile: entry.compatibility.profile,
      compatibility: entry.compatibility,
      ...(entry.storedOverride === undefined ? {} : { storedOverride: entry.storedOverride }),
      sessionSignals: entry.compatibility.sessionSignals ?? entry.definition.sessionSignals,
      integration: entry.compatibility.integration ?? entry.definition.integration,
    };
  });
  return {
    documentRevision: body.documentRevision === undefined ? null : body.documentRevision,
    harnesses,
  };
}

export async function listSessions(
  baseUrl: string,
): Promise<SessionInfo[]> {
  const resp = await fetch(`${baseUrl}/sessions`);
  if (!resp.ok) throw new Error(`list sessions failed: ${resp.status}`);
  return resp.json();
}

export async function deleteSession(
  baseUrl: string,
  id: string,
): Promise<void> {
  const resp = await fetch(`${baseUrl}/sessions/${id}`, {
    method: "DELETE",
  });
  if (!resp.ok) throw new Error(`delete session failed: ${resp.status}`);
}

export async function forgetSession(
  baseUrl: string,
  id: string,
): Promise<void> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/forget`, {
    method: "DELETE",
  });
  if (!resp.ok) throw new Error(`forget session failed: ${resp.status}`);
}

export interface WorkspaceInfo {
  path: string;
  repo_root: string | null;
  branch: string | null;
  dirty: boolean | null;
  lastActiveSessionId?: string | null;
  activeHarnessIds: string[];
  activeHarnessRevision: number;
}

export async function setWorkspace(
  baseUrl: string,
  path: string,
): Promise<WorkspaceInfo> {
  const resp = await fetch(`${baseUrl}/workspace`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!resp.ok) throw new Error(`set workspace failed: ${resp.status}`);
  return resp.json();
}

export async function setActiveWorkspaceSession(
  baseUrl: string,
  sessionId: string,
): Promise<void> {
  const resp = await fetch(`${baseUrl}/workspace/active-session`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ sessionId }),
  });
  if (!resp.ok) throw new Error(`set active session failed: ${resp.status}`);
}

export async function resumeSession(
  baseUrl: string,
  id: string,
): Promise<SessionInfo> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/resume`, {
    method: "POST",
  });
  if (!resp.ok) throw new Error(`resume session failed: ${resp.status}`);
  return resp.json();
}

export async function applyDebugAttention(
  baseUrl: string,
  id: string,
  attention: SessionAttention,
  message?: string,
): Promise<void> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/debug-injection`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ attention, message }),
  });
  if (!resp.ok) throw new Error(`apply debug attention failed: ${resp.status}`);
}

export type TerminalOutputRecord = string | { text: string; delimiter: string };

export async function getTerminalOutput(
  baseUrl: string,
  id: string,
): Promise<{ lines: TerminalOutputRecord[]; cols?: number; rows?: number }> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/terminal-output`);
  if (!resp.ok) throw new Error(`get terminal output failed: ${resp.status}`);
  const data = await resp.json();
  return { lines: data.lines ?? [], cols: data.cols, rows: data.rows };
}

export interface SummaryLogEntry {
  timestamp: string;
  summary: string;
  source: string;
  confidence: number | null;
}

export async function getSummaryLog(
  baseUrl: string,
  id: string,
): Promise<SummaryLogEntry[]> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/summary-log`);
  if (!resp.ok) throw new Error(`get summary log failed: ${resp.status}`);
  const data = await resp.json();
  return data.entries ?? [];
}

export interface ProviderRuntimeEntry {
  id: string;
  label: string;
  origin: "builtin" | "override" | "custom" | "standalone";
  harnessId?: string;
  enabled: boolean;
  fallbackOrder: number;
  effectiveState: ProviderEffectiveState;
  runtime: {
    fallbackStep: number | null;
    lastErrorSummary: string | null;
    resetHint: string | null;
  };
}

export interface ProviderRuntimeResponse {
  appliedRevision: number | null;
  providers: ProviderRuntimeEntry[];
}

export async function getProviders(baseUrl: string): Promise<ProviderRuntimeResponse> {
  const resp = await fetch(`${baseUrl}/providers`);
  if (!resp.ok) throw new Error(`get providers failed: ${resp.status}`);
  return resp.json();
}

export async function saveActiveHarnesses(
  baseUrl: string,
  activeHarnessIds: string[],
  expectedActiveHarnessRevision: number,
): Promise<{ activeHarnessIds: string[]; activeHarnessRevision: number }> {
  const resp = await fetch(`${baseUrl}/workspace/active-harnesses`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ activeHarnessIds, expectedActiveHarnessRevision }),
  });
  if (!resp.ok) throw new Error(`save active harnesses failed: ${resp.status}`);
  return resp.json();
}

export type Impact = "low" | "medium" | "high";
export type RecommendationConfidence = "low" | "medium" | "high";
export type RecommendationStatus =
  | "proposed" | "accepted" | "executing" | "completed"
  | "dismissed" | "superseded" | "expired" | "failed";
export type TargetSurface = "instructions" | "skill" | "test" | "tooling" | "documentation";
export type ObservationKind =
  | "repetition" | "obstacle" | "missing_context" | "assumption"
  | "correction" | "workaround" | "verification_gap";

export interface WorkflowObservationEvidence {
  observationId: string;
  sequence: number;
  sessionId: string;
  kind: ObservationKind;
  description: string;
  evidence: string;
  reportedImpact: Impact;
  source: "agent" | "peon";
  confidence: number;
  observedAt: string;
}

export interface DismissalWatermark {
  dismissedAt: string;
  dismissedThroughSequence: number;
  observationIds: string[];
  qualifyingCount: number;
  highestImpact: Impact;
  affectedSessionIds: string[];
}

export interface WorkflowImprovement {
  proposedImprovement: string;
  targetSurface: TargetSurface;
  observationIds: string[];
  recurrenceCount: number;
  affectedSessionIds: string[];
  impact: Impact;
  expectedBenefit: string;
  supersedesRecommendationId: string | null;
  dismissalWatermark: DismissalWatermark | null;
}

export interface WorkflowRecommendation {
  id: string;
  workspaceId: string;
  chainId: string;
  chainDepth: number;
  type: "improve_workflow";
  status: RecommendationStatus;
  priority: Impact;
  title: string;
  summary: string;
  reason: string[];
  evidence: WorkflowObservationEvidence[];
  sourceSessionIds: string[];
  targetSessionId: string | null;
  suggestedHarnessId: string | null;
  suggestedModel: string | null;
  suggestedWorkingDirectory: string | null;
  suggestedPrompt: string | null;
  confidence: RecommendationConfidence;
  requiresApproval: false;
  dedupeKey: string;
  createdAt: string;
  updatedAt: string;
  expiresAt: string | null;
  workflowImprovement: WorkflowImprovement;
}

export interface ObservationDiagnostic {
  code: string;
  message: string;
  sessionId: string | null;
}

export interface RecommendationListResponse {
  recommendations: WorkflowRecommendation[];
  diagnostics: ObservationDiagnostic[];
}

export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.status = status;
    this.name = "ApiError";
  }
}

async function taskmasterRequest(baseUrl: string, path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(`${baseUrl}${path}`, init);
  if (!response.ok) throw new ApiError(`Taskmaster request failed: ${response.status}`, response.status);
  return response;
}

export async function getTaskmasterRecommendations(baseUrl: string): Promise<RecommendationListResponse> {
  const response = await taskmasterRequest(baseUrl, "/taskmaster/recommendations");
  return response.json();
}

export async function dismissTaskmasterRecommendation(
  baseUrl: string,
  id: string,
  reason?: string,
): Promise<void> {
  await taskmasterRequest(baseUrl, `/taskmaster/recommendations/${encodeURIComponent(id)}/dismiss`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(reason === undefined ? {} : { reason }),
  });
}
