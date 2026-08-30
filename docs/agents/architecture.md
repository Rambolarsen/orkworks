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

### Sidecar lifecycle and runtime recovery

`electron/sidecarLifecycle.ts` owns sidecar process startup, port discovery, and
failure recovery. Each launch is a monotonically numbered generation with its
own readiness promise. Only the current generation may publish a port, change
the active lifecycle state, or notify the renderer; exits, errors, timers, and
stdout from a replaced generation are ignored. Spawn errors, exits before the
port line, and the 10-second readiness timeout reject that generation's
readiness and clear the active port. Once a process has announced a port, a
later process failure invalidates the port and reports the failure to the main
process instead of leaving API callers waiting on a stale promise.

Sidecar readiness is followed by a separate generation-owned restoration gate
in `electron/backendRestoration.ts`. After the port is known, Electron main
restores the remembered workspace, applies persisted retention settings, and
pushes persisted provider settings. These operations share an abort signal. A
restoration timeout or workspace-restoration failure rejects readiness and
publishes an unavailable state; retention-setting and provider-setting failures
are logged as best-effort application failures, and readiness proceeds. The
workspace restoration and settings attempts complete before `get-backend-url`
resolves or the renderer receives the `ready` lifecycle event. Initial startup
uses the last existing workspace path when available, otherwise the
development repository or the packaged home directory. A workspace switch
persists the selected path before starting its replacement generation, and
stale restoration work is aborted so an older workspace cannot become ready
afterward.

Automatic recovery is bounded: one recovery sequence makes at most three
sidecar launches in total (the initial launch plus two automatic retries), with
default delays of 250 ms and 1 second. Only one delayed recovery is scheduled
at a time. After the third failed launch the lifecycle becomes `exhausted` and
waits for the user's explicit Retry action; a generation that remains ready for
five seconds resets the automatic-attempt counter. Explicit retry starts a
fresh generation using the last sidecar working directory and resets the
counter.

The preload bridge exposes `onBackendLifecycle` and `retryBackend`. Lifecycle
events are the narrow union `starting`, `retrying`, `ready` (with a validated
port), `failed` (with a stable failure message), and `exhausted` (with a stable
failure message). Preload canonicalizes exact event shapes and replays the
latest main-process snapshot for late subscribers, while preserving live-event
ordering and returning an unsubscribe function. The renderer maps these events
to its backend status, stops session polling unless the backend is connected
and workspace restoration is complete, and shows a Retry panel for
unreachable or exhausted states without discarding existing workspace/session
state.

React render exceptions are handled in the renderer by `ErrorBoundary`.
Electron main has an independent fallback for main-frame `did-fail-load` and
`render-process-gone` events, with bounded, sanitized console diagnostics. It
loads a resource-free inline recovery document for those failures; the
document has one user-triggered Retry action that returns to the captured
development URL or exact packaged `file:` document. A recovery-load guard
prevents repeated automatic fallback navigation and resets when the original
document begins or finishes loading, or when recovery navigation fails. This
path does not depend on React or the preload bridge, so it remains available
when the normal renderer document is not.

## Packaging and release

Desktop packaging lives under `apps/desktop/`. `electron-builder.yml` defines the product metadata and `extraResources` layout, while `scripts/package-release.mjs` maps the current host platform/arch to the matching Rust target triple, stages the built `orkworksd` binary into `crates/orkworksd/target/release/`, and invokes `electron-builder` with the matching CLI arch flag. CI runs the same path from `.github/workflows/release.yml`, with separate macOS x64 and arm64 jobs so the packaged sidecar always matches the bundled Electron arch.

## Preload bridge (security boundary)

Electron runs with `nodeIntegration: false` and `contextIsolation: true` (ADR 0009). The renderer cannot call Node APIs directly. All privileged operations go through `electron/preload.ts`, which exposes `window.orkworks` with backend discovery, workspace memory, layout memory, menu-command, panel-visibility, and app-settings methods. Adding new capabilities requires extending the preload, not relaxing context isolation. `titleBarStyle: 'hiddenInset'` is set on macOS so the web content extends into the title bar area; the renderer reads `window.orkworks.platform` (exposed synchronously by the preload) to apply a `data-platform` attribute on `<html>`, which CSS uses to add traffic-light clearance (`padding-left: 80px`) on darwin only.

Leaving `sandbox` unset on `webPreferences` means Electron runs `preload.ts` under its sandboxed preload loader, which only resolves Node/Electron built-ins — a plain per-file `tsc` require of any other local `electron/*.ts` module fails at runtime (`module not found`) even though it type-checks and compiles cleanly. `scripts/build-preload.mjs` (config in `scripts/preloadBuildConfig.mjs`) runs esbuild after `tsc` in the `dev`, `build`, and `dist` npm scripts to bundle `electron/preload.ts` and its local imports into a single self-contained `dist-electron/preload.js`, keeping `electron` external so Electron's own binding still resolves it. The rest of `electron/*.ts` (main-process code, not sandboxed) keeps its plain per-file `tsc` output and may freely `require()` sibling modules.

`electron/layoutMemory.ts` persists the Dockview panel layout to `layout.json` in the Electron user data directory, using the same pattern as `workspaceMemory.ts`. Layout is serialized via Dockview's `toJSON()`/`fromJSON()` on every layout change (debounced 500ms) and restored on startup.

`electron/settingsMemory.ts` owns app-level settings in Electron `userData`, including hotkey validation, default hotkeys, a persisted `debug.showSessionIds` flag for gating internal session identifiers in the Details panel, persisted menu accelerators, and durable provider settings (`ProviderSettings`). In user-facing copy these provider settings are model provider settings; internal code keeps the existing `ProviderSettings` name. `getSettings()` and successful `saveHotkeys()` responses include a renderer-facing `defaultHotkeys` copy sourced from the main process, so the settings UI can restore defaults without duplicating canonical accelerators. Electron settings now push both retention and provider settings into the sidecar after port discovery. Explicit saves return sidecar application status so the renderer can distinguish durable local persistence from a pending sidecar application. `electron/providerSettingsSync.ts` handles the `POST /settings/providers` push on startup, workspace switch, and explicit save. Provider model lists are fetched from `GET /providers/:id/models` and cached in memory at startup; the renderer reads them via the `getProviderModels` preload method. Draft Ollama verification in Settings bypasses that cache through the `verifyOllama` preload bridge and `POST /settings/providers/ollama/verify`, so unsaved URLs can be checked before persistence. The preload contract exposes `saveActiveHarnessesWithIntegrations(ids)` as the typed renderer entry point for the Tools subsection's combined active-tool persistence and integration reconciliation flow. That operation is deliberately an Electron-main orchestration seam, not a renderer-to-sidecar mutation shortcut: the renderer submits only the requested active-tool IDs, Electron main persists them first, then reconciles each coding tool independently through the existing integration routes, and returns one structured `ActiveHarnessSaveResult` containing the active-harness outcome plus a per-tool partial result for install, repair, uninstall, unsupported skip, failure, or `stale_workspace`. Electron main rejects old-generation results as `stale_workspace` when the workspace path or sidecar generation changes mid-save, so a replaced workspace cannot report a successful save for the old one.

Electron-main denies renderer popup and navigation requests, delegates valid HTTP(S) links to the OS default browser, and keeps same-origin Vite reloads inside Electron during development. xterm's OSC-8 terminal links use the narrow `openExternalLink` preload bridge, so they bypass renderer `window.open()` while still receiving Electron-main URL validation. Renderer load/process failures log bounded diagnostics and show a resource-free local recovery document; its retry returns to the captured original app URL, with only that exact packaged `file:` target allowlisted.

## Frontend → backend API

`apps/desktop/src/api.ts` defines TypeScript types and fetch wrappers for the REST API. `workspaceSessionController.ts` owns the single renderer polling loop, workspace switching, session refresh/merge, pending-create reconciliation, active-session restoration, session lifecycle actions, terminal pruning, and stale/disposed operation rejection. `App.tsx` retains React state and view wiring, passes controller callbacks into the UI, and enables or disables polling with the application lifecycle. `settingsController.ts` owns the Settings modal's durable snapshot versus draft, subsection-local save/revert behavior, provider verification, and partial-save status; the Coding tools subsection saves through the Electron-main orchestration seam above, Hotkeys owns its own Save/Cancel/Restore lifecycle, Providers keep Apply/Save, and Retention plus Debug stay immediate or field-local. Electron remains authoritative for defaults and persistence. Session state flows: Rust structs → JSON API → `SessionInfo`/`WorkspaceInfo` TS types → controller callbacks → React state → components. The payload exposes canonical `harnessId`, `modelProviderId`, and `modelId` fields alongside legacy fields during the migration window. Its session state is the canonical `creating → alive → stopping → dead` lifecycle, with alive-only attention (`working`, `idle`, `needs_you`, `blocked`, `failed`, or `capped`); `connectivity`, `terminalOutcome`, `lastActivityAt`, `lastOutputAt`, and `resumeOptions` provide supporting runtime and history context. `lastActivityAt` tracks meaningful situation changes for task history, while `lastOutputAt` tracks raw PTY output; session-list recency selects the newest valid timestamp. PTY lifetime is session-runtime-owned in the sidecar; the terminal WebSocket is an attach/detach transport rather than the thing that keeps the PTY alive.

Key endpoints: `POST /workspace`, `POST /workspace/active-session`, `PUT /workspace/active-harnesses`, `GET/POST /sessions`, `DELETE /sessions/:id`, `DELETE /sessions/:id/forget`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, `POST /sessions/:id/attention`, `POST /sessions/:id/plan-path`, `POST /sessions/:id/debug-injection`, `GET /sessions/:id/plan-content`, `POST /sessions/:id/request-plan-review`, `GET /workspace/integrations/:harness_id/status`, `POST /workspace/integrations/:harness_id/install`, `POST /workspace/integrations/:harness_id/uninstall`, `GET /sessions/:id/terminal-output`, `GET /sessions/:id/summary-log`, `POST /sessions/:id/workflow-observations`, `GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`, `POST /settings/retention`, `GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`, and `WS /sessions/:id/terminal`.

`GET /sessions/:id/summary-log` returns `{ "entries": [{ "timestamp", "summary", "source", "confidence" }] }` in append order, where `confidence` is nullable; a missing workspace, session log, or checkpoint returns `{ "entries": [] }`. Internal event types and status fields are excluded. `apps/desktop/src/api.ts`'s `getSummaryLog` fetches it, and `SessionDetailPanel` renders the checkpoint history as a "Task history" section (ADR 0029) — distinct from the session `label`, which is a stable, one-shot Peon-authored topic rather than this turn-by-turn activity log. (design, not yet implemented — see issue #313) ADR 0042 plans to remove this route and "Task history" section in favor of the current-summary snapshot and workflow-observation contract described below.

Every spawned PTY session receives `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` in its environment, so an in-session hook can address the sidecar without any config look-up. Harness-native session IDs are reported through `POST /sessions/:id/harness-session`, which writes `resume.harnessSessionId` plus source/confidence/captured-at metadata. Deterministic supported sources such as the installable OpenCode `session.created` plugin (issue #110; reads the same two env vars via `process.env` inside OpenCode's plugin host) and Claude hook JSON outrank Peon inference; interactive status probes remain user-triggered.

`POST /sessions/:id/attention` accepts `{status, message?, planPath?, cwd?}` from a harness's own notification mechanism. `planPath` is optional: a string sets it, `null` clears it, and omission preserves it. When no harness path is present, sidecar output persistence may associate a validated printed path beneath `docs/superpowers/plans/` or `specs/`. `GET /sessions/:id/plan-content` returns the validated Markdown content only to Electron main; `POST /sessions/:id/request-plan-review` accepts only the selected session ID and injects the fixed, user-approved prompt after PTY acceptance. A terminal plan link uses the narrower authenticated `POST /sessions/:id/select-terminal-plan` handoff: renderer passes only that session ID plus the exact clicked terminal text, while Rust resolves and persists an anchored reference in the launch worktree or a linked worktree from the same Git family. Electron generates a per-sidecar secret for these endpoints, passes it in the sidecar environment, and excludes it from PTY child-process environments. The renderer never receives filesystem paths or supplies arbitrary terminal text; see [ADR 0025](../adr/0025-authenticated-session-plan-handoff.md), [ADR 0034](../adr/0034-user-approved-session-review-prompt.md), and [ADR 0039](../adr/0039-terminal-plan-link-selection.md).

`GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}` back both the explicit per-tool Settings affordance and Electron main's combined Tools Save orchestration. The renderer never calls those mutation routes directly for the combined save path: it invokes `saveActiveHarnessesWithIntegrations(ids)`, and Electron main performs the ordered `PUT /workspace/active-harnesses` plus per-tool status/install/uninstall sequence. The retired Gemini handler remains available to read and preserve existing owned settings, but Gemini is no longer offered as a coding tool for new sessions. Antigravity CLI has no compiled signal or integration binding. Installation writes an idempotent, ownership-marked artifact into the tool's own workspace-local config (e.g. `.claude/settings.local.json`, never `settings.json`) and never runs at session spawn; see [ADR 0026](../adr/0026-resolved-harness-capability-registry.md). OpenCode is the first integration whose owned artifact is a whole file rather than a JSON fragment merged into a shared document — `.opencode/plugins/orkworks-session-reporter.js` reports the native OpenCode session ID via a `session.created` plugin hook (issue #110) — so its ownership check compares the file's bytes directly instead of a JSON marker field. The shared reporter script (`report-harness-event.sh`/`.ps1`) is copied to `~/.orkworks/hook-scripts/` on install, so installed commands remain stable across app updates and AppImage mount changes. For the `claude-code` marker specifically, it also extracts `cwd` from Claude Code's hook JSON payload (alongside the `session_id` it already captures) and forwards it on the same attention POST — see [ADR 0032](../adr/0032-harness-reported-cwd-via-hook-payload.md).

`POST /sessions/:id/debug-injection` accepts `{attention, message?}` from the dev-only Details-panel picker (gated behind `showDebugMetadata`) and writes through the same merge path as `report_attention`, tagged `metadataSource: "debug"`, `metadataConfidence: 0.0` — the lowest documented priority tier (see [ADR 0005](../adr/0005-metadata-source-priority.md)). A `debug`-sourced write is ignored outright if the session currently carries `agent`-sourced metadata (a live coding agent's hook-verified signal), and otherwise any subsequent real signal reclaims the session on its next update. Rejects with 400 if the session's `lifecycle` isn't `alive`. `capped` injections route a non-empty `message` to the in-memory `usageLimitResetHint` handle field rather than the persisted `summary`, matching how the `Capped · <hint>` badge reads it; omitting `message` leaves any existing hint untouched.

`POST /sessions` now accepts `{ harnessId, model, initialPrompt }`. The renderer's New agent session dialog labels harness choices as coding tools, can fall back to the default shell session if harness metadata is temporarily unavailable, and still sends the selected harness config id so session rows and remembered-session resume behavior remain compatible. The response returns as soon as the `"creating"` metadata record is persisted, before the harness has actually spawned — the PTY spawn runs as a detached background task, mirroring how `POST /sessions/:id/resume` and daemon-restart reconciliation already leave a session observable in `"creating"`. A spawn failure surfaces asynchronously as `status: "error"`, observed via the existing `GET /sessions` poll, rather than as a synchronous error response (issue #302).

`electron/workspaceMemory.ts` persists the last workspace path and recent workspace directories to the Electron user data directory, enabling workspace restore on relaunch. The sidecar persists workspace-scoped state to `~/.orkworks/workspaces/<path-hash>/workspace.json`; Aider's versioned notification-command preference is separately stored at `integrations/aider.json`, so no repository Aider configuration is edited.

## Workflow observations and the current-summary snapshot (partially implemented)

This section documents the authoritative target contract from
[ADR 0042](../adr/0042-workflow-observations-replace-summary-checkpoints.md)
and `specs/orkworks-mvp.md`/`specs/taskmaster.md`; implementation is tracked
by [issue #313](https://github.com/Rambolarsen/orkworks/issues/313) and lands
across Tasks 2–4 of the workflow-observation-feedback-loop plan. The
current-summary snapshot on `SessionMetadata` remains design-only below, while
workflow-observation persistence, Peon recording, and passive Taskmaster
recommendations are implemented.

(design, not yet implemented) `SessionMetadata` gains a first-class
current-summary snapshot: `summary`, `summarySource` (`agent` | `peon`),
`summaryConfidence`, and `summaryObservedAt`, all four updated or cleared
together (see "Current-summary snapshot" in `specs/orkworks-mvp.md`). This
replaces the ADR 0024 durable summary-checkpoint mechanism; no new checkpoint
is appended to `events/<id>.ndjson`, and `GET /sessions/:id/summary-log` and
the desktop's "Task history" section are removed. Old event records
containing the superseded checkpoint fields remain readable.

Implemented separately: `workflow_observations.rs` owns `WorkflowObservation`
records (`id`, `sequence`, `sessionId`, `observedAt`, `kind`, `description`,
`evidence`, `reportedImpact`, `source`, `confidence`, `fingerprint`,
`idempotencyKeyHash`) behind a small interface — `record_observation`,
`workspace_observations`, `delete_session_observations`. The authenticated
explicit-report HTTP adapter (`http/workflow_observation_handlers.rs`) and Peon
inference adapter are live. Records
persist as bounded, session-segmented NDJSON under
`~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson`
(newest 1,000 records/2 MiB per session), ordered by a durable
`~/.orkworks/workspaces/<hash>/workflow-observations/sequence` counter;
workspace reconstruction reads the newest 10,000 records across segments.
`DELETE /sessions/:id/forget` and retention delete a session's segment and
every recommendation derived from it.

The workflow-observation and Taskmaster routes this design introduces or repurposes:

- `POST /sessions/:id/workflow-observations` — implemented. Harness-neutral explicit report, authenticated with a per-session, non-persisted `ORKWORKS_REPORT_TOKEN` bearer capability (alongside the existing `ORKWORKS_SESSION_ID`/`ORKWORKS_PORT` env vars, generated from OS randomness at session start/resume and never persisted, logged, or serialized) and an `Idempotency-Key` header; body limited to `kind`/`description`/`evidence`/`reportedImpact`, 8 KiB total, rate-limited to 30/session/60s ahead of the store's own 60/session/minute acceptance cap.
- `GET /taskmaster/recommendations` and `GET /taskmaster/recommendations/:id` — implemented; list responses include persisted observation diagnostics.
- `POST /taskmaster/recommendations/:id/dismiss` — implemented. `improve_workflow` exposes no refresh, accept, or execute action, since `requiresApproval: false` here means "nothing to approve," not "auto-applied."

Taskmaster correlates accepted observations five seconds after the latest
accepted one: a fingerprint cluster of at least two observations at
confidence ≥ 0.6, or one `reportedImpact: high` observation at confidence ≥
0.8, produces one `improve_workflow` recommendation embedding immutable
snapshots of its cited observations, deduped as
`improve_workflow:v1:<target-surface>:<observation-fingerprint>` and
resurfaced past a dismissal watermark only on higher impact or two
newly-qualifying observations including a new session. See
`specs/taskmaster.md`'s "Workflow-improvement recommendations" section for
the full eligibility, kind-to-target mapping, and dismissal-watermark rules.

## Rust sidecar (`crates/orkworksd/src/`)

Single binary. Top-level modules:

- `main.rs` — Axum router, `AppState` / `SessionHandle` / `WorkspaceState` / `PeonState` / `RetentionConfig` struct definitions, `main()`, `health_check()`, `#[cfg(test)] pub(crate) mod test_support` (shared test helpers), and a slim `mod tests` covering route registration and core AppState invariants
- `http/` — HTTP handler submodules (`ErrorResponse` declared in `http/mod.rs`):
  - `harness_handlers.rs` — harness CRUD (`GET/POST /harnesses`, `PUT/DELETE /harnesses/:id`)
  - `integration_handlers.rs` — generic harness integration install/status/uninstall (`GET/POST /workspace/integrations/:harness_id/{status,install,uninstall}`), reporter script path resolution
  - `provider_handlers.rs` — provider query handlers (`GET /providers`, `GET /providers/:id/models`, `POST /settings/providers`, `POST /settings/providers/ollama/verify`)
  - `retention_handlers.rs` — retention config handler (`POST /settings/retention`)
  - `session_handlers.rs` — session/workspace HTTP handlers (`POST /workspace`, `GET/POST /sessions`, `DELETE /sessions/:id`, `POST /sessions/:id/resume`, `POST /sessions/:id/harness-session`, etc.) and associated request/response types. `GET /sessions` is a thin blocking-task adapter over `session_projection.rs`. `POST /workspace` reconciles sessions orphaned by a previous daemon run via `metadata::reconcile_orphaned_session`: stale "running"/"creating" sessions are completed to `ended`, and sessions persisted mid-`ending` consume their `pendingTerminalStatus` as the final status so they cannot stay stuck in the ending phase
  - `workflow_observation_handlers.rs` — the thin, capability-authenticated `POST /sessions/:id/workflow-observations` adapter (ADR 0042): bearer-token auth, the route's own 30-attempts/60s pre-persistence rate limit, `Idempotency-Key` validation, and fixed request-vocabulary enforcement, before mapping onto `workflow_observations::record_observation`. Merged into the router as its own sub-`Router` so `DefaultBodyLimit::max(8 KiB)` scopes to this route only.
  - `taskmaster_handlers.rs` — passive recommendation list/detail/dismiss/refresh adapters; no accept or execute endpoint is exposed for workflow-improvement recommendations.
- `session_application.rs` — typed application seam for workspace opening, session lifecycle commands, attention and plan selection, and delete/forget workflows. It coordinates the existing `AppState` and runtime/metadata modules without owning a second session map; `http/session_handlers.rs` remains responsible for request extraction, authorization, compatibility mapping, and serialization.
- `session_projection.rs` — stateful `GET /sessions` projection. It snapshots live and durable session state, performs capacity/provider write-back and cwd/Git/conflict enrichment, and serializes projection with workspace replacement under `AppState.projection_lock`; the lock order is projection lock, workspace or sessions lock, then provider-manager internal locks. It releases state locks before filesystem, process-cwd, or Git I/O. `session_view.rs` remains pure and reusable for field derivation.
- `runtime/` — background-task and PTY submodules:
  - `observed_status.rs` — owns every write to `observed_status`/`attention` across the live session handle and persisted metadata: `apply_attention_signal` (external hook/debug reports) and `apply_process_transition` (the sidecar's own observations — committed input, idle timeout). See [ADR 0027](../adr/0027-observed-status-attention-owning-module.md).
  - `peon_runtime.rs` — `peon_loop` (continuous Peon observation loop); idle sessions enter an in-memory hold and resume observation only after qualifying user input
  - `retention.rs` — `retention_cleanup_task`, `retention_cleanup_once`
  - `session_runtime.rs` — session-runtime-owned PTY/process startup, bounded PTY/persistence/control backpressure queues (including startup input buffering), output draining, replay state, attachment ownership, child wait/finalization. `start_session_runtime` generates a fresh workflow-observation reporting capability before spawning the child (ADR 0042), aborting startup on OS-randomness failure rather than spawning with no or a weak capability; `clear_ended_session_tracking` revokes it.
  - `terminal_http.rs` — `get_terminal_output`, `get_summary_log`, `session_terminal_handler` (WebSocket upgrade / attach entrypoint)
  - `terminal_runtime.rs` — env helpers (`terminal_env_overrides`, `session_env_overrides`, `should_forward_terminal_env`), `TerminalAction` dispatch, `set_session_status`, websocket attach/detach transport that continues observing client closure while a command is backpressured, and the process-local workflow-observation reporting-capability registry (`new_workflow_report_token`, `set_workflow_report_token`, `verify_workflow_report_token`, `record_report_attempt`) backing `ORKWORKS_REPORT_TOKEN` (ADR 0042)
- `git.rs` — git2-based context detection (repo root, branch, dirty check including untracked files while excluding ignored files), run against each session's *effective* cwd (see `procfs.rs` and `session_view::resolve_effective_cwds`)
- `harness.rs` — neutral command and resume types plus `definition`, `registry`, `store`, `integration`, and `integrations` submodules. The versioned built-in resource and user overrides resolve once into an immutable `ResolvedHarnessRegistry`; each runtime operation captures one snapshot. Definitions can be `retired`: they remain resolvable for historical sessions and settings, but are excluded from new-session selection and launch. `integration` supplies canonical workspace confinement, component-by-component no-follow parent creation, Git-local-only checks, optimistic revision-checked configuration transactions, and the cross-platform publication helper; `integrations` is the closed compiled-handler dispatch boundary. Definitions declare launch, resume, model, Peon, capacity, signal, integration, and voice capabilities without adapter code paths. Windows uses target-only `windows-sys`: `ReplaceFileW` for an expected existing target and non-replacing `MoveFileExW` for an expected new target. The final check remains best-effort optimistic concurrency, not a portable CAS guarantee.
- `metadata.rs` — reads/writes session, workspace, and event files under the global metadata root (`~/.orkworks/workspaces/<hash>/`). Raw `events/<id>.terminal` replay is trimmed on append to the newest 1,000 lines and 1 MiB; existing oversized dormant files remain unchanged until append. The PTY's last known `cols`/`rows` is persisted once to `events/<id>.terminal-size` at the terminal-status transition and cleared on resume, so dead-session replay renders at the recorded grid instead of fit-to-container; see [ADR 0033](../adr/0033-recorded-terminal-replay-size-sidecar.md). Accepted Peon inference and attention reports preserve exact summaries as durable NDJSON checkpoints with accepted provenance, omitting only whitespace summaries and exact consecutive duplicates. See [ADR 0024](../adr/0024-bounded-terminal-replay-durable-summary-checkpoints.md). (design, not yet implemented — see issue #313) ADR 0042 plans to replace this with updates to a current-summary snapshot on `SessionMetadata` instead of an appended checkpoint; see "Workflow observations and the current-summary snapshot" below.
- `migration.rs` — one-time migration of legacy `<workspace>/.orkworks/` data into the global store
- `peon.rs` — observer config, ring buffer, in-memory observation state, prompt building, inference parsing/validation, source-priority overwrite rules, timer-based idle detection (`PEON_IDLE_TIMEOUT`, default 15s), summary normalization (strips "User is/wants/typed" prefixes), and usage-limit detection from terminal output
- `procfs.rs` — `live_cwds(pids)`: cross-platform (Linux/macOS/Windows) batched probe for running processes' current working directories in one `sysinfo` scan. Backs the pid-probe tier of live session git-context tracking (issue #241, [ADR 0031](../adr/0031-live-session-cwd-via-sysinfo-probe.md)); pids that are gone, denied, or unsupported are simply absent from the result and callers fall back further down the chain. Only actually tracks bare shell sessions — see `resolve_effective_cwds` below for why command-template harness sessions need [ADR 0032](../adr/0032-harness-reported-cwd-via-hook-payload.md)'s harness-reported cwd instead.
- `providers.rs` — provider definitions, applied-settings store, persisted runtime state, fallback runner (`run_inference` skips disabled/capped providers in fallback order), and model listing. `builtin_provider_registry()` contains only ollama (HTTP-based, no harness). Harness-backed provider definitions are projected from the captured resolved registry, so Peon configuration remains with its harness definition rather than being duplicated. `ProcessRunner` starts harness providers through plain `Command::spawn()` with piped stdin/stdout/stderr; it has no Unix fork-time callback, setsid operation, or inherited-file-descriptor sweep. This module still carries the historical `Provider*` names, but today it is modeling the Peon inference tool registry rather than the user-facing coding-tool selector. It exposes `GET /providers` for live runtime state, `GET /providers/:id/models` for available models, and `POST /settings/providers` for durable settings application. The session Peon loop routes through `ProviderManager::run_inference`. Per-provider peon model is configured in Settings.
- `session_types.rs` — `SessionInfo` struct, lifecycle and attention enums, and the renderer-facing session contract
- `session_view.rs` — lifecycle-aware session projection, connectivity and terminal-outcome derivation, conflict detection, and resume-option derivation. `resolve_effective_cwds` centralizes the harness-reported → pid-probed → launch-cwd fallback chain (ADR 0032 → ADR 0031 → launch `cwd`) so git-context enrichment and cwd-collision conflict warnings agree on where a session actually is.
- `watcher.rs` — `notify`-based file watcher for session metadata changes under the global store
- `workflow_observations.rs` — durable, bounded `WorkflowObservation` recording (ADR 0042): validation, idempotency (15-minute replay window via tombstones), sequencing, per-session (1,000 records/2 MiB) and per-workspace (10,000 records) bounds, and a live 60-accepted/session/minute rate cap. Public surface: `open`, `record_observation`, `workspace_observations`, `delete_session_observations`, `diagnostics`. Callers never see file paths or on-disk formats.
- `taskmaster/` — canonical passive recommendation contract, deterministic workflow-improvement evaluator, five-second generation-debounced refresh, and atomic recommendation persistence with dismissal watermarks and orphan/session cleanup.
- `workspace_runtime.rs` — `iso_now`, `orkworks_global_dir` (workspace path hashing to global store location)

For the current Rust domain model itself, see [domain-entities.md](./domain-entities.md).

## Dockview panel layout

The renderer uses Dockview for sessions, session detail, terminal, and optional utility panels. `DockviewApp` owns the panel registration and passes app state through a React context to panel components. The single reusable Review tab joins Terminal's tab group on demand and renders selected-session plan/spec content as Markdown via `react-markdown`/`remark-gfm` — plan/spec paths are sidecar-enforced to end in `.md` (see `resolve_openable_plan_reference` and `normalize_reported_plan_path` in the sidecar), so the Review tab does not need to branch on file type. `TerminalPanel` hosts the active live PTY session through `CenterPanel` and xterm.js over the backend WebSocket attach channel. Inactive sessions do not need to stay attached to keep their PTYs running; only the active terminal stays attached. The session detail panel includes read-only `Coding tool`, `Model provider`, `Model`, and `Provider state` fields for the selected session, plus debug-only `OrkWorks session ID` / `Harness session ID` fields and the read-only `Peon diagnostics` block when `Show debug metadata` is enabled.

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
