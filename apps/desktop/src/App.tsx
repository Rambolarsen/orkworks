import { useCallback, useEffect, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import DockviewApp from "./components/DockviewApp";
import NewSessionDialog from "./components/NewSessionDialog";
import SettingsModal from "./components/SettingsModal";
import ToastRack from "./components/ToastRack";
import { EMPTY_UNREAD_STATE, clearUnread, trackUnread, type UnreadState } from "./sessionUnread";
import { PANEL_DEFAULTS, buildDefaultLayout } from "./components/DockviewApp";
import { VOCAB } from "./labels";
import { pushToast } from "./feedback";
import { activeNewSessionHarnesses } from "./newSessionDialogState";
import {
  type SessionInfo,
  type SessionAttention,
  type WorkspaceInfo,
  type ProviderRuntimeResponse,
  listHarnesses,
  applyDebugAttention,
  saveActiveHarnesses,
  setActiveWorkspaceSession,
  getProviders,
} from "./api";
import { disposeTerminal, getTerminal, pruneTerminals, getLiveTerminalCount, getLiveTerminalIds } from "./terminalStore";
import { captureRendererHealth, type RendererHealthSample } from "./rendererHealthProbe";
import type { AppSettings } from "./appSettingsTypes";
import type { HarnessConfig, CreateSessionOptions } from "./harnessTypes";
import type { BackendLifecycleEvent } from "./orkworksWindow";
import { shouldEnableSessionPolling, type BackendStatus } from "./backendPollingGate";
import { createWorkspaceSessionController } from "./workspaceSessionController";

function App() {
  const [backendStatus, setBackendStatus] = useState<BackendStatus>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [unreadState, setUnreadState] = useState<UnreadState>(EMPTY_UNREAD_STATE);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
  const [isSwitchingWorkspace, setIsSwitchingWorkspace] = useState(false);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [providerRuntime, setProviderRuntime] = useState<ProviderRuntimeResponse | null>(null);
  const [noProvidersPrompt, setNoProvidersPrompt] = useState(false);
  const [resumeTick, setResumeTick] = useState(0);
  const [harnesses, setHarnesses] = useState<HarnessConfig[]>([]);
  const [activeHarnessIds, setActiveHarnessIds] = useState<string[]>([]);
  const [newSessionDialogOpen, setNewSessionDialogOpen] = useState(false);
  const dockviewApiRef = useRef<DockviewApi | null>(null);
  const sessionsHiddenLayoutRef = useRef<string | null>(null);
  const workspaceSessionControllerRef = useRef<ReturnType<typeof createWorkspaceSessionController> | null>(null);
  if (!workspaceSessionControllerRef.current) {
    workspaceSessionControllerRef.current = createWorkspaceSessionController({
      onWorkspace: (info) => {
        setWorkspaceState(info);
        setActiveHarnessIds(info.activeHarnessIds ?? []);
      },
      onSessions: (next) => setSessions([...next]),
      onActiveSession: setActiveSessionId,
      onError: ({ message }) => pushToast("error", message),
      deps: {
        // Controller pruning keeps terminal attachments for sessions whose lifecycle !== "dead".
        pruneTerminals: (ids) => pruneTerminals(ids),
        disposeTerminal,
      },
    });
  }
  const workspaceSessionController = workspaceSessionControllerRef.current;

  const handleBackendLifecycle = useCallback((event: BackendLifecycleEvent) => {
    if (event.state === "ready") {
      setBackendStatus("connected");
    } else if (event.state === "failed") {
      setBackendStatus("unreachable");
    } else if (event.state === "exhausted") {
      setBackendStatus("exhausted");
    } else {
      setBackendStatus("connecting…");
    }
  }, []);

  useEffect(() => window.orkworks.onBackendLifecycle(handleBackendLifecycle), [handleBackendLifecycle]);

  const handleRetryBackend = useCallback(() => {
    setBackendStatus("connecting…");
    void window.orkworks.retryBackend().catch(() => {
      setBackendStatus("unreachable");
    });
  }, []);

  useEffect(() => {
    const intervalMs = settings?.debug?.rendererHealthLogMs ?? 0;
    if (!intervalMs || intervalMs < 1) {
      (window as unknown as { __orkworksCaptureRendererHealth?: unknown }).__orkworksCaptureRendererHealth = undefined;
      return;
    }
    const deps = {
      panelCountProvider: () => {
        const api = dockviewApiRef.current;
        if (!api) return 0;
        try { return api.size; } catch { return 0; }
      },
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
  }, [settings?.debug?.rendererHealthLogMs, dockviewApiRef]);

  useEffect(() => () => workspaceSessionController.dispose(), [workspaceSessionController]);

  useEffect(() => {
    const enabled = shouldEnableSessionPolling(backendStatus, workspace !== null, isSwitchingWorkspace);
    workspaceSessionController.setPollingEnabled(enabled);
    return () => workspaceSessionController.setPollingEnabled(false);
  }, [backendStatus, workspace, isSwitchingWorkspace, workspaceSessionController]);

  useEffect(() => {
    if (backendStatus !== "connecting…") return;
    let cancelled = false;

    async function checkHealth() {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        for (let i = 0; i < 30; i++) {
          try {
            const resp = await fetch(`${baseUrl}/health`);
            if (resp.ok) {
              if (!cancelled) {
                setBackendStatus("connected");
              }
              return;
            }
          } catch {
            await new Promise((r) => setTimeout(r, 500));
          }
        }
        if (!cancelled) setBackendStatus("unreachable");
      } catch {
        if (!cancelled) setBackendStatus("unreachable");
      }
    }

    checkHealth();
    return () => {
      cancelled = true;
    };
  }, [backendStatus]);

  const refreshSessions = useCallback(() => workspaceSessionController.refreshSessions(), [workspaceSessionController]);

  useEffect(() => {
    if (backendStatus !== "connected") return;
    async function loadHarnesses() {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        const list = await listHarnesses(baseUrl);
        setHarnesses(list);
      } catch {
        // Non-fatal: dialog will show empty list, user can still create bare sessions
      }
    }
    loadHarnesses();
  }, [backendStatus]);

  const filteredHarnesses = activeNewSessionHarnesses(harnesses, activeHarnessIds);

  const handleSaveActiveHarnesses = useCallback(async (ids: string[]) => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      await saveActiveHarnesses(baseUrl, ids);
      setActiveHarnessIds(ids);
    } catch {
      pushToast("error", "Couldn't save active harnesses.");
    }
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
      const info = await window.orkworks.openWorkspace();
      if (info) {
        setBackendStatus("connecting…");
        await workspaceSessionController.openWorkspace(info.path);
      }
    } catch {
      pushToast("error", "Couldn't open workspace.");
    } finally {
      setIsSwitchingWorkspace(false);
    }
  }, [workspaceSessionController]);

  useEffect(() => {
    window.orkworks.getSettings().then(setSettings).catch(() => {
      pushToast("error", "Couldn't load app settings.");
    });
  }, []);

  const openSettings = useCallback(async () => {
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

  const handleCreateSession = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const runtime = await getProviders(baseUrl);
      setProviderRuntime(runtime);
    } catch {
      // dialog still opens; provider states just won't show
    }
    setNewSessionDialogOpen(true);
  }, []);

  const handleConfirmNewSession = useCallback(async (opts: CreateSessionOptions) => {
    setNewSessionDialogOpen(false);
    try {
      await workspaceSessionController.createSession(opts);

      const api = dockviewApiRef.current;
      if (api) {
        const panel = api.getPanel("terminal");
        if (panel) panel.api.setActive();
      }
    } catch {
      pushToast("error", "Couldn't start a new session.");
    }
  }, [workspaceSessionController]);

  // Unread ("changed since you looked") is derived by diffing attention
  // status between session snapshots; selecting a session marks it read.
  useEffect(() => {
    setUnreadState((prev) => trackUnread(prev, sessions, activeSessionId));
  }, [sessions, activeSessionId]);

  const handleSelectSession = useCallback((id: string) => {
    setUnreadState((prev) => clearUnread(prev, id));
    workspaceSessionController.selectSession(id);
    const api = dockviewApiRef.current;
    if (api) {
      const panel = api.getPanel("terminal");
      if (panel) panel.api.setActive();
    }
  }, [workspaceSessionController]);

  const handleKillSession = useCallback(
    async (id: string) => {
      try {
        await workspaceSessionController.deleteSession(id, false);
      } catch {
        pushToast("error", "Couldn't end session.");
      }
    },
    [workspaceSessionController],
  );

  const handleForgetSession = useCallback(
    async (id: string) => {
      try {
        await workspaceSessionController.deleteSession(id, true);
      } catch {
        pushToast("error", "Couldn't delete session.");
      }
    },
    [workspaceSessionController],
  );

  const handleFocusTerminal = useCallback(() => {
    if (!activeSessionId) return;
    getTerminal(activeSessionId)?.terminal.focus();
  }, [activeSessionId]);

  const handleReviewPlan = useCallback(() => {
    const api = dockviewApiRef.current;
    if (!api) return;
    const panel = api.getPanel("review") ?? api.addPanel({
      id: "review", component: "review", title: "Review",
      position: { referencePanel: "terminal" },
    });
    panel?.api.setActive();
  }, []);

  useEffect(() => {
    const onSelected = (event: Event) => {
      const sessionId = (event as CustomEvent<{ sessionId?: unknown }>).detail?.sessionId;
      if (typeof sessionId !== "string") return;
      workspaceSessionController.selectSession(sessionId);
      void refreshSessions().then((refreshed) => {
        if (refreshed) handleReviewPlan();
      });
    };
    window.addEventListener("orkworks:terminal-plan-selected", onSelected);
    return () => window.removeEventListener("orkworks:terminal-plan-selected", onSelected);
  }, [handleReviewPlan, refreshSessions]);

  const handleResumeSession = useCallback(async (id: string) => {
    try {
      await workspaceSessionController.resumeSession(id);
      setResumeTick(t => t + 1);
    } catch {
      pushToast("error", "Couldn't resume session.");
    }
  }, [workspaceSessionController]);

  const handleApplyDebugAttention = useCallback(async (id: string, attention: SessionAttention, message?: string) => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      await applyDebugAttention(baseUrl, id, attention, message);
      await refreshSessions();
    } catch {
      pushToast("error", "Couldn't apply debug attention.");
    }
  }, [refreshSessions]);

  useEffect(() => {
    if (backendStatus !== "connected" || workspace) return;
    let cancelled = false;
    async function loadInitialWorkspace() {
      const info = await window.orkworks.getInitialWorkspace();
      if (!cancelled && info) {
        await workspaceSessionController.openWorkspace(info.path);
      }
    }
    loadInitialWorkspace();
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
    if (backendStatus !== "connected" || !activeSessionId) return;
    const sid = activeSessionId;
    async function persistActiveSession() {
      const baseUrl = await window.orkworks.getBackendUrl();
      await setActiveWorkspaceSession(baseUrl, sid);
    }
    persistActiveSession().catch(() => {
      // Silent: backend may not be ready yet on first load; the next active-
      // session change will retry.
    });
  }, [activeSessionId, backendStatus]);

  useEffect(() => {
    return window.orkworks.onMenuCommand(({ action, panelId }) => {
      if (action === "open-settings") {
        openSettings();
        return;
      }

      if (action === "new-session") {
        handleCreateSession();
        return;
      }

      const api = dockviewApiRef.current;
      if (!api) return;

      if (action === "focus" && panelId) {
        const def = PANEL_DEFAULTS[panelId];
        if (!def) return;
        const existing = api.getPanel(def.component);

        if (panelId === "sessions") {
          const focusList = () => {
            setTimeout(() => {
              document.getElementById("sessions-list")?.focus({ preventScroll: true });
            }, 0);
          };
          if (!existing) {
            const snapshot = sessionsHiddenLayoutRef.current;
            if (snapshot) {
              try {
                api.fromJSON(JSON.parse(snapshot));
                sessionsHiddenLayoutRef.current = null;
                focusList();
                return;
              } catch {
                sessionsHiddenLayoutRef.current = null;
              }
            }
            const options: { id: string; component: string; position?: { referencePanel: string; direction?: "below" | "right" | "left" | "above" } } = {
              id: def.component,
              component: def.component,
            };
            if (def.position && api.getPanel(def.position.referencePanel)) {
              const direction = def.position.direction;
              options.position = direction && direction !== "within"
                ? { referencePanel: def.position.referencePanel, direction }
                : { referencePanel: def.position.referencePanel };
            }
            api.addPanel(options);
            focusList();
            return;
          }
          const listEl = document.getElementById("sessions-list");
          const isFocused = !!listEl && listEl.contains(document.activeElement);
          if (isFocused) {
            sessionsHiddenLayoutRef.current = JSON.stringify(api.toJSON());
            existing.api.close();
          } else if (!existing.api.isActive) {
            existing.api.setActive();
            focusList();
          } else {
            focusList();
          }
          return;
        }

        if (existing) {
          existing.api.close();
        } else {
          const options: { id: string; component: string; position?: { referencePanel: string; direction?: "below" | "right" | "left" | "above" } } = {
            id: def.component,
            component: def.component,
          };
          if (def.position && api.getPanel(def.position.referencePanel)) {
            const direction = def.position.direction;
            options.position = direction && direction !== "within"
              ? { referencePanel: def.position.referencePanel, direction }
              : { referencePanel: def.position.referencePanel };
          }
          api.addPanel(options)?.api.setActive();
        }
      } else if (action === "reset-layout") {
        sessionsHiddenLayoutRef.current = null;
        api.clear();
        buildDefaultLayout(api);
      }
    });
  }, [handleCreateSession]);

  return (
    <div className="app-shell">
      <ToastRack />
      <div className="titlebar">
        <div className="titlebar-left">
          {workspace ? (
            <>
              <span
                className="titlebar-text"
                title={workspace.path}
              >
                {workspace.path.split("/").pop() || workspace.path}
              </span>
              <button
                className="titlebar-switch-button"
                type="button"
                onClick={handleOpenWorkspace}
                title={VOCAB.switchWorkspace}
                aria-label={VOCAB.switchWorkspace}
              >
                &#x21C4;
              </button>
            </>
          ) : (
            <>
              <span className="titlebar-text">No workspace</span>
              <button
                className="titlebar-open-button"
                type="button"
                onClick={handleOpenWorkspace}
              >
                {VOCAB.openWorkspace}
              </button>
            </>
          )}
        </div>
        <div className="titlebar-right">
          <span
            className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
          >
            {backendStatus}
          </span>
        </div>
      </div>
      <DockviewApp
        backendStatus={backendStatus}
        workspace={workspace}
        isSwitchingWorkspace={isSwitchingWorkspace}
        debugSettings={settings?.debug ?? { showSessionIds: false, rendererHealthLogMs: 0 }}
        sessions={sessions}
        activeSessionId={activeSessionId}
        unreadIds={unreadState.unreadIds}
        harnesses={harnesses}
        resumeTick={resumeTick}
        onSelectSession={handleSelectSession}
        onCreateSession={handleCreateSession}
        onKillSession={handleKillSession}
        onForgetSession={handleForgetSession}
        onResumeSession={handleResumeSession}
        onApplyDebugAttention={handleApplyDebugAttention}
        onFocusTerminal={handleFocusTerminal}
        onOpenWorkspace={handleOpenWorkspace}
        onReviewPlan={handleReviewPlan}
        dockviewApiRef={dockviewApiRef}
      />
      {(backendStatus === "unreachable" || backendStatus === "exhausted") && (
        <div className="backend-recovery-backdrop" role="alert">
          <section className="backend-recovery-card">
            <h1>Backend unavailable</h1>
            <p>
              {backendStatus === "exhausted"
                ? "OrkWorks could not start its sidecar after several attempts."
                : "OrkWorks lost its connection to the sidecar."}
            </p>
            <button type="button" className="backend-recovery-button" onClick={handleRetryBackend}>
              Retry
            </button>
          </section>
        </div>
      )}
      {newSessionDialogOpen && (
        <NewSessionDialog
          harnesses={filteredHarnesses}
          providerRuntime={providerRuntime}
          onConfirm={handleConfirmNewSession}
          onCancel={() => setNewSessionDialogOpen(false)}
        />
      )}
      {settingsOpen && settings && (
        <SettingsModal
          initialSettings={settings}
          harnesses={harnesses}
          activeHarnessIds={activeHarnessIds}
          providerRuntime={providerRuntime}
          onClose={() => setSettingsOpen(false)}
          onSaved={(nextSettings) => setSettings(nextSettings)}
          onSaveActiveHarnesses={handleSaveActiveHarnesses}
        />
      )}
      {noProvidersPrompt && (
        <div className="settings-backdrop" role="presentation">
          <section className="settings-modal" role="dialog" aria-modal="true">
            <header className="settings-modal-header">
              <h2>No active coding tools</h2>
            </header>
            <div className="settings-section">
              <p>No coding tools are active in this workspace. Open settings to enable at least one.</p>
            </div>
            <footer className="settings-modal-footer">
              <button type="button" onClick={() => setNoProvidersPrompt(false)}>Later</button>
              <button type="button" className="settings-primary-button" onClick={() => { setNoProvidersPrompt(false); openSettings(); }}>Open Settings</button>
            </footer>
          </section>
        </div>
      )}
    </div>
  );
}

export default App;
