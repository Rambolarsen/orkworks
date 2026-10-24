import type { GitChangesRequest, GitChangesResult } from "./gitChangesTypes.ts";
export function validateGitChangesRequest(value: unknown): asserts value is GitChangesRequest {
  if (!value || typeof value !== "object") throw new Error("Invalid changes request.");
  const r = value as Record<string, unknown>;
  const id = (v: unknown) => typeof v === "string" && v.length <= 256 && /^[a-zA-Z0-9_-]+$/.test(v);
  if (!id(r.sessionId)) throw new Error("Invalid session ID.");
  if (r.kind === "bases") return;
  if (r.kind === "capture" && (r.comparison === "uncommitted" || r.comparison === "branch") &&
    (r.baseRef === undefined || (typeof r.baseRef === "string" && r.baseRef.length <= 4096 && /^refs\/(heads|remotes)\/[^\s~^:?*\[\]\\]+$/.test(r.baseRef)))) return;
  if (r.kind === "file" && id(r.snapshotId) && id(r.fileId)) return;
  throw new Error("Invalid changes request.");
}
export async function readGitChanges(baseUrl: string, input: unknown, token: string, fetchImpl: typeof fetch, signal: AbortSignal): Promise<GitChangesResult> {
  validateGitChangesRequest(input);
  const root = `${baseUrl}/sessions/${encodeURIComponent(input.sessionId)}`;
  let path: string;
  if (input.kind === "bases") path = `${root}/git-change-bases`;
  else if (input.kind === "file") path = `${root}/git-changes/${encodeURIComponent(input.snapshotId)}/files/${encodeURIComponent(input.fileId)}`;
  else {
    const query = new URLSearchParams({ comparison: input.comparison });
    if (input.comparison === "branch" && input.baseRef) query.set("baseRef", input.baseRef);
    path = `${root}/git-changes?${query}`;
  }
  const response = await fetchImpl(path, { headers: { "x-orkworks-open-plan-token": token }, signal });
  const body = await response.json();
  if (!response.ok) throw new Error(typeof body?.error === "string" ? body.error : "Changes unavailable; refresh.");
  return body as GitChangesResult;
}
