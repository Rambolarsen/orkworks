export type GitComparison = "uncommitted" | "branch";
export type GitChangesRequest = { sessionId: string } & (
  | { kind: "bases" }
  | { kind: "capture"; comparison: GitComparison; baseRef?: string }
  | { kind: "file"; snapshotId: string; fileId: string }
);
export interface GitChangeBases { refs: string[]; defaultRef: string | null; limited: boolean; unavailableReason: string | null }
export interface GitChangedFile { id: string; path: string; oldPath: string | null; status: string; additions: number | null; deletions: number | null; notice: string | null }
export interface GitChangesCapture {
  snapshotId: string; sessionId: string; worktree: string; branch: string; comparison: GitComparison;
  gitContextSource?: "agent" | "harness" | "process" | "launch";
  baseRef: string | null; baseOid: string | null; baselineOid: string | null; headOid: string | null;
  capturedAt: string; partial: boolean; notice: string | null; files: GitChangedFile[];
}
export interface GitFilePatch { snapshotId: string; fileId: string; patch: string; notice: string | null }
export type GitChangesResult = GitChangeBases | GitChangesCapture | GitFilePatch;
