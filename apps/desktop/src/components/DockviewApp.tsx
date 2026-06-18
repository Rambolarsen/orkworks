import { createContext, useContext, useRef } from "react";
import {
  DockviewReact,
  type DockviewReadyEvent,
  type DockviewApi,
  type IDockviewHeaderActionsProps,
} from "dockview-react";
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
  dockviewApiRef: React.MutableRefObject<DockviewApi | null>;
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
      onKillSession={ctx.onKillSession}
    />
  );
}

function SessionsHeaderActions(props: IDockviewHeaderActionsProps) {
  const ctx = useContext(DockviewContext);

  if (!ctx.workspace || props.activePanel?.id !== PANEL_DEFAULTS.sessions.component) {
    return null;
  }

  return (
    <button
      className="dockview-header-action"
      type="button"
      title="New session"
      onClick={() => ctx.onCreateSession()}
    >
      +
    </button>
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

export interface PanelDefault {
  component: string;
  title: string;
  position?: { referencePanel: string; direction: "below" | "right" | "left" | "above" };
}

export const PANEL_DEFAULTS: Record<string, PanelDefault> = {
  sessions:        { component: "sessions", title: "Sessions" },
  detail:          { component: "detail", title: "Detail", position: { referencePanel: "sessions", direction: "below" } },
  terminal:        { component: "terminal", title: "Terminal", position: { referencePanel: "sessions", direction: "right" } },
  capacity:        { component: "capacity", title: "Capacity", position: { referencePanel: "terminal", direction: "right" } },
  recommendations: { component: "recommendations", title: "Recommendations", position: { referencePanel: "capacity", direction: "below" } },
};

function DockviewApp(props: DockviewAppData) {
  const { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession, dockviewApiRef } = props;

  const ctxValue: DockviewAppData = { backendStatus, workspace, sessions, activeSessionId, onOpenWorkspace, onSelectSession, onCreateSession, onKillSession, onResumeSession, dockviewApiRef };

  const initializedRef = useRef(false);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function reportVisibility(api: DockviewApi) {
    for (const [id, def] of Object.entries(PANEL_DEFAULTS)) {
      const visible = api.getPanel(def.component) != null;
      window.orkworks.notifyPanelVisibility(id, visible);
    }
  }

  function buildDefaultLayout(api: DockviewApi) {
    api.addPanel({
      id: PANEL_DEFAULTS.sessions.component,
      component: PANEL_DEFAULTS.sessions.component,
      title: PANEL_DEFAULTS.sessions.title,
    });
    for (const id of ["detail", "terminal", "capacity", "recommendations"]) {
      const def = PANEL_DEFAULTS[id];
      if (def.position) {
        api.addPanel({
          id: def.component,
          component: def.component,
          title: def.title,
          position: { referencePanel: def.position.referencePanel, direction: def.position.direction },
        });
      }
    }
  }

  return (
    <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
      <DockviewContext.Provider value={ctxValue}>
        <DockviewReact
          components={COMPONENTS}
          className="orkworks-dockview"
          rightHeaderActionsComponent={SessionsHeaderActions}
          onReady={(event: DockviewReadyEvent) => {
            if (initializedRef.current) return;
            initializedRef.current = true;

            const api = event.api;
            dockviewApiRef.current = api;

            window.orkworks.getLayout().then((layout) => {
              if (layout) {
                try {
                  api.fromJSON(JSON.parse(layout));
                  reportVisibility(api);
                  return;
                } catch (e) {
                  console.warn("[DockviewApp] failed to restore layout, using default", e);
                }
              }
              buildDefaultLayout(api);
              reportVisibility(api);
            });

            api.onDidLayoutChange(() => {
              reportVisibility(api);
              if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
              saveTimerRef.current = setTimeout(() => {
                window.orkworks.saveLayout(JSON.stringify(api.toJSON()));
              }, 500);
            });
          }}
        />
      </DockviewContext.Provider>
    </div>
  );
}

export default DockviewApp;
