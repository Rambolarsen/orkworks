export type CentralSurface =
  | { kind: "terminal"; sessionId: string | null }
  | { kind: "review"; sessionId: string; artifactId: string };

export type TemporaryUtilityDestination =
  | "sessions"
  | "details"
  | "actions"
  | "recommendations"
  | "capacity";

export type InspectorDestination = Exclude<TemporaryUtilityDestination, "sessions">;

export type InspectedSubject =
  | { kind: "session"; sessionId: string }
  | { kind: "artifact"; sessionId: string; artifactId: string }
  | { kind: "workspace"; workspaceKey: string };

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
  acknowledgedSessionIds: readonly string[];
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
  | {
      type: "target-missing";
      target:
        | { kind: "session"; sessionId: string }
        | { kind: "artifact"; sessionId: string; artifactId: string };
      reason: string;
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
    acknowledgedSessionIds: [],
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
  target: Extract<ShellNavigationEvent, { type: "target-missing" }>['target'],
): boolean {
  if (!subject || subject.kind !== target.kind) return false;
  if (subject.kind === "session" && target.kind === "session") {
    return subject.sessionId === target.sessionId;
  }
  if (subject.kind === "artifact" && target.kind === "artifact") {
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
      centralSurface: { kind: "terminal", sessionId: state.activeSessionId },
      inspector: null,
      inspectedSubject: null,
      focusTarget: terminalFocus(state.activeSessionId),
      returnTarget: null,
      inspectorReturnFocus: null,
      visibleFallbackReason: null,
    };
  }

  if (!isCurrentGeneration(state, event.generation)) return state;

  switch (event.type) {
    case "session-selected": {
      const acknowledgedSessionIds = state.acknowledgedSessionIds.includes(event.sessionId)
        ? state.acknowledgedSessionIds
        : [...state.acknowledgedSessionIds, event.sessionId];
      return {
        ...state,
        activeSessionId: event.sessionId,
        acknowledgedSessionIds,
        centralSurface: { kind: "terminal", sessionId: event.sessionId },
        inspector: null,
        inspectedSubject: null,
        focusTarget: { kind: "terminal", sessionId: event.sessionId },
        returnTarget: null,
        inspectorReturnFocus: null,
        visibleFallbackReason: null,
      };
    }
    case "review-opened": {
      if (
        !event.sessionId ||
        !event.artifactId ||
        event.sessionId !== state.activeSessionId ||
        state.centralSurface.kind !== "terminal"
      ) {
        return state;
      }
      const focusTarget: FocusTarget = {
        kind: "review",
        sessionId: event.sessionId,
        artifactId: event.artifactId,
      };
      return {
        ...state,
        centralSurface: { kind: "review", sessionId: event.sessionId, artifactId: event.artifactId },
        inspector: null,
        inspectedSubject: null,
        focusTarget,
        returnTarget: {
          surface: "terminal",
          sessionId: event.sessionId,
          focusTarget: { kind: "terminal", sessionId: event.sessionId },
        },
        inspectorReturnFocus: null,
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
      const reviewMatches = state.centralSurface.kind === "review" &&
        state.centralSurface.sessionId === event.target.sessionId &&
        (event.target.kind === "session" || state.centralSurface.artifactId === event.target.artifactId);
      const inspectedMatches = subjectMatchesTarget(state.inspectedSubject, event.target);
      const returnMatches = state.returnTarget?.sessionId === event.target.sessionId;
      if (!reviewMatches && !inspectedMatches && !returnMatches) return state;

      const sessionIsMissing = event.target.kind === "session";
      const activeSessionId = sessionIsMissing && state.activeSessionId === event.target.sessionId
        ? null
        : state.activeSessionId;
      const fallbackSessionId = sessionIsMissing
        ? activeSessionId
        : (state.activeSessionId === event.target.sessionId ? event.target.sessionId : activeSessionId);
      const mustLeaveReview = reviewMatches;
      return {
        ...state,
        activeSessionId,
        centralSurface: mustLeaveReview
          ? { kind: "terminal", sessionId: fallbackSessionId }
          : state.centralSurface,
        inspector: inspectedMatches ? null : state.inspector,
        inspectedSubject: inspectedMatches ? null : state.inspectedSubject,
        focusTarget: mustLeaveReview
          ? terminalFocus(fallbackSessionId)
          : inspectedMatches
            ? state.inspectorReturnFocus ?? terminalFocus(activeSessionId)
            : state.focusTarget,
        returnTarget: returnMatches || mustLeaveReview ? null : state.returnTarget,
        inspectorReturnFocus: inspectedMatches ? null : state.inspectorReturnFocus,
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
