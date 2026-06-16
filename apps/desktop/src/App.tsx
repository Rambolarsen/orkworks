import { useCallback, useEffect, useState } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";
import LeftSidebar from "./components/LeftSidebar";
import CenterPanel from "./components/CenterPanel";
import RightSidebar from "./components/RightSidebar";
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

type PanelId = "left" | "center" | "right";

const SNAP_SIZES = [15, 20, 25, 30];

const snapToPreset = (size: number): number => {
  for (const preset of SNAP_SIZES) {
    if (Math.abs(size - preset) <= 3) return preset;
  }
  return size;
};

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);
  const [panelOrder, setPanelOrder] = useState<PanelId[]>(["left", "center", "right"]);
  const [panelSizes, setPanelSizes] = useState<number[]>([20, 58, 22]);

  const handleLayoutChanged = useCallback((layout: Record<string, number>) => {
    const sizes = panelOrder.map((id) => snapToPreset(layout[id] ?? panelSizes[panelOrder.indexOf(id)]));
    setPanelSizes(sizes);
  }, [panelOrder]);

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
    creating: 0,
    running: 1,
    ended: 2,
    killed: 3,
    error: 4,
  };

  const refreshSessions = useCallback(async () => {
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const list = await listSessions(baseUrl);
      list.sort((a, b) => {
        const sa = stateOrder[a.status] ?? 5;
        const sb = stateOrder[b.status] ?? 5;
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
        await refreshSessions();
      } catch {
        /* ignore */
      }
    },
    [refreshSessions],
  );

  const cyclePanelOrder = useCallback(() => {
    setPanelOrder((prev) => [prev[1], prev[2], prev[0]]);
  }, []);

  const renderPanel = (id: PanelId) => {
    if (id === "left") {
      return (
        <LeftSidebar
          workspace={workspace}
          onOpenWorkspace={handleOpenWorkspace}
          sessions={sessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSession}
          onCreateSession={handleCreateSession}
          onKillSession={handleKillSession}
        />
      );
    }
    if (id === "center") {
      return (
        <CenterPanel
          backendStatus={backendStatus}
          sessionId={activeSessionId}
        />
      );
    }
    return (
      <RightSidebar
        sessions={sessions}
        activeSessionId={activeSessionId}
      />
    );
  };

  const panelClass = (id: PanelId) => {
    if (id === "left") return "panel left-sidebar";
    if (id === "center") return "panel center-panel";
    return "panel right-sidebar";
  };

  return (
    <div className="app-shell">
      <div className="titlebar">
        <span className="titlebar-text">OrkWorks</span>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <button
            className="titlebar-cycle-button"
            type="button"
            onClick={cyclePanelOrder}
            title="Cycle panel order"
          >
            &#x21C4;
          </button>
          <span
            className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
          >
            {backendStatus}
          </span>
        </div>
      </div>
      <div className="app-layout">
        <Group orientation="horizontal" onLayoutChanged={handleLayoutChanged}>
          {panelOrder.map((id, i) => (
            <Panel
              key={id}
              id={id}
              defaultSize={panelSizes[i]}
              minSize={id === "center" ? 30 : 14}
              className={panelClass(id)}
            >
              {renderPanel(id)}
            </Panel>
          )).reduce<(React.ReactNode[])>((acc, panel, i) => {
            acc.push(panel);
            if (i < 2) {
              acc.push(
                <Separator key={`sep-${i}`} className="panel-resize-handle" />,
              );
            }
            return acc;
          }, [])}
        </Group>
      </div>
    </div>
  );
}

export default App;
