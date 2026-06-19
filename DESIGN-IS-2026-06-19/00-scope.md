# Scope — Design Audit 2026-06-19

## Target
Whole OrkWorks desktop UI (Electron + React/TS) as it stands on `main` (HEAD `39fd19b`), including uncommitted in-flight changes (settings/hotkeys design + new-session hotkey work).

## Surfaces in scope
- App shell (titlebar with workspace identity)
- Dockview layout container + empty-state overlay + reset-layout flow
- Sessions panel (`SessionListPanel`) — grouped list, keyboard nav, focus/selection
- Terminal panel (`TerminalPanel`) + tabs (`TerminalTabs`)
- Side/auxiliary panels: `LeftSidebar`, `RightSidebar`, `SessionDetailPanel`, `CapacityPanel`, `RecommendationsPanel`, `CenterPanel`

## Primary files
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/App.css`
- `apps/desktop/src/components/*.tsx`

## Primary user
Power developer running many concurrent AI coding sessions.

## Primary task
Maintain situational awareness across multiple agent sessions — see what's running, where it's stuck, switch focus fast, and keep one terminal session in driver's seat without losing the others.

## Constraints
- Local-first; no cloud UI patterns
- MVP scope per `specs/orkworks-mvp.md`: observe + recommend (not control)
- Vocabulary: "OrkWorks" + "Peon" only — no broader fantasy naming
- Stack: Electron + React + Dockview + xterm.js

## Inputs
- Source code at HEAD + working tree
- No live screenshots this pass (static-read mode) — visual subagent will mark inferred CSS facts "INFERRED"

## Out of scope
- Marketing site / docs site
- Rust sidecar APIs (not user-facing surface)
- Settings/hotkeys panel design doc itself (`docs/superpowers/specs/2026-06-18-app-settings-hotkeys-design.md`) — this audit scores the *current* UI, not the planned one
