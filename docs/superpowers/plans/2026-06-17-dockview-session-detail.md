# Dockview Session Detail — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace fixed three-column flexbox layout with dockview draggable panels, add session detail panel docked below session list, add capacity/recommendations placeholders on the right.

**Architecture:** Install `dockview` + `dockview-react`, drop `react-resizable-panels`. Five dockview panels: sessions, detail, terminal, capacity, recommendations. Session state stays in `App.tsx`. Terminal panel manages its own tabs internally (one `CenterPanel` per session). Panels communicate through `activeSessionId` prop and callbacks.

**Tech Stack:** dockview, dockview-react, React 19, TypeScript, xterm.js

---

### Task 1: Install dockview and remove react-resizable-panels

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/pnpm-lock.yaml`

- [ ] **Step 1: Install dockview**

```bash
cd apps/desktop && pnpm add dockview dockview-react
```

- [ ] **Step 2: Verify dockview installed**

Run:
```bash
cd apps/desktop && node -e "require('dockview'); require('dockview-react')"
```
Expected: No errors.

- [ ] **Step 3: Clean up old import references**

Search for `react-resizable-panels` references (should be none since we removed Group/Panel/Separator already — confirm):

```bash
rg "resizable-panels" apps/desktop/src/ apps/desktop/package.json
```

Expected: No matches (if there are, they need removal in later tasks).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/package.json apps/desktop/pnpm-lock.yaml
git commit -m "chore: add dockview, dockview-react"
```

---

### Task 2: Create placeholder panels

**Files:**
- Create: `apps/desktop/src/components/CapacityPanel.tsx`
- Create: `apps/desktop/src/components/RecommendationsPanel.tsx`

- [ ] **Step 1: Write CapacityPanel**

```tsx
function CapacityPanel() {
  return (
    <div style={{ padding: "12px", height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{
        fontSize: "11px", fontWeight: 600, textTransform: "uppercase",
        letterSpacing: "0.5px", color: "#999", marginBottom: "12px"
      }}>
        Capacity
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <p style={{ color: "#666", fontSize: 12, fontStyle: "italic" }}>
          Capacity tracking coming in M8
        </p>
      </div>
    </div>
  );
}

export default CapacityPanel;
```

- [ ] **Step 2: Write RecommendationsPanel**

```tsx
function RecommendationsPanel() {
  return (
    <div style={{ padding: "12px", height: "100%", display: "flex", flexDirection: "column" }}>
      <div style={{
        fontSize: "11px", fontWeight: 600, textTransform: "uppercase",
        letterSpacing: "0.5px", color: "#999", marginBottom: "12px"
      }}>
        Start Next Task
      </div>
      <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <p style={{ color: "#666", fontSize: 12, fontStyle: "italic" }}>
          Recommendations coming in M9
        </p>
      </div>
    </div>
  );
}

export default RecommendationsPanel;
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/CapacityPanel.tsx apps/desktop/src/components/RecommendationsPanel.tsx
git commit -m "feat: add capacity and recommendations placeholder panels"
```

---

### Task 3: Adapt SessionListPanel from LeftSidebar

**Files:**
- Create: `apps/desktop/src/components/SessionListPanel.tsx`
- Modify: `apps/desktop/src/components/LeftSidebar.tsx` — keep as thin re-export for backward compat
- Modify: `apps/desktop/src/components/RightSidebarHelpers.ts`

- [ ] **Step 1: Replace attention helpers in RightSidebarHelpers.ts**

Replace the contents of `apps/desktop/src/components/RightSidebarHelpers.ts` with:

```typescript
import type { SessionInfo } from "../api";

export const ATTENTION_PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  failed: 2,
  done: 3,
  stale: 4,
  working: 5,
  idle: 6,
  creating: 7,
  running: 8,
  ended: 9,
  killed: 10,
  error: 11,
};

export function needsAttention(status: string): boolean {
  return status === "blocked" || status === "failed" || status === "waiting_for_input";
}

export function sessionAttentionStatus(session: SessionInfo): string {
  return session.observedStatus ?? session.status;
}

export function isLive(status: string): boolean {
  return status === "running" || status === "creating";
}

export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const pa = ATTENTION_PRIORITY[sessionAttentionStatus(a)] ?? 99;
    const pb = ATTENTION_PRIORITY[sessionAttentionStatus(b)] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.label.localeCompare(b.label);
  });
}

export function borderColor(status: string): string {
  return attentionBorderColor(status);
}

export function statusDotColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#666";
  if (status === "working" || status === "running" || status === "creating") return "#4ec94e";
  return "#666";
}

export function attentionBorderColor(status: string): string {
  if (status === "waiting_for_input" || status === "failed") return "#cc4444";
  if (status === "blocked") return "#d4d44e";
  if (status === "done") return "#4ec94e";
  if (status === "stale" || status === "idle") return "#4a4a4a";
  return "#3c3c3c";
}

export function sourceColor(source: string | undefined): string {
  if (source === "agent") return "#4ec94e";
  if (source === "peon") return "#57c7ff";
  return "#858585";
}
```

- [ ] **Step 2: Create SessionListPanel.tsx** — copy logic from LeftSidebar, strip outer `<aside>`, use new helpers

```tsx
import type { SessionInfo, WorkspaceInfo } from "../api";
import {
  needsAttention,
  sessionAttentionStatus,
  sourceColor,
  statusDotColor,
  attentionBorderColor,
} from "./RightSidebarHelpers.ts";
import WorkspaceHeader from "./WorkspaceHeader";

interface SessionListPanelProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
}

function SessionListPanel({
  workspace,
  onOpenWorkspace,
  sessions,
  activeSessionId,
  onSelectSession,
  onCreateSession,
  onKillSession,
}: SessionListPanelProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {workspace ? (
        <>
          <WorkspaceHeader workspace={workspace} onOpenWorkspace={onOpenWorkspace} />
          <div className="panel-header">
            <span>Sessions</span>
            <button
              className="session-new-button"
              type="button"
              onClick={onCreateSession}
              title="New session"
            >
              +
            </button>
          </div>
          <div className="panel-content">
            {sessions.length === 0 ? (
              <p className="empty-state">No active sessions</p>
            ) : (
              <ul className="session-list">
                {sessions.map((s) => {
                  const attn = sessionAttentionStatus(s);
                  const isActive = s.id === activeSessionId;
                  return (
                    <li
                      key={s.id}
                      className={`session-item${isActive ? " session-item--active" : ""}`}
                      style={{ borderLeft: `3px solid ${attentionBorderColor(attn)}` }}
                      onClick={() => onSelectSession(s.id)}
                    >
                      <div className="session-item-main">
                        <span
                          className="session-status"
                          style={{ background: statusDotColor(attn) }}
                        />
                        <div className="session-item-info">
                          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                            {needsAttention(attn) && (
                              <span className="session-item-alert" title="Needs attention">&#x26A0;</span>
                            )}
                            <span className="session-item-label">{s.label}</span>
                          </div>
                          <span className="session-item-meta">
                            {attn} &middot; {s.cwd.split("/").pop() || s.cwd}
                          </span>
                          {s.metadataSource && (
                            <span
                              className="session-item-badge"
                              style={{
                                background: sourceColor(s.metadataSource) + "22",
                                color: sourceColor(s.metadataSource),
                              }}
                            >
                              {s.metadataSource} &middot; {Math.round((s.metadataConfidence ?? 1) * 100)}%
                            </span>
                          )}
                        </div>
                      </div>
                      <button
                        className="session-kill-button"
                        type="button"
                        title="Kill session"
                        onClick={(e) => {
                          e.stopPropagation();
                          onKillSession(s.id);
                        }}
                      >
                        &times;
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </>
      ) : (
        <WorkspaceHeader workspace={null} onOpenWorkspace={onOpenWorkspace} />
      )}
    </div>
  );
}

export default SessionListPanel;
```

- [ ] **Step 3: Keep LeftSidebar as a re-export**

Replace `apps/desktop/src/components/LeftSidebar.tsx` with:

```tsx
export { default } from "./SessionListPanel";
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/components/SessionListPanel.tsx apps/desktop/src/components/LeftSidebar.tsx apps/desktop/src/components/RightSidebarHelpers.ts
git commit -m "feat: add SessionListPanel with attention priority styling"
```

---

### Task 4: Create SessionDetailPanel from RightSidebar content

**Files:**
- Create: `apps/desktop/src/components/SessionDetailPanel.tsx`

- [ ] **Step 1: Write SessionDetailPanel.tsx**

```tsx
import type { SessionInfo } from "../api";
import { sessionAttentionStatus, sourceColor, statusDotColor } from "./RightSidebarHelpers.ts";

interface SessionDetailPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
}

function SessionDetailPanel({ sessions, activeSessionId }: SessionDetailPanelProps) {
  const active = sessions.find((s) => s.id === activeSessionId);

  if (!active) {
    return (
      <div style={{ padding: "12px", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
        <p className="empty-state">Select a session to see details</p>
      </div>
    );
  }

  const attn = sessionAttentionStatus(active);

  return (
    <div style={{ padding: "8px 12px", height: "100%", overflowY: "auto" }}>
      {active.summary && (
        <div className="session-detail-section">
          <div className="session-detail-label">Task</div>
          <div className="session-detail-value">{active.summary}</div>
        </div>
      )}

      <div className="session-detail-section">
        <div className="session-detail-label">Status</div>
        <div className="session-detail-value">
          <span style={{
            display: "inline-block",
            width: 8, height: 8, borderRadius: "50%",
            background: statusDotColor(attn), marginRight: 6,
            verticalAlign: "middle",
          }} />
          {attn}
        </div>
      </div>

      <div className="session-detail-section">
        <div className="session-detail-label">Directory</div>
        <div className="session-detail-value">{active.cwd.split("/").pop() || active.cwd}</div>
      </div>

      {active.branch && (
        <div className="session-detail-section">
          <div className="session-detail-label">Git</div>
          <div className="session-detail-value">
            {active.branch}
            {active.isWorktree && (
              <span style={{ color: "#4ec94e", marginLeft: 6, fontSize: 10 }}>worktree</span>
            )}
          </div>
          <div style={{ display: "flex", gap: 8, marginTop: 2, fontSize: 10 }}>
            <span style={{ color: active.dirty ? "#d4d44e" : "#4ec94e" }}>
              {active.dirty ? "dirty" : "clean"}
            </span>
            {active.changedFiles !== undefined && active.changedFiles > 0 && (
              <span style={{ color: "#858585" }}>{active.changedFiles} files changed</span>
            )}
          </div>
        </div>
      )}

      {active.conflictWarning && (
        <div className="session-detail-section">
          <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
        </div>
      )}

      {active.recommendation && (
        <div className="session-detail-section">
          <div className="recommendation-text">{active.recommendation}</div>
        </div>
      )}

      {active.metadataSource && (
        <div className="session-detail-section">
          <div className="session-detail-label">Source</div>
          <span
            className="overview-card-badge"
            style={{
              background: sourceColor(active.metadataSource) + "22",
              color: sourceColor(active.metadataSource),
            }}
          >
            {active.metadataSource} &middot; {Math.round((active.metadataConfidence ?? 1) * 100)}%
          </span>
        </div>
      )}

      {active.peonLastInference && (
        <div className="session-detail-section">
          <div className="session-detail-label">Peon</div>
          <span className="session-detail-value" style={{ color: "#57c7ff" }}>
            observed {active.peonLastInference}
          </span>
        </div>
      )}
    </div>
  );
}

export default SessionDetailPanel;
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/components/SessionDetailPanel.tsx
git commit -m "feat: add SessionDetailPanel with status, git, source sections"
```

---

### Task 5: Create TerminalPanel for dockview

**Files:**
- Create: `apps/desktop/src/components/TerminalPanel.tsx`

The terminal panel wraps the existing tab bar + CenterPanel pattern inside a dockview-friendly container. Each session gets its own `CenterPanel` instance with its own WebSocket. The panel manages its own tab bar internally.

- [ ] **Step 1: Write TerminalPanel.tsx**

```tsx
import { useEffect, useState } from "react";
import CenterPanel from "./CenterPanel";
import { sessionAttentionStatus, statusDotColor } from "./RightSidebarHelpers";
import type { SessionInfo } from "../api";

interface TerminalPanelProps {
  backendStatus: string;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
}

function TerminalPanel({
  backendStatus,
  sessions,
  activeSessionId,
  onSelectSession,
  onKillSession,
}: TerminalPanelProps) {
  const [localActive, setLocalActive] = useState<string | null>(activeSessionId);

  useEffect(() => {
    if (activeSessionId) setLocalActive(activeSessionId);
  }, [activeSessionId]);

  const liveSessions = sessions.filter(
    (s) => s.status === "running" || s.status === "creating"
  );
  const currentSession = liveSessions.find((s) => s.id === localActive) ?? liveSessions[0] ?? null;

  if (!currentSession) {
    return (
      <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%" }}>
        <p style={{ color: "#666", fontSize: 12 }}>No active terminal</p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{
        display: "flex", alignItems: "stretch",
        background: "#252526", borderBottom: "1px solid #3c3c3c",
        minHeight: 30, overflowX: "auto",
      }}>
        {liveSessions.map((s) => {
          const attn = sessionAttentionStatus(s);
          const isActive = s.id === currentSession.id;
          return (
            <div
              key={s.id}
              onClick={() => {
                setLocalActive(s.id);
                onSelectSession(s.id);
              }}
              style={{
                display: "flex", alignItems: "center", gap: 6,
                padding: "4px 10px", cursor: "pointer",
                borderRight: "1px solid #2a2a2b",
                fontSize: 12, whiteSpace: "nowrap", userSelect: "none",
                color: isActive ? "#d4d4d4" : "#858585",
                background: isActive ? "#1e1e1e" : "transparent",
                borderBottom: isActive ? "1px solid #1e1e1e" : "none",
              }}
            >
              <span style={{
                width: 8, height: 8, borderRadius: "50%",
                background: statusDotColor(attn), flexShrink: 0,
              }} />
              <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{s.label}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onKillSession(s.id);
                }}
                style={{
                  border: "none", background: "none", color: "#666",
                  cursor: "pointer", fontSize: 14, padding: "0 2px", lineHeight: 1,
                }}
                title="Kill session"
              >
                &times;
              </button>
            </div>
          );
        })}
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
        <CenterPanel
          backendStatus={backendStatus}
          sessionId={currentSession.id}
          embedded
        />
      </div>
    </div>
  );
}

export default TerminalPanel;
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/components/TerminalPanel.tsx
git commit -m "feat: add TerminalPanel with per-session tabs and attention styling"
```

---

### Task 6: Create DockviewApp component

**Files:**
- Create: `apps/desktop/src/components/DockviewApp.tsx`

- [ ] **Step 1: Write DockviewApp.tsx**

```tsx
import { DockviewReact, type DockviewReadyEvent } from "dockview-react";
import "dockview-react/dist/styles/dockview.css";
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
          const sessionsPanel = event.api.addPanel({
            id: "sessions",
            title: "Sessions",
            component: "sessions",
            initialWidth: 260,
          });

          event.api.addPanel({
            id: "detail",
            title: "Detail",
            component: "detail",
            position: { referencePanel: sessionsPanel, direction: "below" },
            initialHeight: 240,
          });

          const terminalPanel = event.api.addPanel({
            id: "terminal",
            title: "Terminal",
            component: "terminal",
            position: { referencePanel: sessionsPanel, direction: "right" },
            initialWidth: 800,
          });

          const capacityPanel = event.api.addPanel({
            id: "capacity",
            title: "Capacity",
            component: "capacity",
            position: { referencePanel: terminalPanel, direction: "right" },
            initialWidth: 250,
          });

          event.api.addPanel({
            id: "recommendations",
            title: "Start Next Task",
            component: "recommendations",
            position: { referencePanel: capacityPanel, direction: "below" },
            initialHeight: 220,
          });
        }}
      />
    </div>
  );
}

export default DockviewApp;
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/components/DockviewApp.tsx
git commit -m "feat: add DockviewApp with initial dock panels"
```

---

### Task 7: Update App.tsx to use DockviewApp

**Files:**
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Replace layout in App.tsx**

At the top of `App.tsx`, change the React import and component imports to:

```tsx
import { useCallback, useEffect, useState } from "react";
import DockviewApp from "./components/DockviewApp";
import { sortSessions } from "./components/RightSidebarHelpers";
```

Remove these imports from `App.tsx`:
- `import LeftSidebar from "./components/LeftSidebar";`
- `import RightSidebar from "./components/RightSidebar";`
- `import TerminalTabs from "./components/TerminalTabs";`
- `import type { TerminalTabsHandle } from "./components/TerminalTabs";`
- `import { sessionAttentionStatus } from "./components/RightSidebarHelpers";`

Remove `const terminalTabsRef = useRef<TerminalTabsHandle>(null);`

Remove the local `stateOrder` constant.

Replace the sort block in `refreshSessions` with:

```tsx
setSessions(sortSessions(list));
```

Replace the return JSX in `App.tsx` with:

```tsx
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
    />
  </div>
);
```

The session ordering must come from `sortSessions` so the list follows the design priority: `waiting_for_input`, `blocked`, `failed`, `done`, `stale`, `working`, `idle`, then lifecycle-only states.

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "refactor: replace three-column layout with DockviewApp"
```

---

### Task 8: Update CSS for dockview dark theme

**Files:**
- Modify: `apps/desktop/src/App.css`
- Modify: `apps/desktop/src/main.tsx` (add dockview CSS import)

- [ ] **Step 1: Remove old layout CSS, add dockview overrides**

Remove from `App.css` — these classes are no longer needed (the flexbox three-column layout):
- `.app-layout` (lines 59-63)
- `.panel` (lines 65-69)
- `.left-sidebar` (lines 71-76)
- `.center-panel` (lines 78-88)
- `.right-sidebar` (lines 169-174)
- `.terminal-tabs` (lines 176-180)
- `.terminal-tabs-empty` (lines 182-189)
- `.terminal-tab-bar` (lines 190-197)
- `.terminal-tab` (lines 199-210)
- `.terminal-tab:hover` (lines 212-215)
- `.terminal-tab--active` (lines 217-221)
- `.terminal-tab-dot` (lines 223-229)
- `.terminal-tab-label` (lines 231-234)
- `.terminal-tab-close` (lines 236-248)
- `.terminal-tab-content` (lines 250-254)
- `.overview-list` (lines 478-481)
- `.overview-group` (lines 483-485)
- `.overview-group-header` (lines 487-498)
- `.overview-card` (lines 500-520)
- `.overview-card--active` (lines 522-524)
- `.overview-card--done` (lines 526-527)
- `.overview-card-main` (lines 529-532)
- `.overview-alert` (lines 534-536)
- `.overview-card-label` (lines 538-545)
- `.overview-card-meta` (lines 547-551)

**Keep** (still used by session list, detail panel, workspace header, etc.):
- `*`, `html, body, #root`, `.app-shell`, `.titlebar`, `.status-badge`
- `.panel-header`, `.session-new-button`, `.session-list`, `.session-item*`, `.session-status*`, `.session-item-*`
- `.session-kill-button`, `.panel-content`, `.empty-state`
- `.workspace-*`
- `.session-detail*`, `.conflict-warning`, `.recommendation-text`, `.overview-card-badge`
- `.terminal-shell`, `.terminal-toolbar`, `.terminal-title`, `.terminal-subtitle`, `.terminal-actions`, `.terminal-launch-button`, `.terminal-container`, `.terminal-container--ended`, `.center-placeholder`

Append dockview dark theme overrides to `App.css`:

```css
/* dockview dark theme overrides */
.orkworks-dockview {
  --dv-background-color: #1e1e1e;
  --dv-paneview-active-outline-color: #4b6b7f;
  --dv-active-group-visible-border-color: #3c3c3c;
  --dv-drag-over-background-color: #2a2a2b;
  --dv-drag-over-border-color: #4b6b7f;
  --dv-tabs-and-actions-container-background-color: #252526;
  --dv-tab-divider-color: #2a2a2b;
  --dv-active-tab-background-color: transparent;
  --dv-inactive-tab-background-color: transparent;
  --dv-tab-font-size: 12px;
  --dv-tab-font-weight: 400;
  --dv-active-tab-color: #d4d4d4;
  --dv-inactive-tab-color: #858585;
  --dv-separator-border: #3c3c3c;
  --dv-paneview-separator-border-color: #3c3c3c;
  --dv-tabs-and-actions-container-font-size: 12px;
  --dv-sash-size: 4px;
  --dv-sash-hover-size: 4px;
}

.orkworks-dockview .dv-groupview {
  background: #1e1e1e;
}

.orkworks-dockview .dv-tab {
  text-transform: none;
}

.orkworks-dockview .dv-resize-handle {
  background: #3c3c3c;
}
```

- [ ] **Step 2: Add dockview CSS import to main.tsx**

Add to `apps/desktop/src/main.tsx` after `import "./App.css";`:
```tsx
import "dockview-react/dist/styles/dockview.css";
```

**Note:** dockview CSS is already imported by `DockviewApp.tsx`. The import in `main.tsx` ensures it's available globally.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/App.css apps/desktop/src/main.tsx
git commit -m "refactor: remove three-column layout CSS, add dockview dark theme"
```

---

### Task 9: Clean up unused files

**Files:**
- Remove: `apps/desktop/src/components/SessionView.tsx`

- [ ] **Step 1: Stage SessionView.tsx deletion**

```bash
git rm apps/desktop/src/components/SessionView.tsx
```

**Note:** `LeftSidebar.tsx` and `RightSidebar.tsx` are kept for now — they're referenced by existing tests. Remove them in a follow-up after tests are migrated to the new panel components.

- [ ] **Step 2: Commit**

```bash
git commit -m "chore: remove unused SessionView component"
```

---

### Task 10: Write tests for new components

**Files:**
- Create: `apps/desktop/tests/dockview.test.ts`
- Modify: `apps/desktop/tests/rightSidebar.test.ts`

- [ ] **Step 1: Write tests for new helpers**

Replace `apps/desktop/tests/rightSidebar.test.ts` with:

```typescript
import test from "node:test";
import assert from "node:assert/strict";

import {
  attentionBorderColor,
  borderColor,
  isLive,
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
  sourceColor,
  statusDotColor,
} from "../src/components/RightSidebarHelpers.ts";
import type { SessionInfo } from "../src/api.ts";

test("needsAttention returns true for blocked, failed, waiting_for_input", () => {
  assert.equal(needsAttention("blocked"), true);
  assert.equal(needsAttention("failed"), true);
  assert.equal(needsAttention("waiting_for_input"), true);
});

test("needsAttention returns false for running, creating, ended", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("creating"), false);
  assert.equal(needsAttention("ended"), false);
});

test("isLive returns true for running and creating", () => {
  assert.equal(isLive("running"), true);
  assert.equal(isLive("creating"), true);
  assert.equal(isLive("ended"), false);
  assert.equal(isLive("killed"), false);
});

test("attentionBorderColor returns correct colors", () => {
  assert.equal(attentionBorderColor("waiting_for_input"), "#cc4444");
  assert.equal(attentionBorderColor("failed"), "#cc4444");
  assert.equal(attentionBorderColor("blocked"), "#d4d44e");
  assert.equal(attentionBorderColor("done"), "#4ec94e");
  assert.equal(attentionBorderColor("stale"), "#4a4a4a");
  assert.equal(attentionBorderColor("idle"), "#4a4a4a");
  assert.equal(attentionBorderColor("working"), "#3c3c3c");
});

test("borderColor delegates to attentionBorderColor for compatibility", () => {
  assert.equal(borderColor("waiting_for_input"), "#cc4444");
  assert.equal(borderColor("blocked"), "#d4d44e");
  assert.equal(borderColor("done"), "#4ec94e");
  assert.equal(borderColor("running"), "#3c3c3c");
});

test("statusDotColor returns correct colors per attention status", () => {
  assert.equal(statusDotColor("waiting_for_input"), "#cc4444");
  assert.equal(statusDotColor("failed"), "#cc4444");
  assert.equal(statusDotColor("blocked"), "#d4d44e");
  assert.equal(statusDotColor("done"), "#4ec94e");
  assert.equal(statusDotColor("stale"), "#666");
  assert.equal(statusDotColor("idle"), "#666");
  assert.equal(statusDotColor("working"), "#4ec94e");
  assert.equal(statusDotColor("running"), "#4ec94e");
  assert.equal(statusDotColor("creating"), "#4ec94e");
  assert.equal(statusDotColor("unknown-status"), "#666");
});

test("sourceColor returns agent/peon colors or default", () => {
  assert.equal(sourceColor("agent"), "#4ec94e");
  assert.equal(sourceColor("peon"), "#57c7ff");
  assert.equal(sourceColor("process"), "#858585");
  assert.equal(sourceColor(undefined), "#858585");
});

test("sortSessions sorts by design attention priority then label", () => {
  const sessions: SessionInfo[] = [
    { id: "running", label: "H running", status: "running", cwd: "/tmp", created_at: "now" },
    { id: "blocked", label: "B blocked", status: "running", observedStatus: "blocked", cwd: "/tmp", created_at: "now" },
    { id: "idle", label: "G idle", status: "running", observedStatus: "idle", cwd: "/tmp", created_at: "now" },
    { id: "done", label: "D done", status: "running", observedStatus: "done", cwd: "/tmp", created_at: "now" },
    { id: "failed", label: "C failed", status: "running", observedStatus: "failed", cwd: "/tmp", created_at: "now" },
    { id: "stale", label: "E stale", status: "running", observedStatus: "stale", cwd: "/tmp", created_at: "now" },
    { id: "working", label: "F working", status: "running", observedStatus: "working", cwd: "/tmp", created_at: "now" },
    { id: "waiting", label: "A waiting", status: "running", observedStatus: "waiting_for_input", cwd: "/tmp", created_at: "now" },
  ];

  assert.deepEqual(sortSessions(sessions).map((s) => s.id), [
    "waiting",
    "blocked",
    "failed",
    "done",
    "stale",
    "working",
    "idle",
    "running",
  ]);
});

test("sessionAttentionStatus prefers observed status over lifecycle status", () => {
  const session: SessionInfo = {
    id: "1",
    label: "Running session needing input",
    status: "running",
    observedStatus: "waiting_for_input",
    cwd: "/tmp",
    created_at: "now",
  };

  assert.equal(sessionAttentionStatus(session), "waiting_for_input");
  assert.equal(needsAttention(sessionAttentionStatus(session)), true);
});
```

- [ ] **Step 2: Write Dockview integration source tests**

Create `apps/desktop/tests/dockview.test.ts`:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { SessionInfo } from "../src/api.ts";
import {
  needsAttention,
  sessionAttentionStatus,
  sortSessions,
  statusDotColor,
} from "../src/components/RightSidebarHelpers.ts";

test("DockviewApp registers panels through onReady instead of unsupported defaultLayout", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /onReady=\{\(event: DockviewReadyEvent\) =>/);
  assert.doesNotMatch(source, /defaultLayout=/);
  assert.match(source, /event\.api\.addPanel/);
});

test("DockviewApp registers the five expected panel ids", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  for (const id of ["sessions", "detail", "terminal", "capacity", "recommendations"]) {
    assert.match(source, new RegExp(`id: "${id}"`));
  }
});

test("SessionDetailPanel includes the core detail sections", () => {
  const source = readFileSync(new URL("../src/components/SessionDetailPanel.tsx", import.meta.url), "utf8");

  for (const label of ["Task", "Status", "Directory", "Git", "Source", "Peon"]) {
    assert.match(source, new RegExp(`>${label}<`));
  }
  assert.match(source, /Select a session to see details/);
});

test("session list sorts by attention priority with lifecycle fallback", () => {
  const sessions: SessionInfo[] = [
    { id: "1", label: "s1", status: "running", cwd: "/tmp", created_at: "now" },
    { id: "2", label: "s2", status: "running", observedStatus: "waiting_for_input", cwd: "/tmp", created_at: "now" },
    { id: "3", label: "s3", status: "ended", cwd: "/tmp", created_at: "now" },
    { id: "4", label: "s4", status: "running", observedStatus: "failed", cwd: "/tmp", created_at: "now" },
    { id: "5", label: "s5", status: "running", observedStatus: "blocked", cwd: "/tmp", created_at: "now" },
    { id: "6", label: "s6", status: "running", observedStatus: "done", cwd: "/tmp", created_at: "now" },
  ];
  const sorted = sortSessions(sessions);
  assert.equal(sorted[0].id, "2"); // waiting_for_input
  assert.equal(sorted[1].id, "5"); // blocked
  assert.equal(sorted[2].id, "4"); // failed
  assert.equal(sorted[3].id, "6"); // done
  assert.equal(sorted[4].id, "1"); // running
  assert.equal(sorted[5].id, "3"); // ended
});

test("ended sessions do not have live status dot", () => {
  assert.equal(statusDotColor("ended"), "#666");
  assert.equal(statusDotColor("killed"), "#666");
  assert.equal(statusDotColor("error"), "#666");
});

test("sessionAttentionStatus falls back to lifecycle status", () => {
  const session: SessionInfo = {
    id: "1", label: "test", status: "running", cwd: "/tmp", created_at: "now",
  };
  assert.equal(sessionAttentionStatus(session), "running");
});

test("needsAttention lifecycle statuses do not trigger from raw lifecycle", () => {
  assert.equal(needsAttention("running"), false);
  assert.equal(needsAttention("ended"), false);
  assert.equal(needsAttention("creating"), false);
});
```

- [ ] **Step 3: Run all tests and verify they pass**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/rightSidebar.test.ts tests/dockview.test.ts
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/tests/rightSidebar.test.ts apps/desktop/tests/dockview.test.ts
git commit -m "test: add attention sorting and helper unit tests"
```

---

### Task 11: TypeScript type-check and verify

**Files:**
- (none — verification only)

- [ ] **Step 1: Run TypeScript type check**

```bash
cd apps/desktop && npx tsc --noEmit
```

Expected: No errors. Fix any type issues before proceeding.

- [ ] **Step 2: Run all existing tests**

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: All existing and new tests pass.

- [ ] **Step 3: Verify the build compiles**

```bash
cd apps/desktop && pnpm build
```

Expected: Vite build succeeds with no errors.

- [ ] **Step 4: Run doc currency check**

```bash
bash .claude/hooks/doc-check.sh
```

Address any flagged doc files.

- [ ] **Step 5: Commit any fixes**

If type errors or test failures required fixes, commit them:

```bash
git add -A
git commit -m "fix: type errors and test fixes from dockview migration"
```

---

### Task 12: Update AGENTS.md to reflect removed dependency

**Files:**
- Modify: `AGENTS.md`
- Modify: `README.md`

- [ ] **Step 1: Check if react-resizable-panels is referenced in docs**

```bash
grep -ri "resizable-panels\|react-resizable" AGENTS.md README.md docs/
```

Expected: If matches found, update to reference dockview instead.

- [ ] **Step 2: Commit doc updates**

```bash
git add AGENTS.md README.md
git commit -m "docs: update for dockview migration"
```
