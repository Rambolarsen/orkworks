# 01 — Consolidated Evidence

All claims trace to `apps/desktop/src/`. Subagents gathered facts; this document consolidates them with anchors (E#) that `02-scorecard.md` cites. No scoring here.

## Structural (S)

- **S1** — Components exceeding a 7-element budget per surface: `SessionListPanel` (10 element types), `SessionDetailPanel` (11), `RightSidebar` (11, dead), `TerminalTabs` (8, dead). Source: `SessionListPanel.tsx:80-188`, `SessionDetailPanel.tsx:10-135`, `RightSidebar.tsx:9-99`, `TerminalTabs.tsx:21-100`.
- **S2** — Three dead components shipped: `RightSidebar.tsx`, `LeftSidebar.tsx`, `TerminalTabs.tsx` — none imported anywhere in `src/`. Type `TerminalTabsHandle` also dead.
- **S3** — Duplicate session-detail content: `SessionDetailPanel.tsx:10-135` ≈ `RightSidebar.tsx:9-99` render overlapping Status / Summary / Directory / Git / Conflict / Recommendation / Source / Peon sections; git block at `SessionDetailPanel.tsx:59-77` ≈ `RightSidebar.tsx:48-66` verbatim.
- **S4** — Two divergent default-layout builders: `DockviewApp.tsx:143-160` (`buildDefaultLayout`) and `App.tsx:260-268` (reset-layout menu handler) iterate `["detail","terminal","capacity","recommendations"]` independently. The menu version omits panel `title`; will drift.
- **S5** — Two empty-state copies for "no session": `SessionListPanel.tsx:116` ("No active sessions") vs `TerminalPanel.tsx:18` ("No active terminal") vs `SessionDetailPanel.tsx:16` ("Select a session to see details") vs `CenterPanel.tsx:117-123` ("OrkWorks / Mission Control for AI Agents / backend:") — four flavors for the same idle state on different panels.
- **S6** — Two ways to kill, two ways to switch sessions: kill at `SessionListPanel.tsx:178` and `TerminalPanel.tsx:33`; switch at `SessionListPanel.tsx:146` (click), `SessionListPanel.tsx:91-105` (arrow keys), and `App.tsx:202` (menu `focus`).
- **S7** — Max component nesting depth 7: `App → DockviewApp → Context.Provider → DockviewReact → TermPanel → TerminalPanel → CenterPanel`.
- **S8** — Total distinct interactive surfaces in rendered tree ≈ 10 plus 2N per session. Static count is small; structural complexity is in duplication, not options.
- **S9** — Per-panel placeholder content for unbuilt milestones: `CapacityPanel.tsx:12` ("Capacity tracking coming in M8"), `RecommendationsPanel.tsx:12` ("Recommendations coming in M9"). Panel titles and headers diverge: panel title "Recommendations" (`DockviewApp.tsx:119`) vs header "Start Next Task" (`RecommendationsPanel.tsx:8`).

## Visual (V)

- **V1** — No design-token layer. 21 CSS custom properties exist, all scoped to `.orkworks-dockview` (`App.css:459-481`) — they only override Dockview internals; no `:root` tokens.
- **V2** — ~118 hardcoded color literals across `App.css` + inline styles in `.tsx` files. ~50 distinct unique color values total. Backgrounds ~18, foregrounds ~10, accents/borders ~13, terminal palette ~9. Orphans (single-use): `#323233`, `#5a3a1a`, `#3a2a1a`, `#5c879e`, `#9fb7d7`, `#e8f0ff`, `#3d5f8f`, `#1d2f49`, `#6aa6ff`, `#4a4a4a`. The literal `#211d1b` is declared as a custom prop at `App.css:459` then re-pasted as a literal at `App.css:484` instead of referenced.
- **V3** — Spacing scale has ~16 distinct values (0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 26, 30, 36, 46 px). No system; literals strewn across `App.css` and inline `style={{}}` in `SessionDetailPanel`, `CapacityPanel`, `RecommendationsPanel`, `TerminalPanel`.
- **V4** — Type scale uses 7 sizes: 9, 10, 11, 12, 13, 14, 16 px. Body base 13px (`App.css:13`).
- **V5** — Zero `:focus` or `:focus-visible` rules anywhere in `App.css`. The only focus-related rule actively removes outline: `.session-list { outline: none }` at `App.css:233`. Browser defaults are the only focus indication anywhere — and Dockview tabs, the `+` button, kill buttons, and Resume button have no custom focus ring.
- **V6** — Missing states checklist (per surface):
  - Session list: focus MISSING; loading MISSING; error MISSING.
  - Buttons (titlebar open, terminal launch, resume, reset, header `+`, switch): focus MISSING; loading MISSING; error MISSING.
  - Status badge: distinct error variant MISSING (reuses `.warn`).
  - Terminal: error MISSING (websocket failures not surfaced).
  - Hover/active/disabled/selected: PRESENT for most surfaces.
- **V7** — Transitions used on exactly one surface: `.dockview-header-action` (`App.css:220`). Otherwise the UI is animation-free at idle.

## Copy & Honesty (C)

- **C1** — One inflation: "Mission Control for AI Agents" at `CenterPanel.tsx:119`. Marketing tagline embedded as the center placeholder copy; appears only in unembedded mode (currently unreachable in shipped layout, but still committed).
- **C2** — Internal enum values leaked to users:
  - Snake-case `attentionStatus`: `waiting_for_input`, `blocked`, `failed`, `stale`, `creating` — `SessionListPanel.tsx:157`, `SessionDetailPanel.tsx:51`.
  - `memoryState`: `resumable`, `remembered`, `live`, `unsupported` rendered raw — `SessionListPanel.tsx:172`, `SessionDetailPanel.tsx:95`.
  - `resumeStrategy`: `latest_cwd`, `latest_repo`, `unsupported` rendered raw — `SessionDetailPanel.tsx:95`.
  - `metadataSource`: `agent`, `peon`, `user`, `process`, `backend_inference`, `unknown` rendered raw alongside a bare `N%` (no "confidence" label) — `SessionListPanel.tsx:167`, `SessionDetailPanel.tsx:110`.
  - Roadmap codes: "M8", "M9" — `CapacityPanel.tsx:12`, `RecommendationsPanel.tsx:12`.
  - Implementation jargon: "backend:" — `App.tsx:307-311`, `CenterPanel.tsx:122`.
- **C3** — Naming drift for the same concept:
  - Workspace vs Folder vs (untitled): "Open Folder" (`App.tsx:302`) / "Switch workspace" tooltip (`App.tsx:289`) / "Open a workspace to begin" (`SessionListPanel.tsx:80`) / "No workspace" (`App.tsx:296`).
  - Recommendations panel header drift: panel title "Recommendations" vs header "Start Next Task" vs empty "Recommendations coming in M9" — three names per render.
  - Session-detail field divergence: same backend field rendered as "Task" in `SessionDetailPanel.tsx:36` and as "Summary" in dead `RightSidebar.tsx:34`.
  - "Sessions" panel feeds "Terminal" panel; same entity, different chrome word.
- **C4** — Label/behavior mismatch on the empty-state recovery: hint says "Open one from the View menu, or" (`DockviewApp.tsx:208`) but the button is "Reset Layout" (`DockviewApp.tsx:218`) and restores the full 5-panel default — not "one" panel as the hint suggests.
- **C5** — Glyph-only controls with hover-only tooltips: `⇄` (workspace switch, `App.tsx:289-291`), `+` (new session, `DockviewApp.tsx:60`), `×` (kill, `SessionListPanel.tsx:187`, `TerminalPanel.tsx:41`), `⚠` (attention, `SessionListPanel.tsx:152`). No `aria-label`; touch users get no labels.
- **C6** — Destructive action without confirmation: kill session executes immediately (`App.tsx:126-142`, `SessionListPanel.tsx:182-185`, `TerminalPanel.tsx:34`). This is a UX risk (data loss in a running PTY), but not a dark pattern.
- **C7** — Silent error swallowing on user-facing actions: `App.tsx:73-76, 95-97, 112-114, 137-139, 182-184` all `catch { /* ignore */ }`. No toast system exists. User receives no feedback when "Open Folder", "New session", or "Kill session" fails.

## Weight & Friction (W)

- **W1** — Initial JS payload: **842 KB** single chunk (`dist/assets/index-DRrj2qSd.js`), CSS 108 KB. No code-splitting in `vite.config.ts`; all 5 panel components statically imported in `DockviewApp.tsx:11-15`. Stack contributors: `@xterm/xterm` ~250 KB, `dockview` + `dockview-react` ~150 KB, `react` + `react-dom` ~140 KB.
- **W2** — Network requests on initial load with healthy backend: 2 assets + `GET /health` + `GET /sessions` = 4 requests, then a 2 s `setInterval` polling `GET /sessions` forever (`App.tsx:82`). One WS per session opened lazily on selection (`terminalStore.ts:60`).
- **W3** — Estimated TTI: ~500-1500 ms (parse 842 KB + V8 init + first sidecar round-trip). Not measured.
- **W4** — Animations on idle screen: 0 CSS animations active. 1 JS-driven xterm cursor blink once a terminal mounts (`terminalStore.ts:37`, `cursorBlink: true`). Vendored Dockview transitions (26) are hover/drag-only.
- **W5** — `prefers-reduced-motion`: respected by vendored `dockview.css:2952` (1 block); NOT respected by xterm cursor blink. `prefers-color-scheme`: not respected anywhere — UI is dark-only (`App.css:14-15` hard-coded).
- **W6** — Default layout opens 5 panels; ~11 interactive targets visible at idle with no workspace (Open Folder button + 5 Dockview tab headers + 5 sash drag handles). Once a workspace is opened: +"+" new-session button + workspace-switch arrow.
- **W7** — No toast system, no modal system, no notification system. Status conveyed only via the single titlebar badge.

## Accessibility (A)

- **A1** — Contrast failures (`[INFERRED]`, computed from CSS hex):
  - `#444` text on `#1e1e1e` ≈ 1.6:1 — `CenterPanel.tsx:121` "backend:" line. FAIL.
  - `#555` text on `#1e1e1e` ≈ 2.2:1 — `CenterPanel.tsx:118` tagline. FAIL.
  - `#666` text on `#1e1e1e` ≈ 2.8:1 — `App.css:341-350` session-kill, `App.css:361-366` `.empty-state`, `CapacityPanel.tsx:12`, `RecommendationsPanel.tsx:12`, `TerminalPanel.tsx:18,36`. FAIL on at least 6 distinct surfaces.
  - `#6e6e6e` text on `#1e1e1e` ≈ 3.5:1 — `App.css:247-257` session group header, `App.css:281-284` empty-state hint. FAIL.
  - `#858585` text on `#252526` ≈ 3.7:1 — terminal subtitle `App.css:122-126`. FAIL.
  - Titlebar switch `#858585` on `#323233` ≈ 3.3:1. FAIL.
  - Status badge default `#cccccc` on `#555` ≈ 3.4:1. FAIL.
- **A2** — ARIA landmarks: **0**. No `role`, `<main>`, `<nav>`, `<aside>`, `<header>`, `<footer>` anywhere. No skip-link.
- **A3** — Selection is visual-only: session "selected" conveyed only by background `#37373d` (`App.css:306-312`) and a colored left border. No `aria-selected`, no `aria-current`. The `<ul>` is not `role="listbox"` despite arrow-key single-selection behavior.
- **A4** — Status / attention conveyed by color and glyph alone. `⚠` glyph (`SessionListPanel.tsx:152`) and `statusDotColor` dot (`SessionDetailPanel.tsx:44-51`) have no text alternative. Status badge updates silently — no `aria-live`.
- **A5** — Zero `aria-label`, `aria-describedby`, `aria-expanded`, `aria-current` anywhere. Only `aria-hidden="true"` on group header (`SessionListPanel.tsx:127`) and `title=""` tooltips on icon buttons.
- **A6** — Focus stealers: terminal `focus()` on attach (`CenterPanel.tsx:63-67`) — guarded by checking sessions-list `activeElement`, but any other DOM mutation that re-runs `attachTerminal` yanks focus from controls like Resume or `+`. Clicking the embedded toolbar kill button (`TerminalPanel.tsx:33-42`) lacks `stopPropagation`, so focus moves to terminal even when user intended to kill.
- **A7** — Keyboard reachability: most primary actions reachable (new session, switch, close, focus terminal, reset layout). Toggle Sessions panel ONLY reachable via Electron menu/IPC — no in-renderer keybinding. Terminal-tab switching is unreachable in shipped UI because `TerminalTabs` isn't wired into `COMPONENTS` (`DockviewApp.tsx:100-106`).
- **A8** — No `screenReaderMode` set in `terminalStore.ts:36-42`. xterm not announced.

## Cross-cutting gaps

- **G1** — No theme/token JSON. Design system is implicit in scattered hex literals.
- **G2** — No toast/modal/notification primitive. Errors silently swallowed; non-trivial async state has no surface.
- **G3** — Three dead `.tsx` files shipping with the bundle (`RightSidebar`, `LeftSidebar`, `TerminalTabs`). Carry copy and components that contradict the live UI.
- **G4** — Two divergent default-layout codepaths likely to drift (`DockviewApp.tsx` vs `App.tsx` reset menu).
- **G5** — Marketing line "Mission Control for AI Agents" baked into product chrome (`CenterPanel.tsx:119`).

## Known gaps in evidence

- Dockview internals (focusability of tab bar, sash handles, drag drop) are third-party and not fully inspectable.
- xterm.js internals (screen reader mode, scrollback announcement) are third-party.
- Electron main-process menu / accelerator bindings live outside `apps/desktop/src/` and were not inspected.
- All contrast figures are computed from hex, not measured against rendered pixels.
- No live screenshots taken this pass.
