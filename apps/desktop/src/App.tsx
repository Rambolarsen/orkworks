import { useCallback, useEffect, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import DockviewApp from "./components/DockviewApp";
import NewSessionDialog from "./components/NewSessionDialog";
import SettingsModal from "./components/SettingsModal";
import ToastRack from "./components/ToastRack";
import { mergeSessionsById } from "./sessionSort";
import { EMPTY_UNREAD_STATE, clearUnread, trackUnread, type UnreadState } from "./sessionUnread";
import { PANEL_DEFAULTS, buildDefaultLayout } from "./components/DockviewApp";
import { VOCAB } from "./labels";
import { pushToast } from "./feedback";
import {
  type SessionInfo,
  type WorkspaceInfo,
  type ProviderRuntimeResponse,
  createSession,
  listHarnesses,
  listSessions,
  deleteSession,
  forgetSession,
  resumeSession,
  saveActiveHarnesses,
  setActiveWorkspaceSession,
  getProviders,
} from "./api";
import { disposeTerminal, getTerminal, pruneTerminals } from "./terminalStore";
import type { AppSettings } from "./appSettingsTypes";
import type { HarnessConfig, CreateSessionOptions } from "./harnessTypes";

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [unreadState, setUnreadState] = useState<UnreadState>(EMPTY_UNREAD_STATE);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
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

  const refreshSessions = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const list = await listSessions(baseUrl);
      pruneTerminals(new Set(list.filter((session) => session.lifecycle === "alive").map((session) => session.id)));
      setSessions(mergeSessionsById([], list));
    } catch {
      // Silent: polled every 2s; transient failures are reflected by the
      // backendStatus badge, not by spamming toasts.
    }
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected") return;
    refreshSessions();
    const interval = setInterval(refreshSessions, 2000);
    return () => clearInterval(interval);
  }, [backendStatus, refreshSessions]);

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

  const filteredHarnesses = activeHarnessIds.length === 0
    ? harnesses.filter((h) => h.id === "generic-shell")
    : harnesses.filter((h) => h.id === "generic-shell" || activeHarnessIds.includes(h.id));

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
    try {
      const info = await window.orkworks.openWorkspace();
      if (info) {
        setWorkspaceState(info);
        setActiveHarnessIds(info.activeHarnessIds ?? []);
        setBackendStatus("connecting…");
        setSessions([]);
        setActiveSessionId(info.lastActiveSessionId ?? null);
      }
    } catch {
      pushToast("error", "Couldn't open workspace.");
    }
  }, []);

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
      const baseUrl = await window.orkworks.getBackendUrl();
      const session = await createSession(baseUrl, opts);
      setSessions((prev) => mergeSessionsById(prev, [session]));
      setActiveSessionId(session.id);

      const api = dockviewApiRef.current;
      if (api) {
        const panel = api.getPanel("terminal");
        if (panel) panel.api.setActive();
      }
    } catch {
      pushToast("error", "Couldn't start a new session.");
    }
  }, []);

  // Unread ("changed since you looked") is derived by diffing attention
  // status between session snapshots; selecting a session marks it read.
  useEffect(() => {
    setUnreadState((prev) => trackUnread(prev, sessions, activeSessionId));
  }, [sessions, activeSessionId]);

  const handleSelectSession = useCallback((id: string) => {
    setUnreadState((prev) => clearUnread(prev, id));
    setActiveSessionId(id);
    const api = dockviewApiRef.current;
    if (api) {
      const panel = api.getPanel("terminal");
      if (panel) panel.api.setActive();
    }
  }, []);

  const handleKillSession = useCallback(
    async (id: string) => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        await deleteSession(baseUrl, id);
        disposeTerminal(id);

        if (activeSessionId === id) {
          setActiveSessionId(null);
        }
        await refreshSessions();
      } catch {
        pushToast("error", "Couldn't end session.");
      }
    },
    [activeSessionId, refreshSessions],
  );

  const handleForgetSession = useCallback(
    async (id: string) => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        await forgetSession(baseUrl, id);
        disposeTerminal(id);
        if (activeSessionId === id) setActiveSessionId(null);
        await refreshSessions();
      } catch {
        pushToast("error", "Couldn't delete session.");
      }
    },
    [activeSessionId, refreshSessions],
  );

  const handleFocusTerminal = useCallback(() => {
    if (!activeSessionId) return;
    getTerminal(activeSessionId)?.terminal.focus();
  }, [activeSessionId]);

  const handleResumeSession = useCallback(async (id: string) => {
    try {
      disposeTerminal(id);
      const baseUrl = await window.orkworks.getBackendUrl();
      const session = await resumeSession(baseUrl, id);
      setSessions((prev) => prev.map(s => s.id === id ? session : s));
      setActiveSessionId(session.id);
      setResumeTick(t => t + 1);
    } catch {
      pushToast("error", "Couldn't resume session.");
    }
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected" || workspace) return;
    let cancelled = false;
    async function loadInitialWorkspace() {
      const info = await window.orkworks.getInitialWorkspace();
      if (!cancelled && info) {
        setWorkspaceState(info);
        setActiveHarnessIds(info.activeHarnessIds ?? []);
        await refreshSessions();
        if (info.lastActiveSessionId) {
          setActiveSessionId(info.lastActiveSessionId);
        }
      }
    }
    loadInitialWorkspace();
    return () => {
      cancelled = true;
    };
  }, [backendStatus, refreshSessions, workspace]);

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
            const options: { id: string; component: string; position?: { referencePanel: string; direction: "below" | "right" | "left" | "above" } } = {
              id: def.component,
              component: def.component,
            };
            if (def.position && api.getPanel(def.position.referencePanel)) {
              options.position = { referencePanel: def.position.referencePanel, direction: def.position.direction };
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
          const options: { id: string; component: string; position?: { referencePanel: string; direction: "below" | "right" | "left" | "above" } } = {
            id: def.component,
            component: def.component,
          };
          if (def.position && api.getPanel(def.position.referencePanel)) {
            options.position = { referencePanel: def.position.referencePanel, direction: def.position.direction };
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
        debugSettings={settings?.debug ?? { showSessionIds: false }}
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
        onFocusTerminal={handleFocusTerminal}
        onOpenWorkspace={handleOpenWorkspace}
        dockviewApiRef={dockviewApiRef}
      />
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
