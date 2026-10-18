import test from "node:test";
import assert from "node:assert/strict";
import { buildActionsOverview } from "../src/actionsOverview.ts";
import type { SessionInfo, WorkflowRecommendation } from "../src/api.ts";

const session = (id: string, fields: Partial<SessionInfo>): SessionInfo => ({
  id, label: id, cwd: "/repo", created_at: "2026-10-14T10:00:00Z", status: "running", lifecycle: "alive", ...fields,
});
const recommendation = (id: string): WorkflowRecommendation => ({
  id, title: id, status: "proposed", summary: "Suggested improvement", type: "improve_workflow",
  workspaceId: "workspace", chainId: "chain", chainDepth: 0, priority: "medium",
  reason: [], evidence: [], sourceSessionIds: [], targetSessionId: null,
  suggestedHarnessId: null, suggestedModel: null, suggestedWorkingDirectory: null, suggestedPrompt: null,
  confidence: "high", requiresApproval: true, dedupeKey: `improve_workflow:v1:${id}`,
  createdAt: "2026-10-14T10:00:00Z", updatedAt: "2026-10-14T10:00:00Z", expiresAt: null,
  rollupMemberIds: [], rollupMemberDedupeKeys: [], rollupGeneration: null, rolledUpBy: null,
  completionPacket: null, audit: null, proposedChange: null,
  workflowImprovement: {
    proposedImprovement: "Add a review step", targetSurface: "instructions", observationIds: [],
    recurrenceCount: 1, affectedSessionIds: [], impact: "medium", expectedBenefit: "Fewer defects",
    supersedesRecommendationId: null, dismissalWatermark: null,
  },
});

test("canonical attention order precedes recommendations; dead history cannot become live attention", () => {
  const model = buildActionsOverview([
    session("idle", { attention: "idle" }), session("working", { attention: "working" }),
    session("blocked", { attention: "blocked" }), session("needs", { attention: "needs_you" }),
    session("failed", { attention: "failed" }),
    session("old", { lifecycle: "dead", attention: "needs_you", terminalOutcome: "ended" }),
    session("capped", { attention: "capped" }),
  ], "ready", [recommendation("r2"), recommendation("r1")], "ready");
  assert.deepEqual(model.rows.map(row => row.key), [
    "session:needs", "session:blocked", "session:failed", "session:working", "session:idle",
    "recommendation:r2", "recommendation:r1",
  ]);
  assert.equal(model.sessionCount, 3);
  assert.equal(model.recommendationCount, 2);
  assert.equal(model.rows[0].group, "Needs You");
});

test("unavailable and partial sources preserve known rows and never claim zero", () => {
  const model = buildActionsOverview([session("blocked", { attention: "blocked" })], "unavailable", [], "partial");
  assert.equal(model.sessionCount, null);
  assert.equal(model.recommendationCount, null);
  assert.equal(model.rows[0].unavailable, true);
  assert.match(model.sessionCountLabel, /unavailable/i);
  assert.match(model.recommendationCountLabel, /partial/i);
});

test("session and recommendation identities are distinct and duplicate source entries count once", () => {
  const rec = recommendation("same");
  const model = buildActionsOverview([
    session("same", { attention: "needs_you" }), session("same", { attention: "needs_you" }),
  ], "ready", [rec, rec], "ready");
  assert.deepEqual(model.rows.map(row => row.key), ["session:same", "recommendation:same"]);
  assert.equal(model.sessionCount, 1);
  assert.equal(model.recommendationCount, 1);
});

test("archived stale lifecycle never becomes a current action obligation", () => {
  const model = buildActionsOverview([
    session("saved", {sessionState:"archived", lifecycle:"alive", attention:"needs_you"}),
    session("live", {sessionState:"active", attention:"working"}),
  ], "ready", [], "ready");
  assert.deepEqual(model.rows.map(row => row.id), ["live"]);
});
