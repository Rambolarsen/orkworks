export interface SessionInfo {
  id: string;
  label: string;
  status: string;
  cwd: string;
  created_at: string;
  metadataSource?: string;
  metadataConfidence?: number;
  repoRoot?: string;
  branch?: string;
  dirty?: boolean;
  changedFiles?: number;
  isWorktree?: boolean;
  conflictWarning?: string;
  recommendation?: string;
  peonLastInference?: string;
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
