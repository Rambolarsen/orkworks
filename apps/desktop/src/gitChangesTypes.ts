export type GitComparison = "uncommitted" | "branch";
export type GitCommitKind = "ordinary" | "merge" | "root";
export interface GitBranchCommit {
  oid: string;
  shortOid: string;
  subject: string;
  kind: GitCommitKind;
  parentCount: number;
  baselineOid: string | null;
}
export type GitChangesRequest = { sessionId: string } & (
  | { kind: "bases" }
  | { kind: "capture"; comparison: GitComparison; baseRef?: string; commitOid?: string }
  | { kind: "file"; snapshotId: string; fileId: string }
);
export interface GitChangeBases { refs: string[]; defaultRef: string | null; limited: boolean; unavailableReason: string | null }
export interface GitChangedFile { id: string; path: string; oldPath: string | null; status: string; additions: number | null; deletions: number | null; notice: string | null }
export interface GitChangesCapture {
  snapshotId: string; sessionId: string; worktree: string; branch: string; comparison: GitComparison;
  gitContextSource?: "agent" | "harness" | "process" | "launch";
  baseRef: string | null; baseOid: string | null; baselineOid: string | null; headOid: string | null;
  commitOid: string | null; commitKind: GitCommitKind | null; commitSubject: string | null; commitParentCount: number | null;
  commits: GitBranchCommit[]; commitsLimited: boolean;
  capturedAt: string; partial: boolean; notice: string | null; files: GitChangedFile[];
}
export interface GitFilePatch { snapshotId: string; fileId: string; patch: string; notice: string | null }
export type GitChangesResult = GitChangeBases | GitChangesCapture | GitFilePatch;
