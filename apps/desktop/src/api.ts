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
  const harnesses = body.harnesses.map((entry) => mapHarnessEntry(entry, documentRevision, "list harnesses"));
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
  if (typeof body.documentRevision !== "string" || !body.harness || typeof body.harness !== "object") {
    throw new Error("harness mutation failed: malformed response");
  }
  return {
    ...body,
    documentRevision: body.documentRevision,
    harness: mapHarnessEntry(body.harness, body.documentRevision, "harness mutation"),
  } as HarnessMutationResponse;
}

function mapHarnessEntry(
  value: unknown,
  documentRevision: string | null,
  context: string,
): HarnessConfigEntry {
  if (!value || typeof value !== "object") throw new Error(`${context} failed: malformed response`);
  const entry = value as {
    definition?: HarnessConfig;
    origin?: HarnessConfigEntry["origin"];
    storedOverride?: unknown;
    compatibility?: HarnessConfigEntry["compatibility"];
  };
  if (!entry.definition || !entry.origin || !entry.compatibility) {
    throw new Error(`${context} failed: malformed response`);
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
  workspaceIdentity: string;
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
  id: string,
  attention: SessionAttention,
  message?: string,
): Promise<void> {
  await window.orkworks.applyDebugAttention(id, attention, message);
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
  recommendationId?: string;
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

export interface WorkflowObservationEntry {
  id: string;
  sequence: number;
  observedAt: string;
  kind: string;
  description: string;
  evidence: string;
  problemArea?: string;
  reportedImpact: string;
  source: string;
  confidence: number;
}

export async function getSessionWorkflowObservations(
  baseUrl: string,
  id: string,
): Promise<WorkflowObservationEntry[]> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/workflow-observations`);
  if (!resp.ok) throw new Error(`get workflow observations failed: ${resp.status}`);
  const data = await resp.json();
  return data.observations ?? [];
}

export interface ProviderRuntimeEntry {
  id: string;
  inferenceOnly?: boolean;
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

export type Impact = "low" | "medium" | "high";
export type RecommendationConfidence = "low" | "medium" | "high";
export type RecommendationStatus =
  | "proposed" | "accepted" | "executing" | "completed"
  | "dismissed" | "rolled_up" | "superseded" | "expired" | "failed";
export type TargetSurface = "instructions" | "skill" | "test" | "tooling" | "documentation";
export type ObservationKind =
  | "repetition" | "obstacle" | "missing_context" | "assumption"
  | "correction" | "workaround" | "verification_gap";
export type PacketAttribution = "unambiguous" | "inconclusive";
export type PacketReadiness =
  | "verification_needed" | "review_ready" | "findings_need_fix" | "ready_for_user_review";
export type CompletionVerificationResult = "passed" | "failed" | "not_run";
export type CompletionReviewOutcome = "no_findings" | "findings";
export type CompletionActionRole = "review" | "verification" | "fix" | "user_review";
export type CompletionEvidenceIssueKind = "missing" | "conflict";

export interface CompletionEvidenceProvenance {
  sourceSessionId: string;
  workspaceId: string;
  workspaceSnapshotId: string;
  observedAt: string;
}

export interface CompletionChangeSubject {
  workspaceId: string;
  scope: string;
  snapshotId: string;
  attribution: PacketAttribution;
  changedPaths: string[];
}

export interface CompletionVerification {
  command: string;
  result: CompletionVerificationResult;
  applicableRevision: string;
  observedAt: string;
}

export interface CompletionReview {
  reviewerSessionId: string;
  reviewerIdentity: string;
  independent: boolean;
  outcome: CompletionReviewOutcome;
  applicableRevision: string;
  observedAt: string;
}

export interface CompletionEvidenceIssue {
  kind: CompletionEvidenceIssueKind;
  detail: string;
}

export interface CompletionAction {
  prompt: string;
  model: string | null;
  scope: string;
  role: CompletionActionRole;
}

export interface CompletionApproval {
  approvedAt: string;
  approver: string;
  revision: number;
  evidenceFingerprint: string;
  actionFingerprint: string;
  idempotencyKey: string;
}

export interface CompletionPacketLineage {
  packetId: string;
  revision: number;
  evidenceFingerprint: string;
  supersededAt: string;
}

export interface CompletionPacket {
  schemaVersion: number;
  evidenceVersion: number;
  packetId: string;
  sourceSessionId: string;
  observedAt: string;
  provenance: CompletionEvidenceProvenance;
  subject: CompletionChangeSubject;
  verification: CompletionVerification | null;
  review: CompletionReview | null;
  missingEvidence: CompletionEvidenceIssue[];
  conflictingEvidence: CompletionEvidenceIssue[];
  action: CompletionAction;
  readiness: PacketReadiness;
  revision: number;
  evidenceFingerprint: string;
  approval: CompletionApproval | null;
  supersedesPacketId: string | null;
  lineage: CompletionPacketLineage[];
  completionIdempotencyKey: string | null;
}

export interface WorkflowObservationEvidence {
  observationId: string;
  sequence: number;
  sessionId: string;
  kind: ObservationKind;
  description: string;
  evidence: string;
  problemArea?: string | null;
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
  repositoryEvidence?: Array<{ path: string; sha256: string; excerpt: string; observedAt: string }>;
  knowledgeEvidence?: Array<{ pageId: string; title: string; status: string; bundleVersion: string; sha256: string; excerpt: string }>;
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
  completionPacket?: CompletionPacket | null;
  rollupMemberIds: string[];
  rollupMemberDedupeKeys: string[];
  rollupGeneration: number | null;
  rolledUpBy: string | null;
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

export interface ManualTaskmasterAnalysisResponse {
  status:
    | "scheduled"
    | "active_recommendation"
    | "already_running"
    | "daily_limit_reached"
    | "unavailable";
  recommendation?: WorkflowRecommendation;
  message?: string;
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

export async function getTaskmasterRecommendation(
  baseUrl: string,
  id: string,
): Promise<WorkflowRecommendation> {
  const response = await taskmasterRequest(
    baseUrl,
    `/taskmaster/recommendations/${encodeURIComponent(id)}`,
  );
  return response.json();
}

export async function dismissTaskmasterRecommendation(
  id: string,
  reason?: string,
): Promise<void> {
  await window.orkworks.dismissTaskmasterRecommendation(id, reason);
}

export interface AcceptRecommendationOptions {
  sessionId: string;
  prompt?: string;
  packetRevision?: number;
  evidenceFingerprint?: string;
  idempotencyKey?: string;
}

export async function acceptTaskmasterRecommendation(
  id: string,
  opts: AcceptRecommendationOptions,
): Promise<WorkflowRecommendation> {
  return window.orkworks.acceptTaskmasterRecommendation(id, opts);
}
