import test from "node:test";
import assert from "node:assert/strict";
import { initialReviewHandoffState, reviewHandoffReducer as reduce, type ReviewHandoffSnapshot } from "../src/components/review/reviewHandoffPresentation.ts";

const snapshot: ReviewHandoffSnapshot = {
  scope: "workspace-A/backend-1", proposalId: "review-1", revision: "1",
  source: { id: "S41", runtime: "S41/3", title: "Retry implementation", tool: "Codex" },
  reviewer: { label: "Proposed new session R7", role: "Review", tool: "Copilot", model: "Configured reviewer (unverified)" },
  sourceLocation: { worktree: "/work/demo", branch: "feature/retry" },
  reviewerLocation: { worktree: "/work/demo", branch: "feature/retry" },
  artifact: { state: "available", path: "specs/retry.md", anchor: "/work/demo", version: "A", pinned: false },
  instruction: "Review S41 against version A.", unavailableReason: "Launch is unavailable in this synthetic preview.",
};
const open = () => reduce(initialReviewHandoffState, { type: "open", snapshot });

test("opening preserves exact source identity and separate work locations", () => {
  const state = open();
  assert.equal(state.snapshot?.source.id, "S41");
  assert.equal(state.snapshot?.reviewer.label, "Proposed new session R7");
  const companion = { ...snapshot, source: { ...snapshot.source, id: "S42", runtime: "S42/1" } };
  assert.equal(reduce(state, { type: "open", snapshot: companion }).snapshot?.source.id, "S42");
});

test("material changes retain the entire inspected instruction until explicit refresh", () => {
  for (const reason of ["Plan content changed", "Plan anchor changed", "Pin changed", "Reviewer changed", "Instruction changed", "Working location changed"]) {
    const state = reduce(open(), { type: "changed", reason });
    assert.equal(state.snapshot?.instruction, "Review S41 against version A.");
    assert.equal(state.staleReason, reason);
    assert.equal(state.request, null);
  }
});

test("refresh installs one complete matching snapshot and rejects out-of-order completion", () => {
  const first = reduce(open(), { type: "refresh-started" });
  const second = reduce(first, { type: "refresh-started" });
  const next = { ...snapshot, revision: "2", instruction: "Review version B.", artifact: { ...snapshot.artifact, version: "B" } } as ReviewHandoffSnapshot;
  const stale = reduce(second, { type: "refresh-completed", request: first.request!, snapshot: next });
  assert.equal(stale.snapshot?.instruction, "Review S41 against version A.");
  const current = reduce(stale, { type: "refresh-completed", request: second.request!, snapshot: next });
  assert.equal(current.snapshot?.instruction, "Review version B.");
  assert.equal(current.snapshot?.revision, "2");
  assert.equal(current.staleReason, null);
  assert.equal(current.request, null);
  assert.equal(current.refreshCount, 1);
});

test("late completions cannot reopen closed, removed or scope-replaced details", () => {
  const loading = reduce(open(), { type: "refresh-started" });
  for (const type of ["close", "reset"] as const) {
    const closed = reduce(loading, { type });
    assert.equal(reduce(closed, { type: "refresh-completed", request: loading.request!, snapshot }).snapshot, null);
    const reopened = reduce(closed, { type: "open", snapshot });
    assert.equal(reduce(reopened, { type: "refresh-completed", request: loading.request!, snapshot: { ...snapshot, instruction: "late" } }).snapshot?.instruction, "Review S41 against version A.");
  }
});

test("scope/proposal mismatches cannot replace inspected context", () => {
  const loading = reduce(open(), { type: "refresh-started" });
  for (const next of [{ ...snapshot, scope: "workspace-B/backend-2" }, { ...snapshot, proposalId: "review-2" }]) {
    const state = reduce(loading, { type: "refresh-completed", request: loading.request!, snapshot: next });
    assert.equal(state.snapshot?.scope, "workspace-A/backend-1");
    assert.equal(state.snapshot?.proposalId, "review-1");
    assert.ok(state.staleReason);
    assert.equal(state.request, null);
  }
});

test("A to B to A and runtime changes invalidate outstanding refresh", () => {
  const loading = reduce(open(), { type: "refresh-started" });
  const changed = reduce(loading, { type: "changed", reason: "Runtime or plan changed A → B → A" });
  const state = reduce(changed, { type: "refresh-completed", request: loading.request!, snapshot });
  assert.equal(state.staleReason, "Runtime or plan changed A → B → A");
  assert.equal(state.refreshCount, 0);
});

test("missing and unavailable artifacts remain explicit after refresh failure", () => {
  for (const artifact of [{ state: "missing" }, { state: "unavailable", reason: "Anchor unavailable" }] as const) {
    const state = reduce(initialReviewHandoffState, { type: "open", snapshot: { ...snapshot, artifact } });
    const loading = reduce(state, { type: "refresh-started" });
    const failed = reduce(loading, { type: "refresh-failed", request: loading.request!, reason: "Context unavailable" });
    assert.equal(failed.snapshot?.artifact.state, artifact.state);
    assert.equal(failed.staleReason, "Context unavailable");
    assert.equal(failed.request, null);
  }
});
