# Architecture

```text
orkworks/
├─ apps/desktop/          # Electron + React/TypeScript + Dockview + xterm.js
├─ crates/orkworksd/      # Rust sidecar (Axum HTTP/WS, PTY via portable-pty)
├─ docs/
│  ├─ adr/                # Architecture Decision Records
│  └─ agents/             # Agent-facing docs (this file, domain-entities, apm)
├─ skills/                # Repo-level agent skills
└─ specs/                 # Authoritative product specs
```

## Electron ↔ Rust sidecar communication

`electron/main.ts` spawns `orkworksd` as a child process and discovers its port by reading stdout for the line `ORKWORKSD_PORT=<n>`. The app icon is platform-aware: macOS uses `icon.png`/`icon-dark.png` (squircle background baked in) via `app.dock.setIcon()`; Windows uses `icon.ico`/`icon-dark.ico` (transparent background, multi-resolution) via `BrowserWindow.setIcon()`. Both swap on `nativeTheme` change. The port is dynamic — there is no fixed localhost port. The frontend gets the URL via the preload bridge: `window.orkworks.getBackendUrl()`.

## Packaging and release

Desktop packaging lives under `apps/desktop/`. `electron-builder.yml` defines the product metadata and `extraResources` layout, while `scripts/package-release.mjs` maps the current host platform/arch to the matching Rust target triple, stages the built `orkworksd` binary into `crates/orkworksd/target/release/`, and invokes `electron-builder` with the matching CLI arch flag. CI runs the same path from `.github/workflows/release.yml`, with separate macOS x64 and arm64 jobs so the packaged sidecar always matches the bundled Electron arch.

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with backend discovery, workspace memory, layout memory, menu-command, panel-visibility, and app-settings methods. Adding new capabilities requires extending the preload, not relaxing context isolation. `titleBarStyle: 'hiddenInset'` is set on macOS so the web content extends into the title bar area; the renderer reads `window.orkworks.platform` (exposed synchronously by the preload) to apply a `data-platform` attribute on `<html>`, which CSS uses to add traffic-light clearance (`padding-left: 80px`) on darwin only.

`electron/layoutMemory.ts` persists the Dockview panel layout to `layout.json` in the Electron user data directory, using the same pattern as `workspaceMemory.ts`. Layout is serialized via Dockview's `toJSON()`/`fromJSON()` on every layout change (debounced 500ms) and restored on startup.

`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, a persisted `debug.showSessionIds` flag for gating internal session identifiers in the Details panel, persisted menu accelerators, and durable provider settings (`ProviderSettings`). In user-facing copy these provider settings are model provider settings; internal code keeps the existing `ProviderSettings` name. `getSettings()` and successful `saveHotkeys()` responses include a renderer-facing `defaultHotkeys` copy sourced from the main process, so the settings UI can restore defaults without duplicating canonical accelerators. Electron settings now push both retention and provider settings into the sidecar after port discovery. `electron/providerSettingsSync.ts` handles the `POST /settings/providers` push on startup, workspace switch, and explicit save. Provider model lists are fetched from `GET /providers/:id/models` and cached in memory at startup; the renderer reads them via the `getProviderModels` preload method. Draft Ollama verification in Settings bypasses that cache through the `verifyOllama` preload bridge and `POST /settings/providers/ollama/verify`, so unsaved URLs can be checked before persistence.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `App.tsx` polls `/sessions` every 2 seconds, restores the last active workspace session when `POST /workspace` returns `lastActiveSessionId`, and persists the newly selected active session back through `POST /workspace/active-session`. Session state flows: Rust structs → JSON API → `SessionInfo`/`WorkspaceInfo` TS types → React state → components. The payload exposes canonical `harnessId`, `modelProviderId`, and `modelId` fields alongside legacy fields during the migration window. Its session state is the canonical `creating → alive → stopping → dead` lifecycle, with alive-only attention (`working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`); `connectivity`, `terminalOutcome`, `lastActivityAt`, and `resumeOptions` provide supporting runtime and history context. PTY lifetime is session-runtime-owned in the sidecar; the terminal WebSocket is an attach/detach transport rather than the thing that keeps the PTY alive.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `PUT /workspace/active-harnesses`, `GET/POST /sessions`, `DELETE /sessions/:id`, `DELETE /sessions/:id/forget`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, `POST /sessions/:id/attention`, `GET /workspace/attention-hook/status`, `POST /workspace/attention-hook/install`, `GET /sessions/:id/terminal-output`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`, `POST /settings/retention`, `GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`, and `WS /sessions/:id/terminal`.

Every spawned PTY session receives `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` in its environment, so an in-session hook can address the sidecar without any config look-up. Harness-native session IDs are reported through `POST /sessions/:id/harness-session`, which writes `resume.harnessSessionId` plus source/confidence/captured-at metadata. Deterministic harness sources such as OpenCode env, Claude hook JSON, and Codex exec JSONL outrank Peon inference; interactive status probes remain user-triggered.

`POST /sessions/:id/attention` accepts `{status, message?}` from a harness's own notification mechanism (e.g. Claude Code's `Notification` hook, via `crates/orkworksd/scripts/report-claude-session-from-hook.sh`) and writes `observedStatus`/`summary` with `metadataSource: "agent"`, `metadataConfidence: 1.0`, gated by the same `peon::should_overwrite` priority/staleness rule Peon uses. `GET /workspace/attention-hook/status` and `POST /workspace/attention-hook/install` (implemented in `http/hook_handlers.rs`) back the "Install Notification hook" affordance in Settings: install is always explicit and user-confirmed, merges one idempotent hook entry into the workspace's `.claude/settings.local.json` (never `settings.json`), and never runs automatically at session spawn — see [ADR 0019](../adr/0019-attention-signal-endpoint-opt-in-hook-install.md). The reporter script ships as `extraResources` alongside the `orkworksd` binary (`electron-builder.yml`) and is located at runtime via `hook_handlers::claude_hook_script_path()`, but the installed hook command never persists that path directly — on Linux AppImage builds it resolves inside the per-launch temporary FUSE mount and would go stale on the next relaunch. Instead, install copies the script to a stable, packaging-independent location (`~/.orkworks/hook-scripts/`, via `hook_handlers::ensure_stable_claude_hook_script()`) and points the installed command there; the copy refreshes on every install, so it self-heals across app updates too.

`POST /sessions` now accepts `{ harnessId, model, initialPrompt }`. The renderer's New agent session dialog labels harness choices as coding tools, can fall back to the default shell session if harness metadata is temporarily unavailable, and still sends the selected harness config id so session rows and remembered-session resume behavior remain compatible.

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling workspace restore on relaunch. The sidecar persists workspace-scoped state to `~/.orkworks/workspaces/<path-hash>/workspace.json`.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary. Top-level modules:

- `main.rs` — Axum router, `AppState` / `SessionHandle` / `WorkspaceState` / `PeonState` / `RetentionConfig` struct definitions, `main()`, `health_check()`, `#[cfg(test)] pub(crate) mod test_support` (shared test helpers), and a slim `mod tests` covering route registration and core AppState invariants
- `http/` — HTTP handler submodules (`ErrorResponse` declared in `http/mod.rs`):
  - `harness_handlers.rs` — harness CRUD (`GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`)
  - `hook_handlers.rs` — Claude Code attention hook install/status (`GET /workspace/attention-hook/status`, `POST /workspace/attention-hook/install`), reporter script path resolution
  - `provider_handlers.rs` — provider query handlers (`GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`)
  - `retention_handlers.rs` — retention config handler (`POST /settings/retention`)
  - `session_handlers.rs` — session/workspace HTTP handlers (`POST /workspace`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, etc.) and associated request/response types. `POST /workspace` reconciles sessions orphaned by a previous daemon run via `metadata::reconcile_orphaned_session`: stale "running"/"creating" sessions are completed to `ended`, and sessions persisted mid-`ending` consume their `pendingTerminalStatus` as the final status so they cannot stay stuck in the ending phase
- `runtime/` — background-task and PTY submodules:
  - `peon_runtime.rs` — `peon_loop` (continuous Peon observation loop); idle sessions enter an in-memory hold and resume observation only after qualifying user input
  - `retention.rs` — `retention_cleanup_task`, `retention_cleanup_once`
  - `session_runtime.rs` — session-runtime-owned PTY/process startup, bounded PTY/persistence backpressure queues, output draining, replay state, attachment ownership, child wait/finalization
  - `terminal_http.rs` — `get_terminal_output`, `session_terminal_handler` (WebSocket upgrade / attach entrypoint)
  - `terminal_runtime.rs` — env helpers (`terminal_env_overrides`, `session_env_overrides`, `should_forward_terminal_env`), `TerminalAction` dispatch, `set_session_status`, websocket attach/detach transport
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files)
- `harness.rs` — harness adapter types, command templates, resume strategy selection, capability flags
- `harness_registry.rs` — built-in harness configs and adapters, `resolve_adapter_harness_id`, `default_shell_command`, disk persistence helpers. `HarnessConfig` carries an optional `HarnessPeonConfig` sub-struct that embeds all peon inference parameters (headless args, model arg template, static model list, list-models command) for that instance. Adding a new harness with peon support requires one `HarnessConfig` entry; `providers.rs` derives `ProviderDefinition`s from it at startup.
- `metadata.rs` — reads/writes `sessions/<id>.json`, `workspace.json`, and `events/<id>.terminal` (terminal output ring buffer) files under the global metadata root (`~/.orkworks/workspaces/<hash>/`), including lifecycle, attention, session connectivity, terminal outcome, last activity, and resume-option metadata
- `migration.rs` — one-time migration of legacy `<workspace>/.orkworks/` data into the global store
- `peon.rs` — observer config, ring buffer, in-memory observation state, prompt building, inference parsing/validation, source-priority overwrite rules, timer-based idle detection (`PEON_IDLE_TIMEOUT`, default 15s), summary normalization (strips "User is/wants/typed" prefixes), and usage-limit detection from terminal output
- `providers.rs` — provider definitions, applied-settings store, persisted runtime state, fallback runner (`run_inference` skips disabled/capped providers in fallback order), model listing (`list_models` runs each provider's configured list-models CLI command). `builtin_provider_registry()` contains only ollama (HTTP-based, no harness). All other provider definitions are derived at startup by `derive_from_harness_configs()`, which maps each `HarnessConfig.peon` to a `ProviderDefinition` — so peon config lives in one place (the harness entry) rather than being duplicated here. On Unix, `ProcessRunner` calls `setsid()` + closes inherited fds ≥ 3 before spawning the harness subprocess (via `libc`), preventing PTY leakage into provider processes. This module still carries the historical `Provider*` names, but today it is modeling the Peon inference tool registry rather than the user-facing coding-tool selector. It exposes `GET /providers` for live runtime state, `GET /providers/:id/models` for available models, and `POST /settings/providers` for durable settings application. The session Peon loop routes through `ProviderManager::run_inference`. Per-provider peon model is configured in Settings.
- `session_types.rs` — `SessionInfo` struct, lifecycle and attention enums, and the renderer-facing session contract
- `session_view.rs` — lifecycle-aware session projection, connectivity and terminal-outcome derivation, conflict detection, and resume-option derivation
- `watcher.rs` — `notify`-based file watcher for session metadata changes under the global store
- `workspace_runtime.rs` — `iso_now`, `orkworks_global_dir` (workspace path hashing to global store location)

For the current Rust domain model itself, see [domain-entities.md](./domain-entities.md).

## Dockview panel layout

The renderer uses Dockview for a four-panel workspace: sessions, session detail, terminal, and recommendations. The capacity panel exists as a non-Providers stub, closed by default. `DockviewApp` owns the panel registration and passes app state through a React context to panel components. `TerminalPanel` hosts the active live PTY session through `CenterPanel` and xterm.js over the backend WebSocket attach channel. Inactive sessions do not need to stay attached to keep their PTYs running; only the active terminal stays attached. The session detail panel includes read-only `Coding tool`, `Model provider`, `Model`, and `Provider state` fields for the selected session, plus debug-only `OrkWorks session ID` / `Harness session ID` fields when `Show debug metadata` is enabled.

The titlebar shows the active workspace name and a workspace-switch action when a repo is open. A `ViewMenu` component in the titlebar provides per-panel shortcuts/toggles plus a "Reset Layout" action. Panel layouts persist to Electron userData via `layout.json` and restore on startup via Dockview's `toJSON()`/`fromJSON()` serialization.

The Sessions panel uses Dockview's native header chrome rather than an inner duplicated panel header. In the single-tab case, `DockviewApp` enables Dockview's full-width tab/header mode and renders the "new session" action in the header's right-actions slot so the header still behaves like a tab while matching the rest of the workspace subheader styling. Dockview tabs use a shared default tab component that hides the built-in close affordance; panel visibility is managed through the View menu and shortcuts instead of per-tab close buttons. Session sorting and attention routing are lifecycle-aware: only alive sessions receive live attention, while dead sessions remain as historical context.

- PTY handles only text I/O; voice (native harness) bypasses PTY entirely

## Update triggers

Update this file when:

- A new module is added to or removed from `crates/orkworksd/src/`
- `electron/preload.ts` exposes new or removed `window.orkworks` methods
- `apps/desktop/src/api.ts` adds or removes endpoints
- Port-discovery mechanism changes in `electron/main.ts`
- Panel layout changes (new panels, library swap)
- A major npm or Cargo dependency is added or removed
