# Dockview Header Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Dockview-owned Sessions header the only visible header row, move the create-session `+` action into that Dockview header, and remove the duplicate inner Sessions header from the panel body.

**Architecture:** Keep Dockview in charge of header chrome and tabs, and keep `SessionListPanel` responsible only for list content. Implement the add button through Dockview's `rightHeaderActionsComponent`, then tighten the Dockview header CSS to match the old subheader visual language.

**Tech Stack:** React 19, TypeScript, `dockview-react`, app-local CSS, Node built-in test runner

---

## File Structure

- Modify: `apps/desktop/src/components/DockviewApp.tsx`
  Why: this is where Dockview is configured, so it is the right place to attach a Sessions-only header action and ensure panels get explicit titles.
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
  Why: this component currently renders the duplicate `.panel-header` and owns the inline `+` button that should move into Dockview.
- Modify: `apps/desktop/src/App.css`
  Why: Dockview header styling and the old `.panel-header` styles live here.
- Modify: `apps/desktop/tests/dockview.test.ts`
  Why: existing UI-structure tests already verify Dockview wiring through source inspection and are the cheapest place to lock in the regression coverage for this cleanup.

### Task 1: Lock In the New Header Ownership With Failing Tests

**Files:**
- Modify: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Write the failing tests for the unified header behavior**

Add these tests near the existing Dockview and SessionListPanel source-inspection checks in `apps/desktop/tests/dockview.test.ts`:

```ts
test("DockviewApp exposes a right-side header action for the Sessions panel", () => {
  const source = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /rightHeaderActionsComponent=\{SessionsHeaderActions\}/);
  assert.match(source, /activePanel\?\.id !== PANEL_DEFAULTS\.sessions\.component/);
  assert.match(source, /dockview-header-action/);
});

test("SessionListPanel no longer renders duplicate header chrome", () => {
  const source = readFileSync(
    new URL("../src/components/SessionListPanel.tsx", import.meta.url),
    "utf8",
  );

  assert.doesNotMatch(source, /className="panel-header"/);
  assert.doesNotMatch(source, /className="session-new-button"/);
  assert.doesNotMatch(source, /onCreateSession:/);
});
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: FAIL because `DockviewApp.tsx` does not yet expose `rightHeaderActionsComponent={SessionsHeaderActions}`, and `SessionListPanel.tsx` still contains `className="panel-header"` plus the `onCreateSession` prop.

- [ ] **Step 3: Commit the test-only red state**

```bash
git add apps/desktop/tests/dockview.test.ts
git commit -m "test: cover dockview sessions header unification"
```

### Task 2: Move the Sessions Create Action Into the Dockview Header

**Files:**
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Implement a Sessions-only Dockview header action**

Update `apps/desktop/src/components/DockviewApp.tsx` so it imports `IDockviewHeaderActionsProps`, defines a `SessionsHeaderActions` component, passes it to `DockviewReact`, and applies panel titles when building the default layout:

```tsx
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

// ...existing DockviewContext and panel components...

function SessionsHeaderActions(props: IDockviewHeaderActionsProps) {
  const ctx = useContext(DockviewContext);

  if (props.activePanel?.id !== PANEL_DEFAULTS.sessions.component) {
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
        position: {
          referencePanel: def.position.referencePanel,
          direction: def.position.direction,
        },
      });
    }
  }
}

// inside <DockviewReact ... />
<DockviewReact
  components={COMPONENTS}
  className="orkworks-dockview"
  rightHeaderActionsComponent={SessionsHeaderActions}
  onReady={(event: DockviewReadyEvent) => {
    // existing onReady body unchanged
  }}
/>
```

- [ ] **Step 2: Remove the duplicate inner Sessions header from the panel body**

Update `apps/desktop/src/components/SessionListPanel.tsx` to drop the `onCreateSession` prop entirely and render only list content:

```tsx
interface SessionListPanelProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onKillSession: (id: string) => void;
}

function SessionListPanel({
  sessions,
  activeSessionId,
  onSelectSession,
  onKillSession,
}: SessionListPanelProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="panel-content">
        {sessions.length === 0 ? (
          <p className="empty-state">No active sessions</p>
        ) : (
          <ul className="session-list">
            {sessions.map((s) => {
              const attn = sessionAttentionStatus(s);
              return (
                <li
                  key={s.id}
                  className={[
                    "session-item",
                    s.id === activeSessionId ? "session-item--active" : "",
                    s.memoryState !== "live" ? "session-item--remembered" : "",
                    s.memoryState === "resumable" ? "session-item--resumable" : "",
                  ].filter(Boolean).join(" ")}
                  style={{ borderLeft: `3px solid ${attentionBorderColor(attn)}` }}
                  onClick={() => onSelectSession(s.id)}
                >
                  {/* existing session item body unchanged */}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
```

Also update the `SessionsPanel` call site inside `DockviewApp.tsx`:

```tsx
<SessionListPanel
  sessions={ctx.sessions}
  activeSessionId={ctx.activeSessionId}
  onSelectSession={ctx.onSelectSession}
  onKillSession={ctx.onKillSession}
/>
```

- [ ] **Step 3: Run the focused tests to verify the new structure passes**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: PASS for the two new tests plus the existing Dockview structure assertions.

- [ ] **Step 4: Commit the Dockview wiring change**

```bash
git add apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/SessionListPanel.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat: move sessions add action into dockview header"
```

### Task 3: Match the Dockview Header Styling to the Old Subheader Look

**Files:**
- Modify: `apps/desktop/src/App.css`
- Test: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Add a focused CSS regression check for the new header action class**

Append this source-inspection test to `apps/desktop/tests/dockview.test.ts`:

```ts
test("App.css defines dockview header action styling for the unified sessions header", () => {
  const source = readFileSync(new URL("../src/App.css", import.meta.url), "utf8");

  assert.match(source, /\.dockview-header-action/);
  assert.match(source, /\.orkworks-dockview \.dv-tabs-and-actions-container/);
});
```

- [ ] **Step 2: Run the focused tests to verify the CSS assertion fails first**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: FAIL because `.dockview-header-action` is not yet defined in `App.css`.

- [ ] **Step 3: Update the Dockview header CSS and remove the dead inner-header rule**

Replace the unscoped Dockview header rules in `apps/desktop/src/App.css` with scoped versions and remove the unused `.panel-header` / `.session-new-button` styles:

```css
.orkworks-dockview .dv-tabs-and-actions-container {
  background: #1f1f20;
  border-bottom: 1px solid #3c3c3c;
  min-height: 35px;
}

.orkworks-dockview .dv-tab .dv-default-tab .dv-default-tab-content {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #999;
}

.dockview-header-action {
  border: none;
  background: none;
  color: #999;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}

.dockview-header-action:hover {
  color: #d4d4d4;
}
```

Also delete these no-longer-used selectors:

```css
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #999;
  border-bottom: 1px solid #3c3c3c;
}

.session-new-button {
  border: none;
  background: none;
  color: #999;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}

.session-new-button:hover {
  color: #d4d4d4;
}
```

- [ ] **Step 4: Run the focused tests again to verify the CSS cleanup passes**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts
```

Expected: PASS, including the new CSS source-inspection check.

- [ ] **Step 5: Commit the styling cleanup**

```bash
git add apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts
git commit -m "style: unify dockview sessions header appearance"
```

### Task 4: Final Verification and Repository Hygiene

**Files:**
- Verify only: `apps/desktop/src/components/DockviewApp.tsx`
- Verify only: `apps/desktop/src/components/SessionListPanel.tsx`
- Verify only: `apps/desktop/src/App.css`
- Verify only: `apps/desktop/tests/dockview.test.ts`

- [ ] **Step 1: Run the full desktop test command for this scope**

Run:

```bash
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
```

Expected: PASS with no new failures outside the Dockview tests.

- [ ] **Step 2: Manually verify the visible UI behavior**

Run:

```bash
cd apps/desktop && pnpm dev
```

Verify in the app:

- only one visible `Sessions` header row remains
- the `+` button appears in the Dockview header, not in the list body
- clicking `+` still creates a session
- the session list starts immediately below the Dockview header
- other panels still render normal Dockview headers

- [ ] **Step 3: Run the repo doc currency check before closing the task**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: either no output, or a short list of docs to update before completion.

- [ ] **Step 4: Create the final implementation commit**

```bash
git add apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/SessionListPanel.tsx apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts
git commit -m "feat: unify dockview sessions header"
```
