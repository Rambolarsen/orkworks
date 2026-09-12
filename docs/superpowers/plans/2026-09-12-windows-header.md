# Windows Integrated Header Implementation Plan

> Use executing-plans inline for this single deliverable; request a diff-scoped /code-review low before handoff.

**Goal:** Deliver the user-approved Windows header.
**Architecture:** Electron owns native chrome; React/CSS owns the icon and workspace header. No new IPC or dependency.
**Tech Stack:** Electron 44, React, TypeScript, CSS, Node test runner.

## Global constraints

- Windows-only chrome change; preserve macOS/Linux behavior.
- Keep native controls, dragging, resizing, Alt menu access, and keyboard shortcuts.
- Reuse existing branded SVG asset; no generated artwork.
- Dark app appearance stays independent of OS theme (light app support is tracked separately).

## Task 1: Integrated Windows header

- [x] Record approved layout in the MVP shell specification and create its tracking issue.
- [x] Add platform chrome-option tests; run Node test runner to establish failure before implementation.
- [x] Define getWindowChromeOptions(platform) in electron/windowChrome.ts; Windows returns hidden style, overlay { color: '#0c0d10', symbolColor: '#eceef1', height: 38 }, autoHideMenuBar true; macOS returns hiddenInset; Linux returns an empty object. Spread it in BrowserWindow construction.
- [x] Import build/icon-dark.svg in App.tsx and place a Windows-only 24px img with alt OrkWorks before the workspace name.
- [x] Reserve env(titlebar-area-width) and env(titlebar-area-x) in Windows header CSS. Apply min-width:0 and text ellipsis to workspace text; keep icon/buttons/status from shrinking.
- [x] Add an always-available 38px draggable strip to the renderer recovery document.
- [x] Run desktop tests, pnpm exec tsc --noEmit, pnpm build, and Windows Electron smoke validation.
- [x] Run diff check, documentation checks, and /code-review low (no findings).
- [ ] Commit/push/open the required PR and inspect CI.
