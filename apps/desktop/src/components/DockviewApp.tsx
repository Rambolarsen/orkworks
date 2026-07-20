import { createContext, useContext, useRef, useState } from "react";
import {
  DockviewDefaultTab,
  DockviewReact,
  type DockviewReadyEvent,
  type DockviewApi,
  type IDockviewHeaderActionsProps,
  type IDockviewPanelHeaderProps,
} from "dockview-react";
import type { SessionAttention, SessionInfo, WorkspaceInfo } from "../api";
import type { HarnessConfig } from "../harnessTypes";
import type { DebugSettings } from "../appSettingsTypes";
import SessionListPanel from "./SessionListPanel";
import SessionDetailPanel from "./SessionDetailPanel";
import TerminalPanel from "./TerminalPanel";
import CapacityPanel from "./CapacityPanel";
import RecommendationsPanel from "./RecommendationsPanel";

interface DockviewAppData {
  backendStatus: string;
  workspace: WorkspaceInfo | null;
  debugSettings: DebugSettings;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  unreadIds: ReadonlySet<string>;
  harnesses: HarnessConfig[];
  resumeTick: number;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
  onForgetSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  onApplyDebugAttention: (id: string, attention: SessionAttention, message?: string) => void;
  onFocusTerminal: () => void;
  onOpenWorkspace: () => void;
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
      unreadIds={ctx.unreadIds}
      harnesses={ctx.harnesses}
      onSelectSession={ctx.onSelectSession}
      onKillSession={ctx.onKillSession}
      onForgetSession={ctx.onForgetSession}
      onFocusTerminal={ctx.onFocusTerminal}
      onOpenWorkspace={ctx.onOpenWorkspace}
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
      onApplyDebugAttention={ctx.onApplyDebugAttention}
      showDebugMetadata={ctx.debugSettings.showSessionIds}
    />
  );
}

function TermPanel() {
  const ctx = useContext(DockviewContext);
  const session = ctx.sessions.find((s) => s.id === ctx.activeSessionId) ?? null;
  return <TerminalPanel key={`${session?.id ?? 'none'}-${ctx.resumeTick}`} backendStatus={ctx.backendStatus} session={session} />;
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
  terminal:        { component: "terminal", title: "Terminal" },
  sessions:        { component: "sessions", title: "Sessions", position: { referencePanel: "terminal", direction: "left" } },
  detail:          { component: "detail", title: "Detail", position: { referencePanel: "sessions", direction: "below" } },
  capacity:        { component: "capacity", title: "Capacity", position: { referencePanel: "terminal", direction: "right" } },
  recommendations: { component: "recommendations", title: "Recommendations", position: { referencePanel: "capacity", direction: "below" } },
};

/** Single source of truth for first-launch / Reset Layout. Capacity and
 *  Recommendations are reachable via View menu hotkeys but closed by default
 *  until they carry signal. */
export const DEFAULT_LAYOUT_PANELS: ReadonlyArray<string> = ["terminal", "sessions", "detail"];

export function buildDefaultLayout(api: DockviewApi): void {
  for (const id of DEFAULT_LAYOUT_PANELS) {
    const def = PANEL_DEFAULTS[id];
    const options: Parameters<typeof api.addPanel>[0] = {
      id: def.component,
      component: def.component,
      title: def.title,
      ...(def.position
        ? { position: { referencePanel: def.position.referencePanel, direction: def.position.direction } }
        : {}),
    };
    if (id === "terminal") options.minimumWidth = 400;
    api.addPanel(options);
  }
}

/** Pre-redesign 5-panel default layouts referenced Capacity and/or
 *  Recommendations panel ids. Their positions cascade off removed siblings
 *  so the cleanest cutover is to rebuild the default. One-time per existing
 *  user; versioned layouts never trigger this. */
function layoutNeedsMigration(json: Record<string, unknown>): boolean {
  const text = JSON.stringify(json);
  return text.includes('"capacity"') || text.includes('"recommendations"');
}

function DockviewApp(props: DockviewAppData) {
  const { dockviewApiRef } = props;
  const ctxValue = props;

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
                    const parsed = JSON.parse(layout);
                    if (!parsed || typeof parsed !== "object") {
                      throw new Error("unrecognized layout");
                    }
                    if (!("v" in parsed) && layoutNeedsMigration(parsed as Record<string, unknown>)) {
                      console.info("[DockviewApp] migrating stored layout to redesigned default");
                      buildDefaultLayout(api);
                    } else {
                      api.fromJSON(
                        "v" in parsed ? (parsed as { d: unknown }).d : parsed,
                      );
                    }
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
                window.orkworks.saveLayout(
                  JSON.stringify({ v: 1, d: api.toJSON() }),
                );
              }, 500);
            });
          }}
        />
        {isEmpty && (
          <div className="dockview-empty-state">
            <p>All panels are closed.</p>
            <button
              type="button"
              className="dockview-empty-reset"
              onClick={() => {
                const api = dockviewApiRef.current;
                if (api) resetLayout(api);
              }}
            >
              Restore default layout
            </button>
          </div>
        )}
      </DockviewContext.Provider>
    </div>
  );
}

export default DockviewApp;
