# 03 — Verdict

## Verdict: REDESIGN

Total score 10/30 — well below the 20-point REFINE threshold. The IA itself is correct (sessions list as multi-view, single active terminal as the context primitive, switch-to-change-context as the model); the design failures are structural in the *substrate*, not the architecture. Two principles scored 0 (#3 aesthetic — no design-token layer; #10 as-little-as-possible — dead components + placeholder panels) and three more (#4 understandable, #6 honest, #8 thorough) scored 1. A refine pass would polish noise; the redesign re-derives the substrate while preserving the IA.

Stack stays. IA stays. Tokens, vocabulary, default-layout composition, and state coverage are re-derived.

## Why not REFINE

REFINE requires total ≥ 20 AND no principle at 0. Two principles are at 0 and total is 10. The token-layer absence and the structural duplication (3 dead components + 2 placeholder panels + 4 divergent empty-state copies) are foundational gaps, not surface tweaks.

## Why not NEW

A real shipping artifact exists with the *right* IA: M1 complete, Dockview shell working, sessions list with keyboard nav, terminal store, IPC bridge, focus-handoff guard, single-active-context model. Throwing it out would discard correctly-decided architecture.

## Top 5 highest-leverage moves

1. **#2 useful + #4 understandable — Turn the sessions list into a real situational-awareness dashboard.** The sessions list is the primary surface for "what's happening across N sessions"; it currently fails the at-a-glance test because every status, source, and memory state is a raw enum (`waiting_for_input`, `agent · 100%`, `resumable · latest_cwd`). Add: plain-language attention/status labels, last-activity timestamp per row, a single-line "what the agent is doing" summary if available, and visual weight that differentiates "needs you now" from "humming along". Evidence: `S6`, `C2`, `C3`. Affects `SessionListPanel.tsx:118-188`, `SessionDetailPanel.tsx:42-110`, new label module.

2. **#3 aesthetic — Stand up a real token layer before any further CSS work.** Introduce `:root` design tokens for color, spacing, type (e.g. `--surface-0/-1/-2`, `--text-primary/-muted/-faint`, `--space-1..-6`, `--text-xs..-lg`, `--state-ok/-warn/-error/-info`). Migrate `App.css` and inline-style hex literals to vars. Delete the 10 orphan single-use colors as part of the migration. Evidence: `V1`, `V2`, `V3`, `V4`. Affects `App.css` (~118 literals), inline styles in `SessionDetailPanel.tsx`, `TerminalPanel.tsx`, `CapacityPanel.tsx`, `RecommendationsPanel.tsx`, `CenterPanel.tsx`, `RightSidebarHelpers.ts`.

3. **#4 understandable / #6 honest — Build an enum→label module and one canonical vocabulary.** Map every internal enum (`attentionStatus`, `memoryState`, `resumeStrategy`, `metadataSource`) to a human-readable label table. Pick one word per concept ("Workspace", not "Folder" or "Workspace" alternately). Drop roadmap codes from user copy ("Capacity coming soon", not "M8"). Label the confidence number ("agent · 100% confidence"). Replace the empty-state hint that doesn't match its button. Evidence: `C2`, `C3`, `C4`. Affects `SessionListPanel.tsx:157-172`, `SessionDetailPanel.tsx:42-110`, `App.tsx:281-303`, `CapacityPanel.tsx:12`, `RecommendationsPanel.tsx:12`, `DockviewApp.tsx:204-221`.

4. **#10 as-little-as-possible — Delete dead components and remove placeholder panels.** Delete `RightSidebar.tsx`, `LeftSidebar.tsx`, `TerminalTabs.tsx`, `RightSidebarHelpers.ts` (verify usage first). Drop Capacity and Recommendations from the default layout until they carry signal — keep the components for later but stop opening them on first launch. Collapse the four empty-state copies into one shared component with one voice. Evidence: `S2`, `S5`, `S9`, `G3`. Affects `apps/desktop/src/components/*.tsx`, `DockviewApp.tsx:114-160`, `App.tsx:260-268`.

5. **#8 thorough — Add global focus styling and a minimal feedback primitive.** A single `:focus-visible` rule for the whole app (`outline: 2px solid var(--accent-focus); outline-offset: 2px`); remove `outline: none` from `.session-list`. Introduce a small toast/inline-status primitive and route the 5 silently-swallowed catch blocks through it. Evidence: `V5`, `V6`, `C7`. Affects `App.css:233`, `App.tsx:73-184` (catch handlers), new `Toast`/`StatusBar` component.

## Out of scope for the redesign

- Rust sidecar API surface (`crates/orkworksd/`) — unchanged.
- Electron main process and IPC contract — unchanged.
- Switching from Dockview or xterm — both stay.
- **Multi-terminal / split / tiled terminal views — explicitly rejected: session = context, switching is the context-switch primitive; parallel terminal visibility is context degradation, not visibility.**
- Light-mode support — defer; document as a known gap.
- Reduced-motion compliance for xterm cursor blink — defer; document.
