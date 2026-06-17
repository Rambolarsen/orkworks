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

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with `getBackendUrl()`, `getInitialWorkspace()`, and `openWorkspace()`. Adding new capabilities requires extending the preload, not relaxing context isolation.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `App.tsx` polls `/sessions` every 2 seconds and sorts by status (`creating → running → ended → killed → error`). Session state flows: Rust structs → JSON API → `SessionInfo` TS type → React state → components.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `WS /sessions/:id/terminal`.

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling session resume on relaunch.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary, five modules:

- `main.rs` — Axum router, `AppState` (sessions + workspace), all HTTP/WS handlers, PTY lifecycle, session resume
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files)
- `harness.rs` — harness adapter types, command templates, resume strategy selection, capability flags
- `metadata.rs` — reads/writes `.orkworks/sessions/<id>.json` and `.orkworks/workspace.json` files
- `watcher.rs` — `notify`-based file watcher for `.orkworks/` changes

## Dockview panel layout

The renderer uses Dockview for a five-panel workspace: sessions, session detail, terminal, capacity, and recommendations. `DockviewApp` owns the panel registration and passes app state through a React context to panel components. `TerminalPanel` hosts the active live PTY session through `CenterPanel` and xterm.js over the backend WebSocket.

- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Update triggers

Update this file when:

- A new module is added to or removed from `crates/orkworksd/src/`
- `electron/preload.ts` exposes new or removed `window.orkworks` methods
- `apps/desktop/src/api.ts` adds or removes endpoints
- Port-discovery mechanism changes in `electron/main.ts`
- Panel layout changes (new panels, library swap)
- A major npm or Cargo dependency is added or removed
