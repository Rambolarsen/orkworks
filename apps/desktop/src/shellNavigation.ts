import type { PlanDocumentKind, SessionInfo, WorkspaceInfo } from "./api.ts";
import type {
  BackendLifecycleEvent,
  LastCentralSurface,
  ShellMemoryDiagnostic,
  ShellMemoryRebuildResult,
  ShellMemoryResult,
  WorkspaceNavigationView,
} from "./orkworksWindow";
import { isSessionAlive } from "./sessionRuntimeState.ts";

export type CentralSurface =
  | { kind: "terminal"; sessionId: string | null }
  | { kind: "review"; sessionId: string; artifactId: string };

export type TemporaryUtilityDestination =
  | "sessions"
  | "changes"
  | "details"
  | "actions"
  | "recommendations"
  | "skills";

export type InspectorDestination = Exclude<TemporaryUtilityDestination, "sessions">;
export type ShellDestination = InspectorDestination;

export type ShellCommand =
  | "sessions"
  | "terminal"
  | "new-session"
  | "cancel-new-session"
  | "reset-layout"
  | "review"
  | "next-action"
  | ShellDestination;

export type ShellCommandHandler = (command: ShellCommand, reveal?: boolean) => void;

export type InspectedSubject =
  | { kind: "session"; sessionId: string }
  | { kind: "artifact"; sessionId: string; artifactId: string }
  | { kind: "workspace"; workspaceKey: string }
  | { kind: "personal" };

export function resolveInspectorSubject(
  destination: InspectorDestination,
  activeSessionId: string | null,
  workspaceKey: string | null,
): InspectedSubject | null {
  if ((destination === "details" || destination === "changes") && activeSessionId) {
    return { kind: "session", sessionId: activeSessionId };
  }
  if (destination === "skills" && !workspaceKey) return { kind: "personal" };
  return workspaceKey ? { kind: "workspace", workspaceKey } : null;
}

export type FocusTarget =
  | { kind: "sessions" }
  | { kind: "terminal"; sessionId: string }
  | { kind: "review"; sessionId: string; artifactId: string }
  | { kind: "inspector"; destination: InspectorDestination };

export type ReturnDescriptor = {
  surface: "terminal";
  sessionId: string;
  focusTarget: FocusTarget;
};

export interface ShellNavigationState {
  workspaceGeneration: number;
  activeSessionId: string | null;
  centralSurface: CentralSurface;
  inspector: InspectorDestination | null;
  inspectedSubject: InspectedSubject | null;
  focusTarget: FocusTarget;
  returnTarget: ReturnDescriptor | null;
  inspectorReturnFocus: FocusTarget | null;
  visibleFallbackReason: string | null;
}

export type ShellNavigationEvent =
  | { type: "session-selected"; sessionId: string; generation: number }
  | { type: "review-opened"; sessionId: string; artifactId: string; generation: number }
  | { type: "review-closed"; generation: number }
  | {
      type: "inspector-opened";
      destination: InspectorDestination;
      subject: InspectedSubject;
      generation: number;
    }
  | { type: "inspector-closed"; generation: number }
  | { type: "workspace-generation-changed"; generation: number }
  | { type: "session-restored"; sessionId: string | null; generation: number }
  | {
      type: "target-missing";
      target:
        | { kind: "session"; sessionId: string }
        | { kind: "artifact"; sessionId: string; artifactId: string };
      reason: string | null;
      generation: number;
    }
  | {
      type: "preferences-restored";
      surface: "terminal" | "review";
      artifactId: string | null;
      generation: number;
    };

export function createShellNavigationState(options: {
  workspaceGeneration: number;
  activeSessionId: string | null;
}): ShellNavigationState {
  return {
    workspaceGeneration: options.workspaceGeneration,
    activeSessionId: options.activeSessionId,
    centralSurface: { kind: "terminal", sessionId: options.activeSessionId },
    inspector: null,
    inspectedSubject: null,
    focusTarget: options.activeSessionId
      ? { kind: "terminal", sessionId: options.activeSessionId }
      : { kind: "sessions" },
    returnTarget: null,
    inspectorReturnFocus: null,
    visibleFallbackReason: null,
  };
}

function isCurrentGeneration(state: ShellNavigationState, generation: number): boolean {
  return generation === state.workspaceGeneration;
}

function terminalFocus(sessionId: string | null): FocusTarget {
  return sessionId ? { kind: "terminal", sessionId } : { kind: "sessions" };
}

function subjectMatchesTarget(
  subject: InspectedSubject | null,
  target: Extract<ShellNavigationEvent, { type: "target-missing" }>["target"],
): boolean {
  if (!subject) return false;
  if (target.kind === "session") {
    return (subject.kind === "session" || subject.kind === "artifact")
      && subject.sessionId === target.sessionId;
  }
  if (subject.kind === "artifact") {
    return subject.sessionId === target.sessionId && subject.artifactId === target.artifactId;
  }
  return false;
}

export function reduceShellNavigation(
  state: ShellNavigationState,
  event: ShellNavigationEvent,
): ShellNavigationState {
  if (event.type === "workspace-generation-changed") {
    if (event.generation <= state.workspaceGeneration) return state;
    return {
      ...state,
      workspaceGeneration: event.generation,
      activeSessionId: null,
      centralSurface: { kind: "terminal", sessionId: null },
      inspector: null,
      inspectedSubject: null,
      focusTarget: { kind: "sessions" },
      returnTarget: null,
      inspectorReturnFocus: null,
      visibleFallbackReason: null,
    };
  }

  if (!isCurrentGeneration(state, event.generation)) return state;

  switch (event.type) {
    case "session-selected": {
      return {
        ...state,
        activeSessionId: event.sessionId,
        centralSurface: { kind: "terminal", sessionId: event.sessionId },
        inspector: null,
        inspectedSubject: null,
        focusTarget: { kind: "terminal", sessionId: event.sessionId },
        returnTarget: null,
        inspectorReturnFocus: null,
        visibleFallbackReason: null,
      };
    }
    case "session-restored": {
      if (state.activeSessionId === event.sessionId) return state;
      return {
        ...state,
        activeSessionId: event.sessionId,
        centralSurface: { kind: "terminal", sessionId: event.sessionId },
        inspector: null,
        inspectedSubject: null,
        focusTarget: terminalFocus(event.sessionId),
        returnTarget: null,
        inspectorReturnFocus: null,
        visibleFallbackReason: null,
      };
    }
    case "review-opened": {
      if (
        !event.sessionId
        || !event.artifactId
        || event.sessionId !== state.activeSessionId
        || (state.centralSurface.kind !== "terminal" && state.centralSurface.sessionId !== event.sessionId)
      ) {
        return state;
      }
      const refreshing = state.centralSurface.kind === "review";
      const focusTarget: FocusTarget = {
        kind: "review",
        sessionId: event.sessionId,
        artifactId: event.artifactId,
      };
      return {
        ...state,
        centralSurface: { kind: "review", sessionId: event.sessionId, artifactId: event.artifactId },
        inspector: refreshing ? state.inspector : null,
        inspectedSubject: refreshing ? state.inspectedSubject : null,
        focusTarget: refreshing ? state.focusTarget : focusTarget,
        returnTarget: state.returnTarget ?? {
          surface: "terminal",
          sessionId: event.sessionId,
          focusTarget: { kind: "terminal", sessionId: event.sessionId },
        },
        inspectorReturnFocus: refreshing ? state.inspectorReturnFocus : null,
        visibleFallbackReason: null,
      };
    }
    case "review-closed": {
      if (state.centralSurface.kind !== "review") return state;
      const returnSessionId = state.returnTarget?.sessionId ?? state.centralSurface.sessionId;
      return {
        ...state,
        centralSurface: { kind: "terminal", sessionId: returnSessionId },
        inspector: null,
        inspectedSubject: null,
        focusTarget: state.returnTarget?.focusTarget ?? terminalFocus(returnSessionId),
        returnTarget: null,
        inspectorReturnFocus: null,
        visibleFallbackReason: null,
      };
    }
    case "inspector-opened": {
      return {
        ...state,
        inspector: event.destination,
        inspectedSubject: event.subject,
        focusTarget: { kind: "inspector", destination: event.destination },
        inspectorReturnFocus: state.inspectorReturnFocus ?? state.focusTarget,
      };
    }
    case "inspector-closed": {
      if (state.inspector === null) return state;
      return {
        ...state,
        inspector: null,
        inspectedSubject: null,
        focusTarget: state.inspectorReturnFocus ?? (
          state.centralSurface.kind === "terminal"
            ? terminalFocus(state.centralSurface.sessionId)
            : {
                kind: "review",
                sessionId: state.centralSurface.sessionId,
                artifactId: state.centralSurface.artifactId,
              }
        ),
        inspectorReturnFocus: null,
      };
    }
    case "target-missing": {
      const reviewMatches = state.centralSurface.kind === "review"
        && state.centralSurface.sessionId === event.target.sessionId
        && (event.target.kind === "session" || state.centralSurface.artifactId === event.target.artifactId);
      const inspectedMatches = subjectMatchesTarget(state.inspectedSubject, event.target);
      const activeTerminalMatches = event.target.kind === "session"
        && state.centralSurface.kind === "terminal"
        && state.centralSurface.sessionId === event.target.sessionId;
      const activeSelectionMatches = event.target.kind === "session"
        && state.activeSessionId === event.target.sessionId;
      const returnMatches = event.target.kind === "session"
        && state.returnTarget?.sessionId === event.target.sessionId;
      if (!reviewMatches && !inspectedMatches && !activeTerminalMatches && !activeSelectionMatches && !returnMatches) {
        return state;
      }

      const sessionIsMissing = event.target.kind === "session";
      const activeSessionId = sessionIsMissing && state.activeSessionId === event.target.sessionId
        ? null
        : state.activeSessionId;
      const fallbackSessionId = sessionIsMissing
        ? activeSessionId
        : (state.activeSessionId === event.target.sessionId ? event.target.sessionId : activeSessionId);
      const mustLeaveCentral = reviewMatches || activeTerminalMatches;
      return {
        ...state,
        activeSessionId,
        centralSurface: mustLeaveCentral
          ? { kind: "terminal", sessionId: fallbackSessionId }
          : state.centralSurface,
        inspector: inspectedMatches ? null : state.inspector,
        inspectedSubject: inspectedMatches ? null : state.inspectedSubject,
        focusTarget: mustLeaveCentral
          ? terminalFocus(fallbackSessionId)
          : inspectedMatches
            ? state.inspectorReturnFocus ?? terminalFocus(activeSessionId)
            : state.focusTarget,
        returnTarget: returnMatches || mustLeaveCentral ? null : state.returnTarget,
        inspectorReturnFocus: mustLeaveCentral
          ? terminalFocus(fallbackSessionId)
          : inspectedMatches ? null : state.inspectorReturnFocus,
        visibleFallbackReason: event.reason,
      };
    }
    case "preferences-restored": {
      if (event.surface === "terminal") {
        return {
          ...state,
          centralSurface: { kind: "terminal", sessionId: state.activeSessionId },
          inspector: null,
          inspectedSubject: null,
          focusTarget: terminalFocus(state.activeSessionId),
          returnTarget: null,
          inspectorReturnFocus: null,
          visibleFallbackReason: null,
        };
      }
      if (!state.activeSessionId || !event.artifactId) {
        return {
          ...state,
          centralSurface: { kind: "terminal", sessionId: state.activeSessionId },
          inspector: null,
          inspectedSubject: null,
          focusTarget: terminalFocus(state.activeSessionId),
          returnTarget: null,
          inspectorReturnFocus: null,
          visibleFallbackReason: "Review could not be restored because there is no current readable artifact for the selected session.",
        };
      }
      const sessionId = state.activeSessionId;
      return {
        ...state,
        centralSurface: { kind: "review", sessionId, artifactId: event.artifactId },
        inspector: null,
        inspectedSubject: null,
        focusTarget: { kind: "review", sessionId, artifactId: event.artifactId },
        returnTarget: {
          surface: "terminal",
          sessionId,
          focusTarget: { kind: "terminal", sessionId },
        },
        inspectorReturnFocus: null,
        visibleFallbackReason: null,
      };
    }
  }
}

export interface ReviewDocument {
  sessionId: string;
  generation: number;
  artifactId: string;
  content: string;
  kind?: PlanDocumentKind;
  scopeKey?: string;
  selectionId?: string;
  branch?: string | null;
}

export interface ReviewState {
  document: ReviewDocument | null;
  loading: boolean;
  error: boolean;
  changed: boolean;
}

export interface ShellNavigationSnapshot {
  navigation: ShellNavigationState;
  review: ReviewState;
  navigationDiagnostic: ShellMemoryDiagnostic | null;
  navigationRebuilding: boolean;
  visibleSubjectId: string | null;
}

export interface ShellNavigationRuntime {
  backendConnected: boolean;
  admissionEnabled: boolean;
  workspace: WorkspaceInfo | null;
  sessions: readonly SessionInfo[];
}

export interface ShellNavigationPort {
  getPlanContent: (sessionId: string, kind?: PlanDocumentKind, selectionId?: string) => Promise<string>;
  selectTerminalPlan: (
    sessionId: string,
    printedPath: string,
  ) => Promise<{ kind: PlanDocumentKind; selectionId: string; branch?: string | null } | undefined>;
  requestPlanReview: (sessionId: string, kind?: PlanDocumentKind, selectionId?: string) => Promise<void>;
  getWorkspaceNavigation: () => Promise<WorkspaceNavigationView>;
  completeWorkspaceNavigation: (surface: LastCentralSurface) => Promise<ShellMemoryResult>;
  rebuildWorkspaceNavigation: () => Promise<ShellMemoryRebuildResult>;
  digest?: (input: string) => Promise<string>;
}

export interface ShellNavigationControllerOptions {
  port?: Partial<ShellNavigationPort>;
  initialWorkspaceGeneration?: number;
  initialActiveSessionId?: string | null;
  onPlanObserved?: (sessionId: string) => void;
}

export interface ShellNavigationController {
  getSnapshot(): ShellNavigationSnapshot;
  subscribe(listener: () => void): () => void;
  beginNavigationIntent(): number;
  isNavigationIntentCurrent(intent: number, generation: number): boolean;
  registerCommandHandler(handler: ShellCommandHandler): () => void;
  dispatchCommand(command: ShellCommand, reveal?: boolean): void;
  handleLifecycleTransition(
    generation: number,
    state: BackendLifecycleEvent["state"],
    workspace?: WorkspaceInfo | null,
  ): void;
  fenceRuntime(patch: Partial<ShellNavigationRuntime>): void;
  syncRuntime(runtime: ShellNavigationRuntime): void;
  handleOwnerActiveSession(sessionId: string | null, cause?: { deliberate?: boolean }): void;
  selectSession(sessionId: string): void;
  inspect(destination: InspectorDestination | null): void;
  openReview(kind?: PlanDocumentKind): Promise<void>;
  selectTerminalPlan(sessionId: string, printedPath: string): Promise<void>;
  refreshReview(): Promise<void>;
  closeReview(): void;
  requestIndependentReview(sessionId: string, kind?: PlanDocumentKind, selectionId?: string): Promise<void>;
  saveNavigation(surface: LastCentralSurface): Promise<void>;
  rebuildNavigation(): Promise<void>;
  dispose(): void;
}

const defaultPort: ShellNavigationPort = {
  getPlanContent: (sessionId, kind, selectionId) =>
    kind === undefined
      ? window.orkworks.getPlanContent(sessionId)
      : window.orkworks.getPlanContent(sessionId, kind, selectionId),
  selectTerminalPlan: (sessionId, printedPath) => window.orkworks.selectTerminalPlan(sessionId, printedPath),
  requestPlanReview: (sessionId, kind, selectionId) =>
    kind === undefined
      ? window.orkworks.requestPlanReview(sessionId)
      : window.orkworks.requestPlanReview(sessionId, kind, selectionId),
  getWorkspaceNavigation: () => window.orkworks.getWorkspaceNavigation(),
  completeWorkspaceNavigation: (surface) => window.orkworks.completeWorkspaceNavigation(surface),
  rebuildWorkspaceNavigation: () => window.orkworks.rebuildWorkspaceNavigation(),
};

async function sha256Hex(input: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(input));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function reviewLocationKey(session: SessionInfo | undefined): string {
  return JSON.stringify([
    session?.enrichmentRevision,
    session?.repoRoot ?? session?.cwd,
    session?.planDocuments?.branch ?? session?.branch,
  ]);
}

function reviewScopeKey(session: SessionInfo | undefined): string {
  return JSON.stringify([
    session?.enrichmentRevision,
    session?.repoRoot ?? session?.cwd,
    session?.planDocuments?.branch ?? session?.branch,
    session?.planDocuments?.steps.map((step) => [step.kind, step.selectionId, step.available]),
  ]);
}

/**
 * Synchronous render-time gate ensuring stale Review bytes never paint for a
 * frame when session scope, workspace generation, or backend readiness changes.
 */
export function resolveVisibleReviewDocument(
  document: ReviewDocument | null,
  session: SessionInfo | undefined,
  generation: number,
  backendConnected: boolean,
): ReviewDocument | null {
  if (!document || !backendConnected || !session) return null;
  if (
    document.sessionId !== session.id
    || document.generation !== generation
    || document.scopeKey !== reviewScopeKey(session)
  ) {
    return null;
  }
  return document;
}

function selectPreferredStep(
  session: SessionInfo | undefined,
  preferredKind?: PlanDocumentKind,
): { kind?: PlanDocumentKind; selectionId?: string } {
  const steps = session?.planDocuments?.steps;
  const kind = preferredKind
    ?? steps?.find((step) => step.kind === "implementation" && step.available)?.kind
    ?? steps?.find((step) => step.kind === "design" && step.available)?.kind;
  return {
    kind,
    selectionId: steps?.find((step) => step.kind === kind)?.selectionId ?? undefined,
  };
}

function deriveVisibleSubjectId(navigation: ShellNavigationState): string | null {
  if (navigation.inspectedSubject && "sessionId" in navigation.inspectedSubject) {
    return navigation.inspectedSubject.sessionId;
  }
  return navigation.centralSurface.sessionId;
}

type ScopeTuple = {
  sessionId: string | null;
  generation: number;
  enabled: boolean;
  scopeKey: string;
  locationKey: string;
};

/**
 * Single renderer-side owner of shell navigation, workspace-generation / intent
 * fencing, Review document loading and invalidation, persisted navigation
 * hydration, and direct shell command dispatch.
 */
export function createShellNavigationController(
  options: ShellNavigationControllerOptions = {},
): ShellNavigationController {
  const port: ShellNavigationPort = { ...defaultPort, ...options.port };
  const digestFn = port.digest ?? sha256Hex;

  let disposed = false;
  let navigation = createShellNavigationState({
    workspaceGeneration: options.initialWorkspaceGeneration ?? 0,
    activeSessionId: options.initialActiveSessionId ?? null,
  });

  // Single unified intent / generation / hydration fence.
  let navigationIntent = 0;
  let restorationIntent = 0;
  let restorationInProgress = false;
  let reviewLoadIntent = 0;
  let navigationRebuildPending = false;
  let navigationReadSequence = 0;
  let restoredNavigationGeneration: number | null = null;
  let lastHydrationKey: string | null = null;

  // Navigation persistence diagnostic state.
  let navigationDiagnostic: ShellMemoryDiagnostic | null = null;
  let navigationRebuilding = false;

  // Review document state (previously split across useSessionReview.ts and App.tsx).
  let reviewDocument: ReviewDocument | null = null;
  let reviewLoading = false;
  let reviewError = false;
  let reviewChanged = false;
  let previousDigest: string | null = null;
  let reviewRequest = 0;
  let selectedKind: PlanDocumentKind | undefined;
  let selectedId: string | undefined;
  let selectedBranch: string | null | undefined;
  let explicitSelection: number | null = null;
  let lastReviewKind: PlanDocumentKind | undefined;
  let lastReviewDocument: ReviewDocument | null = null;
  const reviewRequests = new Map<string, Promise<void>>();

  // Runtime snapshot synced from React / caller.
  let runtime: ShellNavigationRuntime = {
    backendConnected: false,
    admissionEnabled: false,
    workspace: null,
    sessions: [],
  };

  function captureScopeTuple(): ScopeTuple {
    const session = activeSession();
    return {
      sessionId: navigation.activeSessionId,
      generation: navigation.workspaceGeneration,
      enabled: runtime.backendConnected && !!session,
      scopeKey: reviewScopeKey(session),
      locationKey: reviewLocationKey(session),
    };
  }

  let fencedScope: ScopeTuple = captureScopeTuple();
  let syncedScope: ScopeTuple = { ...fencedScope };

  // Presenter command handler (direct dispatch, no ShellCommandRequest prop ping-pong).
  let commandHandler: ShellCommandHandler | null = null;
  let pendingCommand: { command: ShellCommand; reveal: boolean } | null = null;

  const listeners = new Set<() => void>();
  let snapshot = buildSnapshot();

  function activeSession(): SessionInfo | undefined {
    return runtime.sessions.find((session) => session.id === navigation.activeSessionId);
  }

  function isReviewEnabled(): boolean {
    return runtime.backendConnected && !!activeSession();
  }

  function visibleReviewDocument(): ReviewDocument | null {
    return resolveVisibleReviewDocument(
      reviewDocument,
      activeSession(),
      navigation.workspaceGeneration,
      runtime.backendConnected,
    );
  }

  function buildSnapshot(): ShellNavigationSnapshot {
    return {
      navigation,
      review: {
        document: visibleReviewDocument(),
        loading: reviewLoading,
        error: reviewError,
        changed: reviewChanged,
      },
      navigationDiagnostic,
      navigationRebuilding,
      visibleSubjectId: deriveVisibleSubjectId(navigation),
    };
  }

  function emit(): void {
    if (disposed) return;
    snapshot = buildSnapshot();
    for (const listener of listeners) listener();
  }

  function dispatch(event: ShellNavigationEvent): boolean {
    const next = reduceShellNavigation(navigation, event);
    if (next === navigation) return false;
    navigation = next;
    return true;
  }

  function resetReviewState(): boolean {
    const changed = reviewDocument !== null || reviewLoading || reviewError || reviewChanged;
    ++reviewRequest;
    reviewDocument = null;
    selectedKind = undefined;
    selectedId = undefined;
    selectedBranch = undefined;
    previousDigest = null;
    reviewChanged = false;
    reviewLoading = false;
    reviewError = false;
    explicitSelection = null;
    return changed;
  }

  function markReviewUnavailable(kind?: PlanDocumentKind, selectionId?: string): void {
    resetReviewState();
    selectedKind = kind;
    selectedId = selectionId;
    reviewError = true;
  }

  function acceptReviewResult(generation: number, persist: boolean): boolean {
    return navigation.workspaceGeneration === generation
      && (!(persist || navigation.centralSurface.kind !== "review")
        || navigationIntent === reviewLoadIntent);
  }

  function applySynchronousScopeFence(): void {
    const next = captureScopeTuple();
    const tupleChanged =
      fencedScope.sessionId !== next.sessionId
      || fencedScope.generation !== next.generation
      || fencedScope.enabled !== next.enabled
      || fencedScope.scopeKey !== next.scopeKey;
    if (tupleChanged) {
      const sameLocation =
        fencedScope.sessionId === next.sessionId
        && fencedScope.generation === next.generation
        && fencedScope.enabled === next.enabled
        && fencedScope.locationKey === next.locationKey;
      fencedScope = next;
      if (explicitSelection === null || !sameLocation) {
        ++reviewRequest;
      }
    } else {
      fencedScope.locationKey = next.locationKey;
    }
    if (!runtime.backendConnected) {
      lastHydrationKey = null;
    }
  }

  async function saveNavigation(surface: LastCentralSurface): Promise<void> {
    if (!runtime.admissionEnabled || disposed) return;
    const generation = navigation.workspaceGeneration;
    try {
      const result = await port.completeWorkspaceNavigation(surface);
      if (disposed || navigation.workspaceGeneration !== generation) return;
      navigationDiagnostic = result.ok ? null : result.diagnostic;
      emit();
    } catch {
      if (disposed || navigation.workspaceGeneration !== generation) return;
      navigationDiagnostic = "write_failed";
      emit();
    }
  }

  async function loadReview(
    persist = true,
    restoring = false,
    printedPath?: string,
    kind?: PlanDocumentKind,
    selectionId?: string,
    branch?: string | null,
  ): Promise<void> {
    const sessionId = navigation.activeSessionId;
    const generation = navigation.workspaceGeneration;
    if (!sessionId || !isReviewEnabled() || disposed) return;

    const token = ++reviewRequest;
    let requestedKind = printedPath !== undefined ? undefined : kind ?? selectedKind;
    let requestedId = printedPath !== undefined
      ? undefined
      : selectionId ?? (kind === undefined || kind === selectedKind ? selectedId : undefined);
    let requestedBranch = printedPath !== undefined
      ? undefined
      : branch !== undefined ? branch : kind === undefined ? selectedBranch : undefined;

    if (printedPath !== undefined) explicitSelection = token;
    selectedKind = requestedKind;
    selectedId = requestedId;
    selectedBranch = requestedBranch;
    reviewLoading = true;
    // Never retain old document bytes across a refresh that may fail validation.
    reviewDocument = null;
    emit();

    try {
      if (printedPath !== undefined) {
        const selection = await port.selectTerminalPlan(sessionId, printedPath);
        if (disposed || reviewRequest !== token) return;
        requestedKind = selection?.kind;
        requestedId = selection?.selectionId;
        requestedBranch = selection?.branch;
        selectedKind = requestedKind;
        selectedId = requestedId;
        selectedBranch = requestedBranch;
      }
      const content = requestedKind === undefined
        ? await port.getPlanContent(sessionId)
        : await port.getPlanContent(sessionId, requestedKind, requestedId);
      const artifactId = await digestFn(`${requestedKind ?? "legacy"}\0${content}`);
      if (disposed || reviewRequest !== token) return;
      const next: ReviewDocument = {
        sessionId,
        generation,
        artifactId,
        content,
        kind: requestedKind,
        scopeKey: reviewScopeKey(activeSession()),
        selectionId: requestedId,
        branch: requestedBranch,
      };
      if (!acceptReviewResult(next.generation, persist)) return;
      lastReviewKind = next.kind;
      lastReviewDocument = next;
      // A clicked artifact may be selected before polling reports availability.
      // The successful validated read is current evidence for this exact session.
      options.onPlanObserved?.(next.sessionId);
      dispatch({
        type: "review-opened",
        sessionId: next.sessionId,
        artifactId: next.artifactId,
        generation: next.generation,
      });
      if (persist) void saveNavigation("review");
      reviewChanged = previousDigest !== null && previousDigest !== artifactId;
      previousDigest = artifactId;
      reviewDocument = next;
      reviewError = false;
      emit();
    } catch (error) {
      // A rejected click stays in Terminal and reports the sidecar's explanation.
      if (printedPath !== undefined) {
        if (!disposed && reviewRequest === token) throw error;
        return;
      }
      if (!disposed && reviewRequest === token && acceptReviewResult(generation, persist)) {
        if (restoring) {
          dispatch({ type: "preferences-restored", surface: "review", artifactId: null, generation });
          emit();
        } else {
          reviewError = true;
          emit();
        }
      }
    } finally {
      if (explicitSelection === token) explicitSelection = null;
      if (!disposed && reviewRequest === token) {
        reviewLoading = false;
        emit();
      }
    }
  }

  function hydrateWorkspaceNavigation(): void {
    const generation = navigation.workspaceGeneration;
    if (!runtime.backendConnected || !runtime.workspace || restoredNavigationGeneration === generation) return;
    const session = activeSession();
    const hydrationKey = `${generation}:${runtime.workspace.workspaceIdentity || runtime.workspace.path}:${session?.id ?? ""}`;
    if (lastHydrationKey === hydrationKey) return;
    lastHydrationKey = hydrationKey;

    // Diagnostics belong to the ready workspace even before a session is restored.
    if (session) restoredNavigationGeneration = generation;
    const intent = navigationIntent;
    const sessionId = session?.id ?? null;
    const readSequence = ++navigationReadSequence;

    void port.getWorkspaceNavigation().then((saved) => {
      if (disposed || navigation.workspaceGeneration !== generation || navigationReadSequence !== readSequence) return;
      navigationDiagnostic = saved.diagnostic;
      emit();
      if (
        saved.diagnostic
        || !session
        || intent !== restorationIntent
        || navigationIntent !== intent
        || navigation.activeSessionId !== sessionId
      ) {
        return;
      }
      if (saved.lastCentralSurface === "review") {
        if (session.hasOpenablePlan) {
          const preferred = selectPreferredStep(session);
          reviewLoadIntent = intent;
          void loadReview(false, true, undefined, preferred.kind, preferred.selectionId);
        } else {
          dispatch({ type: "preferences-restored", surface: "review", artifactId: null, generation });
          emit();
        }
      }
    }).catch(() => {
      if (!disposed && navigation.workspaceGeneration === generation && navigationReadSequence === readSequence) {
        navigationDiagnostic = "restore_failed";
        emit();
      }
    });
  }

  function beginNavigationIntent(): number {
    const intent = ++navigationIntent;
    if (reviewLoading && !reviewError && navigation.centralSurface.kind === "terminal") {
      if (resetReviewState()) emit();
    }
    return intent;
  }

  return {
    getSnapshot() {
      return snapshot;
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },

    beginNavigationIntent,

    isNavigationIntentCurrent(intent, generation) {
      return navigationIntent === intent && navigation.workspaceGeneration === generation;
    },

    registerCommandHandler(handler) {
      commandHandler = handler;
      if (pendingCommand) {
        const queued = pendingCommand;
        pendingCommand = null;
        handler(queued.command, queued.reveal);
      }
      return () => {
        if (commandHandler === handler) commandHandler = null;
      };
    },

    dispatchCommand(command, reveal = false) {
      if (disposed) return;
      beginNavigationIntent();
      if (commandHandler) commandHandler(command, reveal);
      else pendingCommand = { command, reveal };
    },

    handleLifecycleTransition(generation, state, nextWorkspace) {
      // Preserve the transition-start intent through opening/starting/retrying.
      // Readiness must not make intervening user navigation look like hydration.
      if (!restorationInProgress) restorationIntent = navigationIntent;
      restorationInProgress = ["opening", "closing", "starting", "retrying"].includes(state);
      const isReady = state === "ready";
      const clearWorkspace = state === "picker" || state === "closing" || state === "unresolved";
      runtime = {
        backendConnected: isReady,
        admissionEnabled: isReady,
        workspace: isReady
          ? (nextWorkspace !== undefined ? nextWorkspace : runtime.workspace)
          : (clearWorkspace ? null : runtime.workspace),
        sessions: [],
      };
      navigationDiagnostic = null;
      navigationRebuilding = false;
      lastHydrationKey = null;
      resetReviewState();
      dispatch({ type: "workspace-generation-changed", generation });
      fencedScope = captureScopeTuple();
      syncedScope = { ...fencedScope };
      emit();
    },

    fenceRuntime(patch) {
      if (disposed) return;
      runtime = { ...runtime, ...patch };
      applySynchronousScopeFence();
    },

    syncRuntime(nextRuntime) {
      if (disposed) return;
      runtime = nextRuntime;
      if (!runtime.backendConnected) {
        lastHydrationKey = null;
      }
      const session = activeSession();
      const sessionId = navigation.activeSessionId;
      const generation = navigation.workspaceGeneration;
      const next = captureScopeTuple();

      const scopeChanged = syncedScope.scopeKey !== next.scopeKey;
      const locationUnchanged = syncedScope.locationKey === next.locationKey;
      const tupleChanged =
        syncedScope.sessionId !== next.sessionId
        || syncedScope.generation !== next.generation
        || syncedScope.enabled !== next.enabled
        || scopeChanged;

      let stateMutated = false;
      if (tupleChanged) {
        const sameLocation =
          syncedScope.sessionId === next.sessionId
          && syncedScope.generation === next.generation
          && syncedScope.enabled === next.enabled
          && locationUnchanged;
        syncedScope = next;
        fencedScope = { ...next };
        if (explicitSelection === null || !sameLocation) {
          if (resetReviewState()) stateMutated = true;
        }
      } else {
        syncedScope.locationKey = next.locationKey;
        fencedScope.locationKey = next.locationKey;
      }

      if (scopeChanged && navigation.centralSurface.kind === "review") {
        const previous = lastReviewDocument;
        if (
          locationUnchanged
          && previous?.sessionId === sessionId
          && previous.generation === generation
          && previous.branch !== undefined
          && previous.branch !== session?.planDocuments?.branch
        ) {
          reviewLoadIntent = ++navigationIntent;
          void loadReview(false, false, undefined, previous.kind, previous.selectionId, previous.branch);
        } else {
          const steps = session?.planDocuments?.steps;
          const priorStep = steps?.find((step) => step.kind === lastReviewKind);
          if (!session?.hasOpenablePlan || (priorStep && !priorStep.available)) {
            markReviewUnavailable(priorStep?.kind ?? lastReviewKind, priorStep?.selectionId ?? undefined);
            stateMutated = true;
          } else {
            const preferredKind =
              steps?.find((step) => step.kind === lastReviewKind && step.available)?.kind
              ?? steps?.find((step) => step.kind === "implementation" && step.available)?.kind
              ?? steps?.find((step) => step.kind === "design" && step.available)?.kind;
            reviewLoadIntent = ++navigationIntent;
            void loadReview(
              false,
              false,
              undefined,
              preferredKind,
              steps?.find((step) => step.kind === preferredKind)?.selectionId ?? undefined,
            );
          }
        }
      }

      if (navigation.centralSurface.kind === "review" && !session?.hasOpenablePlan) {
        if (dispatch({
          type: "target-missing",
          target: {
            kind: "artifact",
            sessionId: navigation.centralSurface.sessionId,
            artifactId: navigation.centralSurface.artifactId,
          },
          reason: "The reviewed plan is no longer available.",
          generation,
        })) {
          stateMutated = true;
        }
      }

      if (stateMutated || snapshot.review.document !== visibleReviewDocument()) {
        emit();
      }
      hydrateWorkspaceNavigation();
    },

    handleOwnerActiveSession(sessionId, { deliberate } = {}) {
      if (disposed) return;
      const generation = navigation.workspaceGeneration;
      let changed = false;
      if (sessionId === null && navigation.activeSessionId !== null) {
        changed = dispatch({
          type: "target-missing",
          target: { kind: "session", sessionId: navigation.activeSessionId },
          reason: deliberate ? null : "The selected session is no longer available.",
          generation,
        });
      } else {
        changed = dispatch({ type: "session-restored", sessionId, generation });
      }
      if (changed) {
        resetReviewState();
        fencedScope = captureScopeTuple();
        syncedScope = { ...fencedScope };
        emit();
      }
    },

    selectSession(sessionId) {
      if (disposed) return;
      ++navigationIntent;
      resetReviewState();
      dispatch({
        type: "session-selected",
        sessionId,
        generation: navigation.workspaceGeneration,
      });
      fencedScope = captureScopeTuple();
      syncedScope = { ...fencedScope };
      emit();
      void saveNavigation("terminal");
    },

    inspect(destination) {
      if (disposed) return;
      ++navigationIntent;
      if (destination) {
        const workspaceKey = runtime.workspace
          ? runtime.workspace.workspaceIdentity || runtime.workspace.path
          : null;
        const subject = resolveInspectorSubject(destination, navigation.activeSessionId, workspaceKey);
        if (!subject) return;
        if (dispatch({
          type: "inspector-opened",
          destination,
          subject,
          generation: navigation.workspaceGeneration,
        })) {
          emit();
        }
      } else if (dispatch({
        type: "inspector-closed",
        generation: navigation.workspaceGeneration,
      })) {
        emit();
      }
    },

    openReview(kind) {
      reviewLoadIntent = ++navigationIntent;
      const preferred = selectPreferredStep(activeSession(), kind);
      return loadReview(true, false, undefined, preferred.kind, preferred.selectionId);
    },

    selectTerminalPlan(sessionId, printedPath) {
      if (disposed || !runtime.admissionEnabled || sessionId !== navigation.activeSessionId) {
        return Promise.reject(new Error("Select the session again before opening its document."));
      }
      reviewLoadIntent = ++navigationIntent;
      return loadReview(true, false, printedPath);
    },

    refreshReview() {
      reviewLoadIntent = ++navigationIntent;
      return loadReview(false);
    },

    closeReview() {
      if (disposed) return;
      const wasReview = navigation.centralSurface.kind === "review";
      ++navigationIntent;
      resetReviewState();
      dispatch({ type: "review-closed", generation: navigation.workspaceGeneration });
      emit();
      if (wasReview) void saveNavigation("terminal");
    },

    requestIndependentReview(sessionId, kind, selectionId) {
      const target = runtime.sessions.find((session) => session.id === sessionId);
      if (disposed || !runtime.admissionEnabled || !isSessionAlive(target)) {
        return Promise.reject(new Error("Independent review requires an active session."));
      }
      const key = `${navigation.workspaceGeneration}:${sessionId}:${kind ?? "legacy"}:${selectionId ?? ""}`;
      const pending = reviewRequests.get(key);
      if (pending) return pending;
      const request = port.requestPlanReview(sessionId, kind, selectionId);
      reviewRequests.set(key, request);
      const settled = () => { reviewRequests.delete(key); };
      void request.then(settled, settled);
      return request;
    },

    saveNavigation,

    async rebuildNavigation() {
      if (navigationRebuildPending || disposed) return;
      navigationRebuildPending = true;
      ++navigationReadSequence;
      const generation = navigation.workspaceGeneration;
      navigationRebuilding = true;
      emit();
      try {
        const result = await port.rebuildWorkspaceNavigation();
        if (disposed || navigation.workspaceGeneration !== generation) return;
        if (result.ok) navigationDiagnostic = null;
        else if (result.diagnostic !== "user_cancelled") navigationDiagnostic = result.diagnostic;
        emit();
      } catch {
        if (!disposed && navigation.workspaceGeneration === generation) {
          navigationDiagnostic = "restore_failed";
          emit();
        }
      } finally {
        navigationRebuildPending = false;
        if (!disposed && navigation.workspaceGeneration === generation) {
          ++navigationReadSequence;
          navigationRebuilding = false;
          emit();
        }
      }
    },

    dispose() {
      disposed = true;
      ++reviewRequest;
      reviewDocument = null;
      lastReviewDocument = null;
      reviewRequests.clear();
      commandHandler = null;
      pendingCommand = null;
      listeners.clear();
    },
  };
}
