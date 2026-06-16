# Architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + xterm.js + react-resizable-panels
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  └─ adr/                # Architecture Decision Records
├─ skills/                # Repo-level agent skills
└─ examples/
```

## Electron ↔ Rust sidecar communication

`electron/main.ts` spawns `orkworksd` as a child process and discovers its port by reading stdout for the line `ORKWORKSD_PORT=<n>`. The port is dynamic — there is no fixed localhost port. The frontend gets the URL via the preload bridge: `window.orkworks.getBackendUrl()`.

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with only `getBackendUrl()` and `openWorkspace()`. Adding new capabilities requires extending the preload, not relaxing context isolation.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `App.tsx` polls `/sessions` every 2 seconds and sorts by status (`creating → running → ended → killed → error`). Session state flows: Rust structs → JSON API → `SessionInfo` TS type → React state → components.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary, four modules:

- `main.rs` — Axum router, `AppState` (sessions + workspace), all HTTP/WS handlers, PTY lifecycle
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files)
- `metadata.rs` — reads/writes `.orkworks/sessions/<id>.json` files
- `watcher.rs` — `notify`-based file watcher for `.orkworks/` changes

## Three-panel layout

Left sidebar: workspace + session list. Center: `TerminalTabs` (xterm.js tabs, one PTY per session, WebSocket to backend). Right sidebar: session overview grouped by status (Needs You / Blocked / Failed / Done / Stale / Working / Idle / Capacity). Panel sizes via `react-resizable-panels`.

- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Update triggers

Update this file when:

- A new module is added to or removed from `crates/orkworksd/src/`
- `electron/preload.ts` exposes new or removed `window.orkworks` methods
- `apps/desktop/src/api.ts` adds or removes endpoints
- Port-discovery mechanism changes in `electron/main.ts`
- Panel layout changes (new panels, library swap)
- A major npm or Cargo dependency is added or removed
