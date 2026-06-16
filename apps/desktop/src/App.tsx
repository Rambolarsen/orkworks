import { useCallback, useEffect, useRef, useState } from "react";
import LeftSidebar from "./components/LeftSidebar";
import RightSidebar from "./components/RightSidebar";
import TerminalTabs from "./components/TerminalTabs";
import type { TerminalTabsHandle } from "./components/TerminalTabs";
import { sessionAttentionStatus } from "./components/RightSidebarHelpers";
import {
  type SessionInfo,
  type WorkspaceInfo,
  createSession,
  listSessions,
  deleteSession,
} from "./api";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
    };
  }
}

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
  const terminalTabsRef = useRef<TerminalTabsHandle>(null);

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

  const stateOrder: Record<string, number> = {
    waiting_for_input: 0,
    blocked: 1,
    failed: 2,
    creating: 3,
    running: 4,
    working: 5,
    idle: 6,
    done: 7,
    stale: 8,
    ended: 9,
    killed: 10,
    error: 11,
  };

  const refreshSessions = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const list = await listSessions(baseUrl);
      list.sort((a, b) => {
        const sa = stateOrder[sessionAttentionStatus(a)] ?? 5;
        const sb = stateOrder[sessionAttentionStatus(b)] ?? 5;
        if (sa !== sb) return sa - sb;
        return a.label.localeCompare(b.label);
      });
      setSessions(list);
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
      <div className="app-layout">
        <aside className="panel left-sidebar">
          <LeftSidebar
            workspace={workspace}
            onOpenWorkspace={handleOpenWorkspace}
            sessions={sessions}
            activeSessionId={activeSessionId}
            onSelectSession={handleSelectSession}
            onCreateSession={handleCreateSession}
            onKillSession={handleKillSession}
          />
        </aside>
        <main className="panel center-panel">
          <TerminalTabs
            ref={terminalTabsRef}
            backendStatus={backendStatus}
            activeSessionId={activeSessionId}
            sessions={sessions.map((s) => ({ id: s.id, label: s.label }))}
          />
        </main>
        <aside className="panel right-sidebar">
          <RightSidebar
            sessions={sessions}
            activeSessionId={activeSessionId}
          />
        </aside>
      </div>
    </div>
  );
}

export default App;
