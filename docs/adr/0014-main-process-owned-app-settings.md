# Main-process-owned app settings and menu accelerators

- Status: accepted
- Deciders: user
- Date: 2026-06-19

## Context

OrkWorks already persists app-owned desktop state in Electron `userData` for workspace memory and Dockview layout. Menu accelerators are still hard-coded in the Electron main process, so they cannot be customized without code changes.

The first settings slice is configurable hotkeys for existing menu actions. Active shortcuts must remain native Electron menu accelerators rather than renderer-only keyboard handlers, because the menu is the current source of command dispatch for these actions.

## Decision

Persist a versioned `AppSettings` document in Electron `userData` as `settings.json`. The Electron main process owns loading, default merging, validation, writing, and applying app settings.

The renderer accesses settings only through narrow preload APIs:

- `getSettings()`
- `saveHotkeys(hotkeys)`
- `setHotkeyCaptureActive(active)`

The Electron application menu is built from canonical settings. Hotkey saves validate and build the next menu before writing settings, then write settings and apply the new menu as one observable operation.

## Consequences

- **Easier**: App settings have a clear owner and future settings sections can share the same persistence boundary.
- **Easier**: Native Electron menu accelerators remain the source of truth for active shortcuts.
- **Easier**: Renderer code can focus on presentation and proposed edits instead of privileged filesystem or menu state.
- **Harder**: Main-process settings changes need IPC and menu regression tests.
- **Harder**: Hotkey capture must suppress existing accelerators while recording a replacement chord.
