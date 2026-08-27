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

test("SessionDetailPanel gates Peon diagnostics behind debug metadata", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");
  const factsGridIndex = source.indexOf('<div className="detail-facts-grid">');
  const debugGateIndex = source.indexOf("{showDebugMetadata && (", factsGridIndex);
  const debugGateEndIndex = source.indexOf("\n          )}\n        </div>", debugGateIndex);

  assert.ok(factsGridIndex >= 0, "expected the selected-session facts grid");
  assert.ok(debugGateIndex > factsGridIndex, "expected the existing debug metadata gate in the facts grid");
  assert.ok(debugGateEndIndex > debugGateIndex, "expected the debug metadata gate to close in the facts grid");

  const debugBlock = source.slice(debugGateIndex, debugGateEndIndex);
  assert.match(debugBlock, /Peon diagnostics/);
  assert.match(debugBlock, /peonDiagnostics/);
  assert.match(debugBlock, /schedulerState/);
  assert.match(debugBlock, /lastAttemptAt/);
  assert.match(debugBlock, /lastSuccessfulInferenceAt/);
  assert.match(debugBlock, /providerId/);
  assert.match(debugBlock, /providerModel/);
  assert.match(debugBlock, /fallbackStep/);
  assert.match(debugBlock, /attemptCount/);
  assert.match(debugBlock, /errorSummary/);
  assert.match(debugBlock, /observationCount/);
  assert.match(debugBlock, /errorSummary[^\n]*\?\? "—"/);
  assert.doesNotMatch(source.slice(0, debugGateIndex), /Peon diagnostics/);
});

test("Recommendations panel does not poll the sidecar before a workspace is loaded", () => {
  // Regression: the panel mounts as part of the default layout and used to
  // fetch immediately, racing the sidecar's async /workspace bootstrap and
  // surfacing a spurious 409 in the console and error banner on every
  // startup. It must gate polling on workspace readiness instead.
  const panel = readFileSync(
    new URL("../src/components/RecommendationsPanel.tsx", import.meta.url),
    "utf8",
  );
  const dockview = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(panel, /hasWorkspace: boolean/);
  assert.match(panel, /if \(!hasWorkspace\) \{/);
  assert.match(dockview, /hasWorkspace=\{!!ctx\.workspace && !ctx\.isSwitchingWorkspace\}/);
});

test("Recommendations polling also pauses across a workspace switch, not just initial startup", () => {
  // Regression: ctx.workspace alone isn't enough. handleOpenWorkspace keeps
  // the OLD WorkspaceInfo set (non-null) for the entire duration Electron
  // kills the old sidecar and boots a new one on a new port; only once the
  // new sidecar's POST /workspace has already succeeded does it flip to the
  // NEW WorkspaceInfo. A poll tick landing in that window hits the same 409
  // this PR set out to fix, just via the sidecar-restart path instead of
  // app startup. isSwitchingWorkspace must be true for that whole window.
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  const dockview = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(app, /setIsSwitchingWorkspace\(true\)/);
  assert.match(app, /setIsSwitchingWorkspace\(false\)/);
  assert.match(dockview, /isSwitchingWorkspace: boolean/);
});

test("Recommendations panel drops the previous workspace's data instead of leaving it displayed mid-switch", () => {
  const panel = readFileSync(
    new URL("../src/components/RecommendationsPanel.tsx", import.meta.url),
    "utf8",
  );

  const guardBlock = panel.slice(panel.indexOf("if (!hasWorkspace) {"));
  assert.match(guardBlock, /setRecommendations\(\[\]\)/);
  assert.match(guardBlock, /setDiagnostics\(\[\]\)/);
});
