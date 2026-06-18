export type MemoryState = "live" | "remembered" | "resumable" | "unsupported";
export type ResumeStrategy = "exact" | "latest_cwd" | "latest_repo" | "none";

export interface ResumeMemory {
  state: "available" | "unavailable";
  preferredStrategy: ResumeStrategy;
  harnessSessionId?: string;
  latestFallback: boolean;
  lastSeenAt?: string;
}

export interface SessionInfo {
  id: string;
  label: string;
  status: string;
  cwd: string;
  created_at: string;
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
  resumedFrom?: string;
}

export async function createSession(
  baseUrl: string,
): Promise<SessionInfo> {
  const resp = await fetch(`${baseUrl}/sessions`, { method: "POST" });
  if (!resp.ok) throw new Error(`create session failed: ${resp.status}`);
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

export interface WorkspaceInfo {
  path: string;
  repo_root: string | null;
  branch: string | null;
  dirty: boolean | null;
  lastActiveSessionId?: string | null;
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
