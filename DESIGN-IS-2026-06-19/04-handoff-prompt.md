# 04 — /make-plan Handoff

Paste the block below into a fresh session to plan the redesign work.

````
/make-plan Redesign the OrkWorks desktop UI substrate (apps/desktop/). Current design failed a Dieter Rams audit at 10/30 with critical gaps in principles #3 (aesthetic — no design-token layer) and #10 (as-little-as-possible — dead components + placeholder panels), and broad weakness in #4 (understandable), #6 (honest), #8 (thorough). The IA is correct and stays; the substrate (tokens, vocabulary, default-layout composition, state coverage) is what gets re-derived.

Verdict paragraph (quoted from the audit):
> Total score 10/30 — well below the 20-point REFINE threshold. The IA itself is correct (sessions list as multi-view, single active terminal as the context primitive, switch-to-change-context as the model); the design failures are structural in the substrate, not the architecture. Two principles scored 0 (#3 aesthetic — no design-token layer; #10 as-little-as-possible — dead components + placeholder panels) and three more (#4 understandable, #6 honest, #8 thorough) scored 1. A refine pass would polish noise; the redesign re-derives the substrate while preserving the IA.

Why redesign and not refine: total is 10/30 (REFINE requires ≥20) and two principles scored 0; the token-layer absence and the structural duplication are foundational gaps, not surface tweaks.

Primary user: power developer running many concurrent AI coding sessions.
Primary task: maintain situational awareness across N sessions via the sessions list (the multi-view), then switch session to switch context for focused work. Showing many terminals at once is explicitly rejected as context degradation — session = context, switching is the context-switch primitive.

Preserve from current design (LOAD-BEARING — do not touch in the redesign):
- **Single-active-terminal context model.** Session = context; switching sessions = switching context. The sessions list is the multi-view; the active terminal is single by design. Multi-terminal / tiled / split views are forbidden.
- Electron + Vite + React stack and the preload/IPC contract — `apps/desktop/electron/` and `apps/desktop/src/main.tsx`.
- Dockview-based panel system and layout-persistence flow — `apps/desktop/src/components/DockviewApp.tsx:122-224`, `apps/desktop/src/App.tsx:160-271`.
- xterm.js terminal lifecycle in `terminalStore.ts` — module boundary stays; rendering wrapper may change.
- Sessions list as the primary multi-view surface, including keyboard navigation (Arrow/Enter), focus-handoff guard against the terminal stealing focus, and time-bucket grouping ("Today" / "This week" / "Earlier") — `apps/desktop/src/components/SessionListPanel.tsx:22-188`, `apps/desktop/src/components/CenterPanel.tsx:63-67`.
- Detail panel as the secondary context for the currently-focused session — `apps/desktop/src/components/SessionDetailPanel.tsx`.
- Workspace identity in the titlebar and the `.orkworks/workspace.json` "last active session" restore — `apps/desktop/src/App.tsx:160-186, 273-326`.
- Brand vocabulary per `AGENTS.md`: OrkWorks, orkworksd, Peon — no other fantasy naming.

Discard (each caused a failure):
- 5-panel default layout that opens Capacity and Recommendations as placeholders for unshipped milestones — `apps/desktop/src/components/DockviewApp.tsx:114-160`. Caused failure on #5 unobtrusive, #10 as-little-as-possible.
- All raw-enum surface text: `attentionStatus` (snake_case), `memoryState`, `resumeStrategy`, `metadataSource`, "M8"/"M9", "backend:" — `SessionListPanel.tsx:157-172`, `SessionDetailPanel.tsx:42-110`, `CapacityPanel.tsx:12`, `RecommendationsPanel.tsx:12`, `App.tsx:307-311`, `CenterPanel.tsx:122`. Caused failure on #2 useful (dashboard illegibility), #4 understandable and #6 honest.
- Hardcoded color/spacing/type literals everywhere (no token layer) — `App.css` and inline styles across `*.tsx`. Caused failure on #3 aesthetic.
- Dead components: `RightSidebar.tsx`, `LeftSidebar.tsx`, `TerminalTabs.tsx`, `RightSidebarHelpers.ts` — `apps/desktop/src/components/`. Caused failure on #10.
- "Mission Control for AI Agents" decorative tagline in center placeholder — `CenterPanel.tsx:119`. Caused failure on #5 unobtrusive and #6 honest.
- Naming drift: "Open Folder" vs "Switch workspace" vs "Workspace" vs "Folder" for the same concept — `App.tsx:281-303`, `SessionListPanel.tsx:80`. Caused failure on #4 and #6.
- Four divergent empty-state copies for the same idle — `SessionListPanel.tsx:80,116`, `TerminalPanel.tsx:18`, `SessionDetailPanel.tsx:16`, `CenterPanel.tsx:117-123`. Caused failure on #10.
- Empty-state recovery mismatch: hint says "Open one from the View menu" while button restores the full default layout — `DockviewApp.tsx:204-221`. Caused failure on #6.
- `outline: none` removing the only focus signal — `App.css:233`; zero `:focus-visible` rules anywhere in `App.css`. Caused failure on #8 thorough.
- Silent `catch { /* ignore */ }` in 5 user-facing handlers — `App.tsx:73-76, 95-97, 112-114, 137-139, 182-184`. Caused failure on #8 and #2.
- Two divergent default-layout builders that will drift — `DockviewApp.tsx:143-160` (`buildDefaultLayout`) vs `App.tsx:260-268` (reset-layout menu handler). Caused failure on #10.

Top 5 moves from the audit (verbatim):

1. **#2 useful + #4 understandable — Turn the sessions list into a real situational-awareness dashboard.** The sessions list is the primary surface for "what's happening across N sessions"; it currently fails the at-a-glance test because every status, source, and memory state is a raw enum (`waiting_for_input`, `agent · 100%`, `resumable · latest_cwd`). Add: plain-language attention/status labels, last-activity timestamp per row, a single-line "what the agent is doing" summary if available, and visual weight that differentiates "needs you now" from "humming along". Evidence: `SessionListPanel.tsx:118-188`, `SessionDetailPanel.tsx:42-110`. Pairs with move #3 (enum module).

2. **#3 aesthetic — Stand up a real token layer before any further CSS work.** Introduce `:root` design tokens for color, spacing, type (`--surface-0/-1/-2`, `--text-primary/-muted/-faint`, `--space-1..-6`, `--text-xs..-lg`, `--state-ok/-warn/-error/-info`). Migrate `App.css` and inline-style hex literals to vars. Delete the 10 orphan single-use colors as part of the migration. Evidence: zero `:root` tokens in `App.css`; ~50 distinct hex literals and ~118 color occurrences across `App.css` and inline styles in `SessionDetailPanel.tsx`, `TerminalPanel.tsx`, `CapacityPanel.tsx`, `RecommendationsPanel.tsx`, `CenterPanel.tsx`, `RightSidebarHelpers.ts`.

3. **#4 understandable / #6 honest — Build an enum→label module and one canonical vocabulary.** Map every internal enum (`attentionStatus`, `memoryState`, `resumeStrategy`, `metadataSource`) to a human-readable label table. Pick one word per concept ("Workspace", not "Folder" or "Workspace" alternately). Drop roadmap codes from user copy ("Capacity coming soon", not "M8"). Label the confidence number ("agent · 100% confidence"). Replace the empty-state hint that doesn't match its button. Evidence: `SessionListPanel.tsx:157-172`, `SessionDetailPanel.tsx:42-110`, `App.tsx:281-303`, `CapacityPanel.tsx:12`, `RecommendationsPanel.tsx:12`, `DockviewApp.tsx:204-221`.

4. **#10 as-little-as-possible — Delete dead components and remove placeholder panels.** Delete `RightSidebar.tsx`, `LeftSidebar.tsx`, `TerminalTabs.tsx`, `RightSidebarHelpers.ts` (verify usage first). Drop Capacity and Recommendations from the default layout until they carry signal — keep the components for later but stop opening them on first launch. Collapse the four empty-state copies into one shared component with one voice. Evidence: dead files unimported in `src/`; `DockviewApp.tsx:114-160`; four empty-states at `SessionListPanel.tsx:80,116`, `TerminalPanel.tsx:18`, `SessionDetailPanel.tsx:16`, `CenterPanel.tsx:117-123`.

5. **#8 thorough — Add global focus styling and a minimal feedback primitive.** A single `:focus-visible` rule for the whole app (`outline: 2px solid var(--accent-focus); outline-offset: 2px`); remove `outline: none` from `.session-list`. Introduce a small toast / inline-status primitive and route the 5 silently-swallowed catch blocks through it. Evidence: `App.css:233`; no `:focus*` rules in `App.css`; `App.tsx:73-76, 95-97, 112-114, 137-139, 182-184` all swallow errors.

Redesign principles in priority order:

1. #2 useful — success: a user with 5 sessions can tell at a glance from the sessions list which one needs attention, what each is doing, and how recent each is — without opening a single terminal.
2. #4 understandable — success: every label on every screen is plain English (no snake_case enums, no roadmap codes, no "backend:"); a first-time user can name every visible control.
3. #3 aesthetic — success: every CSS color/spacing/type value resolves through a token; zero hardcoded hex/px in `.tsx` inline styles for any of those properties.
4. #6 honest — success: every button label maps 1:1 to what it does; no decorative tagline; one word per concept consistently.
5. #10 as-little-as-possible — success: zero dead components in `src/`; no placeholder panels open by default; one shared empty-state component.

Non-goals (do not design these now):
- **Multi-terminal / split / tiled / stacked terminal views.** Session = context; switching is the context-switch primitive. Parallel terminal visibility is rejected as context degradation, not visibility. The single-active-terminal model is correct and load-bearing.
- Light-mode / `prefers-color-scheme` support — defer; document as a known gap.
- `prefers-reduced-motion` for xterm cursor blink — defer; document.
- Rust sidecar API changes — out of scope for this redesign.
- Electron menu / accelerator bindings rework — out of scope (existing IPC contract stays).
- Settings/hotkeys panel implementation (separate spec at `docs/superpowers/specs/2026-06-18-app-settings-hotkeys-design.md`) — coordinate but don't fold in.
- Capacity (M8) and Recommendations (M9) panel content — only their default-layout presence is in scope; the panels themselves remain stubs until those milestones.

Deliverables for the plan:
- Sessions-list dashboard redesign: per-row information architecture (label, attention, last activity, agent action summary, source confidence) with visual weight rules.
- Design-token file (e.g. `apps/desktop/src/styles/tokens.css` or `tokens.ts`) with color/spacing/type/state scales; migration list of every literal in `App.css` and inline `style={{}}` callsites to replace.
- Enum-to-label mapping module (e.g. `apps/desktop/src/labels.ts`) covering `attentionStatus`, `memoryState`, `resumeStrategy`, `metadataSource`, plus a copy-style guide (one word per concept) for "workspace", "session", "terminal".
- States checklist (empty, loading, error, success, focus, disabled) per primary surface — what each looks like and where it lives in the token system.
- Toast / inline-status primitive design (one component, one API) and a routing plan for the 5 swallowed catch blocks.
- Deletion list: which `.tsx`/`.ts` files are removed; which panels are removed from default layout; which empty-state copies are collapsed.
- Single canonical default-layout builder (collapse the two divergent ones).
- Migration / cutover criteria: when the old layout is retired, how layout-persistence migrates users from a stored layout that referenced Capacity/Recommendations panels.

Anti-patterns to guard against (specific to REDESIGN):
- **Introducing any multi-terminal pattern under any name** — split, tile, preview, picture-in-picture. The single-active-terminal model is load-bearing.
- Porting old structure under new styling — if the new code still has hex literals, the redesign failed.
- Keeping both designs behind a flag indefinitely — pick a cutover date and migrate stored layouts.
- Redesigning to follow a trend rather than the principles above — no glassmorphism, no gradient skeuomorph, no animation flourishes.
- Treating the Preserve list as optional — the Dockview shell, IPC contract, terminal store, sessions-list keyboard nav, single-active-context model, and workspace identity are explicitly preserved.
- Re-introducing raw enums anywhere user-visible — all enum text routes through the label module.
- Adding new placeholder panels — if a panel has no signal, it doesn't open by default.
````
