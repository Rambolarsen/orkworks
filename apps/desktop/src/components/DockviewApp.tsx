import { DockviewReact, type DockviewReadyEvent } from "dockview-react";
import "dockview/dist/styles/dockview.css";
import type { SessionInfo, WorkspaceInfo } from "../api";
import SessionListPanel from "./SessionListPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import CapacityPanel from "./CapacityPanel";
import RecommendationsPanel from "./RecommendationsPanel";

interface DockviewAppProps {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onOpenWorkspace: () => void;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
}

function DockviewApp(props: DockviewAppProps) {
  const {
    backendStatus, workspace, sessions, activeSessionId,
    onOpenWorkspace, onSelectSession, onCreateSession, onKillSession,
  } = props;

  const components = {
    sessions: () => (
      <SessionListPanel
        workspace={workspace}
        onOpenWorkspace={onOpenWorkspace}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        onCreateSession={onCreateSession}
        onKillSession={onKillSession}
      />
    ),
    detail: () => (
      <SessionDetailPanel
        sessions={sessions}
        activeSessionId={activeSessionId}
      />
    ),
    terminal: () => (
      <TerminalPanel
        backendStatus={backendStatus}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        onKillSession={onKillSession}
      />
    ),
    capacity: () => <CapacityPanel />,
    recommendations: () => <RecommendationsPanel />,
  };

  return (
    <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
      <DockviewReact
        components={components}
        className="orkworks-dockview"
        onReady={(event: DockviewReadyEvent) => {
          event.api.fromJSON({
            grid: {
              root: {
                type: "branch" as const,
                data: [
                  {
                    type: "branch" as const,
                    size: 260,
                    data: [
                      {
                        type: "leaf" as const,
                        data: { views: ["sessions"], activeView: "sessions" },
                        size: 300,
                      },
                      {
                        type: "leaf" as const,
                        data: { views: ["detail"], activeView: "detail" },
                        size: 300,
                      },
                    ],
                  },
                  {
                    type: "leaf" as const,
                    data: { views: ["terminal"], activeView: "terminal" },
                    size: 800,
                  },
                  {
                    type: "branch" as const,
                    size: 250,
                    data: [
                      {
                        type: "leaf" as const,
                        data: { views: ["capacity"], activeView: "capacity" },
                        size: 200,
                      },
                      {
                        type: "leaf" as const,
                        data: { views: ["recommendations"], activeView: "recommendations" },
                        size: 200,
                      },
                    ],
                  },
                ],
              },
            },
            panels: {
              sessions: {},
              detail: {},
              terminal: {},
              capacity: {},
              recommendations: {},
            },
          });
        }}
      />
    </div>
  );
}

export default DockviewApp;
