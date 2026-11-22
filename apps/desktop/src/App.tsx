import { isUnverifiedAdvice } from './advice.ts';
import { isSessionAlive } from "./sessionRuntimeState";
import orkworksIcon from "../build/icon-dark.svg?no-inline";
import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import ApplicationShell, { type ShellCommand, type ShellDestination } from "./components/ApplicationShell";
import ShellPreferencesNotice from "./components/ShellPreferencesNotice";
import WorkspaceHookNotice from "./components/WorkspaceHookNotice";
import { useShellPreferences } from "./useShellPreferences";
import NewSessionDialog from "./components/NewSessionDialog";
import FixWithAiDialog from "./components/FixWithAiDialog";
import SettingsModal from "./components/SettingsModal";
import type { SettingsSection } from "./components/SettingsModal";
import ToastRack from "./components/ToastRack";
import { WorkspaceHistoryDropdown } from "./components/WorkspaceHistoryDropdown";
import { WorkspaceHistoryList } from "./components/WorkspaceHistoryList";
import {
  EMPTY_UNREAD_STATE,
  acknowledgeSession,
  clearUnread,
  trackUnread,
  type UnreadState,
} from "./sessionUnread";
import { VOCAB } from "./labels";
import { pushToast } from "./feedback";
import { activeNewSessionHarnesses } from "./newSessionDialogState";
import {
  type SessionAction,
  type ResumeStrategy,
  type SessionInfo,
  type SessionAttention,
  type WorkspaceInfo,
  type ProviderRuntimeResponse,
  type HarnessConfigEntry,
  type HarnessListResponse,
  type WorkflowRecommendation,
  listHarnesses,
  applyDebugAttention,
  getProviders,
  acceptTaskmasterRecommendation,
  acceptCleanupRecommendation,
} from "./api";
import { buildRecommendationAcceptOptions, buildFixPromptDraft } from "./taskmaster";
import { readImprovementReference } from "./improvement";
import { handleAcceptedFixWithAi } from "./taskmasterFixHandoff";
import { disposeTerminal, getTerminal, pruneTerminals, getLiveTerminalCount, getLiveTerminalIds } from "./terminalStore";
import { captureRendererHealth, type RendererHealthSample } from "./rendererHealthProbe";
import type { AppSettings } from "./appSettingsTypes";
import type { CreateSessionOptions } from "./harnessTypes";
import type { ActiveHarnessSaveResult, BackendLifecycleEvent, InitialWorkspaceSnapshot, IntegrationKey, WorkspaceHistoryDiagnostic } from "./orkworksWindow";
import type { UpdateStatus } from "./orkworksWindow";
import { shouldEnableSessionPolling, type BackendStatus } from "./backendPollingGate";
import { probeBackendHealth } from "./backendHealthProbe";
import { createBackendRetryGuard } from "./backendRetryGuard";
import { createWorkspaceSessionController } from "./workspaceSessionController";
import { createShellNavigationController, resolveVisibleReviewDocument } from "./shellNavigation";

import ReviewPanel from "./components/ReviewPanel";

function App() {
  const [backendStatus, setBackendStatus] = useState<BackendStatus>("picker");
  const { preferences: shellPreferences, change: changeShellPreferences, reset: resetShellPreferences, needsRebuild: shellNeedsRebuild, migrationNotice: shellMigrationNotice, dismissMigrationNotice } = useShellPreferences();
  const [sessionAdmissionEnabled, setSessionAdmissionEnabled] = useState(false);
  const [sessionsSourceStatus, setSessionsSourceStatus] = useState<"loading" | "ready" | "unavailable">("loading");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const shellNavigationControllerRef = useRef<ReturnType<typeof createShellNavigationController> | null>(null);
  if (!shellNavigationControllerRef.current) {
    shellNavigationControllerRef.current = createShellNavigationController({
      onPlanObserved: (sessionId) => {
        setSessions(previous => previous.map(session => session.id === sessionId
          ? { ...session, hasOpenablePlan: true } : session));
      },
    });
  }
  const shellNavigationController = shellNavigationControllerRef.current;
  const navigationSnapshot = useSyncExternalStore(
    shellNavigationController.subscribe,
    shellNavigationController.getSnapshot,
  );
  const shellNavigation = navigationSnapshot.navigation;
  const review = navigationSnapshot.review;
  const navigationDiagnostic = navigationSnapshot.navigationDiagnostic;
  const navigationRebuilding = navigationSnapshot.navigationRebuilding;
  const requestShellCommand = useCallback((command: ShellCommand, reveal = false) => {
    shellNavigationController.dispatchCommand(command, reveal);
  }, [shellNavigationController]);
  const activeSessionId = shellNavigation.activeSessionId;
  const [unreadState, setUnreadState] = useState<UnreadState>(EMPTY_UNREAD_STATE);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
  const [workspaceHistoryDiagnostic, setWorkspaceHistoryDiagnostic] = useState<WorkspaceHistoryDiagnostic | null>(null);
  const [workspaceSwitchDiagnostic, setWorkspaceSwitchDiagnostic] = useState<string | null>(null);
  const [isSwitchingWorkspace, setIsSwitchingWorkspace] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const saveDesktopSessionNotifications = useCallback(async (enabled: boolean) => {
    await window.orkworks.saveDesktopSessionNotifications(enabled);
    setSettings(current => current && { ...current, desktopSessionNotifications: enabled });
  }, []);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("tools");
  const [updateState, setUpdateState] = useState<{
    status: UpdateStatus | null;
    currentVersion: string | null;
    channel: "latest" | "nightly" | null;
  }>({ status: null, currentVersion: null, channel: null });
  const [providerRuntime, setProviderRuntime] = useState<ProviderRuntimeResponse | null>(null);
  const [noProvidersPrompt, setNoProvidersPrompt] = useState(false);
  const [resumingSessionIds, setResumingSessionIds] = useState<ReadonlySet<string>>(new Set());
  const [resumeTick, setResumeTick] = useState(0);
  const [harnesses, setHarnesses] = useState<HarnessConfigEntry[]>([]);
  const [harnessDocumentRevision, setHarnessDocumentRevision] = useState<string | null>(null);
  const [activeHarnessIds, setActiveHarnessIds] = useState<string[]>([]);
  const [newSessionOpen, setNewSessionOpen] = useState(false);
  const [automaticComposerDismissed, setAutomaticComposerDismissed] = useState(false);
  const composerIntent = useRef(0);
  const cancelNewSession = useCallback(() => {
    composerIntent.current += 1;
    setNewSessionOpen(false);
    setAutomaticComposerDismissed(true);
  }, []);
  const backendRetryGuardRef = useRef(createBackendRetryGuard());
  const workspaceLifecycleRef = useRef({ generation: 0, readyGeneration: null as number | null });
  const initialWorkspaceSnapshotRef = useRef<{
    generation: number;
    promise: Promise<InitialWorkspaceSnapshot>;
  } | null>(null);
  const workspaceSessionControllerRef = useRef<ReturnType<typeof createWorkspaceSessionController> | null>(null);
  if (!workspaceSessionControllerRef.current) {
    workspaceSessionControllerRef.current = createWorkspaceSessionController({
      initialAdmissionEnabled: false,
      onWorkspace: (info) => {
        shellNavigationController.fenceRuntime({ workspace: info });
        setWorkspaceState(info);
        setActiveHarnessIds(info?.activeHarnessIds ?? []);
      },
      onSessionSourceStatus: setSessionsSourceStatus,
      onResumingSessions: setResumingSessionIds,
      onSessions: (next) => {
        const list = [...next];
        shellNavigationController.fenceRuntime({ sessions: list });
        setSessions(list);
      },
      onActiveSession: (sessionId, cause) => {
        shellNavigationController.handleOwnerActiveSession(sessionId, cause);
      },
      onError: ({ message }) => pushToast("error", message),
      deps: {
        // Controller pruning retains only non-historical runtime attachments.
        pruneTerminals: (ids) => pruneTerminals(ids),
        disposeTerminal,
      },
    });
  }
  const workspaceSessionController = workspaceSessionControllerRef.current;

  const handleBackendLifecycle = useCallback((event: BackendLifecycleEvent) => {
    workspaceSessionController.setAdmissionEnabled(event.state === "ready");
    setSessionAdmissionEnabled(event.state === "ready");
    const generation = workspaceLifecycleRef.current.generation + 1;
    workspaceLifecycleRef.current = {
      generation,
      readyGeneration: event.state === "ready" ? generation : workspaceLifecycleRef.current.readyGeneration,
    };
    cancelNewSession();
    setAutomaticComposerDismissed(false);
    setProviderRuntime(null);
    shellNavigationController.handleLifecycleTransition(
      generation,
      event.state,
      event.state === "ready" ? event.workspace : undefined,
    );
    if (event.state === "ready") {
      setIsSwitchingWorkspace(false);
      setWorkspaceSwitchDiagnostic(null);
      workspaceSessionController.setPollingEnabled(false);
      void workspaceSessionController.adoptRestoredWorkspace(event.workspace);
      setWorkspaceHistoryDiagnostic(event.historyDiagnostic);
      setBackendStatus("connected");
    } else if (event.state === "picker") {
      setIsSwitchingWorkspace(false);
      setWorkspaceSwitchDiagnostic(event.failure?.message ?? null);
      workspaceSessionController.setPollingEnabled(false);
      void workspaceSessionController.adoptRestoredWorkspace(null);
      setBackendStatus("picker");
    } else if (event.state === "opening" || event.state === "closing") {
      setIsSwitchingWorkspace(true);
      setBackendStatus("connecting…");
    } else if (event.state === "unresolved") {
      setIsSwitchingWorkspace(false);
      setWorkspaceSwitchDiagnostic(event.failure.message);
      setBackendStatus("unresolved");
    } else if (event.state === "failed") {
      setBackendStatus("unreachable");
    } else if (event.state === "exhausted") {
      setBackendStatus("exhausted");
    } else {
      setBackendStatus("connecting…");
    }
  }, [workspaceSessionController, shellNavigationController, cancelNewSession]);

  useEffect(() => window.orkworks.onBackendLifecycle(handleBackendLifecycle), [handleBackendLifecycle]);

  useEffect(() => window.orkworks.onUpdateStatus((status) => {
    setUpdateState((current) => ({
      status,
      currentVersion: "currentVersion" in status ? status.currentVersion : current.currentVersion,
      channel: "channel" in status
        ? status.channel
        : "candidate" in status && status.candidate
          ? status.candidate.identity.channel
          : current.channel,
    }));
  }), []);

  const handleRetryBackend = useCallback(() => {
    setBackendStatus("connecting…");
    const token = backendRetryGuardRef.current.begin();
    void window.orkworks.retryBackend()
      .then((result) => {
        // A superseded retry (e.g. a rapid double-click) must not clobber a
        // newer retry that is still in flight or already succeeded — see #356.
        if (!backendRetryGuardRef.current.isCurrent(token)) return;
        if (!result.ok) {
          setWorkspaceSwitchDiagnostic(result.failure.message);
          setBackendStatus(result.state);
        }
      })
      .catch(() => {
        // A genuine IPC failure has no lifecycle result to preserve, so it is
        // the only retry failure that maps to unreachable.
        if (backendRetryGuardRef.current.isCurrent(token)) {
          setBackendStatus("unreachable");
        }
      });
  }, []);

  const handleBackendUnavailable = useCallback(() => {
    shellNavigationController.fenceRuntime({ backendConnected: false });
    setBackendStatus("unreachable");
  }, [shellNavigationController]);

  useEffect(() => {
    const intervalMs = settings?.debug?.rendererHealthLogMs ?? 0;
    if (!intervalMs || intervalMs < 1) {
      (window as unknown as { __orkworksCaptureRendererHealth?: unknown }).__orkworksCaptureRendererHealth = undefined;
      return;
    }
    const deps = {
      panelCountProvider: () => document.querySelectorAll("[data-shell-region]").length,
      liveTerminalCountProvider: () => getLiveTerminalCount(),
      liveTerminalIdsProvider: () => getLiveTerminalIds(),
    };
    const timer = window.setInterval(() => {
      const sample = captureRendererHealth(deps);
      console.info("[orkworks:health]", sample);
    }, intervalMs);
    (window as unknown as { __orkworksCaptureRendererHealth?: () => RendererHealthSample }).__orkworksCaptureRendererHealth =
      () => captureRendererHealth(deps);
    return () => {
      window.clearInterval(timer);
      (window as unknown as { __orkworksCaptureRendererHealth?: unknown }).__orkworksCaptureRendererHealth = undefined;
    };
  }, [settings?.debug?.rendererHealthLogMs]);

  useEffect(() => () => {
    workspaceSessionController.dispose();
    shellNavigationController.dispose();
  }, [workspaceSessionController, shellNavigationController]);

  useEffect(() => {
    const enabled = shouldEnableSessionPolling(backendStatus, workspace !== null, isSwitchingWorkspace);
    workspaceSessionController.setPollingEnabled(enabled);
    return () => workspaceSessionController.setPollingEnabled(false);
  }, [backendStatus, workspace, isSwitchingWorkspace, workspaceSessionController]);

  useEffect(() => {
    if (backendStatus !== "connecting…") return;
    let cancelled = false;

    void probeBackendHealth({
      getBackendUrl: () => window.orkworks.getBackendUrl(),
      fetch: (url) => fetch(url),
      delay: (ms) => new Promise((r) => setTimeout(r, ms)),
    }).then((result) => {
      if (!cancelled) setBackendStatus(result);
    });

    return () => {
      cancelled = true;
    };
  }, [backendStatus]);

  const refreshSessions = useCallback(() => workspaceSessionController.refreshSessions(), [workspaceSessionController]);

  const refreshHarnesses = useCallback(async (): Promise<HarnessListResponse> => {
    const baseUrl = await window.orkworks.getBackendUrl();
    const list = await listHarnesses(baseUrl);
    setHarnesses(list.harnesses);
    setHarnessDocumentRevision(list.documentRevision);
    return list;
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected") return;
    void refreshHarnesses().catch(() => {
      // Non-fatal: dialog will show empty list, user can still create bare sessions
    });
  }, [backendStatus, refreshHarnesses]);

  const filteredHarnesses = activeNewSessionHarnesses(harnesses, activeHarnessIds);
  const activeSession = sessions.find((session) => session.id === activeSessionId);
  const reviewEnrichmentPending = activeSession?.enrichment?.state === "pending";
  const visibleReviewDocument = resolveVisibleReviewDocument(
    review.document,
    activeSession,
    shellNavigation.workspaceGeneration,
    backendStatus === "connected",
  );

  useEffect(() => {
    shellNavigationController.syncRuntime({
      backendConnected: backendStatus === "connected",
      admissionEnabled: sessionAdmissionEnabled,
      workspace,
      sessions,
    });
  }, [backendStatus, sessionAdmissionEnabled, workspace, sessions, activeSessionId, shellNavigation.workspaceGeneration, shellNavigationController]);

  const rebuildNavigation = useCallback(() => shellNavigationController.rebuildNavigation(), [shellNavigationController]);
  const requestIndependentReview = useCallback(
    (sessionId: string, kind?: import("./api").PlanDocumentKind, selectionId?: string) =>
      shellNavigationController.requestIndependentReview(sessionId, kind, selectionId),
    [shellNavigationController],
  );
  const openReview = useCallback(
    (kind?: import("./api").PlanDocumentKind) => { void shellNavigationController.openReview(kind); },
    [shellNavigationController],
  );
  const selectTerminalPlan = useCallback(
    (sessionId: string, path: string) => shellNavigationController.selectTerminalPlan(sessionId, path),
    [shellNavigationController],
  );
  const refreshReview = useCallback(
    () => { void shellNavigationController.refreshReview(); },
    [shellNavigationController],
  );
  const closeReview = useCallback(
    () => shellNavigationController.closeReview(),
    [shellNavigationController],
  );

  useEffect(() => {
    if (shellNavigation.visibleFallbackReason) {
      pushToast("info", shellNavigation.visibleFallbackReason);
    }
  }, [shellNavigation.visibleFallbackReason]);

  const handleSaveActiveHarnesses = useCallback(async (ids: string[], scope?: IntegrationKey): Promise<ActiveHarnessSaveResult> => {
    const result = scope
      ? await window.orkworks.enableHarnessIntegrationImmediate(ids, scope.adapterId, scope.targetId)
      : await window.orkworks.saveActiveHarnessesWithIntegrations(ids);
    if (result.activeHarnesses.outcome === "persisted") {
      setActiveHarnessIds(ids);
    }
    return result;
  }, []);

  const handleOpenWorkspace = useCallback(async () => {
    // window.orkworks.openWorkspace() shows a native picker, then (if
    // confirmed) kills the old sidecar and boots a new one on a new port
    // before its own POST /workspace resolves. `workspace` state stays
    // pointed at the OLD workspace for that entire window, so anything
    // gated only on "is a workspace set" (e.g. RecommendationsPanel's
    // polling) would keep hitting the backend and could race a poll tick
    // against the new sidecar's not-yet-set workspace, reproducing the
    // startup 409 via a switch instead. isSwitchingWorkspace covers that
    // whole window regardless of outcome (confirmed, cancelled, or failed).
    setIsSwitchingWorkspace(true);
    try {
      // Adoption is owned by the backend-lifecycle "ready" handler: main
      // publishes the restored workspace with that event, and open-workspace
      // resolves with the same restoration. Adopting here as well would run
      // the adoption twice — a second /sessions fetch that clears and
      // repopulates the just-adopted list (the issue #357 flash).
      await window.orkworks.openWorkspace();
    } catch {
      pushToast("error", "Couldn't open workspace.");
    } finally {
      setIsSwitchingWorkspace(false);
    }
  }, []);

  const handleOpenRememberedWorkspace = useCallback(async (path: string) => {
    setIsSwitchingWorkspace(true);
    try {
      await window.orkworks.openRememberedWorkspace(path);
    } catch {
      pushToast("error", "Couldn't open workspace.");
    } finally {
      setIsSwitchingWorkspace(false);
    }
  }, []);

  const handleWorkspaceHistoryError = useCallback((message: string) => {
    pushToast("error", message);
  }, []);

  useEffect(() => {
    window.orkworks.getSettings().then(setSettings).catch(() => {
      pushToast("error", "Couldn't load app settings.");
    });
  }, []);

  const openSettings = useCallback(async (section: SettingsSection = "tools") => {
    setSettingsSection(section);
    try {
      const loaded = await window.orkworks.getSettings();
      setSettings(loaded);
      setSettingsOpen(true);
    } catch {
      pushToast("error", "Couldn't open settings.");
    }
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const runtime = await getProviders(baseUrl);
      setProviderRuntime(runtime);
    } catch {
      // Settings are already open; provider runtime will be null
    }
  }, []);

  const checkForUpdates = useCallback(async () => {
    try {
      await window.orkworks.checkForUpdates();
    } catch {
      pushToast("error", "Couldn't check for updates.");
    }
  }, []);

  const downloadUpdate = useCallback(async () => {
    try {
      await window.orkworks.downloadUpdate();
    } catch {
      pushToast("error", "Couldn't download the update.");
    }
  }, []);

  const handleCreateSession = useCallback(async () => {
    if (!workspaceSessionController.isAdmissionEnabled()) return;
    composerIntent.current += 1;
    setNewSessionOpen(true);
    requestShellCommand("new-session");
  }, [workspaceSessionController, requestShellCommand]);

  const showNewSession = sessionAdmissionEnabled && !isSwitchingWorkspace && !!workspace &&
    (newSessionOpen || (sessionsSourceStatus !== "loading" && activeSessionId === null && !automaticComposerDismissed));

  useEffect(() => {
    if (!showNewSession) return;
    let cancelled = false;
    setProviderRuntime(null);
    void window.orkworks.getBackendUrl().then(getProviders).then(runtime => {
      if (!cancelled) setProviderRuntime(runtime);
    }).catch(() => {});
    return () => { cancelled = true; };
  }, [showNewSession, shellNavigation.workspaceGeneration]);

  const handleConfirmNewSession = useCallback(async (opts: CreateSessionOptions) => {
    const intent = composerIntent.current;
    try {
      const created = await workspaceSessionController.createSession(opts, () => intent === composerIntent.current);
      if (!created || intent !== composerIntent.current) return;
      cancelNewSession();
      requestShellCommand("terminal");
      void shellNavigationController.saveNavigation("terminal");
    } catch (error) {
      if (opts.initialImages?.length) throw error;
      pushToast("error", "Couldn't start a new session.");
    }
  }, [workspaceSessionController, requestShellCommand, cancelNewSession, shellNavigationController]);

  const [fixRequest, setFixRequest] = useState<{
    recommendation: WorkflowRecommendation;
    sessionId: string;
    generation: number;
  } | null>(null);
  const fixRecommendation = fixRequest?.recommendation ?? null;

  const handleSelectSession = useCallback((id: string) => {
    if (!workspaceSessionController.selectSession(id)) return;
    cancelNewSession();
    shellNavigationController.selectSession(id);
    setUnreadState((prev) => acknowledgeSession(clearUnread(prev, id), id));
  }, [workspaceSessionController, cancelNewSession, shellNavigationController]);

  const handleFixWithAi = useCallback((recommendation: WorkflowRecommendation) => {
    if (!workspaceSessionController.isAdmissionEnabled()) return;
    const activeSession = sessions.find((session) => session.id === activeSessionId);
    if (!isSessionAlive(activeSession)) return;
    try { buildFixPromptDraft(recommendation); } catch {
      pushToast("error", "This recommendation uses an unsupported handoff.");
      return;
    }
    setFixRequest({ recommendation, sessionId: activeSession.id, generation: workspaceLifecycleRef.current.generation });
  }, [activeSessionId, sessions, workspaceSessionController]);

  const handleRunCleanup = useCallback(async (recommendation: WorkflowRecommendation) => {
    try {
      const result = await acceptCleanupRecommendation(recommendation.id, recommendation.workspaceId);
      if (result.skipped.length === 0) {
        pushToast("info", "Cleanup complete — flagged recommendations dismissed.");
        return;
      }
      const skippedText = result.skipped
        .slice(0, 5)
        .map((entry) => `${entry.id.slice(0, 8)} is now ${entry.status}`)
        .join("; ");
      pushToast("info", `Cleanup complete. Skipped ${result.skipped.length} changed entr${result.skipped.length === 1 ? "y" : "ies"}: ${skippedText}${result.skipped.length > 5 ? "…" : ""}`);
    } catch (cause) {
      pushToast("error", "Couldn't run the cleanup.");
      throw cause;
    }
  }, []);

  const handleConfirmFixWithAi = useCallback(async (prompt: string) => {
    setFixRequest(null);
    if (!fixRequest || !activeSessionId) {
      pushToast("error", "Couldn't send the fix — no session is active.");
      return;
    }
    if (fixRequest.sessionId !== activeSessionId
        || fixRequest.generation !== workspaceLifecycleRef.current.generation) {
      pushToast("error", "The session for this fix changed. Reopen the fix to continue.");
      return;
    }
    const { recommendation, sessionId } = fixRequest;
    const activeSession = sessions.find((session) => session.id === sessionId);
    if (!isSessionAlive(activeSession) || !workspaceSessionController.isAdmissionEnabled()) {
      pushToast("error", "Couldn't send the fix — no session is active.");
      return;
    }
    const admissionToken = workspaceSessionController.captureAdmission();
    const lifecycleGeneration = workspaceLifecycleRef.current.generation;
    const isCurrentHandoff = () => workspaceSessionController.isAdmissionCurrent(admissionToken ?? -1)
      && workspaceLifecycleRef.current.generation === lifecycleGeneration;
    try {
      if (!await workspaceSessionController.submitActiveSession(sessionId, admissionToken)) return;
      if (!isCurrentHandoff()) return;
      const acceptedRecommendation = await acceptTaskmasterRecommendation(recommendation.id,
        buildRecommendationAcceptOptions(recommendation, sessionId, prompt), recommendation.workspaceId);
      if (!isCurrentHandoff()) return;
      handleAcceptedFixWithAi(
        acceptedRecommendation,
        sessions,
        handleSelectSession,
        (message) => pushToast("info", message),
      );
    } catch {
      pushToast("error", "Couldn't send the fix to the session.");
    }
  }, [fixRequest, activeSessionId, sessions, workspaceSessionController, handleSelectSession]);

  // Unread ("changed since you looked") is derived by diffing attention
  // status between session snapshots; selecting a session marks it read.
  useEffect(() => {
    setUnreadState((prev) => trackUnread(prev, sessions, activeSessionId));
  }, [sessions, activeSessionId]);

  const handleSessionAction = useCallback(
    (id: string, action: SessionAction, token: string) => workspaceSessionController.sessionAction(id, action, token),
    [workspaceSessionController],
  );

  const handleFocusTerminal = useCallback(() => {
    if (!activeSessionId) return;
    getTerminal(activeSessionId)?.terminal.focus();
  }, [activeSessionId]);

  const [focusedRecommendationId, setFocusedRecommendationId] = useState<string | null>(null);

  const handleOpenRecommendation = useCallback((id: string) => {
    setFocusedRecommendationId(id);
    requestShellCommand("recommendations", true);
  }, [requestShellCommand]);

  const handleInspect = useCallback((destination: ShellDestination | null) => {
    shellNavigationController.inspect(destination);
  }, [shellNavigationController]);

  const handleResumeSession = useCallback(async (id: string, strategy?: ResumeStrategy) => {
    try {
      await workspaceSessionController.resumeSession(id, strategy);
      setResumeTick(t => t + 1);
    } catch {
      pushToast("error", "Couldn't resume session.");
    }
  }, [workspaceSessionController]);

  const handleRowResumeSession = useCallback((id: string) => {
    const target = sessions.find(session => session.id === id);
    if (!target || target.sessionState === "archived") return;
    if (target.resumeStrategy === "exact") void handleResumeSession(id, "exact");
    else {
      handleSelectSession(id);
      requestShellCommand("details", true);
    }
  }, [sessions, handleResumeSession, handleSelectSession, requestShellCommand]);

  const handleApplyDebugAttention = useCallback(async (id: string, attention: SessionAttention, message?: string) => {
    if (!workspaceSessionController.isAdmissionEnabled()) return;
    const admissionToken = workspaceSessionController.captureAdmission();
    try {
      await applyDebugAttention(id, attention, message);
      if (!workspaceSessionController.isAdmissionCurrent(admissionToken ?? -1)) return;
      await refreshSessions();
    } catch {
      pushToast("error", "Couldn't apply debug attention.");
    }
  }, [refreshSessions, workspaceSessionController]);

  useEffect(() => {
    let cancelled = false;
    async function loadInitialWorkspace() {
      try {
        const snapshotRequest = initialWorkspaceSnapshotRef.current ?? (() => {
          const request = {
            generation: workspaceLifecycleRef.current.generation,
            promise: window.orkworks.getInitialWorkspace(),
          };
          initialWorkspaceSnapshotRef.current = request;
          return request;
        })();
        const snapshot = await snapshotRequest.promise;
        if (cancelled) return;
        const lifecycle = workspaceLifecycleRef.current;
        if (
          lifecycle.generation !== snapshotRequest.generation
          || (lifecycle.readyGeneration !== null && lifecycle.readyGeneration >= snapshotRequest.generation)
        ) return;
        setWorkspaceHistoryDiagnostic(snapshot.historyDiagnostic);
        if (backendStatus === "connected" && !workspace && snapshot.workspace) {
          await workspaceSessionController.adoptRestoredWorkspace(snapshot.workspace);
        }
      } catch {
        // Lifecycle events remain the source of truth if the snapshot is
        // unavailable during startup.
      }
    }
    void loadInitialWorkspace();
    return () => {
      cancelled = true;
    };
  }, [backendStatus, workspace, workspaceSessionController]);

  useEffect(() => {
    if (backendStatus !== "connected" || !workspace || !settings) return;
    if (settingsOpen) return;
    if (activeHarnessIds.length === 0) {
      setNoProvidersPrompt(true);
    }
  }, [backendStatus, workspace, settings, settingsOpen, activeHarnessIds]);

  useEffect(() => {
    if (backendStatus !== "connected" || !sessionAdmissionEnabled || !activeSessionId) return;
    const sid = activeSessionId;
    workspaceSessionController.submitActiveSession(sid).catch(() => {
      // Silent: backend may not be ready yet on first load; the next active-
      // session change will retry.
    });
  }, [activeSessionId, backendStatus, sessionAdmissionEnabled, workspaceSessionController]);

  useEffect(() => window.orkworks.onMenuCommand(({ action, panelId }) => {
    if (action === "check-for-updates") {
      void openSettings("updates");
      void checkForUpdates();
      return;
    }
    if (action === "open-settings") void openSettings();
    else if (action === "new-session") void handleCreateSession();
    else if (action === "reset-layout") requestShellCommand("reset-layout");
    else if (action === "focus" && panelId) {
      const destination = panelId === "detail" ? "details" : panelId;
      if (["sessions", "terminal", "review", "details", "recommendations", "actions", "next-action"].includes(destination)) {
        requestShellCommand(destination as ShellCommand);
      }
    }
  }), [handleCreateSession, openSettings, checkForUpdates, requestShellCommand]);

  const workspaceControl = <div className="shell-workspace-control">
  {workspace ? (
    <>
      <span
        className="titlebar-text"
        title={workspace.path}
      >
        {workspace.path.split("/").pop() || workspace.path}
      </span>
      <WorkspaceHistoryDropdown
        triggerLabel="⇄"
        triggerClassName="titlebar-switch-button"
        triggerAriaLabel={VOCAB.switchWorkspace}
        currentWorkspacePath={workspace?.workspaceIdentity ?? workspace?.path ?? null}
        isSwitching={isSwitchingWorkspace}
        onOpenPath={handleOpenRememberedWorkspace}
        onOpenOtherFolder={handleOpenWorkspace}
        onError={handleWorkspaceHistoryError}
      />
    </>
  ) : (
    <span className="titlebar-text">No workspace</span>
  )}
  </div>;

  return (
    <div className="app-shell">
      <ToastRack />
      <div className="titlebar">
        <div className="titlebar-left">
          <img className="titlebar-icon" src={orkworksIcon} alt="" draggable={false} />
          <span className="titlebar-text">OrkWorks</span>
        </div>
        {workspace && workspaceHistoryDiagnostic && (
          <span className="titlebar-history-warning" role="alert" title={workspaceHistoryDiagnostic.message}>
            Workspace history unavailable
          </span>
        )}
      </div>
      {!workspace && shellNavigation.inspector !== "skills" && (
        <div className="workspace-picker-screen">
          <img className="workspace-picker-watermark" src={orkworksIcon} alt="" aria-hidden="true" draggable={false} />
          <section className="workspace-picker-card" aria-label="Workspace picker">
            {backendStatus !== "picker" && <p role="status">{backendStatus}</p>}
            {workspaceHistoryDiagnostic && (
              <span role="alert" title={workspaceHistoryDiagnostic.message}>
                Workspace history unavailable
              </span>
            )}
            {workspaceSwitchDiagnostic && (
              <span role="alert" title={workspaceSwitchDiagnostic}>
                Workspace switch needs attention
              </span>
            )}
            {workspaceSwitchDiagnostic && (
              <button
                className="titlebar-open-button"
                type="button"
                onClick={handleRetryBackend}
              >
                Retry
              </button>
            )}
            <WorkspaceHistoryList
              currentWorkspacePath={null}
              isSwitching={isSwitchingWorkspace}
              onOpenPath={handleOpenRememberedWorkspace}
              onOpenOtherFolder={handleOpenWorkspace}
              onError={handleWorkspaceHistoryError}
            />
          </section>
        </div>
      )}
      {workspace && backendStatus === "connected" && sessionAdmissionEnabled && !isSwitchingWorkspace && (
        <WorkspaceHookNotice
          key={shellNavigation.workspaceGeneration}
          activeHarnessIds={activeHarnessIds}
          settingsOpen={settingsOpen}
          onOpenSettings={() => void openSettings("tools")}
        />
      )}
      <ShellPreferencesNotice migrationNotice={shellMigrationNotice} needsRebuild={shellNeedsRebuild}
        dismissMigrationNotice={dismissMigrationNotice} onCommand={requestShellCommand} />
      {navigationDiagnostic && <div className="shell-preferences-notice" role="alert">
        <span>{navigationDiagnostic === "corrupt_record" || navigationDiagnostic === "unsupported_version"
          ? "Saved navigation could not be read. The current view remains available; rebuild requires confirmation."
          : "Navigation could not be saved or restored. The current view remains available."}</span>
        {(navigationDiagnostic === "corrupt_record" || navigationDiagnostic === "unsupported_version") &&
          <button type="button" data-rebuild-navigation disabled={navigationRebuilding} onClick={() => void rebuildNavigation()}>Rebuild navigation…</button>}
      </div>}
      <ApplicationShell
        workspace={{
          info: workspace,
          control: workspaceControl,
          generation: shellNavigation.workspaceGeneration,
          isSwitching: isSwitchingWorkspace,
          backendStatus,
          onOpen: handleOpenWorkspace,
          onBackendUnavailable: handleBackendUnavailable,
          onRetryBackend: handleRetryBackend,
        }}
        sessions={{
          items: sessions,
          sourceStatus: sessionsSourceStatus,
          alertNavigationBlocked: settingsOpen || fixRequest !== null || noProvidersPrompt,
          activeId: activeSessionId,
          unreadIds: unreadState.unreadIds,
          acknowledgedIds: unreadState.acknowledgedIds,
          harnesses,
          resumeTick,
          resumingIds: resumingSessionIds,
          onSelect: handleSelectSession,
          onAction: handleSessionAction,
          onResume: handleResumeSession,
          onRowResume: handleRowResumeSession,
          onApplyDebugAttention: handleApplyDebugAttention,
          onTitleUpdated: () => { void refreshSessions(); },
          onFocusTerminal: handleFocusTerminal,
        }}
        navigation={{
          controller: shellNavigationController,
          centralSurface: shellNavigation.centralSurface,
          inspector: shellNavigation.inspector,
          visibleSubjectId: navigationSnapshot.visibleSubjectId,
          focusTarget: shellNavigation.focusTarget,
          onInspect: handleInspect,
        }}
        review={{
          content: activeSessionId ? <ReviewPanel key={`${shellNavigation.workspaceGeneration}-${activeSessionId}-${visibleReviewDocument?.kind ?? "legacy"}`}
            sessionId={activeSessionId} content={visibleReviewDocument?.content ?? null} loading={reviewEnrichmentPending || review.loading || isSwitchingWorkspace}
            error={!reviewEnrichmentPending && review.error} changed={review.changed} canRequest={sessionAdmissionEnabled && isSessionAlive(activeSession)}
            onRefresh={refreshReview} onRequest={id => requestIndependentReview(id, visibleReviewDocument?.kind, visibleReviewDocument?.selectionId)} /> : null,
          loading: review.loading,
          error: review.error,
          onOpen: openReview,
          onSelectPlan: selectTerminalPlan,
          onRequestPlanReview: requestIndependentReview,
          onClose: closeReview,
        }}
        composer={{
          content: showNewSession ? <NewSessionDialog
            workspacePath={workspace?.workspaceIdentity ?? ""}
            key={shellNavigation.workspaceGeneration}
            harnesses={filteredHarnesses} providerRuntime={providerRuntime}
            onConfirm={handleConfirmNewSession} onCancel={() => requestShellCommand("cancel-new-session")} /> : null,
          onCreate: handleCreateSession,
          onCancel: cancelNewSession,
        }}
        recommendations={{
          canFixWithAi: sessionAdmissionEnabled && isSessionAlive(activeSession),
          taskmasterReady: sessionAdmissionEnabled,
          focusedId: focusedRecommendationId,
          onOpen: handleOpenRecommendation,
          onFixWithAi: handleFixWithAi,
          onRunCleanup: handleRunCleanup,
        }}
        preferences={{
          value: shellPreferences,
          debugSettings: settings?.debug ?? { showSessionIds: false, rendererHealthLogMs: 0 },
          desktopSessionNotifications: settings?.desktopSessionNotifications,
          onChange: changeShellPreferences,
          onReset: resetShellPreferences,
          onOpenSettings: section => openSettings(section ?? "tools"),
        }}
      />
      {(backendStatus === "unreachable" || backendStatus === "exhausted" || backendStatus === "unresolved") && (
        <div className="backend-recovery-backdrop" role="alert">
          <section className="backend-recovery-card">
            <h1>{backendStatus === "unresolved" ? "Workspace cleanup unresolved" : "Backend unavailable"}</h1>
            <p>
              {workspaceSwitchDiagnostic ?? (backendStatus === "exhausted"
                ? "OrkWorks could not start its sidecar after several attempts."
                : "OrkWorks lost its connection to the sidecar.")}
            </p>
            <button type="button" className="backend-recovery-button" onClick={handleRetryBackend}>
              Retry
            </button>
          </section>
        </div>
      )}
      {fixRecommendation && (
        <FixWithAiDialog
          initialPrompt={[2,3].includes(readImprovementReference(fixRecommendation.assessment)?.version ?? 0) || (fixRecommendation.assessment?.improvementReference?.version === 1 && fixRecommendation.assessment.improvementReference.intent === "investigation")
            ? buildFixPromptDraft(fixRecommendation)
            : fixRecommendation.completionPacket?.action.prompt ?? buildFixPromptDraft(fixRecommendation)}
          promptReadOnly={[2,3].includes(readImprovementReference(fixRecommendation.assessment)?.version ?? 0) || Boolean(fixRecommendation.completionPacket) || fixRecommendation.assessment?.improvementReference?.intent === "investigation"}
          title={fixRecommendation.assessment?.improvementReference?.intent === "investigation" ? "Investigate with AI" : "Fix with AI"}
          confirmLabel={fixRecommendation.assessment?.improvementReference?.intent === "investigation" ? "Start investigation" : "Send to session"}
          unverifiedAdvice={isUnverifiedAdvice(fixRecommendation)}
          onConfirm={handleConfirmFixWithAi}
          onCancel={() => setFixRequest(null)}
        />
      )}
      {settingsOpen && settings && (
        <SettingsModal
          initialSection={settingsSection}
          initialSettings={settings}
          currentWorkspacePath={workspace?.workspaceIdentity ?? workspace?.path ?? null}
          updateStatus={updateState.status}
          updateCurrentVersion={updateState.currentVersion}
          updateChannel={updateState.channel}
          onCheckForUpdates={() => void checkForUpdates()}
          onDownloadUpdate={() => void downloadUpdate()}
          harnesses={harnesses}
          documentRevision={harnessDocumentRevision}
          onRefreshHarnesses={refreshHarnesses}
          activeHarnessIds={activeHarnessIds}
          providerRuntime={providerRuntime}
          onSectionChange={setSettingsSection}
          onClose={() => setSettingsOpen(false)}
          onSaved={(nextSettings) => setSettings(current => ({ ...nextSettings, desktopSessionNotifications: current?.desktopSessionNotifications }))}
          onSaveActiveHarnesses={handleSaveActiveHarnesses}
          onSaveDesktopSessionNotifications={saveDesktopSessionNotifications}
        />
      )}
      {noProvidersPrompt && (
        <div className="new-session-backdrop" role="presentation">
          <section className="new-session-dialog" role="dialog" aria-modal="true" aria-labelledby="no-coding-tools-title" aria-describedby="no-coding-tools-description">
            <header className="new-session-header">
              <h2 id="no-coding-tools-title">No active coding tools</h2>
            </header>
            <div className="new-session-body">
              <p id="no-coding-tools-description">No coding tools are active in this workspace. Open settings to enable at least one.</p>
            </div>
            <footer className="new-session-footer">
              <button type="button" className="new-session-cancel" onClick={() => setNoProvidersPrompt(false)}>Later</button>
              <button type="button" className="new-session-confirm" onClick={() => { setNoProvidersPrompt(false); openSettings(); }}>Open Settings</button>
            </footer>
          </section>
        </div>
      )}
    </div>
  );
}

export default App;
