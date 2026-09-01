import type {
  HarnessConfig,
  HarnessConfigEntry,
  HarnessEditorMode,
  HarnessListResponse,
} from "./harnessTypes.ts";
import type { ProviderEffectiveState } from "./providerTypes.ts";

export type { HarnessConfigEntry, HarnessEditorMode, HarnessListResponse } from "./harnessTypes.ts";

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
  const documentRevision = typeof body.documentRevision === "string" ? body.documentRevision : null;
  const harnesses = body.harnesses.map((entry) => {
    if (!entry.definition || !entry.origin || !entry.compatibility) {
      throw new Error("list harnesses failed: malformed response");
    }
    return {
      ...entry.definition,
      origin: entry.origin,
      profile: entry.compatibility.profile,
      compatibility: entry.compatibility,
      documentRevision,
      ...(entry.storedOverride === undefined ? {} : { storedOverride: entry.storedOverride }),
      sessionSignals: entry.compatibility.sessionSignals ?? entry.definition.sessionSignals,
      integration: entry.compatibility.integration ?? entry.definition.integration,
    };
  });
  return {
    documentRevision,
    harnesses,
  };
}

export interface HarnessApiDiagnostic {
  code?: string;
  message?: string;
  path?: string;
  line?: number;
  column?: number;
}

export const HARNESS_ACTIVE_DELETE_FORBIDDEN_CODE = "active_harness_delete_forbidden";
export const HARNESS_REVISION_CONFLICT_CODE = "harness_config_revision_changed";

export class HarnessApiError extends Error {
  readonly status: number;
  readonly code: string | null;
  readonly diagnostics: HarnessApiDiagnostic[];
  readonly documentRevision: string | null;

  constructor(
    message: string,
    details: {
      status: number;
      code?: string | null;
      diagnostics?: HarnessApiDiagnostic[];
      documentRevision?: string | null;
    },
  ) {
    super(message);
    this.name = "HarnessApiError";
    this.status = details.status;
    this.code = details.code ?? null;
    this.diagnostics = details.diagnostics ?? [];
    this.documentRevision = details.documentRevision ?? null;
  }
}

export interface DuplicateHarnessResponse {
  documentRevision: string | null;
  definition: Record<string, unknown>;
  proposedId: string;
  proposedName: string;
}

export interface HarnessMutationResponse {
  documentRevision: string;
  harness: HarnessConfigEntry;
  integrationCleanup?: unknown;
}

export interface HarnessDeleteResponse {
  documentRevision: string;
  integrationCleanup?: unknown;
}

export interface SaveHarnessConfigurationRequest {
  mode: HarnessEditorMode;
  harnessId?: string;
  definition: unknown;
  expectedRevision: string | null;
  duplicateSourceId?: string;
}

export async function duplicateHarness(
  baseUrl: string,
  sourceId: string,
): Promise<DuplicateHarnessResponse> {
  const resp = await fetch(`${baseUrl}/harnesses/${encodeURIComponent(sourceId)}/duplicate`, { method: "POST" });
  if (!resp.ok) await throwHarnessApiError(resp, "duplicate harness failed");
  const body = await resp.json() as Partial<DuplicateHarnessResponse>;
  if (
    (typeof body.documentRevision !== "string" && body.documentRevision !== null && body.documentRevision !== undefined)
    || !body.definition || typeof body.definition !== "object"
    || typeof body.proposedId !== "string" || typeof body.proposedName !== "string"
  ) {
    throw new Error("duplicate harness failed: malformed response");
  }
  return {
    documentRevision: body.documentRevision === undefined ? null : body.documentRevision,
    definition: body.definition as Record<string, unknown>,
    proposedId: body.proposedId,
    proposedName: body.proposedName,
  };
}

export async function saveHarnessConfiguration(
  baseUrl: string,
  request: SaveHarnessConfigurationRequest,
): Promise<HarnessMutationResponse> {
  const editable = stripDerivedHarnessFields(request.definition);
  let url = `${baseUrl}/harnesses`;
  let method = "POST";
  let body: Record<string, unknown> = {
    definition: editable,
    expectedRevision: request.expectedRevision,
    ...(request.duplicateSourceId ? { duplicateSourceId: request.duplicateSourceId } : {}),
  };
  if (request.mode !== "create") {
    if (!request.harnessId) throw new Error("Harness ID is required for an update.");
    url = `${baseUrl}/harnesses/${encodeURIComponent(request.harnessId)}`;
    method = "PUT";
    body = request.mode === "custom"
      ? { kind: "CustomReplace", definition: editable, expectedRevision: request.expectedRevision }
      : { kind: "BuiltinPatch", patch: editable, expectedRevision: request.expectedRevision };
  }
  const resp = await fetch(url, {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) await throwHarnessApiError(resp, "save harness configuration failed");
  return parseHarnessMutationResponse(await resp.json());
}

export async function removeHarnessProfile(
  baseUrl: string,
  harnessId: string,
  expectedRevision: string | null,
): Promise<HarnessMutationResponse> {
  const resp = await fetch(`${baseUrl}/harnesses/${encodeURIComponent(harnessId)}/remove-profile`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ expectedRevision }),
  });
  if (!resp.ok) await throwHarnessApiError(resp, "remove harness profile failed");
  return parseHarnessMutationResponse(await resp.json());
}

export async function deleteHarness(
  baseUrl: string,
  harnessId: string,
  expectedRevision: string | null,
): Promise<HarnessDeleteResponse> {
  const resp = await fetch(`${baseUrl}/harnesses/${encodeURIComponent(harnessId)}`, {
    method: "DELETE",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ expectedRevision }),
  });
  if (!resp.ok) await throwHarnessApiError(resp, "delete harness failed");
  const body = await resp.json() as Partial<HarnessDeleteResponse>;
  if (typeof body.documentRevision !== "string") throw new Error("delete harness failed: malformed response");
  return body as HarnessDeleteResponse;
}

export function stripDerivedHarnessFields(value: unknown): unknown {
  if (!value || typeof value !== "object" || Array.isArray(value)) return value;
  const editable = { ...(value as Record<string, unknown>) };
  for (const field of ["integration", "sessionSignals", "compatibilityProfile", "compatibilityProfiles", "profile", "compatibility", "origin", "storedOverride", "documentRevision"]) {
    delete editable[field];
  }
  return editable;
}

async function throwHarnessApiError(resp: Response, fallback: string): Promise<never> {
  const body = await resp.json().catch(() => null) as {
    error?: unknown;
    diagnostics?: unknown;
    documentRevision?: unknown;
  } | null;
  const diagnostics = Array.isArray(body?.diagnostics)
    ? body.diagnostics.filter((diagnostic): diagnostic is HarnessApiDiagnostic => !!diagnostic && typeof diagnostic === "object")
    : [];
  const firstCode = typeof diagnostics[0]?.code === "string" ? diagnostics[0].code : null;
  const message = typeof body?.error === "string" ? body.error : fallback;
  throw new HarnessApiError(message, {
    status: resp.status,
    code: firstCode ?? (resp.status === 409 ? HARNESS_REVISION_CONFLICT_CODE : null),
    diagnostics,
    documentRevision: typeof body?.documentRevision === "string" ? body.documentRevision : null,
  });
}

function parseHarnessMutationResponse(value: unknown): HarnessMutationResponse {
  if (!value || typeof value !== "object") throw new Error("harness mutation failed: malformed response");
  const body = value as Partial<HarnessMutationResponse>;
  const harness = body.harness;
  if (typeof body.documentRevision !== "string" || !harness || typeof harness !== "object") {
    throw new Error("harness mutation failed: malformed response");
  }
  return body as HarnessMutationResponse;
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
