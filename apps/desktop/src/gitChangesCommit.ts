import type { GitBranchCommit, GitChangesCapture, GitComparison, GitFilePatch } from "./gitChangesTypes.ts";

export function shortOid(oid: string | null | undefined): string {
  return oid ? oid.slice(0, 8) : "empty tree";
}

export function comparisonKey(comparison: GitComparison, baseRef: string, commitOid: string): string {
  if (comparison !== "branch") return `${comparison}::`;
  return `branch:${baseRef}:${commitOid}`;
}

export function commitOptionLabel(commit: GitBranchCommit): string {
  const tag = commit.kind === "merge" ? " [merge]" : commit.kind === "root" ? " [root]" : "";
  return `${commit.shortOid} · ${commit.subject}${tag}`;
}

export function comparisonBaselineLabel(
  comparison: GitComparison,
  baseRef: string,
  capture: Pick<GitChangesCapture, "commitOid" | "commitKind" | "commitParentCount" | "baselineOid"> | null | undefined,
): string {
  if (comparison === "uncommitted") {
    return "HEAD → working tree · staged, unstaged, new files";
  }
  if (capture?.commitOid) {
    const target = shortOid(capture.commitOid);
    if (capture.commitKind === "root") {
      return `empty tree → ${target} · root commit · committed only`;
    }
    if (capture.commitKind === "merge") {
      const parents = capture.commitParentCount ?? 2;
      return `first parent ${shortOid(capture.baselineOid)} → ${target} · merge commit (${parents} parents, first-parent diff) · committed only`;
    }
    return `${shortOid(capture.baselineOid)} → ${target} · single commit · committed only`;
  }
  return `merge-base(${baseRef.replace(/^refs\/(heads|remotes)\//, "") || "choose base"}, HEAD) → HEAD · committed only`;
}

export function isCaptureMatchingSelection(
  capture: Pick<GitChangesCapture, "sessionId" | "comparison" | "baseRef" | "commitOid">,
  sessionId: string,
  comparison: GitComparison,
  comparisonBase: string,
  comparisonCommit: string,
): boolean {
  if (capture.sessionId !== sessionId || capture.comparison !== comparison) return false;
  if (comparison === "branch") {
    if (capture.baseRef !== comparisonBase) return false;
    if ((capture.commitOid ?? "") !== comparisonCommit) return false;
  }
  return true;
}

export interface BranchCommitCache {
  baseRef: string;
  sessionBranch: string;
  commits: GitBranchCommit[];
  limited: boolean;
}

export function selectActiveBranchCommits(
  comparison: GitComparison,
  comparisonBase: string,
  sessionBranch: string,
  cache: BranchCommitCache | null,
): BranchCommitCache | null {
  if (comparison !== "branch" || !cache) return null;
  if (cache.baseRef !== comparisonBase || cache.sessionBranch !== sessionBranch) return null;
  return cache;
}

export function isPatchMatchingCapture(
  patch: Pick<GitFilePatch, "snapshotId" | "fileId">,
  snapshotId: string,
  fileId: string,
): boolean {
  return patch.snapshotId === snapshotId && patch.fileId === fileId;
}
