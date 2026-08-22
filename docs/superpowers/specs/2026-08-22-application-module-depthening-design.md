# Application Module Deepening Design

## Goal

Improve depth and locality across OrkWorks without changing product behavior, REST payloads, Electron security boundaries, or the single-active-session model.

## Scope

This design covers three independent refactoring plans:

1. A Rust session-application seam behind the HTTP handlers.
2. A renderer workspace/session controller behind `App.tsx`.
3. A renderer settings workflow controller behind `SettingsModal.tsx`.

These are architectural refactors, not new product features. Each plan must preserve existing external contracts and produce independently testable software.

## Design principles

- Prefer deep modules: small interfaces with substantial behavior behind them.
- Use seams where behavior can change without editing callers.
- Keep existing deep modules such as metadata, harness resolution, observed-status ownership, and terminal protocol helpers intact.
- Do not introduce a seam only to wrap one pass-through implementation.
- Tests cross the same interface used by callers.
- Preserve the Electron-main/renderer process seam and its intentional duplicated contract types.
- Preserve the product rule that one active session is the context-switch primitive.

## Plan 1: Rust session application seam

Create a focused application module, likely `crates/orkworksd/src/session_application.rs`, that owns session/workspace use cases rather than HTTP or storage details.

The initial interface is:

```rust
pub(crate) struct SessionApplication {
    // Coordinates the existing AppState capabilities.
}

impl SessionApplication {
    pub(crate) fn open_workspace(&self, path: PathBuf) -> Result<WorkspaceSnapshot, SessionError>;
    pub(crate) async fn create_session(&self, request: CreateSessionCommand) -> Result<SessionSnapshot, SessionError>;
    pub(crate) async fn resume_session(&self, id: &str) -> Result<SessionSnapshot, SessionError>;
    pub(crate) fn report_attention(&self, id: &str, signal: AttentionSignal) -> Result<(), SessionError>;
    pub(crate) fn select_plan(&self, id: &str, selection: PlanSelection) -> Result<(), SessionError>;
    pub(crate) async fn delete_session(&self, id: &str, forget: bool) -> Result<(), SessionError>;
}
```

The module owns lifecycle preconditions and transitions, workspace/session lookup, metadata writes, runtime admission and ownership rules, stable application errors, and coordination between metadata and live `SessionHandle` state.

The HTTP handlers retain request deserialization, authorization/header extraction, application calls, HTTP error mapping, and response serialization. REST routes and JSON payloads remain unchanged.

Existing modules—`metadata`, `session_view`, `observed_status`, `plan_handoff`, and runtime submission—remain internal dependencies. The new module must coordinate existing `AppState`; it must not create a second state owner.

Tests should exercise lifecycle and workflow behavior through the application interface. Handler tests should focus on request/response mapping and authorization.

## Plan 2: Renderer workspace/session controller

Create a renderer-only orchestration module, likely `apps/desktop/src/workspaceSessionController.ts`, to remove workflow coordination from `App.tsx` without introducing a generic state framework.

The controller interface is:

```ts
export interface WorkspaceSessionController {
  refreshSessions(): Promise<RefreshResult>;
  openWorkspace(): Promise<WorkspaceSessionState>;
  createSession(options: CreateSessionOptions): Promise<SessionInfo>;
  resumeSession(id: string): Promise<SessionInfo>;
  selectSession(id: string): Promise<void>;
  deleteSession(id: string, forget?: boolean): Promise<void>;
  dispose(): void;
}
```

It coordinates backend URL discovery, polling and cancellation, pending-create reconciliation, session merge and stale-terminal pruning, active-session persistence, workspace switching, resume/create error handling, and unread-state transitions tied to session updates.

It does not own React rendering state, Dockview layout, terminal construction or WebSocket lifecycle, settings, or provider configuration.

Dependencies are passed to the factory through a narrow dependency object containing backend URL discovery, session/workspace operations, terminal pruning, notifications, and an optional clock. `api.ts` remains the HTTP Adapter.

`App.tsx` retains React state, component callbacks, polling lifecycle mounting/unmounting, and conversion of controller results into state updates.

The controller must preserve the single-active-session model and must not create parallel terminal rendering.

Tests should cover polling failures without toast spam, pending creation resolution, workspace-switch stale-state clearing, active-session restoration ordering, active-session deletion, terminal pruning, and cancellation during workspace switching.

## Plan 3: Renderer settings workflow

Keep settings persistence in Electron main and create a renderer-side draft/commit module, likely `apps/desktop/src/settingsController.ts`.

The controller interface is:

```ts
export interface SettingsController {
  load(): Promise<AppSettings>;
  updateHotkeys(draft: HotkeySettings): Promise<AppSettings>;
  updateProviders(draft: ProviderSettings): Promise<AppSettings>;
  updateRetention(draft: RetentionSettings): Promise<AppSettings>;
  resetHotkeys(): Promise<AppSettings>;
}
```

It owns draft lifecycle, validation, save ordering, and failure recovery. It does not duplicate Electron persistence or reach across the process seam.

`SettingsModal` should become a composition layer around focused sections for hotkeys, providers, harness integrations, retention, and debug settings. Existing provider and integration sections should be reused where they already provide a useful interface.

Electron’s `settingsMemory.ts` remains the persistence module. Storage formats and preload contracts do not change. Provider verification remains separate from saving so unsaved Ollama URLs can be checked without mutating durable settings.

Tests should cover draft-versus-persisted state, failed saves retaining drafts, canonical defaults from Electron, verification without mutation, independent settings domains, and reload behavior.

## Sequencing

1. Implement and validate the Rust session application seam.
2. Implement and validate the renderer workspace/session controller.
3. Implement and validate the settings workflow controller.
4. Reassess `AppState` and cross-application contracts only after the three seams exist.

## Non-goals

- No new product behavior.
- No REST endpoint or payload redesign.
- No Electron security relaxation.
- No replacement of `api.ts` with a generic service layer.
- No broad mechanical file splitting.
- No multi-terminal or parallel-context UI.
- No new dependency or state-management framework.
