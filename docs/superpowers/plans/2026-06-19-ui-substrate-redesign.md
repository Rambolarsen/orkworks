# UI Substrate Redesign — Implementation Plan

**Date**: 2026-06-19
**Scope**: `apps/desktop/src/` only. Electron main, preload, IPC, and Rust sidecar are out of scope.
**Audit reference**: Dieter Rams audit, total 10/30 → REDESIGN verdict (see brief at top of conversation; see also observations [3734](evidence), [3735](scorecard), [3736](verdict), [3737](handoff)).

---

## What this plan delivers

A re-derived UI substrate for the OrkWorks desktop renderer, preserving the IA (sessions list as multi-view; single active terminal as context primitive; switch-to-change-context). The substrate work covers:

1. A real design-token layer (`tokens.css` + `:focus-visible`).
2. A canonical vocabulary (`labels.ts` + copy-style guide), with every internal enum routed through it.
3. A sessions-list "situational-awareness dashboard" row (label, attention, last activity, agent action, source confidence).
4. One default-layout builder, one shared empty-state component, one feedback primitive.
5. Deletion of confirmed dead components and removal of placeholder panels from the default layout.
6. A layout-migration cutover so users with a stored layout pointing at Capacity/Recommendations don't get a broken default.

The single-active-terminal model is **load-bearing** and is preserved verbatim. No multi-terminal / tile / split / preview pattern is introduced under any name.

---

## Phase 0 — Documentation discovery (pre-flight, completed)

These facts were verified directly from the working tree on 2026-06-19. Each phase below cites them; you do **not** need to re-discover them.

### 0.1 — Files actually dead vs. files just unused-as-default

Verified via `grep -rn` across `apps/desktop/src`:

| File | Status | Verification |
| --- | --- | --- |
| `components/RightSidebar.tsx` | **Truly dead** — zero importers. | `grep -rn RightSidebar apps/desktop/src` returns only its own declaration. |
| `components/TerminalTabs.tsx` | **Truly dead** — zero importers. | `grep -rn TerminalTabs apps/desktop/src` returns only its own declaration. |
| `components/LeftSidebar.tsx` | **Truly dead** — file body is `export { default } from "./SessionListPanel";` and nothing imports `LeftSidebar`. | `grep -rn LeftSidebar apps/desktop/src` returns only the file itself. |
| `components/RightSidebarHelpers.ts` | **STILL IMPORTED** — `sortSessions` (App.tsx:4,73), and `sessionAttentionStatus`/`sourceColor`/`statusDotColor`/`attentionBorderColor`/`needsAttention` from SessionListPanel.tsx:3-8 and SessionDetailPanel.tsx:2. | Cannot delete; must be renamed/relocated. The audit's "Delete this file" instruction is wrong for this entry — it gets relocated. |
| `components/CapacityPanel.tsx` | **Kept** — referenced by `PANEL_DEFAULTS` and electron hotkey `CmdOrCtrl+Shift+C`. Removed from default layout only. |
| `components/RecommendationsPanel.tsx` | **Kept** — referenced by `PANEL_DEFAULTS` and electron hotkey `CmdOrCtrl+Shift+R`. Removed from default layout only. |

### 0.2 — Electron main wires hotkeys for all 5 panels (preserve)

`apps/desktop/electron/main.ts:21-46` wires View menu items and accelerators for `sessions`, `detail`, `terminal`, `capacity`, `recommendations`. Reset Layout sends `{ action: "reset-layout" }`. The renderer must keep accepting `focus`/`reset-layout` IPC for **all five panels** even though only some open by default; the electron menu is **not in scope** for this redesign.

### 0.3 — Enum sets to label

Verified from `apps/desktop/src/api.ts:1-2,38-39` and `RightSidebarHelpers.ts:3-16`:

- `memoryState: "live" | "remembered" | "resumable" | "unsupported"`
- `resumeStrategy: "exact" | "latest_cwd" | "latest_repo" | "none"`
- `attentionStatus` (from `ATTENTION_PRIORITY` keys): `waiting_for_input | blocked | failed | done | stale | working | idle | creating | running | ended | killed | error`
- `metadataSource`: string from the sidecar; `sourceColor` handles `"agent"`, `"peon"`, and falls through. **No typed union exists**; the label table must default-case unknown values.

### 0.4 — Duplicate default-layout builders

Two builders today, both 5-panel, drift risk confirmed:
- `DockviewApp.tsx:143-160` — `buildDefaultLayout(api)`, called on first launch and on the empty-state "Reset Layout" button (`DockviewApp.tsx:131-134`).
- `App.tsx:260-268` — inline 5-panel rebuild in response to the electron `reset-layout` IPC.

Both currently open `sessions + detail + terminal + capacity + recommendations`.

### 0.5 — Where every hex literal lives (token migration scope)

`App.css` and these inline-style sites must migrate to tokens:
- `App.css` — all `#hex` and `px` literals (see read at `App.css:1-494`).
- `components/SessionDetailPanel.tsx:44-49, 65-74, 82, 105-110, 117-118`.
- `components/SessionListPanel.tsx:145, 150, 162-167`.
- `components/TerminalPanel.tsx:17-18, 25-29, 30, 36-37`.
- `components/CenterPanel.tsx:117-123`.
- `components/CapacityPanel.tsx:3-7, 10-13`.
- `components/RecommendationsPanel.tsx:3-7, 10-13`.
- `components/RightSidebarHelpers.ts:46-67` — color helpers move into the token layer.

### 0.6 — Catch-blocks that silently swallow user-facing errors

`App.tsx:53-55, 58-60, 73-76, 95-97, 112-114, 137-139, 182-184`. The brief calls out 5; verification on file gives 7 sites (two are health-check retries — internal — and route through `backendStatus` already). The 5 user-facing handlers to route through a feedback primitive are: `refreshSessions`, `handleOpenWorkspace`, `handleCreateSession`, `handleKillSession`, `persistActiveSession`.

### 0.7 — Empty states to collapse

Four copies of "nothing here":
- `SessionListPanel.tsx:80` ("Open a workspace to begin")
- `SessionListPanel.tsx:116` ("No active sessions")
- `TerminalPanel.tsx:18` ("No active terminal")
- `SessionDetailPanel.tsx:16` ("Select a session to see details")
- `CenterPanel.tsx:117-123` ("OrkWorks / Mission Control for AI Agents / backend: …")

The empty-state recovery mismatch is at `DockviewApp.tsx:207-209` (hint "Open one from the View menu") + `DockviewApp.tsx:210-219` (button "Reset Layout").

### 0.8 — Naming drift to converge on "Workspace"

- Titlebar button label: "Open Folder" (`App.tsx:302`).
- Titlebar switch button title: "Switch workspace" (`App.tsx:289`).
- Sessions list empty state: "Open a workspace to begin" (`SessionListPanel.tsx:80`).
- Electron dialog title: "Select Workspace" (`main.ts:251`) — out of scope but consistent.

Pick one word: **Workspace**. Button becomes "Open workspace…".

### 0.9 — APIs we are using (no invented methods)

- `DockviewApi` from `dockview-react`: `addPanel`, `getPanel`, `clear`, `fromJSON`, `toJSON`, `totalPanels`, `onDidLayoutChange`. Used per `DockviewApp.tsx:131-201`. **No other Dockview methods are required.**
- xterm: existing terminalStore module boundary is preserved (`terminalStore.ts:1-136`). No new xterm APIs.

### 0.10 — Anti-patterns to refuse

- Don't introduce any `*Tabs.tsx`, `*Split.tsx`, `*Tiled.tsx`, "preview", "picture-in-picture", or any pattern that renders more than one xterm instance visible at once. Session = context; single-active is load-bearing.
- Don't add `light` mode or `prefers-color-scheme` — defer.
- Don't add `prefers-reduced-motion` for cursor blink — defer.
- Don't gate the redesign behind a flag — pick the cutover and migrate stored layouts (Phase 7).
- Don't keep `RightSidebarHelpers.ts` as a filename — its name describes a dead component. Rename in Phase 2.

---

## Phase 1 — Token layer + global focus

**Goal**: Stand up `tokens.css` and the `:focus-visible` rule. Nothing else changes visually yet; subsequent phases consume the tokens.

### What to implement

1. Create `apps/desktop/src/styles/tokens.css` with these scales (names are canonical for all later phases):

   ```css
   :root {
     /* Surfaces (dark, single mode for now) */
     --surface-0: #1e1e1e;   /* app background */
     --surface-1: #211d1b;   /* panel background (matches dv-background-color) */
     --surface-2: #262220;   /* panel headers, raised */
     --surface-3: #312c29;   /* hover/inactive tab */
     --surface-titlebar: #323233;

     /* Text */
     --text-primary: #d4d4d4;
     --text-secondary: #cccccc;
     --text-muted: #858585;
     --text-faint: #6e6e6e;
     --text-on-accent: #e8f0ff;

     /* Borders / separators */
     --border-default: #3c3c3c;
     --border-subtle: #2a2a2b;

     /* State */
     --state-ok: #4ec94e;
     --state-ok-bg: #1a3a1a;
     --state-warn: #d4d44e;
     --state-warn-bg: #3a3a1a;
     --state-error: #cc4444;
     --state-error-bg: #3a2a1a;
     --state-info: #57c7ff;
     --state-info-bg: #1d2f49;

     /* Attention (semantic alias of state) */
     --attention-needs-you: var(--state-error);
     --attention-blocked: var(--state-warn);
     --attention-done: var(--state-ok);
     --attention-working: var(--state-ok);
     --attention-idle: #4a4a4a;
     --attention-neutral: var(--border-default);

     /* Source */
     --source-agent: var(--state-ok);
     --source-peon: var(--state-info);
     --source-other: var(--text-muted);

     /* Accent / focus */
     --accent: #4b6b7f;
     --accent-strong: #5c879e;
     --accent-bg: #123241;
     --accent-bg-hover: #16465b;
     --accent-focus: #6aa6ff;

     /* Spacing scale */
     --space-1: 2px;
     --space-2: 4px;
     --space-3: 6px;
     --space-4: 8px;
     --space-5: 12px;
     --space-6: 16px;

     /* Type scale */
     --text-xs: 10px;
     --text-sm: 11px;
     --text-md: 12px;
     --text-lg: 13px;
     --text-xl: 14px;

     /* Radius */
     --radius-sm: 3px;
     --radius-md: 4px;
     --radius-lg: 6px;

     /* Type weight + face */
     --font-ui: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
                Ubuntu, Cantarell, "Fira Sans", "Droid Sans", "Helvetica Neue", sans-serif;
   }
   ```

2. Import `tokens.css` from `apps/desktop/src/main.tsx` **before** `App.css`.

3. In `App.css`:
   - Replace every `#hex`, hardcoded `px` value (color/spacing/type/border-radius), and the `dv-*` overrides (`App.css:458-481`) with the token names above. (Mapping table below.)
   - Delete the rule `outline: none;` on `.session-list` (`App.css:233`).
   - Add a single global focus rule:

     ```css
     :focus-visible {
       outline: 2px solid var(--accent-focus);
       outline-offset: 2px;
       border-radius: var(--radius-sm);
     }
     ```

4. Hex → token mapping for `App.css`:

   | Literal | Token |
   | --- | --- |
   | `#1e1e1e` | `--surface-0` |
   | `#211d1b` | `--surface-1` |
   | `#262220` | `--surface-2` |
   | `#312c29` | `--surface-3` |
   | `#252526` | `--surface-2` |
   | `#323233` | `--surface-titlebar` |
   | `#cccccc` | `--text-secondary` |
   | `#d4d4d4` | `--text-primary` |
   | `#858585` | `--text-muted` |
   | `#999`, `#6e6e6e`, `#666` | `--text-faint` (collapse to one) |
   | `#444` | hover state — replace with `var(--surface-3)` |
   | `#3c3c3c` | `--border-default` |
   | `#2a2a2b` | `--border-subtle` |
   | `#37373d` | active-row bg — add `--surface-active: #37373d` to tokens |
   | `#4ec94e`, `#1a3a1a` | `--state-ok`, `--state-ok-bg` |
   | `#d4d44e`, `#3a3a1a` | `--state-warn`, `--state-warn-bg` |
   | `#cc4444` | `--state-error` |
   | `#3a2a1a`, `#5a3a1a` | `--state-error-bg`, replace `#5a3a1a` border with `var(--state-error)` at 40% via `color-mix` if needed; otherwise drop |
   | `#57c7ff` | `--state-info` |
   | `#4b6b7f`, `#5c879e`, `#123241`, `#16465b`, `#d8edf7` | `--accent`, `--accent-strong`, `--accent-bg`, `--accent-bg-hover`, `--text-on-accent` |
   | `#3d5f8f`, `#1d2f49`, `#e8f0ff` | use `--accent` family (drop the third blue family) |
   | `#9fb7d7` | replace with `--text-muted` (orphan blue, low signal) |
   | `#6aa6ff` | `--accent-focus` |

5. Inline-style hex literals in `*.tsx` are **not migrated in this phase** — they're rewritten in phases 4–5 alongside their surrounding components.

### Documentation references

- Existing tokens in dockview overrides: `App.css:458-481` (these stay as `--dv-*` and are re-wired to point at our tokens, e.g. `--dv-background-color: var(--surface-1)`).
- The `outline: none` to delete: `App.css:229-234`.

### Verification checklist

- `grep -E '#[0-9a-fA-F]{3,8}' apps/desktop/src/App.css` returns **zero** matches (all colors now token vars; dockview `--dv-*` overrides resolve to token vars, not literals).
- `grep -E 'outline:\s*none' apps/desktop/src/App.css` returns zero matches.
- Run `cd apps/desktop && pnpm dev` and confirm the app still renders identically to before (this phase is purely substitution).
- Tab-focus a button: a visible focus ring appears.

### Anti-pattern guards

- No new `*.css` file other than `tokens.css`. Don't fan tokens out across files.
- Don't import `tokens.css` from each component — global import via `main.tsx` only.
- Don't introduce a JS/TS token module yet; CSS vars are the single source.

---

## Phase 2 — Vocabulary module (`labels.ts`) and helper rename

**Goal**: One module produces every human-readable string derived from an enum. Renames `RightSidebarHelpers.ts` so the file name no longer references a deleted component.

### What to implement

1. Create `apps/desktop/src/labels.ts`:

   ```ts
   import type { MemoryState, ResumeStrategy } from "./api";

   /** Canonical vocabulary. One word per concept. */
   export const VOCAB = {
     workspace: "Workspace",        // never "Folder"
     openWorkspace: "Open workspace…",
     switchWorkspace: "Switch workspace",
     session: "Session",
     terminal: "Terminal",
     newSession: "New session",
   } as const;

   /** Plain-language attention label. Pairs with attentionTone() for visual weight. */
   export function attentionLabel(status: string): string {
     switch (status) {
       case "waiting_for_input": return "Needs you";
       case "blocked":           return "Blocked";
       case "failed":            return "Failed";
       case "done":              return "Done";
       case "stale":             return "Idle";
       case "idle":              return "Idle";
       case "working":           return "Working";
       case "running":           return "Running";
       case "creating":          return "Starting";
       case "ended":             return "Ended";
       case "killed":            return "Killed";
       case "error":             return "Error";
       default:                  return "Unknown";
     }
   }

   export type AttentionTone = "needs-you" | "blocked" | "done" | "working" | "idle" | "neutral";

   export function attentionTone(status: string): AttentionTone {
     switch (status) {
       case "waiting_for_input": case "failed":      return "needs-you";
       case "blocked":                                return "blocked";
       case "done":                                   return "done";
       case "working": case "running": case "creating": return "working";
       case "stale": case "idle":                     return "idle";
       default:                                       return "neutral";
     }
   }

   export function memoryStateLabel(s: MemoryState): string {
     switch (s) {
       case "live":         return "Live";
       case "resumable":    return "Resumable";
       case "remembered":   return "Remembered";
       case "unsupported":  return "—";
     }
   }

   export function resumeActionLabel(strategy: ResumeStrategy): string {
     switch (strategy) {
       case "exact":        return "Resume session";
       case "latest_cwd":   return "Resume latest in folder";
       case "latest_repo":  return "Resume latest in repo";
       case "none":         return "Resume unavailable";
     }
   }

   export function sourceLabel(source: string | undefined): string {
     if (source === "agent") return "Agent";
     if (source === "peon")  return "Peon";
     if (!source)            return "Unknown";
     return source.charAt(0).toUpperCase() + source.slice(1);
   }

   /** "Agent · 100% confidence" */
   export function sourceWithConfidence(source: string | undefined, confidence: number | undefined): string {
     const c = Math.round((confidence ?? 1) * 100);
     return `${sourceLabel(source)} · ${c}% confidence`;
   }

   /** Relative-time formatting for "last activity". Local-only; no library. */
   export function relativeTime(iso: string | undefined, now: Date = new Date()): string {
     if (!iso) return "";
     const t = new Date(iso).getTime();
     if (Number.isNaN(t)) return "";
     const diffSec = Math.max(0, Math.round((now.getTime() - t) / 1000));
     if (diffSec < 5)     return "just now";
     if (diffSec < 60)    return `${diffSec}s ago`;
     if (diffSec < 3600)  return `${Math.round(diffSec / 60)}m ago`;
     if (diffSec < 86400) return `${Math.round(diffSec / 3600)}h ago`;
     return `${Math.round(diffSec / 86400)}d ago`;
   }
   ```

2. Rename `apps/desktop/src/components/RightSidebarHelpers.ts` → `apps/desktop/src/sessionSort.ts`. Move only the still-used exports:
   - `ATTENTION_PRIORITY`, `sessionAttentionStatus`, `sortSessions`, `needsAttention`.
   - **Delete** the color functions (`statusDotColor`, `attentionBorderColor`, `sourceColor`, `borderColor`, `isLive`): they're either unused (`isLive`, `borderColor`) or replaced by token-driven CSS in Phase 4–5.

3. Update import sites:
   - `App.tsx:4` → `import { sortSessions } from "./sessionSort";`
   - `SessionListPanel.tsx:3-8` → import only `sessionAttentionStatus`, `needsAttention` from `../sessionSort`. Color helpers go away.
   - `SessionDetailPanel.tsx:2` → same; `statusDotColor`/`sourceColor` go away.

4. **Do not yet** rewrite the JSX that consumed the deleted helpers. Leave a TypeScript error here; phases 4 and 5 fix them. (This guarantees a fail-fast check that you actually finished the conversion. If you'd rather keep main green, temporarily inline-color via tokens at the call site, but the next phase will rewrite those lines anyway.)

### Documentation references

- Enum sources of truth: `apps/desktop/src/api.ts:1-2` (memoryState, resumeStrategy types); `RightSidebarHelpers.ts:3-16` (attentionStatus universe).

### Verification checklist

- `grep -rn "RightSidebarHelpers" apps/desktop/src` returns zero matches.
- `grep -rn "from .*sessionSort" apps/desktop/src` shows three importers: `App.tsx`, `SessionListPanel.tsx`, `SessionDetailPanel.tsx`.
- `grep -rn "statusDotColor\|attentionBorderColor\|sourceColor\b" apps/desktop/src` returns zero matches (after phases 4–5 land).
- `node --experimental-strip-types --test tests/*.test.ts` still passes for label-pure tests; type-check failure on the panels is expected until Phase 4 / 5 lands.

### Anti-pattern guards

- Don't re-export anything from `labels.ts` as a default; named exports only — easier to grep.
- Don't fold enum types into `labels.ts` (those stay in `api.ts`).
- Don't add `i18n` framework. Single string table, English-only.
- Don't include "M8" / "M9" / "milestone" anywhere in user-visible copy. Use plain words like "Coming soon" if a stub renders at all.

---

## Phase 3 — One default layout, one empty state, delete dead components

**Goal**: Converge two layout builders into one. Drop placeholder panels from the default. Build the single `EmptyState` primitive. Delete the four truly-dead files.

### What to implement

1. Define the single canonical default layout in `DockviewApp.tsx`:

   ```ts
   // Single source of truth — Phase 3 deliverable.
   export const DEFAULT_LAYOUT_PANELS: ReadonlyArray<{ id: string; position?: PanelDefault["position"] }> = [
     { id: "sessions" },
     { id: "detail",   position: { referencePanel: "sessions", direction: "below" } },
     { id: "terminal", position: { referencePanel: "sessions", direction: "right" } },
   ];

   export function buildDefaultLayout(api: DockviewApi): void {
     for (const entry of DEFAULT_LAYOUT_PANELS) {
       const def = PANEL_DEFAULTS[entry.id];
       api.addPanel({
         id: def.component,
         component: def.component,
         title: def.title,
         ...(entry.position ? { position: entry.position } : {}),
       });
     }
   }
   ```

   Export `buildDefaultLayout` so `App.tsx` can call it instead of duplicating.

2. `App.tsx:260-268` `reset-layout` handler: replace the inline rebuild with:

   ```ts
   import { buildDefaultLayout } from "./components/DockviewApp";
   // …
   } else if (action === "reset-layout") {
     sessionsHiddenLayoutRef.current = null;
     api.clear();
     buildDefaultLayout(api);
   }
   ```

3. **Keep** `PANEL_DEFAULTS` entries for `capacity` and `recommendations` so the electron `View → Capacity` / `View → Recommendations` IPC paths in `App.tsx:197-256` continue to add panels on demand. Only their inclusion in the default layout is removed.

4. Build the empty-state primitive `apps/desktop/src/components/EmptyState.tsx`:

   ```tsx
   interface EmptyStateProps {
     message: string;
     action?: { label: string; onClick: () => void };
   }

   export default function EmptyState({ message, action }: EmptyStateProps) {
     return (
       <div className="empty-state-block">
         <p className="empty-state-text">{message}</p>
         {action && (
           <button type="button" className="empty-state-action" onClick={action.onClick}>
             {action.label}
           </button>
         )}
       </div>
     );
   }
   ```

   Add styles to `App.css` using tokens (`--space-*`, `--text-md`, `--text-muted`).

5. Replace the four empty-state copies (`SessionListPanel.tsx:80,116`, `TerminalPanel.tsx:15-21`, `SessionDetailPanel.tsx:13-19`) with `<EmptyState message="…" />`. Canonical strings:
   - No workspace open: `"Open a workspace to see sessions."` with action `{ label: "Open workspace…", onClick: handleOpenWorkspace }`.
   - Workspace open, no sessions yet: `"No sessions yet. Press ⌘N to start one."`
   - No session selected (detail): `"Select a session to see details."`
   - No session selected (terminal panel): `"Select a session to open its terminal."`
   - Backend not connected (CenterPanel `:114-126`): collapses to `<EmptyState message="Connecting to OrkWorks…" />`. **Delete** "Mission Control for AI Agents" tagline and `backend:` debug line.

6. Fix `DockviewApp.tsx:204-219` empty-state-overlay copy and remove the menu-hint mismatch. Final:

   ```tsx
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
   ```

   Drop `.dockview-empty-hint` paragraph and its CSS rule.

7. Delete dead files:
   - `apps/desktop/src/components/RightSidebar.tsx`
   - `apps/desktop/src/components/LeftSidebar.tsx`
   - `apps/desktop/src/components/TerminalTabs.tsx`

   `RightSidebarHelpers.ts` is **already renamed/relocated in Phase 2** — do not delete it; that would be a regression.

### Documentation references

- Reset-layout duplicates: `DockviewApp.tsx:143-160` and `App.tsx:260-268` (this phase collapses them).
- Empty-state sites: `SessionListPanel.tsx:80,116`, `TerminalPanel.tsx:15-21`, `SessionDetailPanel.tsx:13-19`, `CenterPanel.tsx:114-126`, `DockviewApp.tsx:204-221`.
- Dockview API used: `clear()`, `addPanel()`, `totalPanels` — already in use, no new methods.
- Electron menu wiring stays as-is at `electron/main.ts:21-46`.

### Verification checklist

- `grep -rn "RightSidebar\|LeftSidebar\|TerminalTabs" apps/desktop/src` returns zero matches (helpers file is `sessionSort.ts` now).
- `grep -rn "buildDefaultLayout" apps/desktop/src` returns exactly two: the export site in `DockviewApp.tsx`, and the import in `App.tsx`.
- First-launch `pnpm dev`: only sessions, detail, terminal panels appear. Capacity/Recommendations do **not**.
- `View → Capacity` (`⌘⇧C`) still opens the Capacity panel (verifies preserved IPC path).
- Close all panels → empty-state shows exactly one button "Restore default layout" → click → 3-panel default returns.
- `grep -rn "Mission Control" apps/desktop/src` returns zero.

### Anti-pattern guards

- Don't remove `CapacityPanel.tsx` / `RecommendationsPanel.tsx` themselves — they're still reachable via the View menu and registered in `COMPONENTS`.
- Don't rename `PANEL_DEFAULTS` keys; the electron menu hardcodes `["sessions", "detail", "terminal", "capacity", "recommendations"]` in `main.ts:21`.
- Don't introduce more than one `<EmptyState>` component. If you find you need a "size" prop, push back — same component, same voice.
- Don't change the Dockview `dv-*` CSS-var overrides; just re-point them at tokens (handled in Phase 1).

---

## Phase 4 — Sessions list dashboard (the headline change)

**Goal**: Make the sessions list the situational-awareness surface promised by the IA. Per-row IA, visual weight, no raw enums.

### Per-row information architecture

Each `<li class="session-row">` renders, top to bottom:

```
┌──────────────────────────────────────────────────┐
│ [●] Session label                  2m ago    [×] │  ← row-primary
│     Needs you · main · 4 files                   │  ← row-secondary (attention + folder + dirty)
│     "Running cargo test…"                        │  ← row-action (optional, agent.summary or nextAction)
│     Agent · 95% confidence                       │  ← row-source (optional)
└──────────────────────────────────────────────────┘
```

Tokens used:
- **Visual weight**: left-border 3px using `--attention-{needs-you|blocked|done|working|idle|neutral}` via a `data-attention` attribute on the row.
- **Row-primary label**: `--text-primary`, `--text-lg`. The dot is 8px, color = attention token, only shown when `attentionTone !== "neutral"`.
- **Last activity (right side)**: `--text-faint`, `--text-xs`. Source: `peonLastInference` (preferred), else `created_at`. Computed via `relativeTime(...)` (Phase 2).
- **Row-secondary**: `--text-muted`, `--text-sm`. Format `${attentionLabel(attn)} · ${folder}${dirty ? ` · ${changedFiles} files` : ""}`.
- **Row-action**: `--text-muted`, `--text-sm`, italic, single-line (`text-overflow: ellipsis`). Source: `active.summary || active.nextAction`. Render only if non-empty.
- **Row-source**: `--text-faint`, `--text-xs`. Only render if `metadataSource` is set; uses `sourceWithConfidence(...)` from Phase 2.

### Visual weight rules

- `data-attention="needs-you"` rows get bolder label (`font-weight: 600`) and a 3px solid `--attention-needs-you` left border.
- `data-attention="blocked"` rows: 3px solid `--attention-blocked`.
- All other rows: 3px solid transparent (so width doesn't jump on state change).
- Active selection: `background: var(--surface-active)` (unchanged behavior, just tokenized).
- Remembered/resumable styling: opacity 0.78 stays for `memoryState !== "live"`; the "remembered/resumable" word is rendered via `memoryStateLabel(...)` in the row-secondary line, replacing today's badge.

### Group headers

Today/This week/Earlier headers (`SessionListPanel.tsx:21-25,125-129`) are preserved. Restyle to use `--text-faint`, `--text-xs`, `--space-4` padding.

### Keyboard & focus

Preserved verbatim: Arrow/Enter handling at `SessionListPanel.tsx:85-106`, `scrollIntoView` at `:51-55`, focus-return at `:108-111`. Global `:focus-visible` from Phase 1 now gives a visible ring.

### What to implement

1. Replace the JSX inside `SessionListPanel.tsx:130-191` (the inner `group.items.map(...)` block) with the IA above. Use `attentionLabel`, `attentionTone`, `relativeTime`, `memoryStateLabel`, `sourceWithConfidence` from `labels.ts`. Delete all inline `style={{...}}` color usages; the row's color comes from CSS targeting `[data-attention="..."]`.

2. Add CSS rules in `App.css` (all using tokens):

   ```css
   .session-row {
     padding: var(--space-3) var(--space-5);
     border-left: 3px solid transparent;
     border-bottom: 1px solid var(--border-subtle);
     cursor: pointer;
     display: grid;
     gap: var(--space-1);
     grid-template-columns: 1fr auto;
     grid-template-areas:
       "primary   meta"
       "secondary secondary"
       "action    action"
       "source    source";
   }
   .session-row[data-attention="needs-you"] { border-left-color: var(--attention-needs-you); }
   .session-row[data-attention="blocked"]   { border-left-color: var(--attention-blocked); }
   .session-row[data-attention="done"]      { border-left-color: var(--attention-done); }
   .session-row[data-attention="working"]   { border-left-color: var(--attention-working); }
   .session-row[data-attention="idle"]      { border-left-color: var(--attention-idle); }
   .session-row[aria-current="true"] { background: var(--surface-active); }
   .session-row:hover { background: var(--border-subtle); }
   .session-row--remembered { opacity: 0.78; }
   /* Each grid-area styled with tokens — see plan §4. */
   ```

3. Replace `className="session-list"` with semantics: `role="listbox"`, `aria-label="Sessions"`. Each row gets `role="option"`, `aria-current={s.id === activeSessionId}`, `data-attention={attentionTone(attn)}`.

4. Re-route the "kill" button to its own row in the grid (`grid-area: meta`), using `--text-muted` resting / `--state-error` on hover. Keep `aria-label="Kill session"`.

### Documentation references

- The current row markup we're replacing: `SessionListPanel.tsx:130-191`.
- The keyboard handler we're keeping unchanged: `SessionListPanel.tsx:85-111`.
- `relativeTime`, `attentionLabel`, `attentionTone`, `memoryStateLabel`, `sourceWithConfidence`: all in `apps/desktop/src/labels.ts` (Phase 2).
- Visual weight tokens: `apps/desktop/src/styles/tokens.css` (Phase 1) `--attention-*`.

### Verification checklist

- Pop 5 fake sessions into the list (or run against a live backend) covering each `attentionTone`. Confirm:
  - "Needs you" rows have a visible red left border and bolder label.
  - "Working" rows show a small green dot.
  - Right column shows e.g. `"3m ago"` (not `peonLastInference` raw text, not the snake_case status).
  - Bottom line shows e.g. `"Agent · 95% confidence"`, not `"agent · 95%"`.
- `grep -E "(waiting_for_input|latest_cwd|metadataSource\s*&middot;)" apps/desktop/src/components/SessionListPanel.tsx` returns zero matches.
- `grep -E "style=\\{\\{[^}]*(#|background:|color:)" apps/desktop/src/components/SessionListPanel.tsx` returns zero hex/color inline styles. (Layout-only inline styles such as `display: flex` are acceptable but should be lifted to CSS classes where convenient.)
- Tab into the list, arrow up/down: focus ring appears on the list; rows still navigate; the active row scrolls into view.

### Anti-pattern guards

- **Do not** render a second xterm "preview" for the row. The row is text only.
- **Do not** introduce a per-row hover popup, tooltip card, or expand-panel. If more detail is needed, that's the detail panel's job.
- **Do not** add row reorder / drag-and-drop.
- **Do not** display roadmap codes ("M8" / "M9" / "milestone") anywhere.

---

## Phase 5 — Detail panel + Center / Terminal panels migrate to tokens + labels

**Goal**: All remaining user-visible enum strings vanish. All inline-style colors vanish. The Center placeholder loses its tagline.

### What to implement

1. `SessionDetailPanel.tsx`:
   - Replace `{attn}` (`:51`) with `{attentionLabel(attn)}`. Drop the `statusDotColor` import; render the dot via a class `.detail-status-dot` styled by `data-attention`.
   - Replace `{active.memoryState} · {active.resumeStrategy}` (`:94-96`) with `${memoryStateLabel(active.memoryState)} · ${resumeActionLabel(active.resumeStrategy)}`.
   - Replace the source badge (`:100-113`) to render `sourceWithConfidence(active.metadataSource, active.metadataConfidence)`. Background color via class `.source-badge[data-source="agent|peon|other"]`.
   - Replace `resumeLabel` (`:23-30`) with `resumeActionLabel(active.resumeStrategy)`.
   - Move every inline `style={{ color, background, fontSize, padding }}` to CSS classes keyed off tokens. Layout-only inline styles (`display: flex`, `gap`) may remain but prefer classes.
   - Replace the `Peon` "observed {active.peonLastInference}" line: render `Peon · ${relativeTime(active.peonLastInference)}`.

2. `TerminalPanel.tsx`:
   - Strip the duplicated header (`:25-43`) — the Dockview tab is already the panel's header. Render just `<CenterPanel ... />`. The single-active-context model is unaffected; one Dockview tab is still the only visible terminal.
   - If we need a kill button, attach it to the Dockview `rightHeaderActionsComponent` (mirroring `SessionsHeaderActions` at `DockviewApp.tsx:46-63`). That's optional; first cut: drop the duplicated kill button and rely on the session row's kill button.
   - The empty-state branch already routes through `<EmptyState>` from Phase 3.

3. `CenterPanel.tsx`:
   - Toolbar branch at `:135-144` keeps `embedded` semantics but uses tokens.
   - Empty-state branch at `:114-126` replaced in Phase 3.
   - Drop the `terminalStatus` `"backend: …"` line and the "Mission Control for AI Agents" tagline. The status badge in the titlebar (`App.tsx:307-311`) already communicates backend health.

4. `CapacityPanel.tsx` / `RecommendationsPanel.tsx`:
   - Replace `"Capacity tracking coming in M8"` → `"Capacity tracking coming soon."`
   - Replace `"Recommendations coming in M9"` → `"Recommendations coming soon."`
   - Rewrite both with tokens (header style class, body style class — share with `SessionDetailPanel` section styling where possible).

5. `App.tsx` titlebar:
   - Button label `"Open Folder"` → `VOCAB.openWorkspace` (`"Open workspace…"`).
   - `title="Switch workspace"` stays (matches `VOCAB.switchWorkspace`).

### Documentation references

- Enum rendering sites: `SessionDetailPanel.tsx:42-110`; `CenterPanel.tsx:114-126`; `App.tsx:281-303`; `CapacityPanel.tsx:11-13`; `RecommendationsPanel.tsx:11-13`.
- `<EmptyState>` from Phase 3 at `apps/desktop/src/components/EmptyState.tsx`.
- Label helpers in `apps/desktop/src/labels.ts` (Phase 2).

### Verification checklist

- `grep -rn "M8\|M9\|backend:\|Mission Control" apps/desktop/src` returns zero matches.
- `grep -rn "memoryState\\s*}\|resumeStrategy\\s*}" apps/desktop/src` returns zero matches (no raw-enum interpolation).
- `grep -rnE "style=\\{\\{[^}]*(#[0-9a-fA-F]{3,8})" apps/desktop/src` returns zero matches.
- Manual: open a workspace, select a session, see detail panel display "Needs you", "Live · Resume latest in folder", "Agent · 95% confidence", "Peon · 8s ago".
- Manual: titlebar reads `"Open workspace…"` when no workspace is set.

### Anti-pattern guards

- **Do not** re-route any colors through `RightSidebarHelpers.ts` — that file is gone.
- **Do not** add a status string the label module doesn't cover. Extend `attentionLabel` switch instead.
- **Do not** make `TerminalPanel.tsx` render more than one CenterPanel. Single-active.

---

## Phase 6 — Feedback primitive + route the 5 swallowed catches

**Goal**: A single toast/inline-status component. The 5 user-facing catch blocks surface errors through it instead of swallowing them.

### What to implement

1. Create `apps/desktop/src/components/Toast.tsx` + `apps/desktop/src/feedback.ts`:

   ```ts
   // feedback.ts
   type ToastTone = "info" | "warn" | "error";
   export interface Toast { id: string; tone: ToastTone; message: string; }
   type Listener = (toasts: readonly Toast[]) => void;

   const state: Toast[] = [];
   const listeners = new Set<Listener>();
   let counter = 0;

   function emit(): void { for (const l of listeners) l([...state]); }

   export function pushToast(tone: ToastTone, message: string, timeoutMs = 4000): string {
     const id = `t${++counter}`;
     state.push({ id, tone, message });
     emit();
     if (timeoutMs > 0) setTimeout(() => dismissToast(id), timeoutMs);
     return id;
   }

   export function dismissToast(id: string): void {
     const idx = state.findIndex((t) => t.id === id);
     if (idx >= 0) { state.splice(idx, 1); emit(); }
   }

   export function subscribeToasts(l: Listener): () => void {
     listeners.add(l); l([...state]); return () => listeners.delete(l);
   }
   ```

   ```tsx
   // Toast.tsx
   import { useEffect, useState } from "react";
   import { subscribeToasts, dismissToast, type Toast } from "../feedback";

   export default function ToastRack() {
     const [toasts, setToasts] = useState<readonly Toast[]>([]);
     useEffect(() => subscribeToasts(setToasts), []);
     if (toasts.length === 0) return null;
     return (
       <div className="toast-rack" role="status" aria-live="polite">
         {toasts.map((t) => (
           <div key={t.id} className="toast" data-tone={t.tone}>
             <span>{t.message}</span>
             <button type="button" className="toast-dismiss" onClick={() => dismissToast(t.id)} aria-label="Dismiss">×</button>
           </div>
         ))}
       </div>
     );
   }
   ```

   Styles use `--state-{ok|warn|error|info}` and `--state-*-bg` tokens.

2. Mount `<ToastRack />` once in `App.tsx` inside `.app-shell`, above the Dockview area.

3. Route the 5 catches at `App.tsx:73-76, 95-97, 112-114, 137-139, 182-184`:

   | Site | Replacement |
   | --- | --- |
   | `refreshSessions` `:73-76` | `} catch (e) { pushToast("warn", "Couldn't refresh sessions."); }` — note: this runs on a 2s interval, so include a throttle: only push if the last toast for this message was >10s ago. Easiest: silence the polling path explicitly, surface only on user-initiated refresh (none today → silent is fine here; document why). Recommended: leave silent, add a comment `// silent: 2s polling; surface failures via backendStatus badge instead.` |
   | `handleOpenWorkspace` `:95-97` | `} catch (e) { pushToast("error", "Couldn't open workspace."); }` — but distinguish user-cancel: `if ((e as Error)?.name !== "AbortError" && info !== null)`. Simpler: only toast when `openWorkspace()` throws (rejection); the cancel path returns `null`, not throws — current code handles that already. So: `} catch { pushToast("error", "Couldn't open workspace."); }` |
   | `handleCreateSession` `:112-114` | `} catch { pushToast("error", "Couldn't start a new session."); }` |
   | `handleKillSession` `:137-139` | `} catch { pushToast("error", "Couldn't end session."); }` |
   | `persistActiveSession` `:182-184` | `.catch(() => { /* backend may not be ready yet — silent. */ });` — preserve silence but document. |

   Net: 3 user-facing toasts, 2 documented-silent paths.

### Documentation references

- Catch sites listed at `apps/desktop/src/App.tsx:73-76, 95-97, 112-114, 137-139, 182-184`.
- React state pattern: subscribe-via-effect; mirrors how `terminalStore.ts` is used elsewhere (module-level mutable state, effect-subscribes).

### Verification checklist

- Trigger workspace-open with sidecar killed: a red error toast appears, auto-dismisses after 4s.
- `grep -E '/\\* ignore \\*/' apps/desktop/src/App.tsx` returns zero matches (all replaced with `pushToast(...)` or an explicit-silent comment).
- Focus-trap check: tabbing through the app does **not** trap inside the toast rack.
- Screen-reader check (`aria-live="polite"`): toast text is announced.

### Anti-pattern guards

- **Do not** use a third-party toast library. The primitive is 60 lines.
- **Do not** make toasts blocking (modal). They auto-dismiss.
- **Do not** add toast positions other than top-right.
- **Do not** add `console.error` on top — toasts already inform the user; DevTools still gets the raw error if you `console.warn(e)` for debugging, but don't leak it as a second visible surface.

---

## Phase 7 — Cutover, layout migration, verification

**Goal**: Retire the old layout for users who already shipped a 5-panel `layout.json`. Confirm everything.

### What to implement

1. Stored-layout migration in `DockviewApp.tsx` `onReady`:

   ```ts
   window.orkworks.getLayout().then((layout) => {
     if (layout) {
       try {
         const parsed = JSON.parse(layout);
         if (layoutNeedsMigration(parsed)) {
           // User had the old 5-panel default. Drop and rebuild.
           buildDefaultLayout(api);
         } else {
           api.fromJSON(parsed);
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

   function layoutNeedsMigration(json: unknown): boolean {
     // Heuristic: if the stored layout references "capacity" or "recommendations"
     // panel IDs, it predates the redesign and the user-default is the new 3-panel.
     // We deliberately overwrite — there's no useful per-user customization to preserve
     // for a one-time cutover.
     const text = JSON.stringify(json);
     return text.includes('"capacity"') || text.includes('"recommendations"');
   }
   ```

   Rationale: there's no soft-migration that retains the old custom layout while removing two panels — Dockview's positions reference removed panels. A clean reset is honest about the change. Document this in the PR as a known one-time UX event.

2. Verification sweep — make these greps part of CI or a manual checklist:

   ```bash
   # No hex literals in CSS or component inline styles
   grep -REn '#[0-9a-fA-F]{3,8}\b' apps/desktop/src/App.css apps/desktop/src/components apps/desktop/src/*.ts apps/desktop/src/*.tsx \
     | grep -v 'tokens.css'

   # No raw enum strings in JSX
   grep -REn '"(waiting_for_input|latest_cwd|latest_repo|remembered|resumable)"' apps/desktop/src/components

   # No roadmap codes
   grep -RE '\bM[0-9]+\b|Mission Control|backend:' apps/desktop/src

   # No outline:none
   grep -RnE 'outline:\s*none' apps/desktop/src

   # No silent catches
   grep -REn '/\* ignore \*/' apps/desktop/src/App.tsx
   ```

   All five must return empty (modulo tokens.css and the documented-silent comment in `persistActiveSession`).

3. Smoke-tests:
   - `cd apps/desktop && npx tsc --noEmit` — clean.
   - `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — passes.
   - `cd apps/desktop && pnpm dev` — open workspace, start 3 sessions (one waiting on input, one running, one done), confirm the sessions list shows three different attention tones with plain-language labels.
   - Force an error path: kill the sidecar manually, click "New session" — error toast appears.

4. **PR scope check** — open one PR for this redesign or split by phase, but every PR must:
   - Touch only `apps/desktop/src/**` (and `apps/desktop/src/styles/tokens.css`).
   - Run `/code-review` (medium effort or higher) per `AGENTS.md`.
   - Update `apps/desktop/src/labels.ts` whenever a new enum value is added — if you find an enum not covered, fix the table.

### Verification checklist

- All seven `grep` sweeps above pass.
- Manual UI smoke covers: open workspace, idle sessions list, needs-attention row, terminal active, detail panel, capacity/recs **closed by default**, capacity/recs reachable via `⌘⇧C` / `⌘⇧R`, empty-state recovery via "Restore default layout", focus rings on Tab nav, error toast on bad backend.

### Anti-pattern guards

- **Do not** leave both layouts behind a feature flag.
- **Do not** keep `RightSidebarHelpers.ts` around for compatibility — it's renamed, not aliased.
- **Do not** introduce a "what's new" banner. The change is the change.

---

## Cross-phase deliverables index (single look-up)

| Deliverable | Phase | Location |
| --- | --- | --- |
| Design tokens (color/space/type/state) | 1 | `apps/desktop/src/styles/tokens.css` |
| Global `:focus-visible` rule | 1 | `apps/desktop/src/App.css` |
| Labels module (enum → plain English) | 2 | `apps/desktop/src/labels.ts` |
| Helper rename (no more "RightSidebarHelpers") | 2 | `apps/desktop/src/sessionSort.ts` |
| Single default-layout builder | 3 | `apps/desktop/src/components/DockviewApp.tsx` |
| Empty-state primitive | 3 | `apps/desktop/src/components/EmptyState.tsx` |
| Dead-component deletions | 3 | `RightSidebar.tsx`, `LeftSidebar.tsx`, `TerminalTabs.tsx` |
| Sessions-list dashboard row | 4 | `apps/desktop/src/components/SessionListPanel.tsx` |
| Detail/center/capacity/recs token + label migration | 5 | `SessionDetailPanel.tsx`, `CenterPanel.tsx`, `TerminalPanel.tsx`, `CapacityPanel.tsx`, `RecommendationsPanel.tsx`, `App.tsx` |
| Toast primitive + 5 catch routings | 6 | `apps/desktop/src/feedback.ts`, `apps/desktop/src/components/Toast.tsx`, `App.tsx` |
| Stored-layout migration heuristic | 7 | `apps/desktop/src/components/DockviewApp.tsx` |

---

## States checklist (every primary surface, every state)

| Surface | empty | loading | error | success | focus | disabled |
| --- | --- | --- | --- | --- | --- | --- |
| Sessions list (workspace missing) | `<EmptyState message="Open a workspace to see sessions." action={openWorkspace} />` | n/a — list polls every 2s | toast via `feedback.ts` | row renders with attention tone & last-activity | global `:focus-visible` ring on `<ul>` and rows | n/a |
| Sessions list (workspace, no sessions) | `<EmptyState message="No sessions yet. Press ⌘N to start one." />` | n/a | toast | first row appears | same | n/a |
| Session row | n/a | n/a | row shows `attention="needs-you"` w/ red border if `failed` | normal | `:focus-visible` border ring on `[role=option]` | greyed (`opacity: 0.78`) when `memoryState !== "live"` |
| Detail panel | `<EmptyState message="Select a session to see details." />` | n/a | falls back to empty | section list visible | inputs (resume button) get `:focus-visible` | resume button greyed when `resumeStrategy === "none"` |
| Terminal panel | `<EmptyState message="Select a session to open its terminal." />` | terminal-status "connecting" inside CenterPanel | empty-state if backend disconnected (`"Connecting to OrkWorks…"`) | xterm renders | n/a (xterm handles own focus) | xterm gets `terminal-container--ended` class when session ended |
| Empty Dockview (all panels closed) | `<div class="dockview-empty-state">` with one button "Restore default layout" | n/a | n/a | layout re-fills | button has `:focus-visible` | n/a |
| Capacity / Recommendations panels | `<EmptyState message="Capacity tracking coming soon." />` | n/a | n/a | n/a (no signal until M8/M9) | n/a | n/a |

---

## Open questions deferred (record so they don't get forgotten)

1. **Light mode** — deferred. The token layer makes adding `@media (prefers-color-scheme: light)` a future drop-in. Track as `apps/desktop` follow-up issue.
2. **`prefers-reduced-motion`** — deferred. Affects xterm cursor blink and any future animation. Token layer doesn't gate this; xterm config does. Track as separate issue.
3. **Toast persistence** — current design auto-dismisses after 4s. If we later want a notification history, that's a new surface.
4. **Settings/hotkeys panel** — coordinated with `docs/superpowers/specs/2026-06-18-app-settings-hotkeys-design.md` but not folded in. When that lands, it gets its own panel registered via `PANEL_DEFAULTS` and reuses tokens + labels.
5. **Capacity/Recommendations rendered content** — out of scope. Their layout-presence is changed (closed by default). Their content is owned by M8/M9.

---

## How to execute this plan

- Each phase is self-contained: a fresh chat could pick up Phase 4 with only this plan + `apps/desktop/src/` access and finish it. The "Documentation references" section in each phase tells the executor exactly where to look.
- Phases must be executed in order. Phase 4 depends on Phase 1 (tokens) and Phase 2 (labels). Phase 5 depends on Phases 1–3.
- Per `AGENTS.md`, each phase that touches `apps/desktop/src/` requires a branch + PR + `/code-review` (medium effort or higher). Use the `starting-work` skill to set up the branch/worktree.
- Run `bash .claude/hooks/doc-check.sh` before ending each phase per `AGENTS.md` doc-currency rule.
