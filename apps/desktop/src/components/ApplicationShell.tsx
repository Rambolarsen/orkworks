import { isSessionAlive } from "../sessionRuntimeState";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { PanelLeft, PanelRight, Plus, ChevronDown, Lightbulb, BookOpen, Settings, Pin, PinOff, X } from "lucide-react";
import type { CentralSurface } from "../shellNavigation";
import type { SessionAction, SessionAttention, SessionInfo, WorkflowRecommendation, WorkspaceInfo } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import type { DebugSettings } from "../appSettingsTypes";
import type { ShellPreferences } from "../orkworksWindow";
import { resumeChoices } from "../labels";
import { computeShellLayout } from "../fixedShellLayout";
import SessionListPanel from "./SessionListPanel";
import SessionAppMenu from "./SessionAppMenu";
import ResumeChooser from "./ResumeChooser";
import GitChangesPanel from "./GitChangesPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import RecommendationsPanel from "./RecommendationsPanel";
import SkillsSettings from "./SkillsSettings";
import ActionsOverview from "./ActionsOverview";
import { buildActionsOverview, type ActionRow, type ActionsSourceStatus } from "../actionsOverview";
import { useActionsRecommendations } from "../useActionsRecommendations";
import RegionSeparator from "./RegionSeparator";
import { disposeAllTerminals } from "../terminalStore";

export type ShellDestination = "changes" | "details" | "recommendations" | "skills" | "actions";
export type ShellCommand = "sessions" | "terminal" | "new-session" | "cancel-new-session" | "reset-layout" | "review" | "next-action" | ShellDestination;
export interface ShellCommandRequest { command: ShellCommand; sequence: number; reveal?: boolean }

export function shouldShowRecommendationsPanel(session: Pick<SessionInfo, "harnessId" | "harness"> | null | undefined): boolean {
  const harnessId = session?.harnessId?.trim() || session?.harness?.trim();
  return Boolean(harnessId) && harnessId !== "generic-shell";
}

interface Props {
  centralSurface?: CentralSurface;
  reviewContent?: ReactNode;
  reviewLoading?: boolean;
  reviewError?: boolean;
  onOpenReview?: () => void;
  onSelectPlan?: (sessionId: string, path: string) => Promise<void>;
  onRequestPlanReview?: (sessionId: string) => Promise<void>;
  onNavigate?: () => void;
  onCloseReview?: () => void;
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  workspaceControl: ReactNode;
  workspaceGeneration: number;
  isSwitchingWorkspace: boolean;
  debugSettings: DebugSettings;
  sessions: SessionInfo[];
  sessionsSourceStatus?: ActionsSourceStatus;
  activeSessionId: string | null;
  visibleSubjectId: string | null;
  canFixWithAi: boolean;
  taskmasterReady: boolean;
  focusedRecommendationId: string | null;
  unreadIds: ReadonlySet<string>;
  acknowledgedIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  resumeTick: number;
  resumingSessionIds?: ReadonlySet<string>;
  preferences: ShellPreferences;
  onPreferencesChange: (preferences: ShellPreferences) => void;
  onResetPreferences: () => Promise<boolean>;
  inspector: ShellDestination | null;
  onInspect: (destination: ShellDestination | null) => void;
  commandRequest?: ShellCommandRequest | null;
  onSelectSession: (id: string) => void;
  onOpenRecommendation: (id: string) => void;
  onFixWithAi: (recommendation: WorkflowRecommendation) => void;
  onRunCleanup: (recommendation: WorkflowRecommendation) => void;
  newSessionContent?: ReactNode;
  onCancelNewSession?: () => void;
  onCreateSession: () => void;
  onSessionAction?: (id: string, action: SessionAction, token: string) => Promise<void>;
  onKillSession: (id: string) => void;
  onForgetSession: (id: string) => void;
  onResumeSession: (id: string, strategy?: import("../api").ResumeStrategy) => void;
  onRowResumeSession?: (id: string) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  onFocusTerminal: () => void;
  onOpenSettings: (section?: "tools" | "providers") => void;
  onOpenWorkspace: () => void;
  onBackendUnavailable: () => void;
  onRetryBackend: () => void;
}

const TITLES = { changes: "Changes", actions: "Actions", details: "Details", recommendations: "Recommendations", skills: "Skills" };

export default function ApplicationShell(props: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const invokerRef = useRef<HTMLElement | null>(null);
  const invokerSelectorRef = useRef<string | null>(null);
  const reviewInvoker = useRef<{ element: HTMLElement | null; selector: string | null } | null>(null);
  const navigationIntent = useRef(0);
  const creationReturn = useRef<{ inspector: ShellDestination | null; sessionsPage: boolean } | null>(null);
  const workspaceGenerationRef = useRef(props.workspaceGeneration);
  workspaceGenerationRef.current = props.workspaceGeneration;
  const focusIntent = useRef<"page" | "changes-return" | "return" | "review-return" | "terminal" | "sessions" | null>(null);
  const [width, setWidth] = useState(() => window.innerWidth);
  const [sessionsPage, setSessionsPage] = useState(false);
  const [focusTick, setFocusTick] = useState(0);
  const [actionFocusRequest, setActionFocusRequest] = useState<{ scope: string; sequence: number }>({ scope: "", sequence: 0 });
  const [actionInspection, setActionInspection] = useState<{ scope: string; row: ActionRow | null }>({ scope: "", row: null });
  const actionsScope = `${props.workspace?.workspaceIdentity || props.workspace?.path || ""}:${props.workspaceGeneration}`;
  const sourcesReady = !!props.workspace && !props.isSwitchingWorkspace && props.backendStatus === "connected";
  const recommendations = useActionsRecommendations(actionsScope, sourcesReady && props.taskmasterReady, props.workspace?.path ?? "");
  const actionsModel = buildActionsOverview(sourcesReady ? props.sessions : [], sourcesReady ? props.sessionsSourceStatus ?? "loading" : "unavailable",
    recommendations.items, recommendations.status);
  const recommendationsPage = props.inspector === "recommendations";
  const skillsPage = props.inspector === "skills";
  const centralUtility = recommendationsPage || skillsPage;
  const layout = computeShellLayout(width, props.preferences, props.inspector !== null && !centralUtility);
  const compact = layout.mode === "compact";
  const showSessionsPage = compact && sessionsPage;
  const temporaryUtility = !showSessionsPage && props.inspector !== null && (layout.inspectorPage || centralUtility);
  const showComposer = !!props.newSessionContent && !showSessionsPage && !temporaryUtility;
  const reviewing = (props.centralSurface?.kind === "review" && props.centralSurface.sessionId === props.activeSessionId)
    || (!!props.reviewError && !!props.activeSessionId);
  const showReview = reviewing && !showSessionsPage && !temporaryUtility && !showComposer;
  const showTerminal = !showSessionsPage && !temporaryUtility && !showComposer && !reviewing;
  const showSessions = showSessionsPage || layout.sessionsVisible;
  const showInspector = !showSessionsPage && !centralUtility && layout.inspectorWidth > 0 && props.inspector !== null;
  const session = props.sessions.find(s => s.id === props.activeSessionId) ?? null;
  const isResuming = !!session && !!props.resumingSessionIds?.has(session.id);
  const canResume = session?.lifecycle === "dead" && session.sessionState !== "archived"
    && props.backendStatus === "connected" && !props.isSwitchingWorkspace
    && resumeChoices(session).some(option => !option.unavailable);

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const measure = () => setWidth(root.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(root);
    return () => observer.disconnect();
  }, []);

  const requestFocus = (intent: NonNullable<typeof focusIntent.current>) => {
    focusIntent.current = intent;
    setFocusTick(tick => tick + 1);
  };
  const captureInvoker = (command: ShellCommand) => {
    const element = document.activeElement;
    invokerRef.current = element instanceof HTMLElement && element !== document.body
      ? element
      : rootRef.current?.querySelector<HTMLElement>(`[data-shell-command="${command}"]`) ?? null;
    invokerSelectorRef.current = invokerRef.current?.closest(".shell-navigation-menu")
      ? ".shell-navigation-menu summary"
      : invokerRef.current?.dataset.shellCommand ? `[data-shell-command="${invokerRef.current.dataset.shellCommand}"]` : null;
  };
  const closePage = () => {
    ++navigationIntent.current;
    setSessionsPage(false);
    props.onInspect(null);
    requestFocus("return");
  };
  const closeReviewPage = () => {
    invokerRef.current = reviewInvoker.current?.element ?? null;
    invokerSelectorRef.current = reviewInvoker.current?.selector ?? null;
    reviewInvoker.current = null;
    props.onCloseReview?.();
    requestFocus("review-return");
  };
  const runCommand = (command: ShellCommand, reveal = false, toggleSessions = false) => {
    props.onNavigate?.();
    const intent = ++navigationIntent.current;
    if (command === "next-action") {
      if (!props.inspector && !sessionsPage) captureInvoker(command);
      setSessionsPage(false);
      props.onInspect("actions");
      setActionFocusRequest(old => ({ scope: actionsScope, sequence: old.sequence + 1 }));
      return;
    }
    if (command === "reset-layout") {
      const generation = props.workspaceGeneration;
      void props.onResetPreferences().then(completed => {
        if (!completed || navigationIntent.current !== intent || workspaceGenerationRef.current !== generation) return;
        setSessionsPage(false);
        props.onInspect(null);
        requestFocus("terminal");
      });
    } else if (command === "review") {
      if (!session?.hasOpenablePlan || !props.onOpenReview || props.backendStatus !== "connected" || props.isSwitchingWorkspace) return;
      if (!reviewing) {
        captureInvoker(command);
        reviewInvoker.current = { element: invokerRef.current, selector: invokerSelectorRef.current };
      }
      creationReturn.current = null;
      setSessionsPage(false);
      props.onCancelNewSession?.();
      props.onInspect(null);
      if (reviewing) requestFocus("page");
      else props.onOpenReview();
    } else if (command === "new-session") {
      if (!creationReturn.current) {
        creationReturn.current = { inspector: props.inspector, sessionsPage };
        invokerRef.current = rootRef.current?.querySelector<HTMLElement>("[data-shell-create]") ?? null;
        invokerSelectorRef.current = "[data-shell-create]";
      }
      setSessionsPage(false);
      props.onInspect(null);
    } else if (command === "cancel-new-session") {
      const target = creationReturn.current;
      creationReturn.current = null;
      props.onCancelNewSession?.();
      setSessionsPage(target?.sessionsPage ?? false);
      props.onInspect(target?.inspector ?? null);
      requestFocus(target?.sessionsPage ? "sessions" : "return");
    } else if (command === "terminal") {
      props.onCloseReview?.();
      creationReturn.current = null;
      props.onCancelNewSession?.();
      setSessionsPage(false);
      props.onInspect(null);
      requestFocus("terminal");
    } else if (command === "sessions") {
      const list = document.getElementById("sessions-list");
      const focused = list?.contains(document.activeElement) === true;
      if (compact) {
        if (sessionsPage && (focused || toggleSessions)) closePage();
        else { captureInvoker(command); setSessionsPage(true); props.onInspect(null); requestFocus("sessions"); }
      } else if (layout.sessionsVisible && (focused || toggleSessions)) {
        props.onPreferencesChange({ ...props.preferences, sessionsVisible: false });
        requestFocus(temporaryUtility ? "page" : "terminal");
      } else {
        captureInvoker(command);
        props.onPreferencesChange({ ...props.preferences, sessionsVisible: true });
        requestFocus("sessions");
      }
    } else {
      if (command === "recommendations" && !reveal && !shouldShowRecommendationsPanel(session)) return;
      if (props.inspector === command && !reveal) { closePage(); return; }
      // Preserve the original invoker when changing utilities inside a temporary page.
      if (!props.inspector && !sessionsPage) captureInvoker(command);
      setSessionsPage(false);
      props.onInspect(command);
      requestFocus("page");
    }
  };
  const commandRef = useRef(runCommand);
  commandRef.current = runCommand;
  useEffect(() => {
    if (props.commandRequest) commandRef.current(props.commandRequest.command, props.commandRequest.reveal);
  }, [props.commandRequest]);

  useEffect(() => {
    setSessionsPage(false);
    creationReturn.current = null;
    invokerRef.current = null;
    invokerSelectorRef.current = null;
    reviewInvoker.current = null;
  }, [props.workspaceGeneration]);

  useEffect(() => {
    if (!showInspector || props.preferences.inspectorPinned) return;
    const dismissOutsideInspector = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || target.closest(`.shell-inspector-host, [role="dialog"], [data-shell-command="${props.inspector}"]`)) return;
      // Let the clicked surface receive focus and perform its own action.
      ++navigationIntent.current;
      props.onInspect(null);
    };
    document.addEventListener("pointerdown", dismissOutsideInspector, true);
    return () => document.removeEventListener("pointerdown", dismissOutsideInspector, true);
  }, [showInspector, props.inspector, props.preferences.inspectorPinned, props.onInspect]);

  useLayoutEffect(() => {
    const intent = focusIntent.current;
    focusIntent.current = null;
    const root = rootRef.current;
    if (!root) return;
    if (!intent) {
      // A responsive transition may remove the focused page or region.
      if (document.activeElement === document.body) {
        (root.querySelector<HTMLElement>("[data-shell-page-heading]") ?? root.querySelector<HTMLElement>("[data-shell-review-heading]") ?? root.querySelector<HTMLElement>("[data-shell-terminal-focus]") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))?.focus({ preventScroll: true });
      }
      return;
    }
    let returnTarget = invokerRef.current?.isConnected && !invokerRef.current.closest("details:not([open])") && invokerRef.current.getClientRects().length
      ? invokerRef.current
      : invokerSelectorRef.current ? root.querySelector<HTMLElement>(invokerSelectorRef.current) : null;
    const closedDisclosure = returnTarget?.closest("details:not([open])");
    if (closedDisclosure) returnTarget = closedDisclosure.querySelector<HTMLElement>("summary");
    if (intent === "changes-return") {
      root.querySelector<HTMLElement>("[data-review-changes]")?.focus({ preventScroll: true });
    } else if ((intent === "return" || intent === "review-return") && returnTarget?.getClientRects().length) {
      returnTarget.focus({ preventScroll: true });
      invokerRef.current = null;
      invokerSelectorRef.current = null;
    } else if (intent === "sessions") {
      (document.getElementById("sessions-list") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))?.focus({ preventScroll: true });
    } else if (intent === "terminal" || intent === "review-return") {
      root.querySelector<HTMLElement>("[data-shell-terminal-focus]")?.focus({ preventScroll: true });
      props.onFocusTerminal();
    } else {
      (root.querySelector<HTMLElement>("[data-shell-page-heading]") ?? root.querySelector<HTMLElement>("[data-shell-review-heading]") ?? root.querySelector<HTMLElement>("[data-shell-terminal-focus]") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))?.focus({ preventScroll: true });
    }
  }, [focusTick, showTerminal, showSessionsPage, temporaryUtility, showInspector, props.onFocusTerminal]);
  useLayoutEffect(() => {
    if (reviewing) rootRef.current?.querySelector<HTMLElement>("[data-shell-review-heading]")?.focus({ preventScroll: true });
  }, [reviewing]);
  useLayoutEffect(() => {
    // Retry removes its focused button; recover focus without stealing it from an inspector.
    if (reviewing && !props.reviewError && document.activeElement === document.body)
      rootRef.current?.querySelector<HTMLElement>("[data-shell-review-heading]")?.focus({ preventScroll: true });
  }, [reviewing, props.reviewError]);

  useEffect(() => {
    // Hidden terminal presentation has no CenterPanel effect to observe backend loss.
    if (props.backendStatus !== "connected") disposeAllTerminals();
  }, [props.backendStatus]);

  useEffect(() => {
    for (const [id, visible] of Object.entries({ sessions: showSessions, terminal: showTerminal,
      detail: props.inspector === "details" && (showInspector || temporaryUtility),
      recommendations: props.inspector === "recommendations" && (showInspector || temporaryUtility), review: showReview })) {
      window.orkworks.notifyPanelVisibility(id, visible);
    }
  }, [showSessions, showTerminal, showInspector, temporaryUtility, props.inspector, showReview]);

  const renderUtility = () => {
    if (skillsPage) return <SkillsSettings currentWorkspacePath={props.isSwitchingWorkspace ? null : props.workspace?.workspaceIdentity ?? props.workspace?.path ?? null} />;
    if (props.inspector === "changes") return <GitChangesPanel key={`${props.workspaceGeneration}:${props.activeSessionId}:${session?.repoRoot ?? ""}:${session?.worktreeBinding?.root ?? ""}:${session?.worktreeBinding?.commonDir ?? ""}`}
      session={props.sessions.find(s => s.id === props.activeSessionId) ?? null}
      onBack={() => { props.onInspect("details"); requestFocus("changes-return"); }} />;
    if (props.inspector === "details") return <SessionDetailPanel key={props.workspaceGeneration} sessions={props.sessions}
      visibleSessionId={props.visibleSubjectId} harnesses={props.harnesses}
      onReviewChanges={() => runCommand("changes", true)}
      onReviewPlan={() => runCommand("review")} onRequestPlanReview={props.onRequestPlanReview}
      resumingSessionIds={props.resumingSessionIds} onResumeSession={props.onResumeSession} onApplyDebugAttention={props.onApplyDebugAttention}
      onOpenSettings={() => props.onOpenSettings("providers")} onOpenRecommendation={props.onOpenRecommendation}
      showDebugMetadata={props.debugSettings.showSessionIds} />;
    if (props.inspector === "actions") return <ActionsOverview key={actionsScope} model={actionsModel} focusRequest={actionFocusRequest.scope === actionsScope ? actionFocusRequest.sequence : 0}
      inspected={actionInspection.scope === actionsScope ? actionInspection.row : null}
      onInspectAction={row => setActionInspection({ scope: actionsScope, row })}
      onFocusHandled={() => setActionFocusRequest(old => ({ ...old, scope: "" }))}
      onOpenSession={id => {
        if (!sourcesReady || (props.sessionsSourceStatus !== "ready" && props.sessionsSourceStatus !== "partial") || !props.sessions.some(s => s.id === id)) return;
        props.onSelectSession(id);
        props.onInspect(null);
        setSessionsPage(false);
        requestFocus("terminal");
      }} onOpenRecommendation={props.onOpenRecommendation} />;
    return <RecommendationsPanel hasWorkspace={!!props.workspace && !props.isSwitchingWorkspace}
      workspacePath={props.workspace?.path ?? null}
      recommendationSource={recommendations}
      taskmasterReady={props.taskmasterReady} canFixWithAi={props.canFixWithAi && isSessionAlive(session)}
      onSelectSession={props.onSelectSession} onFixWithAi={props.onFixWithAi} onRunCleanup={props.onRunCleanup}
      focusedRecommendationId={props.focusedRecommendationId} />;
  };
  const utilityHeader = (temporary: boolean) => <header className="shell-region-header">
    <h2 data-shell-page-heading tabIndex={-1}>{props.inspector ? TITLES[props.inspector] : "Details"}</h2>
    {(props.inspector === "recommendations" || props.inspector === "actions") &&
      <span className="shell-utility-scope">Workspace: {props.workspace?.path ?? "No workspace"}</span>}
    {!temporary && <button type="button" data-shell-pin aria-pressed={!!props.preferences.inspectorPinned}
      aria-label={props.preferences.inspectorPinned ? "Unpin inspector" : "Pin inspector"}
      title={props.preferences.inspectorPinned ? "Unpin inspector" : "Pin inspector"}
      onClick={() => props.onPreferencesChange({ ...props.preferences, inspectorPinned: !props.preferences.inspectorPinned })}>
      {props.preferences.inspectorPinned ? <PinOff size={15} aria-hidden="true" /> : <Pin size={15} aria-hidden="true" />}
    </button>}
    <button type="button" data-shell-return onClick={closePage}
      aria-label={temporary ? undefined : "Close inspector"} title={temporary ? undefined : "Close inspector"}>
      {temporary ? (!props.workspace ? "Back to workspaces" : props.newSessionContent ? "Back to New session" : (reviewing ? "Back to Review" : "Back to Terminal")) : <X size={16} aria-hidden="true" />}
    </button>
  </header>;
  const showRail = !compact && !layout.sessionsVisible;
  const columns = [
    ...(showRail ? ["52px"] : []),
    ...(layout.sessionsVisible ? [`${layout.sessionsWidth}px`, "6px"] : []),
    "minmax(0, 1fr)", ...(showInspector && !layout.inspectorOverlay ? ["6px", `${layout.inspectorWidth}px`] : []),
  ];
  const navigationButtons = (iconOnly = false) => <>
    {props.workspace && <button type="button" data-shell-create aria-label="New session" title={iconOnly ? "New session" : undefined} onClick={props.onCreateSession}><Plus size={16} aria-hidden="true" />{!iconOnly && "New session"}</button>}
    {shouldShowRecommendationsPanel(session) && <button type="button" data-shell-command="recommendations" aria-label="Recommendations" title={iconOnly ? "Recommendations" : undefined} aria-expanded={props.inspector === "recommendations"} onClick={() => runCommand("recommendations")}><Lightbulb size={16} aria-hidden="true" />{!iconOnly && "Recommendations"}</button>}
    <button type="button" data-shell-command="skills" aria-label="Skills" title={iconOnly ? "Skills" : undefined} aria-expanded={skillsPage} onClick={() => runCommand("skills")}><BookOpen size={16} aria-hidden="true" />{!iconOnly && "Skills"}</button>
  </>;
  const workspaceNavigation = <>
    <div className="shell-sidebar-heading">
      <div className="shell-workspace">{props.workspaceControl}</div>
      {showSessions && !compact && <button type="button" data-shell-command="sessions" data-shell-collapse aria-label="Collapse sidebar" title="Collapse sidebar" aria-expanded={true} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={16} aria-hidden="true" /></button>}
    </div>
    <nav className="shell-sidebar-navigation" aria-label="Workspace navigation">
      {showSessionsPage && <p className="shell-actions-counts" data-actions-counts>{actionsModel.sessionCountLabel} · {actionsModel.recommendationCountLabel}</p>}
      {navigationButtons()}
    </nav>
  </>;
  const settingsAndStatus = <>
    <button type="button" data-shell-settings onClick={() => props.onOpenSettings()}><Settings size={16} aria-hidden="true" />Settings</button>
    {props.backendStatus !== "picker" && <span className={`shell-connection ${props.backendStatus === "connected" ? "ok" : "warn"}`}><span aria-hidden="true" />{props.backendStatus}</span>}
  </>;
  return <div ref={rootRef} className="shell-layout" data-mode={layout.mode} data-density={props.preferences.density}
    onKeyDown={event => {
      if (event.defaultPrevented || event.key !== "Escape") return;
      const target = event.target as Element;
      if (target.closest("[role=dialog]")) return;
      if (target.closest(".shell-inspector-host, [data-shell-region=utility]")) {
        event.preventDefault(); event.stopPropagation(); closePage(); return;
      }
      if (!showSessionsPage && !showReview) return;
      // This handler owns document/temporary-page return, never terminal input or modal Escape.
      if ((event.target as Element).closest("[role=dialog], [data-shell-region=terminal], .shell-navigation-menu")) return;
      if ((event.target as Element).closest(".workspace-history-dropdown")?.querySelector(".workspace-history-panel")) return;
      if (showReview && !(event.target as Element).closest("[data-shell-region=review], .shell-toolbar")) return;
      event.preventDefault(); event.stopPropagation();
      if (showReview) closeReviewPage();
      else closePage();
    }}>
    <div className="shell-grid" style={{ gridTemplateColumns: compact ? "minmax(0, 1fr)" : columns.join(" ") }}>
      {showRail && <aside className="shell-sidebar-rail" aria-label="Collapsed sidebar" data-shell-region="rail">
        <nav className="shell-sidebar-navigation" aria-label="Workspace navigation">
          <button type="button" data-shell-command="sessions" aria-label="Expand sidebar" title="Expand sidebar" aria-expanded={false} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={17} aria-hidden="true" /></button>
          {navigationButtons(true)}
        </nav>
        <footer className="shell-sidebar-footer">
          <button type="button" data-shell-settings aria-label="Settings" title="Settings" onClick={() => props.onOpenSettings()}><Settings size={16} aria-hidden="true" /></button>
          {props.backendStatus !== "picker" && <span className={`shell-connection ${props.backendStatus === "connected" ? "ok" : "warn"}`} role="img" aria-label={`Connection: ${props.backendStatus}`} title={`Connection: ${props.backendStatus}`}><span aria-hidden="true" /></span>}
        </footer>
      </aside>}
      {showSessions && <section className="shell-region shell-region--sessions" aria-label="Sessions region" data-shell-region="sessions">
        <div className="shell-sidebar-top">{workspaceNavigation}</div>
        <header className="shell-region-header shell-sessions-header"><h2 data-shell-sessions-heading tabIndex={-1}>Sessions</h2>
          {showSessionsPage && <button type="button" data-shell-command="sessions" data-shell-return onClick={closePage}>{reviewing ? "Back to Review" : "Back to Terminal"}</button>}
        </header>
        <SessionListPanel workspaceGeneration={props.workspaceGeneration} workspace={props.workspace} sessions={props.sessions} activeSessionId={props.activeSessionId}
          unreadIds={props.unreadIds} acknowledgedIds={props.acknowledgedIds} harnesses={props.harnesses}
          onSelectSession={id => {
            creationReturn.current = null;
            props.onSelectSession(id);
          }}
          resumingSessionIds={props.resumingSessionIds}
          actionsEnabled={props.backendStatus === "connected" && !props.isSwitchingWorkspace}
          onSessionAction={props.onSessionAction} onResumeSession={props.onRowResumeSession ?? props.onResumeSession}
          onFocusTerminal={() => runCommand("terminal")} onOpenWorkspace={props.onOpenWorkspace} />
        <footer className="shell-sidebar-footer">{settingsAndStatus}</footer>
      </section>}
      {layout.sessionsVisible && <RegionSeparator label="Sessions" minimum={200} maximum={layout.sessionsMaximum} value={layout.sessionsWidth}
        onChange={value => props.onPreferencesChange({ ...props.preferences, sessionsWidth: value })} />}
      {(!showSessionsPage || !!props.newSessionContent) && <main className="shell-main" hidden={showSessionsPage}>
        <nav className="shell-toolbar" aria-label="Content navigation">
          {compact && <button type="button" data-shell-command="sessions" aria-label="Sessions" title="Sessions" aria-expanded={showSessions} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={17} aria-hidden="true" /></button>}
          {!showSessions && <details key={props.workspaceGeneration} className="shell-navigation-menu" onClick={event => {
            if ((event.target as Element).closest("[data-shell-command], [data-shell-create], [data-shell-settings], .workspace-history-row-open, .workspace-history-other-folder")) {
              event.currentTarget.open = false;
              event.currentTarget.querySelector("summary")?.focus();
            }
          }} onKeyDown={event => {
            if (event.key !== "Escape" || event.currentTarget.querySelector(".workspace-history-panel")) return;
            event.preventDefault(); event.stopPropagation();
            event.currentTarget.open = false;
            event.currentTarget.querySelector("summary")?.focus();
          }}>
            <summary>Workspace<ChevronDown size={14} aria-hidden="true" /></summary>
            <div className="shell-navigation-popover">{workspaceNavigation}<footer className="shell-sidebar-footer">{settingsAndStatus}</footer></div>
          </details>}
          <div className="shell-context"><h1 data-shell-terminal-focus tabIndex={-1}>{showComposer ? "New agent session" : skillsPage ? "Skills" : recommendationsPage ? "Recommendations" : session?.label || "Terminal"}</h1><span className="shell-context-scope">{showComposer ? "Start a session in this workspace" : skillsPage ? "Workspace and personal skills" : recommendationsPage ? "Workspace recommendations" : session?.harness || "No session selected"}</span></div>
          {!showComposer && session && props.workspace && <SessionAppMenu key={`${props.workspaceGeneration}:${session.id}:${sourcesReady}`}
            workspacePath={props.workspace.workspaceIdentity || props.workspace.path} session={session} disabled={!sourcesReady} />}
          {!showComposer && canResume && session && <ResumeChooser key={`${props.workspaceGeneration}:${session.id}`} header
            pending={isResuming} options={resumeChoices(session)} onSelect={option => props.onResumeSession(session.id, option.strategy)} />}
          {session?.hasOpenablePlan && props.onOpenReview && !showComposer && <button type="button" data-shell-command="review"
            aria-pressed={reviewing} disabled={props.reviewLoading || props.backendStatus !== "connected" || props.isSwitchingWorkspace}
            onClick={() => runCommand("review")}>{props.reviewLoading ? "Opening Review…" : "Review"}</button>}
          {(!showTerminal || props.reviewLoading) && <button type="button" data-shell-command="terminal" onClick={() => runCommand("terminal")}>Terminal</button>}
          <div className="shell-actions-controls">
            <button type="button" data-shell-command="actions" aria-expanded={props.inspector === "actions"} onClick={() => runCommand("actions")}>Actions</button>
            <button type="button" data-shell-command="next-action" onClick={() => runCommand("next-action")}>Show next action</button>
            <span data-actions-counts>{actionsModel.sessionCountLabel} · {actionsModel.recommendationCountLabel}</span>
          </div>
          <button type="button" data-shell-command="details" aria-expanded={props.inspector === "details"} onClick={() => runCommand("details")}><PanelRight size={15} aria-hidden="true" />Details</button>
        </nav>
        {props.newSessionContent && <section hidden={!showComposer} className="shell-region shell-region--central shell-composer-region" aria-label="New agent session" data-shell-region="new-session">{props.newSessionContent}</section>}
        {showTerminal && <section className="shell-region shell-region--central" aria-label="Terminal" data-shell-region="terminal">
          <TerminalPanel key={`${session?.id ?? "none"}-${props.resumeTick}`} backendStatus={props.backendStatus} session={session}
            onBackendUnavailable={props.onBackendUnavailable} onRetryBackend={props.onRetryBackend} onSelectPlan={props.onSelectPlan} />
        </section>}
        {showReview && <section className="shell-region shell-region--central" aria-label="Review" data-shell-region="review">
          <header className="shell-region-header">
            <h2 data-shell-review-heading tabIndex={-1}>Review — {session?.label || "Selected session"}</h2>
            <button type="button" data-review-return onClick={closeReviewPage}>Back to Terminal</button>
          </header>
          {props.reviewContent}
        </section>}
        {temporaryUtility && <section className="shell-region shell-region--central" aria-label={centralUtility && props.inspector ? TITLES[props.inspector] : "Temporary content page"} data-shell-region="utility">
          {utilityHeader(true)}{renderUtility()}
        </section>}
      </main>}
      {showInspector && <div className={layout.inspectorOverlay ? "shell-inspector-host shell-inspector-host--overlay" : "shell-inspector-host"}
        style={layout.inspectorOverlay ? { width: layout.inspectorWidth + 6 } : undefined}><RegionSeparator label="Inspector" direction={-1} minimum={280} maximum={layout.inspectorMaximum} value={layout.inspectorWidth}
        onChange={value => props.onPreferencesChange({ ...props.preferences, inspectorWidth: value })} />
        <aside className="shell-region shell-region--inspector" aria-label="Contextual inspector" data-shell-region="inspector" tabIndex={-1}>
          {utilityHeader(false)}{renderUtility()}
        </aside></div>}
    </div>
  </div>;
}
