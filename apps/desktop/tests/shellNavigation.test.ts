import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  createShellNavigationState,
  reduceShellNavigation,
} from "../src/shellNavigation.ts";

const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

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
  assert.equal(changed.activeSessionId, null);
  assert.deepEqual(changed.acknowledgedSessionIds, []);
  assert.deepEqual(changed.centralSurface, { kind: "terminal", sessionId: null });
  assert.deepEqual(changed.focusTarget, { kind: "sessions" });
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

test("a missing active Terminal session clears selection and falls back visibly", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "target-missing",
    target: { kind: "session", sessionId: "session-a" },
    reason: "The selected session is no longer available.",
    generation: 4,
  });

  assert.equal(state.activeSessionId, null);
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: null });
  assert.deepEqual(state.focusTarget, { kind: "sessions" });
  assert.equal(state.visibleFallbackReason, "The selected session is no longer available.");
});

test("a missing session clears inspector subjects bound to that session", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const inspected = reduceShellNavigation(initial, {
    type: "inspector-opened",
    destination: "details",
    subject: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    generation: 4,
  });
  const state = reduceShellNavigation(inspected, {
    type: "target-missing",
    target: { kind: "session", sessionId: "session-a" },
    reason: "The selected session is no longer available.",
    generation: 4,
  });

  assert.equal(state.inspector, null);
  assert.equal(state.inspectedSubject, null);
  assert.equal(state.activeSessionId, null);
  assert.equal(state.visibleFallbackReason, "The selected session is no longer available.");
});

test("a missing inspected artifact clears its exact inspector subject", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const inspected = reduceShellNavigation(initial, {
    type: "inspector-opened",
    destination: "details",
    subject: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    generation: 4,
  });
  const state = reduceShellNavigation(inspected, {
    type: "target-missing",
    target: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    reason: "The plan is no longer available.",
    generation: 4,
  });

  assert.equal(state.inspector, null);
  assert.equal(state.inspectedSubject, null);
  assert.equal(state.visibleFallbackReason, "The plan is no longer available.");
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

test("restored preferences change presentation without changing owner selection or unread acknowledgements", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const explicitlySelected = reduceShellNavigation(initial, {
    type: "session-selected",
    sessionId: "session-b",
    generation: 4,
  });
  const restored = reduceShellNavigation(explicitlySelected, {
    type: "preferences-restored",
    surface: "review",
    artifactId: "current-plan-b",
    generation: 4,
  });

  assert.equal(restored.activeSessionId, "session-b");
  assert.deepEqual(restored.acknowledgedSessionIds, ["session-b"]);
  assert.deepEqual(restored.centralSurface, {
    kind: "review",
    sessionId: "session-b",
    artifactId: "current-plan-b",
  });
  assert.equal(restored.inspector, null);
  assert.equal(restored.inspectedSubject, null);
  assert.equal(restored.returnTarget?.sessionId, "session-b");
});

test("owner-restored selection updates navigation without acknowledging it", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: null });
  const state = reduceShellNavigation(initial, {
    type: "session-restored",
    sessionId: "session-a",
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-a");
  assert.deepEqual(state.acknowledgedSessionIds, []);
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: "session-a" });
});

test("App routes explicit selection, owner restoration, and lifecycle generations through the reducer", () => {
  assert.ok(/const \[shellNavigation, dispatchShellNavigation\] = useReducer\([\s\S]*?reduceShellNavigation/.test(appSource));
  assert.ok(/type: "workspace-generation-changed"/.test(appSource));
  assert.ok(/type: "session-restored"/.test(appSource));
  assert.ok(/type: "session-selected"/.test(appSource));
  assert.ok(/workspaceSessionController\.selectSession\(id\)[\s\S]*?type: "session-selected"/.test(appSource));
});

test("App sends controller-confirmed active-session loss through visible navigation fallback", () => {
  const callbackStart = appSource.indexOf("onActiveSession: (sessionId) => {");
  const callbackEnd = appSource.indexOf("onError:", callbackStart);
  assert.ok(callbackStart >= 0 && callbackEnd > callbackStart);
  const callback = appSource.slice(callbackStart, callbackEnd);

  assert.ok(/type: "target-missing"/.test(callback));
  assert.ok(/shellNavigation\.visibleFallbackReason/.test(appSource));
  assert.ok(/pushToast\("info", shellNavigation\.visibleFallbackReason\)/.test(appSource));
});
