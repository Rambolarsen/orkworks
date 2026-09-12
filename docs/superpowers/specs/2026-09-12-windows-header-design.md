# Windows integrated header

Approved by the user on 2026-09-12: replace the separate Windows title bar with one integrated header, OrkWorks icon at the far left before the workspace name, native minimize/maximize/close controls at the right, and normal dragging and resizing.

Use Electron titleBarStyle hidden plus titleBarOverlay on Windows only. Keep macOS hiddenInset and Linux's existing chrome. Auto-hide the Windows application menu, retaining Alt access and all menu shortcuts. Reuse build/icon-dark.svg in the renderer through Vite. The current app is dark regardless of the OS theme; use matching dark header colors for native buttons. OS theme changes must not make them illegible. Full app light-theme support remains issue #76.

Reserve the native overlay safe area with CSS environment variables, preserve clickable workspace controls, and truncate long workspace names before they overlap status or native controls. The existing 38px header remains the drag region. The renderer-failure page must also retain a drag region after native chrome is hidden.

Validation: platform option tests; desktop type checks/build/tests; an isolated Windows Electron smoke window for native controls, safe-area geometry, long names, and packaged asset loading. Verify actual drag/resize where available and report any manual verification gap.

## Acceptance criteria

- [x] Windows uses one integrated header with the OrkWorks icon before the workspace name.
- [x] Native window controls remain available with protected layout space.
- [x] Workspace actions remain clickable and long names truncate.
- [x] The application menu remains accessible with Alt.
- [x] macOS and Linux chrome options remain unchanged.

## Validation record

Tracked by issue #531. Platform and recovery tests pass; desktop type checks and production build pass. An isolated Electron 44 Windows window renders the actual header JSX/CSS and built SVG under the production content-security policy. At 900px width, native safe area ends at x=763 and status ends at x=743; the image loads and drag/no-drag regions match their roles under both OS theme settings. Mouse dragging, resizing, Snap layouts, and native-button clicks remain manual verification items.

Full desktop suite: 675/678 pass. The same three failures reproduce from an untouched archive of base cc80152: concurrent settings writers, preload Windows path expectation, and a CRLF-sensitive SessionDetailPanel source assertion. README and docs/index.md claims were reviewed; this header change does not alter their architecture or onboarding claims.
