import { useCallback, useEffect, useState } from "react";
import DockviewApp from "./components/DockviewApp";
import { sortSessions } from "./components/RightSidebarHelpers";
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
    };
  }
}

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);

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
        setActiveSessionId(null);
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

  return (
    <div className="app-shell">
      <div className="titlebar">
        <span className="titlebar-text">OrkWorks</span>
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
        onOpenWorkspace={handleOpenWorkspace}
        onSelectSession={handleSelectSession}
        onCreateSession={handleCreateSession}
        onKillSession={handleKillSession}
        onResumeSession={handleResumeSession}
      />
    </div>
  );
}

export default App;
