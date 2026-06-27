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

`electron/main.ts` spawns `orkworksd` as a child process and discovers its port by reading stdout for the line `ORKWORKSD_PORT=<n>`. The app icon is platform-aware: macOS uses `icon.png`/`icon-dark.png` (squircle background baked in) via `app.dock.setIcon()`; Windows uses `icon.ico`/`icon-dark.ico` (transparent background, multi-resolution) via `BrowserWindow.setIcon()`. Both swap on `nativeTheme` change. The port is dynamic — there is no fixed localhost port. The frontend gets the URL via the preload bridge: `window.orkworks.getBackendUrl()`.

## Packaging and release

Desktop packaging lives under `apps/desktop/`. `electron-builder.yml` defines the product metadata and `extraResources` layout, while `scripts/package-release.mjs` maps the current host platform/arch to the matching Rust target triple, stages the built `orkworksd` binary into `crates/orkworksd/target/release/`, and invokes `electron-builder` with the matching CLI arch flag. CI runs the same path from `.github/workflows/release.yml`, with separate macOS x64 and arm64 jobs so the packaged sidecar always matches the bundled Electron arch.

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with backend discovery, workspace memory, layout memory, menu-command, panel-visibility, and app-settings methods. Adding new capabilities requires extending the preload, not relaxing context isolation.

`electron/layoutMemory.ts` persists the Dockview panel layout to `layout.json` in the Electron user data directory, using the same pattern as `workspaceMemory.ts`. Layout is serialized via Dockview's `toJSON()`/`fromJSON()` on every layout change (debounced 500ms) and restored on startup.

`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, persisted menu accelerators, and durable provider settings (`ProviderSettings`). In user-facing copy these provider settings are model provider settings; internal code keeps the existing `ProviderSettings` name. `getSettings()` and successful `saveHotkeys()` responses include a renderer-facing `defaultHotkeys` copy sourced from the main process, so the settings UI can restore defaults without duplicating canonical accelerators. Electron settings now push both retention and provider settings into the sidecar after port discovery. `electron/providerSettingsSync.ts` handles the `POST /settings/providers` push on startup, workspace switch, and explicit save. Provider model lists are fetched from `GET /providers/:id/models` and cached in memory at startup; the renderer reads them via the `getProviderModels` preload method.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `App.tsx` polls `/sessions` every 2 seconds, restores the last active workspace session when `POST /workspace` returns `lastActiveSessionId`, and persists the newly selected active session back through `POST /workspace/active-session`. Session state flows: Rust structs → JSON API → `SessionInfo`/`WorkspaceInfo` TS types → React state → components. The session payload now exposes canonical `harnessId`, `modelProviderId`, and `modelId` fields alongside the legacy fields during the migration window.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, `GET /sessions/:id/terminal-output`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `GET /harnesses`, and `WS /sessions/:id/terminal`. Planned per [ADR 0019](../adr/0019-attention-signal-endpoint-opt-in-hook-install.md): `POST /sessions/:id/attention` (harness-agnostic attention signal write), `GET /workspace/attention-hook/status`, `POST /workspace/attention-hook/install` (user-confirmed Claude Code hook installer).

Every spawned PTY session receives `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` in its environment, so an in-session hook can address the sidecar without any config look-up. Harness-native session IDs are reported through `POST /sessions/:id/harness-session`, which writes `resume.harnessSessionId` plus source/confidence/captured-at metadata. Deterministic harness sources such as OpenCode env, Claude hook JSON, and Codex exec JSONL outrank Peon inference; interactive status probes remain user-triggered.

`POST /sessions` now accepts `{ harnessId, model, initialPrompt }`. The renderer's New agent session dialog labels harness choices as coding tools, can fall back to the default shell session if harness metadata is temporarily unavailable, and still sends the selected harness config id so session rows and remembered-session resume behavior remain compatible.

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling workspace restore on relaunch. The sidecar persists workspace-scoped state to `~/.orkworks/workspaces/<path-hash>/workspace.json`.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary, eight modules:

- `main.rs` — Axum router, `AppState` (sessions + workspace + harness adapters + `ProviderManager`), all HTTP/WS handlers, PTY lifecycle, session resume, workspace path hashing, global metadata directory resolution
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files)
- `harness.rs` — harness adapter types, command templates, resume strategy selection, capability flags
- `metadata.rs` — reads/writes `sessions/<id>.json`, `workspace.json`, and `events/<id>.terminal` (terminal output ring buffer) files under the global metadata root (`~/.orkworks/workspaces/<hash>/`)
- `migration.rs` — one-time migration of legacy `<workspace>/.orkworks/` data into the global store
- `peon.rs` — observer config, ring buffer, prompt building, inference parsing/validation, source-priority overwrite rules, timer-based idle detection (`PEON_IDLE_TIMEOUT`, default 15s), summary normalization (strips "User is/wants/typed" prefixes), and `is_terminal_observed_status` for clearing stale states on user input (driven by the debounce loop in `main.rs`; tuning knobs documented in `README.md`)
- `providers.rs` — fixed provider registry, applied-settings store, persisted runtime state, fallback runner (`run_inference` skips disabled/capped providers in fallback order), model listing (`list_models` runs each provider's configured list-models CLI command). On Unix, `ProcessRunner` calls `setsid()` + closes inherited fds ≥ 3 before spawning the harness subprocess (via `libc`), preventing PTY leakage into provider processes. This module still carries the historical `Provider*` names, but today it is modeling the Peon inference tool registry rather than the user-facing coding-tool selector. It exposes `GET /providers` for live runtime state, `GET /providers/:id/models` for available models, and `POST /settings/providers` for durable settings application. The session Peon loop routes through `ProviderManager::run_inference`. Per-provider peon model is configured in Settings.
- `watcher.rs` — `notify`-based file watcher for session metadata changes under the global store

For the current Rust domain model itself, see [domain-entities.md](./domain-entities.md).

## Dockview panel layout

The renderer uses Dockview for a four-panel workspace: sessions, session detail, terminal, and recommendations. The capacity panel exists as a non-Providers stub, closed by default. `DockviewApp` owns the panel registration and passes app state through a React context to panel components. `TerminalPanel` hosts the active live PTY session through `CenterPanel` and xterm.js over the backend WebSocket. The session detail panel includes read-only `Coding tool`, `Model provider`, `Model`, and `Provider state` fields for the selected session.

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
