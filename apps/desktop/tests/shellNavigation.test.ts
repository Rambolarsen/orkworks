import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import type { PlanDocumentKind, SessionInfo, WorkspaceInfo } from "../src/api.ts";
import type { LastCentralSurface, ShellMemoryDiagnostic, ShellMemoryRebuildResult, ShellMemoryResult, WorkspaceNavigationView } from "../src/orkworksWindow";
import {
  createShellNavigationController,
  createShellNavigationState,
  reduceShellNavigation,
  resolveInspectorSubject,
  resolveVisibleReviewDocument,
  type ShellNavigationPort,
} from "../src/shellNavigation.ts";

const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const shellSource = readFileSync(new URL("../src/components/ApplicationShell.tsx", import.meta.url), "utf8");

function deferred<T = void>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

function flushMicrotasks(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function makeSession(overrides: Partial<SessionInfo> = {}): SessionInfo {
  return {
    id: "session-a",
    label: "Session A",
    status: "running",
    lifecycle: "alive",
    cwd: "/tmp/repo",
    created_at: "2026-11-23T00:00:00Z",
    memoryState: "live",
    resumeStrategy: "none",
    hasOpenablePlan: true,
    ...overrides,
  };
}

function makeWorkspace(path = "/tmp/repo"): WorkspaceInfo {
  return {
    path,
    workspaceIdentity: path,
    activeHarnessIds: ["codex"],
    lastActiveSessionId: "session-a",
  } as WorkspaceInfo;
}

function createControllerFixture(overrides: Partial<ShellNavigationPort> = {}) {
  const reads: Array<{ sessionId: string; kind?: PlanDocumentKind; selectionId?: string }> = [];
  const savedSurfaces: LastCentralSurface[] = [];
  const reviewPrompts: Array<{ sessionId: string; kind?: PlanDocumentKind; selectionId?: string }> = [];
  const observedPlans: string[] = [];
  let rebuilds = 0;

  const port: ShellNavigationPort = {
    getPlanContent: async (sessionId, kind, selectionId) => {
      reads.push({ sessionId, kind, selectionId });
      return "# Current plan";
    },
    selectTerminalPlan: async () => ({ kind: "implementation", selectionId: "sel-1", branch: "main" }),
    requestPlanReview: async (sessionId, kind, selectionId) => {
      reviewPrompts.push({ sessionId, kind, selectionId });
    },
    getWorkspaceNavigation: async (): Promise<WorkspaceNavigationView> => ({
      lastCentralSurface: "terminal",
      revision: 0,
      diagnostic: null,
    }),
    completeWorkspaceNavigation: async (surface): Promise<ShellMemoryResult> => {
      savedSurfaces.push(surface);
      return { ok: true };
    },
    rebuildWorkspaceNavigation: async (): Promise<ShellMemoryRebuildResult> => {
      rebuilds += 1;
      return { ok: true };
    },
    digest: async (input) => `digest:${input}`,
    ...overrides,
  };

  const controller = createShellNavigationController({
    port,
    onPlanObserved: (sessionId) => observedPlans.push(sessionId),
  });

  return {
    controller,
    reads,
    savedSurfaces,
    reviewPrompts,
    observedPlans,
    rebuilds: () => rebuilds,
  };
}

test("inspector inspection preserves terminal selection", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "inspector-opened",
    destination: "details",
    subject: { kind: "session", sessionId: "session-b" },
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-a");
  assert.deepEqual(state.inspectedSubject, { kind: "session", sessionId: "session-b" });
  assert.equal(state.inspector, "details");
  assert.deepEqual(state.inspectorReturnFocus, { kind: "terminal", sessionId: "session-a" });
});

test("explicit session selection changes only that session and focuses its terminal", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "session-selected",
    sessionId: "session-b",
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-b");
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: "session-b" });
  assert.deepEqual(state.focusTarget, { kind: "terminal", sessionId: "session-b" });
});

test("selecting a session leaves Changes for its terminal instead of following it in the viewer", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const changes = reduceShellNavigation(initial, {
    type: "inspector-opened",
    destination: "changes",
    subject: { kind: "session", sessionId: "session-a" },
    generation: 4,
  });
  const selected = reduceShellNavigation(changes, { type: "session-selected", sessionId: "session-b", generation: 4 });
  assert.equal(selected.inspector, null);
  assert.equal(selected.inspectedSubject, null);
  assert.deepEqual(selected.centralSurface, { kind: "terminal", sessionId: "session-b" });
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

test("discarding a missing Review retargets inspector return focus to the Terminal fallback", () => {
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
    subject: { kind: "session", sessionId: "session-b" },
    generation: 4,
  });

  const missingReview = reduceShellNavigation(inspected, {
    type: "target-missing",
    target: { kind: "artifact", sessionId: "session-a", artifactId: "plan-a" },
    reason: "The plan is no longer available.",
    generation: 4,
  });
  const inspectorClosed = reduceShellNavigation(missingReview, {
    type: "inspector-closed",
    generation: 4,
  });

  assert.deepEqual(missingReview.inspectorReturnFocus, { kind: "terminal", sessionId: "session-a" });
  assert.deepEqual(inspectorClosed.focusTarget, { kind: "terminal", sessionId: "session-a" });
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

test("a deliberately closed active session clears selection without a visible fallback reason", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" });
  const state = reduceShellNavigation(initial, {
    type: "target-missing",
    target: { kind: "session", sessionId: "session-a" },
    reason: null,
    generation: 4,
  });

  assert.equal(state.activeSessionId, null);
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: null });
  assert.deepEqual(state.focusTarget, { kind: "sessions" });
  assert.equal(state.visibleFallbackReason, null);
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

test("closing Review returns to the exact session", () => {
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
  assert.deepEqual(returned.focusTarget, { kind: "terminal", sessionId: "session-b" });
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

test("restored preferences change presentation without changing owner selection", () => {
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
  assert.deepEqual(restored.centralSurface, {
    kind: "review",
    sessionId: "session-b",
    artifactId: "current-plan-b",
  });
  assert.equal(restored.inspector, null);
  assert.equal(restored.inspectedSubject, null);
  assert.equal(restored.returnTarget?.sessionId, "session-b");
});

test("owner-restored selection updates navigation cleanly", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: null });
  const state = reduceShellNavigation(initial, {
    type: "session-restored",
    sessionId: "session-a",
    generation: 4,
  });

  assert.equal(state.activeSessionId, "session-a");
  assert.deepEqual(state.centralSurface, { kind: "terminal", sessionId: "session-a" });
});

test("App wires lifecycle, active session, and command dispatch through ShellNavigationController with no prop ping-pong", () => {
  assert.match(appSource, /createShellNavigationController\(/);
  assert.match(appSource, /useSyncExternalStore\(/);
  assert.match(appSource, /shellNavigationController\.handleLifecycleTransition\(/);
  assert.match(appSource, /shellNavigationController\.handleOwnerActiveSession\(/);
  assert.match(appSource, /shellNavigationController\.selectSession\(id\)/);
  assert.match(appSource, /shellNavigationController\.dispatchCommand\(/);
  assert.match(appSource, /shellNavigation\.visibleFallbackReason/);
  assert.match(appSource, /pushToast\("info", shellNavigation\.visibleFallbackReason\)/);
  assert.doesNotMatch(appSource, /ShellCommandRequest|setShellCommandRequest/);
  assert.doesNotMatch(shellSource, /ShellCommandRequest|commandRequest/);
  assert.match(shellSource, /registerCommandHandler\(/);
  assert.match(shellSource, /focusTarget\.kind === "terminal"/);
});

for (const destination of ["recommendations", "actions"] as const) {
  test(`${destination} remains workspace-scoped when the active session disappears`, () => {
    const subject = resolveInspectorSubject(destination, "session-a", "workspace-a");
    assert.deepEqual(subject, { kind: "workspace", workspaceKey: "workspace-a" });
    const inspected = reduceShellNavigation(createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "session-a" }), {
      type: "inspector-opened",
      destination,
      subject: subject!,
      generation: 4,
    });
    assert.equal(inspected.activeSessionId, "session-a");
    const missing = reduceShellNavigation(inspected, {
      type: "target-missing",
      target: { kind: "session", sessionId: "session-a" },
      reason: "Gone",
      generation: 4,
    });
    assert.equal(missing.inspector, destination);
    assert.deepEqual(missing.inspectedSubject, { kind: "workspace", workspaceKey: "workspace-a" });
  });
}

test("leaving Changes and switching workspace clears its transient navigation", () => {
  const initial = createShellNavigationState({ workspaceGeneration: 4, activeSessionId: "a" });
  const open = reduceShellNavigation(initial, { type: "inspector-opened", destination: "changes", subject: { kind: "session", sessionId: "a" }, generation: 4 });
  const next = reduceShellNavigation(open, { type: "session-selected", sessionId: "b", generation: 4 });
  assert.equal(next.inspector, null);
  assert.equal(next.inspectedSubject, null);
  assert.deepEqual(next.centralSurface, { kind: "terminal", sessionId: "b" });
  assert.equal(reduceShellNavigation(next, { type: "workspace-generation-changed", generation: 5 }).inspector, null);
});

test("Skills remains available for personal discovery without a workspace", () => {
  assert.deepEqual(resolveInspectorSubject("skills", null, null), { kind: "personal" });
  assert.deepEqual(resolveInspectorSubject("skills", null, "/repo"), { kind: "workspace", workspaceKey: "/repo" });
});

// --- Deterministic ShellNavigationController async & race tests ---

test("controller restores saved Review without persisting defaults, and falls back when unreadable or missing", async () => {
  // 1. Readable saved review restores without writing navigation defaults.
  const readable = createControllerFixture({
    getWorkspaceNavigation: async () => ({ lastCentralSurface: "review", revision: 1, diagnostic: null }),
  });
  const workspace = makeWorkspace();
  const session = makeSession();
  readable.controller.handleLifecycleTransition(1, "ready");
  readable.controller.handleOwnerActiveSession(session.id);
  readable.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  await flushMicrotasks();
  assert.equal(readable.controller.getSnapshot().navigation.centralSurface.kind, "review");
  assert.equal(readable.controller.getSnapshot().review.document?.content, "# Current plan");
  assert.deepEqual(readable.savedSurfaces, []);

  // 2. Unreadable plan during restore falls back to Terminal with visible reason.
  const unreadable = createControllerFixture({
    getWorkspaceNavigation: async () => ({ lastCentralSurface: "review", revision: 1, diagnostic: null }),
    getPlanContent: async (id) => { unreadable.reads.push({ sessionId: id }); throw new Error("Missing plan"); },
  });
  unreadable.controller.handleLifecycleTransition(1, "ready");
  unreadable.controller.handleOwnerActiveSession(session.id);
  unreadable.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  await flushMicrotasks();
  assert.equal(unreadable.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.equal(unreadable.controller.getSnapshot().review.error, false);
  assert.match(unreadable.controller.getSnapshot().navigation.visibleFallbackReason ?? "", /Review could not be restored/);
  assert.equal(unreadable.reads.length, 1);
  assert.deepEqual(unreadable.savedSurfaces, []);

  // 3. Missing plan on session falls back without reading plan content.
  const missing = createControllerFixture({
    getWorkspaceNavigation: async () => ({ lastCentralSurface: "review", revision: 1, diagnostic: null }),
  });
  const noPlanSession = makeSession({ hasOpenablePlan: false });
  missing.controller.handleLifecycleTransition(1, "ready");
  missing.controller.handleOwnerActiveSession(noPlanSession.id);
  missing.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [noPlanSession] });
  await flushMicrotasks();
  assert.equal(missing.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.match(missing.controller.getSnapshot().navigation.visibleFallbackReason ?? "", /Review could not be restored/);
  assert.equal(missing.reads.length, 0);
});

test("controller fences late hydration, opening navigation, and native-menu commands against stale Review reads", async () => {
  // 1. Late hydration cannot override user navigation.
  const navRead = deferred<WorkspaceNavigationView>();
  const staleHydration = createControllerFixture({
    getWorkspaceNavigation: () => navRead.promise,
  });
  const workspace = makeWorkspace();
  const session = makeSession();
  staleHydration.controller.handleLifecycleTransition(1, "ready");
  staleHydration.controller.handleOwnerActiveSession(session.id);
  staleHydration.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  staleHydration.controller.dispatchCommand("terminal");
  navRead.resolve({ lastCentralSurface: "review", revision: 1, diagnostic: null });
  await flushMicrotasks();
  assert.equal(staleHydration.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.equal(staleHydration.reads.length, 0);

  // 2. Navigation during "opening" supersedes saved Review after "ready".
  const opening = createControllerFixture({
    getWorkspaceNavigation: async () => ({ lastCentralSurface: "review", revision: 1, diagnostic: null }),
  });
  opening.controller.handleLifecycleTransition(1, "opening");
  opening.controller.dispatchCommand("terminal");
  opening.controller.handleLifecycleTransition(2, "ready");
  opening.controller.handleOwnerActiveSession(session.id);
  opening.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  await flushMicrotasks();
  assert.equal(opening.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.equal(opening.reads.length, 0);

  // 3. Direct command dispatch synchronously fences an in-flight Review read.
  const planRead = deferred<string>();
  const nativeRace = createControllerFixture({
    getPlanContent: () => planRead.promise,
  });
  nativeRace.controller.handleLifecycleTransition(1, "ready");
  nativeRace.controller.handleOwnerActiveSession(session.id);
  nativeRace.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  const dispatched: string[] = [];
  nativeRace.controller.registerCommandHandler((cmd) => {
    dispatched.push(cmd);
    if (cmd === "details") nativeRace.controller.inspect("details");
  });
  const pendingLoad = nativeRace.controller.openReview();
  assert.equal(nativeRace.controller.getSnapshot().review.loading, true);
  nativeRace.controller.dispatchCommand("details");
  planRead.resolve("# Superseded read");
  await pendingLoad;
  assert.deepEqual(dispatched, ["details"]);
  assert.equal(nativeRace.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.equal(nativeRace.controller.getSnapshot().navigation.inspector, "details");
  assert.deepEqual(nativeRace.savedSurfaces, []);
});

test("controller navigation rebuild recovery ignores concurrent and delayed stale diagnostics", async () => {
  // 1. Empty workspace rebuild clears corrupt diagnostic without reading plans; cancel preserves diagnostic.
  let cancel = true;
  const empty = createControllerFixture({
    getWorkspaceNavigation: async () => ({ lastCentralSurface: "terminal", revision: 0, diagnostic: "corrupt_record" }),
    rebuildWorkspaceNavigation: async () => cancel
      ? { ok: false, diagnostic: "user_cancelled" }
      : { ok: true },
  });
  const workspace = makeWorkspace();
  empty.controller.handleLifecycleTransition(1, "ready");
  empty.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [] });
  await flushMicrotasks();
  assert.equal(empty.controller.getSnapshot().navigationDiagnostic, "corrupt_record");
  assert.equal(empty.reads.length, 0);
  await empty.controller.rebuildNavigation();
  assert.equal(empty.controller.getSnapshot().navigationDiagnostic, "corrupt_record");
  cancel = false;
  await empty.controller.rebuildNavigation();
  assert.equal(empty.controller.getSnapshot().navigationDiagnostic, null);
  assert.deepEqual(empty.savedSurfaces, []);

  // 2. Reads begun during a pending rebuild cannot undo rebuild completion (both resolve and reject).
  for (const outcome of ["resolve", "reject"] as const) {
    let navCalls = 0;
    const secondRead = deferred<WorkspaceNavigationView>();
    const rebuildDeferred = deferred<ShellMemoryRebuildResult>();
    const pending = createControllerFixture({
      getWorkspaceNavigation: () => {
        navCalls += 1;
        if (navCalls === 1) return Promise.resolve({ lastCentralSurface: "terminal", revision: 0, diagnostic: "corrupt_record" as ShellMemoryDiagnostic });
        return secondRead.promise;
      },
      rebuildWorkspaceNavigation: () => rebuildDeferred.promise,
    });
    const session = makeSession();
    pending.controller.handleLifecycleTransition(1, "ready");
    pending.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [] });
    await flushMicrotasks();
    assert.equal(pending.controller.getSnapshot().navigationDiagnostic, "corrupt_record");

    const rebuildPromise = pending.controller.rebuildNavigation();
    assert.equal(pending.controller.getSnapshot().navigationRebuilding, true);
    pending.controller.handleOwnerActiveSession(session.id);
    pending.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
    assert.equal(navCalls, 2);

    rebuildDeferred.resolve({ ok: true });
    await rebuildPromise;
    assert.equal(pending.controller.getSnapshot().navigationDiagnostic, null);
    assert.equal(pending.controller.getSnapshot().navigationRebuilding, false);

    if (outcome === "resolve") {
      secondRead.resolve({ lastCentralSurface: "terminal", revision: 0, diagnostic: "corrupt_record" });
    } else {
      secondRead.reject(new Error("Read during rebuild failed"));
    }
    await flushMicrotasks();
    assert.equal(pending.controller.getSnapshot().navigationDiagnostic, null);
  }

  // 3. Delayed initial read settling after confirmed rebuild cannot re-surface old diagnostic.
  for (const outcome of ["resolve", "reject"] as const) {
    let navCalls = 0;
    const oldRead = deferred<WorkspaceNavigationView>();
    const delayed = createControllerFixture({
      getWorkspaceNavigation: () => {
        navCalls += 1;
        if (navCalls === 1) return oldRead.promise;
        return Promise.resolve({ lastCentralSurface: "terminal", revision: 0, diagnostic: "corrupt_record" as ShellMemoryDiagnostic });
      },
    });
    const session = makeSession();
    delayed.controller.handleLifecycleTransition(1, "ready");
    delayed.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [] });
    delayed.controller.handleOwnerActiveSession(session.id);
    delayed.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
    await flushMicrotasks();
    assert.equal(delayed.controller.getSnapshot().navigationDiagnostic, "corrupt_record");

    await delayed.controller.rebuildNavigation();
    assert.equal(delayed.controller.getSnapshot().navigationDiagnostic, null);
    if (outcome === "resolve") {
      oldRead.resolve({ lastCentralSurface: "terminal", revision: 0, diagnostic: "corrupt_record" });
    } else {
      oldRead.reject(new Error("Old read failure"));
    }
    await flushMicrotasks();
    assert.equal(delayed.controller.getSnapshot().navigationDiagnostic, null);
  }
});

test("controller deduplicates independent review requests per workspace generation and isolates stale settlements", async () => {
  const firstPrompt = deferred<void>();
  const secondPrompt = deferred<void>();
  let promptIndex = 0;
  const f = createControllerFixture({
    requestPlanReview: async (sessionId, kind, selectionId) => {
      f.reviewPrompts.push({ sessionId, kind, selectionId });
      promptIndex += 1;
      if (promptIndex === 1) return firstPrompt.promise;
      return secondPrompt.promise;
    },
  });
  const workspace = makeWorkspace();
  const session = makeSession();
  f.controller.handleLifecycleTransition(1, "ready");
  f.controller.handleOwnerActiveSession(session.id);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });

  const p1 = f.controller.requestIndependentReview(session.id);
  const p2 = f.controller.requestIndependentReview(session.id);
  assert.equal(p1, p2);
  assert.equal(f.reviewPrompts.length, 1);

  // Switch workspace generation while first request is still pending.
  f.controller.handleLifecycleTransition(2, "ready");
  f.controller.handleOwnerActiveSession(session.id);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace: makeWorkspace("/tmp/other"), sessions: [session] });

  const p3 = f.controller.requestIndependentReview(session.id);
  assert.notEqual(p3, p1);
  assert.equal(f.reviewPrompts.length, 2);

  // Settling old generation's request cannot release the new generation's dedupe guard.
  firstPrompt.resolve();
  await flushMicrotasks();
  const p4 = f.controller.requestIndependentReview(session.id);
  assert.equal(p4, p3);
  assert.equal(f.reviewPrompts.length, 2);

  secondPrompt.resolve();
  await p3;
});

test("controller handles initial Review error, read-only Retry, and terminal plan click observation", async () => {
  let failRead = true;
  const f = createControllerFixture({
    getPlanContent: async (sessionId, kind, selectionId) => {
      f.reads.push({ sessionId, kind, selectionId });
      if (failRead) throw new Error("Missing");
      return "# Recovered plan";
    },
  });
  const workspace = makeWorkspace();
  const session = makeSession();
  f.controller.handleLifecycleTransition(1, "ready");
  f.controller.handleOwnerActiveSession(session.id);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [session] });
  await flushMicrotasks();

  await f.controller.openReview();
  assert.equal(f.controller.getSnapshot().review.error, true);
  assert.deepEqual(f.savedSurfaces, []);

  failRead = false;
  await f.controller.refreshReview();
  assert.equal(f.controller.getSnapshot().review.error, false);
  assert.equal(f.controller.getSnapshot().review.document?.content, "# Recovered plan");
  assert.equal(f.controller.getSnapshot().navigation.centralSurface.kind, "review");
  assert.deepEqual(f.savedSurfaces, []);

  f.controller.closeReview();
  assert.equal(f.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.deepEqual(f.savedSurfaces, ["terminal"]);

  await f.controller.selectTerminalPlan(session.id, "specs/plan.md");
  assert.deepEqual(f.observedPlans, [session.id, session.id]);
  assert.equal(f.controller.getSnapshot().navigation.centralSurface.kind, "review");
  assert.deepEqual(f.savedSurfaces, ["terminal", "review"]);

  await assert.rejects(
    f.controller.selectTerminalPlan("other-session", "specs/plan.md"),
    /Select the session again/,
  );
});

test("controller synchronously fences lifecycle transitions, scope updates, selectSession+openReview, and reconnect hydration", async () => {
  // 1. handleLifecycleTransition immediately closes admission before syncRuntime runs.
  const workspace = makeWorkspace();
  const sessionA = makeSession({ id: "session-a", branch: "main" });
  const sessionB = makeSession({ id: "session-b", branch: "feature" });
  const f = createControllerFixture();
  f.controller.handleLifecycleTransition(1, "ready", workspace);
  f.controller.handleOwnerActiveSession(sessionA.id);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA, sessionB] });
  await flushMicrotasks();

  // Switching to "opening" immediately blocks admission-gated operations before any effect runs.
  f.controller.handleLifecycleTransition(2, "opening");
  await assert.rejects(f.controller.requestIndependentReview(sessionA.id), /requires an active session/);
  await assert.rejects(f.controller.selectTerminalPlan(sessionA.id, "specs/plan.md"), /Select the session again/);
  await f.controller.saveNavigation("review");
  assert.deepEqual(f.savedSurfaces, []);

  // 2. selectSession syncs scope so an immediate openReview is not aborted by syncRuntime,
  // and normal session switching does not emit a redundant second update in syncRuntime.
  f.controller.handleLifecycleTransition(3, "ready", workspace);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA, sessionB] });
  await flushMicrotasks();
  let emissions = 0;
  const unsub = f.controller.subscribe(() => { emissions += 1; });
  f.controller.selectSession(sessionA.id);
  assert.equal(emissions, 1);
  f.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA, sessionB] });
  assert.equal(emissions, 1, "syncRuntime must not emit a redundant re-render after selectSession");
  unsub();

  const readB = deferred<string>();
  const f2 = createControllerFixture({ getPlanContent: () => readB.promise });
  f2.controller.handleLifecycleTransition(1, "ready", workspace);
  f2.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA, sessionB] });
  f2.controller.selectSession(sessionB.id);
  const loadPromise = f2.controller.openReview();
  f2.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA, sessionB] });
  readB.resolve("# Plan B");
  await loadPromise;
  assert.equal(f2.controller.getSnapshot().review.document?.content, "# Plan B");
  assert.equal(
    resolveVisibleReviewDocument(f2.controller.getSnapshot().review.document, sessionB, 1, true)?.content,
    "# Plan B",
  );
  // Render-time gate immediately hides stale document when branch/scope or connectivity changes.
  assert.equal(
    resolveVisibleReviewDocument(f2.controller.getSnapshot().review.document, { ...sessionB, branch: "other" }, 1, true),
    null,
  );
  assert.equal(
    resolveVisibleReviewDocument(f2.controller.getSnapshot().review.document, sessionB, 1, false),
    null,
  );

  // 3. fenceRuntime synchronously invalidates an in-flight Review load before syncRuntime runs.
  const staleRead = deferred<string>();
  const f3 = createControllerFixture({ getPlanContent: () => staleRead.promise });
  f3.controller.handleLifecycleTransition(1, "ready", workspace);
  f3.controller.handleOwnerActiveSession(sessionA.id);
  f3.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [sessionA] });
  const pendingLoad = f3.controller.openReview();
  f3.controller.fenceRuntime({ sessions: [{ ...sessionA, branch: "changed-branch" }] });
  staleRead.resolve("# Stale branch plan");
  await pendingLoad;
  assert.equal(f3.controller.getSnapshot().navigation.centralSurface.kind, "terminal");
  assert.deepEqual(f3.savedSurfaces, []);

  // 4. Reconnecting backendConnected false -> true re-runs hydration on an empty workspace.
  let navReads = 0;
  const f4 = createControllerFixture({
    getWorkspaceNavigation: async () => {
      navReads += 1;
      if (navReads === 1) throw new Error("offline");
      return { lastCentralSurface: "terminal", revision: 0, diagnostic: null };
    },
  });
  f4.controller.handleLifecycleTransition(1, "ready", workspace);
  f4.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [] });
  await flushMicrotasks();
  assert.equal(f4.controller.getSnapshot().navigationDiagnostic, "restore_failed");
  f4.controller.fenceRuntime({ backendConnected: false });
  f4.controller.syncRuntime({ backendConnected: false, admissionEnabled: false, workspace, sessions: [] });
  f4.controller.syncRuntime({ backendConnected: true, admissionEnabled: true, workspace, sessions: [] });
  await flushMicrotasks();
  assert.equal(navReads, 2);
  assert.equal(f4.controller.getSnapshot().navigationDiagnostic, null);
});
