import { useCallback, useEffect, useRef, useState } from "react";
import type { DockviewApi } from "dockview-react";
import DockviewApp from "./components/DockviewApp";
import { sortSessions } from "./components/RightSidebarHelpers";
import { PANEL_DEFAULTS } from "./components/DockviewApp";
import {
  type SessionInfo,
  type WorkspaceInfo,
  createSession,
  listSessions,
  deleteSession,
  resumeSession,
  setActiveWorkspaceSession,
} from "./api";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      getInitialWorkspace: () => Promise<WorkspaceInfo | null>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
      getLayout: () => Promise<string | null>;
      saveLayout: (json: string) => Promise<void>;
      onMenuCommand: (callback: (data: { action: string; panelId?: string }) => void) => () => void;
      notifyPanelVisibility: (panelId: string, visible: boolean) => void;
    };
  }
}

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
  const dockviewApiRef = useRef<DockviewApi | null>(null);

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
              if (!cancelled) setBackendStatus("connected");
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
      setSessions(sortSessions(list));
    } catch {
      /* backend not ready */
    }
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected") return;
    refreshSessions();
    const interval = setInterval(refreshSessions, 2000);
    return () => clearInterval(interval);
  }, [backendStatus, refreshSessions]);

  const handleOpenWorkspace = useCallback(async () => {
    try {
      const info = await window.orkworks.openWorkspace();
      if (info) {
        setWorkspaceState(info);
        setBackendStatus("connecting…");
        setSessions([]);
        setActiveSessionId(info.lastActiveSessionId ?? null);
      }
    } catch {
      /* user cancelled */
    }
  }, []);

  const handleCreateSession = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const session = await createSession(baseUrl);
      setSessions((prev) => [...prev, session]);
      setActiveSessionId(session.id);
    } catch {
      /* ignore */
    }
  }, []);

  const handleSelectSession = useCallback((id: string) => {
    setActiveSessionId(id);
  }, []);

  const handleKillSession = useCallback(
    async (id: string) => {
      try {
        const baseUrl = await window.orkworks.getBackendUrl();
        await deleteSession(baseUrl, id);
        if (activeSessionId === id) {
          setActiveSessionId(null);
        }
        await refreshSessions();
      } catch {
        /* ignore */
      }
    },
    [activeSessionId, refreshSessions],
  );

  const handleResumeSession = useCallback(async (id: string) => {
    const baseUrl = await window.orkworks.getBackendUrl();
    const session = await resumeSession(baseUrl, id);
    setSessions((prev) => [...prev, session]);
    setActiveSessionId(session.id);
  }, []);

  useEffect(() => {
    if (backendStatus !== "connected" || workspace) return;
    let cancelled = false;
    async function loadInitialWorkspace() {
      const info = await window.orkworks.getInitialWorkspace();
      if (!cancelled && info) {
        setWorkspaceState(info);
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
    if (backendStatus !== "connected" || !activeSessionId) return;
    const sid = activeSessionId;
    async function persistActiveSession() {
      const baseUrl = await window.orkworks.getBackendUrl();
      await setActiveWorkspaceSession(baseUrl, sid);
    }
    persistActiveSession().catch(() => {
      /* backend not ready */
    });
  }, [activeSessionId, backendStatus]);

  useEffect(() => {
    return window.orkworks.onMenuCommand(({ action, panelId }) => {
      const api = dockviewApiRef.current;
      if (!api) return;

      if (action === "focus" && panelId) {
        const def = PANEL_DEFAULTS[panelId];
        if (!def) return;
        const existing = api.getPanel(def.component);
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
        api.clear();
        api.addPanel({ id: PANEL_DEFAULTS.sessions.component, component: PANEL_DEFAULTS.sessions.component });
        for (const id of ["detail", "terminal", "capacity", "recommendations"]) {
          const def = PANEL_DEFAULTS[id];
          api.addPanel({
            id: def.component,
            component: def.component,
            position: { referencePanel: def.position!.referencePanel, direction: def.position!.direction },
          });
        }
      }
    });
  }, []);

  return (
    <div className="app-shell">
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
                title="Switch workspace"
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
                Open Folder
              </button>
            </>
          )}
        </div>
        <span
          className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
        >
          {backendStatus}
        </span>
      </div>
      <DockviewApp
        backendStatus={backendStatus}
        workspace={workspace}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={handleSelectSession}
        onCreateSession={handleCreateSession}
        onKillSession={handleKillSession}
        onResumeSession={handleResumeSession}
        dockviewApiRef={dockviewApiRef}
      />
    </div>
  );
}

export default App;
