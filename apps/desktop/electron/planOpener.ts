import { realpath, stat } from "fs/promises";
import * as nodePath from "path";

export async function openSessionPlan(
  baseUrl: string,
  sessionId: string,
  token: string,
  workspaceRoot: string,
  fetchImpl: typeof fetch,
  openPath: (filePath: string) => Promise<string>,
): Promise<void> {
  const response = await fetchImpl(`${baseUrl}/sessions/${encodeURIComponent(sessionId)}/open-plan`, {
    method: "POST",
    headers: { "x-orkworks-open-plan-token": token },
  });
  if (!response.ok) throw new Error("Couldn’t open this plan. It may have moved or is no longer available.");

  const { path: validatedPath } = await response.json() as { path: string };
  let workspace: string;
  let candidate: string;
  try {
    [workspace, candidate] = await Promise.all([realpath(workspaceRoot), realpath(validatedPath)]);
  } catch {
    throw new Error("Couldn’t open this plan. It may have moved or is no longer available.");
  }
  if (nodePath.relative(workspace, candidate).startsWith("..") || nodePath.extname(candidate).toLowerCase() !== ".md" || !(await stat(candidate)).isFile()) {
    throw new Error("Couldn’t open this plan. It may have moved or is no longer available.");
  }
  const error = await openPath(candidate);
  if (error) throw new Error("Couldn’t open this plan with the configured application.");
}
