import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { PanelLeft, PanelRight, Plus, ChevronDown, Gauge, Lightbulb, Settings } from "lucide-react";
import type { SessionAttention, SessionInfo, WorkflowRecommendation, WorkspaceInfo } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import type { DebugSettings } from "../appSettingsTypes";
import type { ShellPreferences } from "../orkworksWindow";
import { computeShellLayout } from "../fixedShellLayout";
import SessionListPanel from "./SessionListPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import CapacityPanel from "./CapacityPanel";
import RecommendationsPanel from "./RecommendationsPanel";
import RegionSeparator from "./RegionSeparator";
import { disposeAllTerminals } from "../terminalStore";

export type ShellDestination = "details" | "capacity" | "recommendations";
export type ShellCommand = "sessions" | "terminal" | "reset-layout" | ShellDestination;
export interface ShellCommandRequest { command: ShellCommand; sequence: number; reveal?: boolean }

export function shouldShowRecommendationsPanel(session: Pick<SessionInfo, "harnessId" | "harness"> | null | undefined): boolean {
  const harnessId = session?.harnessId?.trim() || session?.harness?.trim();
  return Boolean(harnessId) && harnessId !== "generic-shell";
}

interface Props {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  workspaceControl: ReactNode;
  workspaceGeneration: number;
  isSwitchingWorkspace: boolean;
  debugSettings: DebugSettings;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  visibleSubjectId: string | null;
  canFixWithAi: boolean;
  taskmasterReady: boolean;
  focusedRecommendationId: string | null;
  unreadIds: ReadonlySet<string>;
  acknowledgedIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  resumeTick: number;
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
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onForgetSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  onFocusTerminal: () => void;
  onOpenSettings: (section?: "tools" | "providers") => void;
  onOpenWorkspace: () => void;
  onBackendUnavailable: () => void;
  onRetryBackend: () => void;
}

const TITLES = { details: "Details", capacity: "Capacity", recommendations: "Recommendations" };

export default function ApplicationShell(props: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const invokerRef = useRef<HTMLElement | null>(null);
  const invokerSelectorRef = useRef<string | null>(null);
  const navigationIntent = useRef(0);
  const workspaceGenerationRef = useRef(props.workspaceGeneration);
  workspaceGenerationRef.current = props.workspaceGeneration;
  const focusIntent = useRef<"page" | "return" | "terminal" | "sessions" | null>(null);
  const [width, setWidth] = useState(() => window.innerWidth);
  const [sessionsPage, setSessionsPage] = useState(false);
  const [focusTick, setFocusTick] = useState(0);
  const layout = computeShellLayout(width, props.preferences, props.inspector !== null);
  const compact = layout.mode === "compact";
  const showSessionsPage = compact && sessionsPage;
  const temporaryUtility = !showSessionsPage && layout.inspectorPage && props.inspector !== null;
  const showTerminal = !showSessionsPage && !temporaryUtility;
  const showSessions = showSessionsPage || layout.sessionsVisible;
  const showInspector = !showSessionsPage && layout.inspectorWidth > 0 && props.inspector !== null;
  const session = props.sessions.find(s => s.id === props.activeSessionId) ?? null;

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
  const runCommand = (command: ShellCommand, reveal = false, toggleSessions = false) => {
    const intent = ++navigationIntent.current;
    if (command === "reset-layout") {
      const generation = props.workspaceGeneration;
      void props.onResetPreferences().then(completed => {
        if (!completed || navigationIntent.current !== intent || workspaceGenerationRef.current !== generation) return;
        setSessionsPage(false);
        props.onInspect(null);
        requestFocus("terminal");
      });
    } else if (command === "terminal") {
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
      if (command === "recommendations" && !shouldShowRecommendationsPanel(session)) return;
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
    invokerRef.current = null;
    invokerSelectorRef.current = null;
  }, [props.workspaceGeneration]);

  useLayoutEffect(() => {
    const intent = focusIntent.current;
    focusIntent.current = null;
    const root = rootRef.current;
    if (!root) return;
    if (!intent) {
      // A responsive transition may remove the focused page or region.
      if (document.activeElement === document.body) {
        (root.querySelector<HTMLElement>("[data-shell-page-heading]") ?? root.querySelector<HTMLElement>("[data-shell-terminal-focus]"))?.focus({ preventScroll: true });
      }
      return;
    }
    let returnTarget = invokerRef.current?.isConnected && !invokerRef.current.closest("details:not([open])") && invokerRef.current.getClientRects().length
      ? invokerRef.current
      : invokerSelectorRef.current ? root.querySelector<HTMLElement>(invokerSelectorRef.current) : null;
    const closedDisclosure = returnTarget?.closest("details:not([open])");
    if (closedDisclosure) returnTarget = closedDisclosure.querySelector<HTMLElement>("summary");
    if (intent === "return" && returnTarget?.getClientRects().length) {
      returnTarget.focus({ preventScroll: true });
      invokerRef.current = null;
      invokerSelectorRef.current = null;
    } else if (intent === "sessions") {
      document.getElementById("sessions-list")?.focus({ preventScroll: true });
    } else if (intent === "terminal") {
      root.querySelector<HTMLElement>("[data-shell-terminal-focus]")?.focus({ preventScroll: true });
      props.onFocusTerminal();
    } else {
      (root.querySelector<HTMLElement>("[data-shell-page-heading]") ?? root.querySelector<HTMLElement>("[data-shell-terminal-focus]"))?.focus({ preventScroll: true });
    }
  }, [focusTick, showTerminal, showSessionsPage, temporaryUtility, showInspector, props.onFocusTerminal]);

  useEffect(() => {
    // Hidden terminal presentation has no CenterPanel effect to observe backend loss.
    if (props.backendStatus !== "connected") disposeAllTerminals();
  }, [props.backendStatus]);

  useEffect(() => {
    for (const [id, visible] of Object.entries({ sessions: showSessions, terminal: showTerminal,
      detail: props.inspector === "details" && (showInspector || temporaryUtility),
      capacity: props.inspector === "capacity" && (showInspector || temporaryUtility),
      recommendations: props.inspector === "recommendations" && (showInspector || temporaryUtility), review: false })) {
      window.orkworks.notifyPanelVisibility(id, visible);
    }
  }, [showSessions, showTerminal, showInspector, temporaryUtility, props.inspector]);

  const renderUtility = () => {
    if (props.inspector === "details") return <SessionDetailPanel sessions={props.sessions}
      visibleSessionId={props.visibleSubjectId} harnesses={props.harnesses}
      onResumeSession={props.onResumeSession} onApplyDebugAttention={props.onApplyDebugAttention}
      onOpenSettings={() => props.onOpenSettings("providers")} onOpenRecommendation={props.onOpenRecommendation}
      showDebugMetadata={props.debugSettings.showSessionIds} />;
    if (props.inspector === "capacity") return <CapacityPanel />;
    return <RecommendationsPanel hasWorkspace={!!props.workspace && !props.isSwitchingWorkspace}
      taskmasterReady={props.taskmasterReady} canFixWithAi={props.canFixWithAi && session?.lifecycle === "alive"}
      onSelectSession={props.onSelectSession} onFixWithAi={props.onFixWithAi} onRunCleanup={props.onRunCleanup}
      focusedRecommendationId={props.focusedRecommendationId} />;
  };
  const utilityHeader = (temporary: boolean) => <header className="shell-region-header">
    <h2 data-shell-page-heading tabIndex={-1}>{props.inspector ? TITLES[props.inspector] : "Details"}</h2>
    {(props.inspector === "capacity" || props.inspector === "recommendations") &&
      <span className="shell-utility-scope">Workspace: {props.workspace?.path ?? "No workspace"}
        {props.inspector === "capacity" ? " · All coding tools" : ""}</span>}
    <button type="button" data-shell-return onClick={closePage}>{temporary ? "Back to Terminal" : "Close inspector"}</button>
  </header>;
  const columns = [
    ...(layout.sessionsVisible ? [`${layout.sessionsWidth}px`, "6px"] : []),
    "minmax(0, 1fr)", ...(showInspector ? ["6px", `${layout.inspectorWidth}px`] : []),
  ];
  const workspaceNavigation = <>
    <div className="shell-workspace">{props.workspaceControl}</div>
    <nav className="shell-sidebar-navigation" aria-label="Workspace navigation">
      {props.workspace && <button type="button" data-shell-create onClick={props.onCreateSession}><Plus size={16} aria-hidden="true" />New session</button>}
      {shouldShowRecommendationsPanel(session) && <button type="button" data-shell-command="recommendations" aria-expanded={props.inspector === "recommendations"} onClick={() => runCommand("recommendations")}><Lightbulb size={16} aria-hidden="true" />Recommendations</button>}
      <button type="button" data-shell-command="capacity" aria-expanded={props.inspector === "capacity"} onClick={() => runCommand("capacity")}><Gauge size={16} aria-hidden="true" />Capacity</button>
    </nav>
  </>;
  const settingsAndStatus = <>
    <button type="button" data-shell-settings onClick={() => props.onOpenSettings()}><Settings size={16} aria-hidden="true" />Settings</button>
    {props.backendStatus !== "picker" && <span className={`shell-connection ${props.backendStatus === "connected" ? "ok" : "warn"}`}><span aria-hidden="true" />{props.backendStatus}</span>}
  </>;
  return <div ref={rootRef} className="shell-layout" data-mode={layout.mode} data-density={props.preferences.density}
    onKeyDown={event => {
      if (event.key !== "Escape" || (!showSessionsPage && !temporaryUtility)) return;
      // This handler only owns temporary-page navigation, never terminal input or modal Escape.
      if ((event.target as Element).closest("[role=dialog], [data-shell-region=terminal], .shell-navigation-menu")) return;
      if ((event.target as Element).closest(".workspace-history-dropdown")?.querySelector(".workspace-history-panel")) return;
      event.preventDefault(); event.stopPropagation(); closePage();
    }}>
    <div className="shell-grid" style={{ gridTemplateColumns: compact ? "minmax(0, 1fr)" : columns.join(" ") }}>
      {showSessions && <section className="shell-region shell-region--sessions" aria-label="Sessions region" data-shell-region="sessions">
        <div className="shell-sidebar-top">{workspaceNavigation}</div>
        <header className="shell-region-header shell-sessions-header"><h2>Sessions</h2>
          {showSessionsPage && <button type="button" data-shell-command="sessions" data-shell-return onClick={closePage}>Back to Terminal</button>}
        </header>
        <SessionListPanel workspace={props.workspace} sessions={props.sessions} activeSessionId={props.activeSessionId}
          unreadIds={props.unreadIds} acknowledgedIds={props.acknowledgedIds} harnesses={props.harnesses}
          onSelectSession={props.onSelectSession}
          onKillSession={props.onKillSession} onForgetSession={props.onForgetSession}
          onFocusTerminal={() => runCommand("terminal")} onOpenWorkspace={props.onOpenWorkspace} />
        <footer className="shell-sidebar-footer">{settingsAndStatus}</footer>
      </section>}
      {layout.sessionsVisible && <RegionSeparator label="Sessions" minimum={200} maximum={layout.sessionsMaximum} value={layout.sessionsWidth}
        onChange={value => props.onPreferencesChange({ ...props.preferences, sessionsWidth: value })} />}
      {!showSessionsPage && <main className="shell-main">
        <nav className="shell-toolbar" aria-label="Content navigation">
          <button type="button" data-shell-command="sessions" aria-label="Sessions" title="Sessions" aria-expanded={showSessions} onClick={() => runCommand("sessions", false, true)}><PanelLeft size={17} aria-hidden="true" /></button>
          {!showSessions && <details className="shell-navigation-menu" onClick={event => {
            if ((event.target as Element).closest("[data-shell-command], [data-shell-create], [data-shell-settings]")) event.currentTarget.open = false;
          }} onKeyDown={event => {
            if (event.key !== "Escape" || event.currentTarget.querySelector(".workspace-history-panel")) return;
            event.preventDefault(); event.stopPropagation();
            event.currentTarget.open = false;
            event.currentTarget.querySelector("summary")?.focus();
          }}>
            <summary>Workspace<ChevronDown size={14} aria-hidden="true" /></summary>
            <div className="shell-navigation-popover">{workspaceNavigation}<footer className="shell-sidebar-footer">{settingsAndStatus}</footer></div>
          </details>}
          <div className="shell-context"><h1 data-shell-terminal-focus tabIndex={-1}>{session?.label || "Terminal"}</h1><span className="shell-context-scope">{session?.harness || "No session selected"}</span></div>
          {!showTerminal && <button type="button" data-shell-command="terminal" onClick={() => runCommand("terminal")}>Terminal</button>}
          <button type="button" data-shell-command="details" aria-expanded={props.inspector === "details"} onClick={() => runCommand("details")}><PanelRight size={15} aria-hidden="true" />Details</button>
        </nav>
        {showTerminal && <section className="shell-region shell-region--central" aria-label="Terminal" data-shell-region="terminal">
          <TerminalPanel key={`${session?.id ?? "none"}-${props.resumeTick}`} backendStatus={props.backendStatus} session={session}
            onBackendUnavailable={props.onBackendUnavailable} onRetryBackend={props.onRetryBackend} />
        </section>}
        {temporaryUtility && <section className="shell-region shell-region--central" aria-label="Temporary content page" data-shell-region="utility">
          {utilityHeader(true)}{renderUtility()}
        </section>}
      </main>}
      {showInspector && <><RegionSeparator label="Inspector" direction={-1} minimum={280} maximum={layout.inspectorMaximum} value={layout.inspectorWidth}
        onChange={value => props.onPreferencesChange({ ...props.preferences, inspectorWidth: value })} />
        <aside className="shell-region shell-region--inspector" aria-label="Contextual inspector" data-shell-region="inspector">
          {utilityHeader(false)}{renderUtility()}
        </aside></>}
    </div>
  </div>;
}
