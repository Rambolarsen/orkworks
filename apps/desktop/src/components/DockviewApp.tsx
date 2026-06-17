import { createContext, useContext, useRef } from "react";
import { DockviewReact, type DockviewReadyEvent } from "dockview-react";
import type { SessionInfo, WorkspaceInfo } from "../api";
import SessionListPanel from "./SessionListPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import CapacityPanel from "./CapacityPanel";
import RecommendationsPanel from "./RecommendationsPanel";

interface DockviewAppData {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onOpenWorkspace: () => void;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onResumeSession: (id: string) => void;
}

const DockviewContext = createContext<DockviewAppData>(null!);

function SessionsPanel() {
  const ctx = useContext(DockviewContext);
  return (
    <SessionListPanel
      workspace={ctx.workspace}
      onOpenWorkspace={ctx.onOpenWorkspace}
      sessions={ctx.sessions}
      activeSessionId={ctx.activeSessionId}
      onSelectSession={ctx.onSelectSession}
      onCreateSession={ctx.onCreateSession}
      onKillSession={ctx.onKillSession}
    />
  );
}

function DetailPanel() {
  const ctx = useContext(DockviewContext);
  return (
    <SessionDetailPanel
      sessions={ctx.sessions}
      activeSessionId={ctx.activeSessionId}
      onResumeSession={ctx.onResumeSession}
    />
  );
}

function TermPanel() {
  const ctx = useContext(DockviewContext);
  return (
    <TerminalPanel
      backendStatus={ctx.backendStatus}
      sessions={ctx.sessions}
      activeSessionId={ctx.activeSessionId}
      onSelectSession={ctx.onSelectSession}
      onKillSession={ctx.onKillSession}
    />
  );
}

function CapPanel() {
  return <CapacityPanel />;
}

function RecPanel() {
  return <RecommendationsPanel />;
}

const COMPONENTS = {
  sessions: SessionsPanel,
  detail: DetailPanel,
  terminal: TermPanel,
  capacity: CapPanel,
  recommendations: RecPanel,
};

function DockviewApp(props: DockviewAppData) {
  const { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession } = props;

  const ctxValue: DockviewAppData = { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession };

  const onReadyRef = useRef(false);

  return (
    <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
      <DockviewContext.Provider value={ctxValue}>
        <DockviewReact
          components={COMPONENTS}
          className="orkworks-dockview"
          onReady={(event: DockviewReadyEvent) => {
            if (onReadyRef.current) return;
            onReadyRef.current = true;

            const sessionsPanel = event.api.addPanel({
              id: "sessions",
              component: "sessions",
            });

            event.api.addPanel({
              id: "detail",
              component: "detail",
              position: { referencePanel: sessionsPanel, direction: "below" },
            });

            const terminalPanel = event.api.addPanel({
              id: "terminal",
              component: "terminal",
              position: { referencePanel: sessionsPanel, direction: "right" },
            });

            const capacityPanel = event.api.addPanel({
              id: "capacity",
              component: "capacity",
              position: { referencePanel: terminalPanel, direction: "right" },
            });

            event.api.addPanel({
              id: "recommendations",
              component: "recommendations",
              position: { referencePanel: capacityPanel, direction: "below" },
            });
          }}
        />
      </DockviewContext.Provider>
    </div>
  );
}

export default DockviewApp;
