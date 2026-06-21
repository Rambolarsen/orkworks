# Architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + Dockview + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  └─ adr/                # Architecture Decision Records
├─ skills/                # Repo-level agent skills
└─ examples/
```

## Electron ↔ Rust sidecar communication

`electron/main.ts` spawns `orkworksd` as a child process and discovers its port by reading stdout for the line `ORKWORKSD_PORT=<n>`. The port is dynamic — there is no fixed localhost port. The frontend gets the URL via the preload bridge: `window.orkworks.getBackendUrl()`.

## Packaging and release

Desktop packaging lives under `apps/desktop/`. `electron-builder.yml` defines the product metadata and `extraResources` layout, while `scripts/package-release.mjs` maps the current host platform/arch to the matching Rust target triple, stages the built `orkworksd` binary into `crates/orkworksd/target/release/`, and invokes `electron-builder` with the matching CLI arch flag. CI runs the same path from `.github/workflows/release.yml`, with separate macOS x64 and arm64 jobs so the packaged sidecar always matches the bundled Electron arch.

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with backend discovery, workspace memory, layout memory, menu-command, panel-visibility, and app-settings methods. Adding new capabilities requires extending the preload, not relaxing context isolation.

`electron/layoutMemory.ts` persists the Dockview panel layout to `layout.json` in the Electron user data directory, using the same pattern as `workspaceMemory.ts`. Layout is serialized via Dockview's `toJSON()`/`fromJSON()` on every layout change (debounced 500ms) and restored on startup.

`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, and persisted menu accelerators. `getSettings()` and successful `saveHotkeys()` responses include a renderer-facing `defaultHotkeys` copy sourced from the main process, so the settings UI can restore defaults without duplicating canonical accelerators.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `App.tsx` polls `/sessions` every 2 seconds, restores the last active workspace session when `POST /workspace` returns `lastActiveSessionId`, and persists the newly selected active session back through `POST /workspace/active-session`. Session state flows: Rust structs → JSON API → `SessionInfo`/`WorkspaceInfo` TS types → React state → components.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `GET /sessions/:id/terminal-output`, `WS /sessions/:id/terminal`.

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling workspace restore on relaunch. The sidecar persists repo-local active session memory in `.orkworks/workspace.json`.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary, six modules:

- `main.rs` — Axum router, `AppState` (sessions + workspace + harness adapters), all HTTP/WS handlers, PTY lifecycle, session resume
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files)
- `harness.rs` — harness adapter types, command templates, resume strategy selection, capability flags
- `metadata.rs` — reads/writes `.orkworks/sessions/<id>.json`, `.orkworks/workspace.json`, and `.orkworks/events/<id>.terminal` (terminal output ring buffer) files
- `peon.rs` — observer config, ring buffer, harness invocation, inference parsing/validation, source-priority overwrite rules (driven by the debounce loop in `main.rs`; tuning knobs documented in `README.md`)
- `watcher.rs` — `notify`-based file watcher for `.orkworks/` changes

## Dockview panel layout

The renderer uses Dockview for a five-panel workspace: sessions, session detail, terminal, capacity, and recommendations. `DockviewApp` owns the panel registration and passes app state through a React context to panel components. `TerminalPanel` hosts the active live PTY session through `CenterPanel` and xterm.js over the backend WebSocket.

The titlebar shows the active workspace name and a workspace-switch action when a repo is open. A `ViewMenu` component in the titlebar provides per-panel shortcuts/toggles plus a "Reset Layout" action. Panel layouts persist to Electron userData via `layout.json` and restore on startup via Dockview's `toJSON()`/`fromJSON()` serialization.

The Sessions panel uses Dockview's native header chrome rather than an inner duplicated panel header. In the single-tab case, `DockviewApp` enables Dockview's full-width tab/header mode and renders the "new session" action in the header's right-actions slot so the header still behaves like a tab while matching the rest of the workspace subheader styling. Dockview tabs use a shared default tab component that hides the built-in close affordance; panel visibility is managed through the View menu and shortcuts instead of per-tab close buttons.

- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Update triggers

Update this file when:

- A new module is added to or removed from `crates/orkworksd/src/`
- `electron/preload.ts` exposes new or removed `window.orkworks` methods
- `apps/desktop/src/api.ts` adds or removes endpoints
- Port-discovery mechanism changes in `electron/main.ts`
- Panel layout changes (new panels, library swap)
- A major npm or Cargo dependency is added or removed
