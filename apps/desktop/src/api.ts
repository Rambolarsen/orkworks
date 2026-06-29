import type { ProviderEffectiveState } from "./providerTypes.ts";

export type MemoryState = "live" | "remembered" | "resumable" | "unsupported";
export type ResumeStrategy = "exact" | "latest_cwd" | "latest_repo" | "none";
export type SessionConnectivity = "online" | "offline";
export type TerminalOutcome = "ended" | "killed" | "error";

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
  status: string;
  connectivity?: SessionConnectivity;
  terminalOutcome?: TerminalOutcome;
  cwd: string;
  created_at: string;
  lastActivityAt?: string;
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
  peonLastInference?: string;
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

export async function listHarnesses(baseUrl: string) {
  const resp = await fetch(`${baseUrl}/harnesses`);
  if (!resp.ok) throw new Error(`list harnesses failed: ${resp.status}`);
  return resp.json();
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

export async function getTerminalOutput(
  baseUrl: string,
  id: string,
): Promise<string[]> {
  const resp = await fetch(`${baseUrl}/sessions/${id}/terminal-output`);
  if (!resp.ok) throw new Error(`get terminal output failed: ${resp.status}`);
  const data = await resp.json();
  return data.lines ?? [];
}

export interface ProviderRuntimeEntry {
  id: string;
  label: string;
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

export async function saveActiveHarnesses(baseUrl: string, activeHarnessIds: string[]): Promise<void> {
  const resp = await fetch(`${baseUrl}/workspace/active-harnesses`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ activeHarnessIds }),
  });
  if (!resp.ok) throw new Error(`save active harnesses failed: ${resp.status}`);
}
