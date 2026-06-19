import { createContext, useContext, useRef, useState } from "react";
import {
  DockviewDefaultTab,
  DockviewReact,
  type DockviewReadyEvent,
  type DockviewApi,
  type IDockviewHeaderActionsProps,
  type IDockviewPanelHeaderProps,
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
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  onFocusTerminal: () => void;
  dockviewApiRef: React.MutableRefObject<DockviewApi | null>;
}

const DockviewContext = createContext<DockviewAppData>(null!);

function SessionsPanel() {
  const ctx = useContext(DockviewContext);
  return (
    <SessionListPanel
      workspace={ctx.workspace}
      sessions={ctx.sessions}
      activeSessionId={ctx.activeSessionId}
      onSelectSession={ctx.onSelectSession}
      onKillSession={ctx.onKillSession}
      onFocusTerminal={ctx.onFocusTerminal}
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

function DockviewTab(props: IDockviewPanelHeaderProps) {
  return <DockviewDefaultTab {...props} hideClose />;
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
  const session = ctx.sessions.find((s) => s.id === ctx.activeSessionId) ?? null;
  return (
    <TerminalPanel
      backendStatus={ctx.backendStatus}
      session={session}
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
  const { backendStatus, workspace, sessions, activeSessionId, onSelectSession, onCreateSession, onKillSession, onResumeSession, onFocusTerminal, dockviewApiRef } = props;

  const ctxValue: DockviewAppData = { backendStatus, workspace, sessions, activeSessionId, onSelectSession, onCreateSession, onKillSession, onResumeSession, onFocusTerminal, dockviewApiRef };

  const initializedRef = useRef(false);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [isEmpty, setIsEmpty] = useState(false);

  function resetLayout(api: DockviewApi) {
    api.clear();
    buildDefaultLayout(api);
  }

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
    <div style={{ flex: 1, display: "flex", overflow: "hidden", position: "relative" }}>
      <DockviewContext.Provider value={ctxValue}>
        <DockviewReact
          components={COMPONENTS}
          className="orkworks-dockview"
          defaultTabComponent={DockviewTab}
          singleTabMode="fullwidth"
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
                  setIsEmpty(api.totalPanels === 0);
                  return;
                } catch (e) {
                  console.warn("[DockviewApp] failed to restore layout, using default", e);
                }
              }
              buildDefaultLayout(api);
              reportVisibility(api);
              setIsEmpty(api.totalPanels === 0);
            });

            api.onDidLayoutChange(() => {
              reportVisibility(api);
              setIsEmpty(api.totalPanels === 0);
              if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
              saveTimerRef.current = setTimeout(() => {
                window.orkworks.saveLayout(JSON.stringify(api.toJSON()));
              }, 500);
            });
          }}
        />
        {isEmpty && (
          <div className="dockview-empty-state">
            <p>All panels are closed.</p>
            <p className="dockview-empty-hint">
              Open one from the View menu, or
            </p>
            <button
              type="button"
              className="dockview-empty-reset"
              onClick={() => {
                const api = dockviewApiRef.current;
                if (api) resetLayout(api);
              }}
            >
              Reset Layout
            </button>
          </div>
        )}
      </DockviewContext.Provider>
    </div>
  );
}

export default DockviewApp;
