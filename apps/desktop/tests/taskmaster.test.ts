import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import type { WorkflowRecommendation } from "../src/api.ts";
import {
  formatImpact,
  formatRecurrence,
  formatTargetSurface,
  sortedEvidence,
} from "../src/taskmaster.ts";

const recommendation: WorkflowRecommendation = {
  id: "rec-1",
  workspaceId: "workspace-1",
  chainId: "chain-1",
  chainDepth: 0,
  type: "improve_workflow",
  status: "proposed",
  priority: "medium",
  title: "Improve review handoff",
  summary: "Review handoff is repeatedly delayed.",
  reason: ["The same transition recurs."],
  evidence: [
    {
      observationId: "observation-2",
      sequence: 2,
      sessionId: "session-2",
      kind: "repetition",
      description: "Second",
      evidence: "Second evidence",
      reportedImpact: "medium",
      source: "agent",
      confidence: 0.8,
      observedAt: "2026-08-21T10:00:00Z",
    },
    {
      observationId: "observation-1",
      sequence: 1,
      sessionId: "session-1",
      kind: "repetition",
      description: "First",
      evidence: "First evidence",
      reportedImpact: "medium",
      source: "agent",
      confidence: 0.8,
      observedAt: "2026-08-21T09:00:00Z",
    },
  ],
  sourceSessionIds: ["session-1", "session-2"],
  targetSessionId: null,
  suggestedHarnessId: null,
  suggestedModel: null,
  suggestedWorkingDirectory: null,
  suggestedPrompt: null,
  confidence: "high",
  requiresApproval: false,
  dedupeKey: "handoff",
  expiresAt: null,
  workflowImprovement: {
    proposedImprovement: "Add a review handoff step.",
    targetSurface: "instructions",
    observationIds: ["observation-1", "observation-2"],
    recurrenceCount: 2,
    affectedSessionIds: ["session-1", "session-2"],
    impact: "medium",
    expectedBenefit: "Shorter review waits.",
    supersedesRecommendationId: null,
    dismissalWatermark: null,
  },
  createdAt: "2026-08-21T10:00:00Z",
  updatedAt: "2026-08-21T10:00:00Z",
};

test("Taskmaster presentation helpers format labels and recurrence", () => {
  assert.equal(formatImpact("high"), "High");
  assert.equal(formatTargetSurface("instructions"), "Instructions");
  assert.equal(formatRecurrence(recommendation), "2 occurrences across 2 sessions");
});

test("Taskmaster evidence is displayed in observation order without mutating the response", () => {
  const sorted = sortedEvidence(recommendation.evidence);
  assert.deepEqual(sorted.map((item) => item.sequence), [1, 2]);
  assert.deepEqual(recommendation.evidence.map((item) => item.sequence), [2, 1]);
});

test("Recommendations panel exposes evidence and dismissal only", () => {
  const source = readFileSync(
    new URL("../src/components/RecommendationsPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /<details\b/);
  assert.match(source, /Dismiss/);
  assert.doesNotMatch(source, />Accept</);
  assert.doesNotMatch(source, />Execute</);
  assert.doesNotMatch(source, /Start session/);
  assert.doesNotMatch(source, />Edit</);
});

test("Recommendations panel links affected sessions through the shared selection callback", () => {
  const panel = readFileSync(
    new URL("../src/components/RecommendationsPanel.tsx", import.meta.url),
    "utf8",
  );
  const dockview = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(panel, /onSelectSession\?\./);
  assert.match(dockview, /onSelectSession=\{ctx\.onSelectSession\}/);
});
