import { isSessionAlive } from "../sessionRuntimeState";
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { PanelLeft, PanelRight, Plus, ChevronDown, ArrowRight, Lightbulb, BookOpen, Settings, Pin, PinOff, X } from "lucide-react";
import type {
  CentralSurface,
  FocusTarget,
  ShellCommand,
  ShellCommandHandler,
  ShellDestination,
  ShellNavigationController,
} from "../shellNavigation";
import type { PlanDocumentKind, ResumeStrategy, SessionAction, SessionAttention, SessionInfo, WorkflowRecommendation, WorkspaceInfo } from "../api";
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
import { useCompactPopover } from "../useCompactPopover";
import { useSessionAttentionAlerts } from "../useSessionAttentionAlerts";

export type { ShellCommand, ShellDestination };

export function shouldShowRecommendationsPanel(session: Pick<SessionInfo, "harnessId" | "harness"> | null | undefined): boolean {
  const harnessId = session?.harnessId?.trim() || session?.harness?.trim();
  return Boolean(harnessId) && harnessId !== "generic-shell";
}

export interface ShellWorkspaceProps {
  info: WorkspaceInfo | null;
  control: ReactNode;
  generation: number;
  isSwitching: boolean;
  backendStatus: string;
  onOpen: () => void;
  onBackendUnavailable: () => void;
  onRetryBackend: () => void;
}

export interface ShellSessionsProps {
  items: SessionInfo[];
  sourceStatus?: ActionsSourceStatus;
  alertNavigationBlocked?: boolean;
  activeId: string | null;
  unreadIds: ReadonlySet<string>;
  acknowledgedIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  resumeTick: number;
  resumingIds?: ReadonlySet<string>;
  onSelect: (id: string) => void;
  onAction?: (id: string, action: SessionAction, token: string) => Promise<void>;
  onResume: (id: string, strategy?: ResumeStrategy) => void;
  onRowResume?: (id: string) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  onTitleUpdated?: () => void;
  onFocusTerminal: () => void;
}

export interface ShellNavigationProps {
  controller?: ShellNavigationController;
  centralSurface?: CentralSurface;
  inspector: ShellDestination | null;
  visibleSubjectId: string | null;
  focusTarget?: FocusTarget;
  onInspect: (destination: ShellDestination | null) => void;
  onRegisterCommand?: (handler: ShellCommandHandler) => () => void;
}

export interface ShellReviewProps {
  content?: ReactNode;
  loading?: boolean;
  error?: boolean;
  onOpen?: (kind?: PlanDocumentKind) => void;
  onSelectPlan?: (sessionId: string, path: string) => Promise<void>;
  onRequestPlanReview?: (sessionId: string) => Promise<void>;
  onClose?: () => void;
}

export interface ShellComposerProps {
  content?: ReactNode;
  onCreate: () => void;
  onCancel?: () => void;
}

export interface ShellRecommendationsProps {
  canFixWithAi: boolean;
  taskmasterReady: boolean;
  focusedId: string | null;
  onOpen: (id: string) => void;
  onFixWithAi: (recommendation: WorkflowRecommendation) => void;
  onRunCleanup: (recommendation: WorkflowRecommendation) => void;
}

export interface ShellPreferencesProps {
  value: ShellPreferences;
  debugSettings: DebugSettings;
  desktopSessionNotifications?: boolean;
  onChange: (preferences: ShellPreferences) => void;
  onReset: () => Promise<boolean>;
  onOpenSettings: (section?: "tools" | "providers" | "recommendations") => void;
}

interface Props {
  workspace: ShellWorkspaceProps;
  sessions: ShellSessionsProps;
  navigation: ShellNavigationProps;
  review: ShellReviewProps;
  composer: ShellComposerProps;
  recommendations: ShellRecommendationsProps;
  preferences: ShellPreferencesProps;
}

const TITLES = { changes: "Changes", actions: "Actions", details: "Details", recommendations: "Recommendations", skills: "Skills" };

export default function ApplicationShell(props: Props) {
  const { workspace, sessions, navigation, review, composer, recommendations, preferences } = props;
  const { backendStatus, generation: workspaceGeneration, isSwitching: isSwitchingWorkspace } = workspace;
  const { inspector, centralSurface, focusTarget } = navigation;
  const controller = navigation.controller;

  const rootRef = useRef<HTMLDivElement>(null);
  const navigationMenuRef = useRef<HTMLDetailsElement>(null);
  const navigationPopoverRef = useRef<HTMLDivElement>(null);
  const [navigationMenuOpen, setNavigationMenuOpen] = useState(false);
  const invokerRef = useRef<HTMLElement | null>(null);
  const invokerSelectorRef = useRef<string | null>(null);
  const reviewInvoker = useRef<{ element: HTMLElement | null; selector: string | null } | null>(null);
  const fallbackIntent = useRef(0);
  const creationReturn = useRef<{ inspector: ShellDestination | null; sessionsPage: boolean } | null>(null);
  const selectSession = useCallback((id: string) => {
    creationReturn.current = null;
    sessions.onSelect(id);
  }, [sessions.onSelect]);
  const workspaceGenerationRef = useRef(workspaceGeneration);
  workspaceGenerationRef.current = workspaceGeneration;
  const focusIntent = useRef<"composer" | "page" | "return" | "review-return" | "terminal" | "sessions" | null>(null);
  const [width, setWidth] = useState(() => window.innerWidth);
  const [sessionsPage, setSessionsPage] = useState(false);
  const [toolbarResumeWarningOpen, setToolbarResumeWarningOpen] = useState(false);
  const [detailsResumeWarningOpen, setDetailsResumeWarningOpen] = useState(false);
  const [focusTick, setFocusTick] = useState(0);
  const [actionFocusRequest, setActionFocusRequest] = useState<{ scope: string; sequence: number }>({ scope: "", sequence: 0 });
  const [actionInspection, setActionInspection] = useState<{ scope: string; row: ActionRow | null }>({ scope: "", row: null });
  const actionsScope = `${workspace.info?.workspaceIdentity || workspace.info?.path || ""}:${workspaceGeneration}`;
  const sourcesReady = !!workspace.info && !isSwitchingWorkspace && backendStatus === "connected";
  const recommendationSource = useActionsRecommendations(actionsScope, sourcesReady && recommendations.taskmasterReady, workspace.info?.path ?? "");
  const actionsModel = buildActionsOverview(sourcesReady ? sessions.items : [], sourcesReady ? sessions.sourceStatus ?? "loading" : "unavailable",
    recommendationSource.items, recommendationSource.status);
  const recommendationsPage = inspector === "recommendations";
  const skillsPage = inspector === "skills";
  const changesPage = inspector === "changes";
  const centralUtility = recommendationsPage || skillsPage || changesPage;
  const layout = computeShellLayout(width, preferences.value, inspector !== null && !centralUtility);
  const compact = layout.mode === "compact";
  const showSessionsPage = compact && sessionsPage;
  const temporaryUtility = !showSessionsPage && inspector !== null && (layout.inspectorPage || centralUtility);
  const showComposer = !!composer.content && !showSessionsPage && !temporaryUtility;
  const reviewing = (centralSurface?.kind === "review" && centralSurface.sessionId === sessions.activeId)
    || (!!review.error && !!sessions.activeId);
  const showReview = reviewing && !showSessionsPage && !temporaryUtility && !showComposer;
  const showTerminal = !showSessionsPage && !temporaryUtility && !showComposer && !reviewing;
  const showSessions = showSessionsPage || layout.sessionsVisible;
  useCompactPopover(compact && !showSessions && navigationMenuOpen, navigationMenuRef, navigationPopoverRef, "start");
  useEffect(() => setNavigationMenuOpen(false), [workspaceGeneration, showSessions]);
  const showInspector = !showSessionsPage && !centralUtility && layout.inspectorWidth > 0 && inspector !== null;
  const session = sessions.items.find(s => s.id === sessions.activeId) ?? null;
  const isResuming = !!session && !!sessions.resumingIds?.has(session.id);
  const canResume = session?.lifecycle === "dead" && session.sessionState !== "archived"
    && backendStatus === "connected" && !isSwitchingWorkspace
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

  const beginNavigation = () => controller ? controller.beginNavigationIntent() : ++fallbackIntent.current;
  const isNavigationCurrent = (intent: number, generation: number) => controller
    ? controller.isNavigationIntentCurrent(intent, generation)
    : fallbackIntent.current === intent && workspaceGenerationRef.current === generation;

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
    beginNavigation();
    setSessionsPage(false);
    navigation.onInspect(null);
    if (changesPage) review.onClose?.();
    requestFocus(changesPage ? "terminal" : "return");
  };
  const closeReviewPage = () => {
    invokerRef.current = reviewInvoker.current?.element ?? null;
    invokerSelectorRef.current = reviewInvoker.current?.selector ?? null;
    reviewInvoker.current = null;
    review.onClose?.();
    requestFocus("review-return");
  };
  const runCommand = (command: ShellCommand, reveal = false, toggleSessions = false) => {
    const intent = beginNavigation();
    if (command === "next-action") {
      if (!inspector && !sessionsPage) captureInvoker(command);
      setSessionsPage(false);
      navigation.onInspect("actions");
      setActionFocusRequest(old => ({ scope: actionsScope, sequence: old.sequence + 1 }));
      return;
    }
    if (command === "reset-layout") {
      const generation = workspaceGeneration;
      void preferences.onReset().then(completed => {
        if (!completed || !isNavigationCurrent(intent, generation)) return;
        setSessionsPage(false);
        navigation.onInspect(null);
        requestFocus("terminal");
      });
    } else if (command === "review") {
      if (!session?.hasOpenablePlan || !review.onOpen || backendStatus !== "connected" || isSwitchingWorkspace) return;
      if (!reviewing) {
        captureInvoker(command);
        reviewInvoker.current = { element: invokerRef.current, selector: invokerSelectorRef.current };
      }
      creationReturn.current = null;
      setSessionsPage(false);
      composer.onCancel?.();
      navigation.onInspect(null);
      if (reviewing) requestFocus("page");
      else review.onOpen();
    } else if (command === "new-session") {
      if (!creationReturn.current) {
        creationReturn.current = { inspector, sessionsPage };
        invokerRef.current = rootRef.current?.querySelector<HTMLElement>("[data-shell-create]") ?? null;
        invokerSelectorRef.current = "[data-shell-create]";
      }
      setSessionsPage(false);
      navigation.onInspect(null);
      requestFocus("composer");
    } else if (command === "cancel-new-session") {
      const target = creationReturn.current;
      creationReturn.current = null;
      composer.onCancel?.();
      setSessionsPage(target?.sessionsPage ?? false);
      navigation.onInspect(target?.inspector ?? null);
      requestFocus(target?.sessionsPage ? "sessions" : "return");
    } else if (command === "terminal") {
      review.onClose?.();
      creationReturn.current = null;
      composer.onCancel?.();
      setSessionsPage(false);
      navigation.onInspect(null);
      requestFocus("terminal");
    } else if (command === "sessions") {
      const list = document.getElementById("sessions-list");
      const focused = list?.contains(document.activeElement) === true;
      if (compact) {
        if (sessionsPage && (focused || toggleSessions)) closePage();
        else { captureInvoker(command); setSessionsPage(true); navigation.onInspect(null); requestFocus("sessions"); }
      } else if (layout.sessionsVisible && (focused || toggleSessions)) {
        preferences.onChange({ ...preferences.value, sessionsVisible: false });
        requestFocus(temporaryUtility ? "page" : "terminal");
      } else {
        captureInvoker(command);
        preferences.onChange({ ...preferences.value, sessionsVisible: true });
        requestFocus("sessions");
      }
    } else {
      if (command === "recommendations" && !reveal && !shouldShowRecommendationsPanel(session)) return;
      if (inspector === command && !reveal) { closePage(); return; }
      // Preserve the original invoker when changing utilities inside a temporary page.
      if (!inspector && !sessionsPage) captureInvoker(command);
      if (command === "changes") {
        review.onClose?.();
        composer.onCancel?.();
        creationReturn.current = null;
      }
      setSessionsPage(false);
      navigation.onInspect(command);
      requestFocus("page");
    }
  };
  const commandRef = useRef(runCommand);
  commandRef.current = runCommand;
  useSessionAttentionAlerts({
    scope: sourcesReady && sessions.sourceStatus === "ready" ? actionsScope : null,
    sessions: sessions.items,
    visibleSessionId: showTerminal ? sessions.activeId : null,
    dialogOpen: sessions.alertNavigationBlocked === true || toolbarResumeWarningOpen || detailsResumeWarningOpen,
    desktopEnabled: preferences.desktopSessionNotifications === true,
    openSession: (id) => {
      selectSession(id);
      commandRef.current("terminal");
    },
  });
  useLayoutEffect(() => {
    const dispatch: ShellCommandHandler = (command, reveal = false) => commandRef.current(command, reveal);
    if (controller) return controller.registerCommandHandler(dispatch);
    return navigation.onRegisterCommand?.(dispatch);
  }, [controller, navigation.onRegisterCommand]);

  useEffect(() => {
    setSessionsPage(false);
    creationReturn.current = null;
    invokerRef.current = null;
    invokerSelectorRef.current = null;
    reviewInvoker.current = null;
  }, [workspaceGeneration]);

  useEffect(() => {
    if (!showInspector || preferences.value.inspectorPinned) return;
    const dismissOutsideInspector = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element) || target.closest(`.shell-inspector-host, [role="dialog"], [data-shell-command="${inspector}"]`)) return;
      // Let the clicked surface receive focus and perform its own action.
      beginNavigation();
      navigation.onInspect(null);
    };
    document.addEventListener("pointerdown", dismissOutsideInspector, true);
    return () => document.removeEventListener("pointerdown", dismissOutsideInspector, true);
  }, [showInspector, inspector, preferences.value.inspectorPinned, navigation.onInspect, controller]);

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
    const focusFallback = () => {
      (root.querySelector<HTMLElement>("[data-shell-page-heading]") ?? root.querySelector<HTMLElement>("[data-shell-review-heading]") ?? root.querySelector<HTMLElement>("[data-shell-terminal-focus]") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))?.focus({ preventScroll: true });
    };
    if (intent === "composer" && showComposer) {
      root.querySelector<HTMLElement>("[data-shell-composer-focus]")?.focus({ preventScroll: true });
    } else if ((intent === "return" || intent === "review-return") && returnTarget?.getClientRects().length) {
      returnTarget.focus({ preventScroll: true });
      invokerRef.current = null;
      invokerSelectorRef.current = null;
    } else if (intent === "sessions") {
      (document.getElementById("sessions-list") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))?.focus({ preventScroll: true });
    } else if (intent === "terminal" || intent === "review-return") {
      root.querySelector<HTMLElement>("[data-shell-terminal-focus]")?.focus({ preventScroll: true });
      sessions.onFocusTerminal();
    } else if (intent === "return" && focusTarget) {
      if (focusTarget.kind === "terminal") {
        const terminalFocus = root.querySelector<HTMLElement>("[data-shell-terminal-focus]");
        if (terminalFocus) {
          terminalFocus.focus({ preventScroll: true });
          sessions.onFocusTerminal();
        } else {
          focusFallback();
        }
      } else {
        const targeted = focusTarget.kind === "review"
          ? root.querySelector<HTMLElement>("[data-shell-review-heading]")
          : focusTarget.kind === "sessions"
            ? (document.getElementById("sessions-list") ?? root.querySelector<HTMLElement>("[data-shell-sessions-heading]"))
            : root.querySelector<HTMLElement>("[data-shell-page-heading]");
        if (targeted) targeted.focus({ preventScroll: true });
        else focusFallback();
      }
    } else {
      focusFallback();
    }
  }, [focusTick, showTerminal, showSessionsPage, temporaryUtility, showInspector, focusTarget, sessions.onFocusTerminal]);
  useLayoutEffect(() => {
    if (showComposer) rootRef.current?.querySelector<HTMLElement>("[data-shell-composer-focus]")?.focus({ preventScroll: true });
  }, [showComposer]);
  useLayoutEffect(() => {
    if (reviewing) rootRef.current?.querySelector<HTMLElement>("[data-shell-review-heading]")?.focus({ preventScroll: true });
  }, [reviewing]);
  useLayoutEffect(() => {
    // Retry removes its focused button; recover focus without stealing it from an inspector.
    if (reviewing && !review.error && document.activeElement === document.body)
      rootRef.current?.querySelector<HTMLElement>("[data-shell-review-heading]")?.focus({ preventScroll: true });
  }, [reviewing, review.error]);

  useEffect(() => {
    // Hidden terminal presentation has no CenterPanel effect to observe backend loss.
    if (backendStatus !== "connected") disposeAllTerminals();
  }, [backendStatus]);

  useEffect(() => {
    for (const [id, visible] of Object.entries({ sessions: showSessions, terminal: showTerminal,
      detail: inspector === "details" && (showInspector || temporaryUtility),
      recommendations: inspector === "recommendations" && (showInspector || temporaryUtility), review: showReview })) {
      window.orkworks.notifyPanelVisibility(id, visible);
    }
  }, [showSessions, showTerminal, showInspector, temporaryUtility, inspector, showReview]);

  const renderUtility = () => {
    if (skillsPage) return <SkillsSettings currentWorkspacePath={isSwitchingWorkspace ? null : workspace.info?.workspaceIdentity ?? workspace.info?.path ?? null} />;
    if (inspector === "changes") return <GitChangesPanel key={`${workspaceGeneration}:${sessions.activeId}:${session?.repoRoot ?? ""}:${session?.worktreeBinding?.root ?? ""}:${session?.worktreeBinding?.commonDir ?? ""}`}
      session={sessions.items.find(s => s.id === sessions.activeId) ?? null} />;
    if (inspector === "details") return <SessionDetailPanel key={workspaceGeneration} sessions={sessions.items}
      workspaceIdentity={isSwitchingWorkspace ? undefined : workspace.info?.workspaceIdentity} onTitleUpdated={sessions.onTitleUpdated}
      visibleSessionId={navigation.visibleSubjectId} harnesses={sessions.harnesses}
      onResumeWarningOpenChange={setDetailsResumeWarningOpen}
      onReviewChanges={() => runCommand("changes", true)}
      onReviewPlan={kind => {
        if (!kind) { runCommand("review"); return; }
        if (backendStatus !== "connected" || isSwitchingWorkspace || !review.onOpen) return;
        captureInvoker("review");
        reviewInvoker.current = { element: invokerRef.current, selector: invokerSelectorRef.current };
        setSessionsPage(false);
        composer.onCancel?.();
        if (layout.inspectorPage) navigation.onInspect(null);
        review.onOpen(kind);
      }} onRequestPlanReview={review.onRequestPlanReview}
      resumingSessionIds={sessions.resumingIds} onResumeSession={sessions.onResume} onApplyDebugAttention={sessions.onApplyDebugAttention}
      onOpenSettings={() => preferences.onOpenSettings("providers")} onOpenRecommendation={recommendations.onOpen}
      showDebugMetadata={preferences.debugSettings.showSessionIds} />;
    if (inspector === "actions") return <ActionsOverview key={actionsScope} model={actionsModel} focusRequest={actionFocusRequest.scope === actionsScope ? actionFocusRequest.sequence : 0}
      inspected={actionInspection.scope === actionsScope ? actionInspection.row : null}
      onInspectAction={row => setActionInspection({ scope: actionsScope, row })}
      onFocusHandled={() => setActionFocusRequest(old => ({ ...old, scope: "" }))}
      onOpenSession={id => {
        if (!sourcesReady || (sessions.sourceStatus !== "ready" && sessions.sourceStatus !== "partial") || !sessions.items.some(s => s.id === id)) return;
        sessions.onSelect(id);
        navigation.onInspect(null);
        setSessionsPage(false);
        requestFocus("terminal");
      }} onOpenRecommendation={recommendations.onOpen} />;
    return <RecommendationsPanel hasWorkspace={!!workspace.info && !isSwitchingWorkspace}
      workspacePath={workspace.info?.path ?? null}
      recommendationSource={recommendationSource}
      taskmasterReady={recommendations.taskmasterReady} canFixWithAi={recommendations.canFixWithAi && isSessionAlive(session)}
      onSelectSession={sessions.onSelect} onFixWithAi={recommendations.onFixWithAi} onRunCleanup={recommendations.onRunCleanup}
      onOpenSettings={() => preferences.onOpenSettings("recommendations")}
      focusedRecommendationId={recommendations.focusedId} />;
  };
  const utilityHeader = (temporary: boolean) => <header className={"shell-region-header" + (temporary ? " shell-utility-header" : "")}>
    <h2 data-shell-page-heading tabIndex={-1}>{inspector ? TITLES[inspector] : "Details"}</h2>
    {(inspector === "recommendations" || inspector === "actions") &&
      <span className="shell-utility-scope">Workspace: {workspace.info?.path ?? "No workspace"}</span>}
    {!temporary && <button type="button" data-shell-pin aria-pressed={!!preferences.value.inspectorPinned}
      aria-label={preferences.value.inspectorPinned ? "Unpin inspector" : "Pin inspector"}
      title={preferences.value.inspectorPinned ? "Unpin inspector" : "Pin inspector"}
      onClick={() => preferences.onChange({ ...preferences.value, inspectorPinned: !preferences.value.inspectorPinned })}>
      {preferences.value.inspectorPinned ? <PinOff size={15} aria-hidden="true" /> : <Pin size={15} aria-hidden="true" />}
    </button>}
    <button type="button" data-shell-return onClick={closePage}
      aria-label={temporary ? undefined : "Close inspector"} title={temporary ? undefined : "Close inspector"}>
      {temporary ? (!workspace.info ? "Back to workspaces" : composer.content ? "Back to New session" : (reviewing ? "Back to Review" : "Back to Terminal")) : <X size={16} aria-hidden="true" />}
    </button>
  </header>;
  const showRail = !compact && !layout.sessionsVisible;
  const columns = [
    ...(showRail ? ["52px"] : []),
    ...(layout.sessionsVisible ? [`${layout.sessionsWidth}px`, "6px"] : []),
    "minmax(0, 1fr)", ...(showInspector && !layout.inspectorOverlay ? ["6px", `${layout.inspectorWidth}px`] : []),
  ];
  const navigationButtons = (iconOnly = false) => <>
    {workspace.info && <button type="button" data-shell-create aria-label="New session" title={iconOnly ? "New session" : undefined} onClick={composer.onCreate}><Plus size={16} aria-hidden="true" />{!iconOnly && "New session"}</button>}
    {shouldShowRecommendationsPanel(session) && <button type="button" data-shell-command="recommendations" aria-label="Recommendations" title={iconOnly ? "Recommendations" : undefined} aria-expanded={inspector === "recommendations"} onClick={() => runCommand("recommendations")}><Lightbulb size={16} aria-hidden="true" />{!iconOnly && "Recommendations"}</button>}
    <button type="button" data-shell-command="skills" aria-label="Skills" title={iconOnly ? "Skills" : undefined} aria-expanded={skillsPage} onClick={() => runCommand("skills")}><BookOpen size={16} aria-hidden="true" />{!iconOnly && "Skills"}</button>
  </>;
  const workspaceNavigation = <>
    <div className="shell-sidebar-heading">
      <div className="shell-workspace">{workspace.control}</div>
      {showSessions && !compact && <button type="button" data-shell-command="sessions" data-shell-collapse aria-label="Collapse sidebar" title="Collapse sidebar" aria-expanded={true} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={16} aria-hidden="true" /></button>}
    </div>
    <nav className="shell-sidebar-navigation" aria-label="Workspace navigation">
      {showSessionsPage && <p className="shell-actions-counts" data-actions-counts>{actionsModel.sessionCountLabel} · {actionsModel.recommendationCountLabel}</p>}
      {navigationButtons()}
    </nav>
  </>;
  const settingsAndStatus = <>
    <button type="button" data-shell-settings onClick={() => preferences.onOpenSettings()}><Settings size={16} aria-hidden="true" />Settings</button>
    {backendStatus !== "picker" && <span className={`shell-connection ${backendStatus === "connected" ? "ok" : "warn"}`}><span aria-hidden="true" />{backendStatus}</span>}
  </>;
  return <div ref={rootRef} className="shell-layout" data-mode={layout.mode} data-density={preferences.value.density}
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
          <button type="button" data-shell-settings aria-label="Settings" title="Settings" onClick={() => preferences.onOpenSettings()}><Settings size={16} aria-hidden="true" /></button>
          {backendStatus !== "picker" && <span className={`shell-connection ${backendStatus === "connected" ? "ok" : "warn"}`} role="img" aria-label={`Connection: ${backendStatus}`} title={`Connection: ${backendStatus}`}><span aria-hidden="true" /></span>}
        </footer>
      </aside>}
      {showSessions && <section className="shell-region shell-region--sessions" aria-label="Sessions region" data-shell-region="sessions">
        <div className="shell-sidebar-top">{workspaceNavigation}</div>
        <header className="shell-region-header shell-sessions-header"><h2 data-shell-sessions-heading tabIndex={-1}>Sessions</h2>
          {showSessionsPage && <button type="button" data-shell-command="sessions" data-shell-return onClick={closePage}>{reviewing ? "Back to Review" : "Back to Terminal"}</button>}
        </header>
        <SessionListPanel workspaceGeneration={workspaceGeneration} workspace={workspace.info} sessions={sessions.items} activeSessionId={sessions.activeId}
          unreadIds={sessions.unreadIds} acknowledgedIds={sessions.acknowledgedIds} harnesses={sessions.harnesses}
          onSelectSession={selectSession}
          resumingSessionIds={sessions.resumingIds}
          actionsEnabled={backendStatus === "connected" && !isSwitchingWorkspace}
          onSessionAction={sessions.onAction} onResumeSession={sessions.onRowResume ?? sessions.onResume}
          onFocusTerminal={() => runCommand("terminal")} onOpenWorkspace={workspace.onOpen} />
        <footer className="shell-sidebar-footer">{settingsAndStatus}</footer>
      </section>}
      {layout.sessionsVisible && <RegionSeparator label="Sessions" minimum={200} maximum={layout.sessionsMaximum} value={layout.sessionsWidth}
        onChange={value => preferences.onChange({ ...preferences.value, sessionsWidth: value })} />}
      {(!showSessionsPage || !!composer.content) && <main className="shell-main" hidden={showSessionsPage}>
        <nav className="shell-toolbar" aria-label="Content navigation">
          {compact && <button type="button" data-shell-command="sessions" aria-label="Sessions" title="Sessions" aria-expanded={showSessions} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={17} aria-hidden="true" /></button>}
          {!showSessions && <details ref={navigationMenuRef} key={workspaceGeneration} className="shell-navigation-menu"
          onToggle={event => setNavigationMenuOpen(event.currentTarget.open)} onClick={event => {
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
            <div ref={navigationPopoverRef} className="shell-navigation-popover">{workspaceNavigation}<footer className="shell-sidebar-footer">{settingsAndStatus}</footer></div>
          </details>}
          <div className="shell-context"><h1 data-shell-terminal-focus tabIndex={-1}>{showComposer ? "New agent session" : skillsPage ? "Skills" : recommendationsPage ? "Recommendations" : session?.label || "Terminal"}</h1><span className="shell-context-scope">{showComposer ? "Start a session in this workspace" : skillsPage ? "Workspace and personal skills" : recommendationsPage ? "Workspace recommendations" : session?.harness || "No session selected"}</span></div>
          <div className="shell-session-controls" role="group" aria-label="Session controls">
          {!showComposer && session && workspace.info && <SessionAppMenu key={`${workspaceGeneration}:${session.id}:${sourcesReady}`}
            workspacePath={workspace.info.workspaceIdentity || workspace.info.path} session={session} disabled={!sourcesReady} compact={compact} />}
          {!showComposer && canResume && session && <ResumeChooser key={`${workspaceGeneration}:${session.id}`} header compact={compact}
            pending={isResuming} options={resumeChoices(session)} onWarningOpenChange={setToolbarResumeWarningOpen} onSelect={option => sessions.onResume(session.id, option.strategy)} />}
          {session?.hasOpenablePlan && review.onOpen && !showComposer && <button type="button" data-shell-command="review"
            aria-pressed={reviewing} disabled={review.loading || backendStatus !== "connected" || isSwitchingWorkspace}
            onClick={() => runCommand("review")}>{review.loading ? "Opening Review…" : "Review"}</button>}
          {!showComposer && session && <button type="button" data-shell-command="changes" aria-expanded={changesPage}
            disabled={!(session.repoRoot || session.branch)} onClick={() => runCommand("changes")}>Changes</button>}
          {(!showTerminal || review.loading) && <button type="button" data-shell-command="terminal" onClick={() => runCommand("terminal")}>Terminal</button>}
          </div>
          <div className="shell-context-controls">
          <div className="shell-actions-controls" role="group" aria-label="Workspace actions">
            <button type="button" data-shell-command="actions" aria-label="Actions"
              aria-description={`${actionsModel.sessionCountLabel}; ${actionsModel.recommendationCountLabel}`}
              aria-expanded={inspector === "actions"} onClick={() => runCommand("actions")}>
              <span>Actions</span>
              <span data-actions-counts aria-hidden="true">
                <span data-actions-source="sessions">Attention <strong>{actionsModel.sessionCountValue}</strong></span>
                <span data-actions-source="recommendations">Recommendations <strong>{actionsModel.recommendationCountValue}</strong></span>
              </span>
            </button>
            <button type="button" data-shell-command="next-action" aria-label="Show next action" title="Show next action" onClick={() => runCommand("next-action")}><ArrowRight size={17} aria-hidden="true" /></button>
          </div>
          <button type="button" data-shell-command="details" aria-label="Details" title="Details" aria-expanded={inspector === "details"} onClick={() => runCommand("details")}><PanelRight size={17} aria-hidden="true" /></button>
          </div>
        </nav>
        {composer.content && <section hidden={!showComposer} className="shell-region shell-region--central shell-composer-region" aria-label="New agent session" data-shell-region="new-session">{composer.content}</section>}
        {showTerminal && <section className="shell-region shell-region--central" aria-label="Terminal" data-shell-region="terminal">
          <TerminalPanel key={`${session?.id ?? "none"}-${sessions.resumeTick}`} backendStatus={backendStatus} session={session}
            onBackendUnavailable={workspace.onBackendUnavailable} onRetryBackend={workspace.onRetryBackend} onSelectPlan={review.onSelectPlan} />
        </section>}
        {showReview && <section className="shell-region shell-region--central" aria-label="Review" data-shell-region="review">
          <header className="shell-region-header">
            <h2 data-shell-review-heading tabIndex={-1}>Review — {session?.label || "Selected session"}</h2>
            <button type="button" data-review-return onClick={closeReviewPage}>Back to Terminal</button>
          </header>
          {review.content}
        </section>}
        {temporaryUtility && <section className="shell-region shell-region--central" aria-label={centralUtility && inspector ? TITLES[inspector] : "Temporary content page"} data-shell-region="utility">
          {!skillsPage && !changesPage && utilityHeader(true)}{renderUtility()}
        </section>}
      </main>}
      {showInspector && <div className={layout.inspectorOverlay ? "shell-inspector-host shell-inspector-host--overlay" : "shell-inspector-host"}
        style={layout.inspectorOverlay ? { width: layout.inspectorWidth + 6 } : undefined}><RegionSeparator label="Inspector" direction={-1} minimum={280} maximum={layout.inspectorMaximum} value={layout.inspectorWidth}
        onChange={value => preferences.onChange({ ...preferences.value, inspectorWidth: value })} />
        <aside className="shell-region shell-region--inspector" aria-label="Contextual inspector" data-shell-region="inspector" tabIndex={-1}>
          {utilityHeader(false)}{renderUtility()}
        </aside></div>}
    </div>
  </div>;
}
