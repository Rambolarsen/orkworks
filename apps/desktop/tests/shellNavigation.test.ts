import test from "node:test";
import assert from "node:assert/strict";

import {
  createShellNavigationState,
  reduceShellNavigation,
} from "../src/shellNavigation.ts";
import type {
  ShellNavigationEvent,
  ShellNavigationState,
} from "../src/shellNavigation.ts";

test("inspector inspection preserves terminal selection and unread acknowledgements", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "inspector-opened",
    destination: "details",
    subject: { kind: "session", sessionId: "session-b" },
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-a");
  assert.deepEqual(state.acknowledgedSessionIds, []);
  assert.deepEqual(state.inspectedSubject, { kind: "session", sessionId: "session-b" });
  assert.equal(state.inspector, "details");
});

test("explicit session selection changes only that session and acknowledges only it", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "session-selected",
    sessionId: "session-b",
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-b");
  assert.deepEqual(state.acknowledgedSessionIds, ["session-b"]);
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: "session-b" });
});

test("workspace generation changes clear stale navigation destinations", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const reviewing = reduceShellNavigation(initial, {
    type: "review-opened",
    sessionId: "session-a",
    artifactId: "plan-a",
    generation: 4,
  });
  const inspected = reduceShellNavigation(reviewing, {
    type: "inspector-opened",
    destination: "details",
    subject: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    generation: 4,
  });
  const changed = reduceShellNavigation(inspected, {
    type: "workspace-generation-changed",
    generation: 5,
  });

  assert.equal(changed.workspaceGeneration, 5);
  assert.equal(changed.activeSessionId, "session-a");
  assert.deepEqual(changed.centralSurface, { kind: "terminal", sessionId: "session-a" });
  assert.equal(changed.inspector, null);
  assert.equal(changed.inspectedSubject, null);
  assert.equal(changed.returnTarget, null);
});

test("a missing Review artifact falls back to its exact Terminal session with a visible reason", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const reviewing = reduceShellNavigation(initial, {
    type: "review-opened",
    sessionId: "session-a",
    artifactId: "plan-a",
    generation: 4,
  });
  const state = reduceShellNavigation(reviewing, {
    type: "target-missing",
    target: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    reason: "The plan is no longer available.",
    generation: 4,
  });

  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: "session-a" });
  assert.equal(state.visibleFallbackReason, "The plan is no longer available.");
  assert.equal(state.activeSessionId, "session-a");
});

test("closing Review returns to the exact session without submitting its prompt", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-b" });
  const reviewing = reduceShellNavigation(initial, {
    type: "review-opened",
    sessionId: "session-b",
    artifactId: "plan-b",
    generation: 4,
  });
  const returned = reduceShellNavigation(reviewing, {
    type: "review-closed",
    generation: 4,
  });

  assert.deepEqual(returned.centralSurface, { kind: "terminal", sessionId: "session-b" });
  assert.equal(returned.activeSessionId, "session-b");
  assert.deepEqual(returned.acknowledgedSessionIds, []);
});

test("restored preferences restore presentation without selection or execution authority", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const restored = reduceShellNavigation(initial, {
    type: "preferences-restored",
    surface: "review",
    artifactId: "current-plan-a",
    generation: 4,
  });

  assert.deepEqual(restored.centralSurface, {
    kind: "review",
    sessionId: "session-a",
    artifactId: "current-plan-a",
  });
  assert.equal(restored.activeSessionId, "session-a");
  assert.deepEqual(restored.acknowledgedSessionIds, []);
  assert.equal(restored.visibleFallbackReason, null);
});

test("restored preferences fall back visibly when the selected session has no current Review artifact", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: null });
  const restored = reduceShellNavigation(initial, {
    type: "preferences-restored",
    surface: "review",
    artifactId: null,
    generation: 4,
  });

  assert.deepEqual(restored.centralSurface, { kind: "terminal", sessionId: null });
  assert.match(restored.visibleFallbackReason ?? "", /review/i);
});

test("restored preference events carry presentation data only", () => {
  type RestoredPreferencesEvent = Extract<ShellNavigationEvent, { type: "preferences-restored" }>;
  const presentationOnly: RestoredPreferencesEvent = {
    type: "preferences-restored",
    surface: "terminal",
    artifactId: null,
    generation: 4,
  };
  const state: ShellNavigationState = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: null });

  // @ts-expect-error Restoration cannot carry session selection commands.
  const selectionCommand: RestoredPreferencesEvent = { ...presentationOnly, selectSessionId: "session-b" };
  // @ts-expect-error Restoration cannot carry resume, launch, or approval commands.
  const executionCommand: RestoredPreferencesEvent = { ...presentationOnly, action: "resume" };

  assert.equal(state.activeSessionId, null);
  assert.equal("selectSessionId" in selectionCommand, true);
  assert.equal("action" in executionCommand, true);
});
