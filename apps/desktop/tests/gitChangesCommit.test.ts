import test from "node:test";
import assert from "node:assert/strict";
import {
  commitOptionLabel,
  comparisonBaselineLabel,
  comparisonKey,
  isCaptureMatchingSelection,
  isPatchMatchingCapture,
  selectActiveBranchCommits,
  shortOid,
} from "../src/gitChangesCommit.ts";

const OID_A = "11111111aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OID_B = "22222222bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OID_BASE = "00000000cccccccccccccccccccccccccccccccc";

test("comparisonBaselineLabel formats ordinary, merge, root, branch, and uncommitted baselines", () => {
  assert.equal(
    comparisonBaselineLabel("uncommitted", "", null),
    "HEAD → working tree · staged, unstaged, new files",
  );
  assert.equal(
    comparisonBaselineLabel("branch", "refs/heads/main", null),
    "merge-base(main, HEAD) → HEAD · committed only",
  );
  assert.equal(
    comparisonBaselineLabel("branch", "refs/heads/main", {
      commitOid: OID_A,
      commitKind: "ordinary",
      commitParentCount: 1,
      baselineOid: OID_BASE,
    }),
    "00000000 → 11111111 · single commit · committed only",
  );
  assert.equal(
    comparisonBaselineLabel("branch", "refs/heads/main", {
      commitOid: OID_B,
      commitKind: "merge",
      commitParentCount: 2,
      baselineOid: OID_A,
    }),
    "first parent 11111111 → 22222222 · merge commit (2 parents, first-parent diff) · committed only",
  );
  assert.equal(
    comparisonBaselineLabel("branch", "refs/heads/main", {
      commitOid: OID_A,
      commitKind: "root",
      commitParentCount: 0,
      baselineOid: null,
    }),
    "empty tree → 11111111 · root commit · committed only",
  );
});

test("commitOptionLabel includes short OID, subject, and merge/root tags", () => {
  assert.equal(
    commitOptionLabel({ oid: OID_A, shortOid: "11111111", subject: "Fix bug", kind: "ordinary", parentCount: 1, baselineOid: OID_BASE }),
    "11111111 · Fix bug",
  );
  assert.equal(
    commitOptionLabel({ oid: OID_B, shortOid: "22222222", subject: "Merge branch", kind: "merge", parentCount: 2, baselineOid: OID_A }),
    "22222222 · Merge branch [merge]",
  );
  assert.equal(
    commitOptionLabel({ oid: OID_A, shortOid: "11111111", subject: "Initial import", kind: "root", parentCount: 0, baselineOid: null }),
    "11111111 · Initial import [root]",
  );
  assert.equal(shortOid(null), "empty tree");
});

test("selection keys and response guards isolate two commits touching the same file", () => {
  const keyAll = comparisonKey("branch", "refs/heads/main", "");
  const keyA = comparisonKey("branch", "refs/heads/main", OID_A);
  const keyB = comparisonKey("branch", "refs/heads/main", OID_B);
  assert.notEqual(keyAll, keyA);
  assert.notEqual(keyA, keyB);

  // A stale response for commit A must be rejected when commit B is now selected.
  const staleCommitACapture = {
    sessionId: "sess-1",
    comparison: "branch" as const,
    baseRef: "refs/heads/main",
    commitOid: OID_A,
  };
  assert.equal(isCaptureMatchingSelection(staleCommitACapture, "sess-1", "branch", "refs/heads/main", OID_A), true);
  assert.equal(isCaptureMatchingSelection(staleCommitACapture, "sess-1", "branch", "refs/heads/main", OID_B), false);
  assert.equal(isCaptureMatchingSelection(staleCommitACapture, "sess-1", "branch", "refs/heads/main", ""), false);

  // A stale patch response from commit A's snapshot cannot attach to commit B's snapshot.
  assert.equal(isPatchMatchingCapture({ snapshotId: "snap-a", fileId: "file-a" }, "snap-b", "file-b"), false);
  assert.equal(isPatchMatchingCapture({ snapshotId: "snap-b", fileId: "file-b" }, "snap-b", "file-b"), true);
});

test("selectActiveBranchCommits preserves commits on detached HEAD and clears them on branch or base switch", () => {
  const commits = [
    { oid: OID_A, shortOid: "11111111", subject: "Detached commit", kind: "ordinary" as const, parentCount: 1, baselineOid: OID_BASE },
  ];
  // Detached HEAD: session.branch is "HEAD" even though capture.branch formats as "Detached HEAD <short>".
  const detachedCache = { baseRef: "refs/heads/main", sessionBranch: "HEAD", commits, limited: true };
  const active = selectActiveBranchCommits("branch", "refs/heads/main", "HEAD", detachedCache);
  assert.equal(active, detachedCache);
  assert.equal(active?.limited, true);

  // Switching branches or base refs clears the cached commit list synchronously.
  assert.equal(selectActiveBranchCommits("branch", "refs/heads/main", "other-branch", detachedCache), null);
  assert.equal(selectActiveBranchCommits("branch", "refs/heads/release", "HEAD", detachedCache), null);
  assert.equal(selectActiveBranchCommits("uncommitted", "refs/heads/main", "HEAD", detachedCache), null);
});
