# Application-module contracts (current behavior freeze)

This is a characterization document for Tasks 2–4. It describes the symbols
and behavior that exist on the `application-module-deepening` branch today. It
is a compatibility contract, not a proposal to change the current HTTP or
renderer protocol.

## Rust symbols and ownership

The current HTTP seam is `crates/orkworksd/src/http/session_handlers.rs`.
The handlers receive `State<Arc<AppState>>`; there is no `SessionApplication`
type and no second application-owned session map. Any future application
module must wrap or borrow the existing `Arc<AppState>` and delegate to the
existing `WorkspaceState`, `SessionHandle`, `SessionRuntime`, and metadata
store. `AppState.sessions` remains the sole in-memory session registry.

Current request/response symbols are:

| Symbol | Wire shape / result |
| --- | --- |
| `WorkspaceRequest` | `{ "path": string }` |
| `WorkspaceResponse` | `{ path, repo_root, branch, dirty, lastActiveSessionId[, activeHarnessIds] }`; empty `activeHarnessIds` is omitted |
| `ActiveSessionRequest` | `{ "sessionId": string }` |
| `ActiveHarnessesRequest` | `{ "activeHarnessIds": string[] }` |
| `CreateSessionRequest` | `{ harnessId?: string, model?: string, initialPrompt?: string }` |
| `AttentionReportRequest` | `{ status, message?, planPath?: string|null, observedAt?, cwd? }` |
| `PlanPathReportRequest` | `{ planPath: string }` |
| `TerminalPlanSelectionRequest` | `{ printedPath: string }` |
| `HarnessSessionReportRequest` | `{ harnessSessionId, source, confidence }` |
| `PlanContentResponse` | `{ content: string }` |
| `SessionInfo` | renderer-facing session projection from `session_types.rs`; serialization is compatibility-sensitive |

The handler symbols consumed by the router and tests are
`set_workspace`, `set_active_session`, `set_active_harnesses`,
`create_session`, `resume_session`, `report_attention`,
`report_session_plan_path`, `select_terminal_plan`, `delete_session`, and
`forget_session`. Plan-content/review handlers are
`get_session_plan_content` and `request_session_plan_review`.

`set_workspace` validates that the supplied path is a directory (`400`, body
`not a directory`), derives the global metadata directory, creates the
metadata subdirectories best-effort, runs migration, loads workspace memory,
starts a metadata watcher, replaces `AppState.workspace`, bumps the harness
probe generation, reconciles persisted `running`/`creating` records that have
no matching live handle, and returns `200` JSON `WorkspaceResponse`. Failure
to derive the home directory is `500` with body `no home directory`. Existing
live handles are not reconciled merely because the renderer reconnects.

`create_session` generates a UUID, resolves the requested harness/model (a
retired harness is `400` with the current explanatory text), detects Git
context, inserts a `SessionHandle` into the sole `AppState.sessions` map, and
persists a `creating` metadata record before spawning. It returns the
pre-spawn `SessionInfo` as `200` JSON. PTY startup is detached; spawn failure
is later represented by polling as `status: "error"`, not as a synchronous
create error. No workspace is installed yields a session using the fallback
cwd, but persistence is only available once a workspace exists.

`resume_session` returns `409` when no workspace exists, `404` for an unknown
session, and `400` when resume metadata or a usable resume strategy/command is
absent. Admission claims/replaces an ended stale handle under the existing
runtime generation checks; an attached/live/ending/racing handle returns
`409`. It resets the persisted run to `creating`, clears prior terminal-size
and final/attention state, and starts the runtime. The `200` JSON result is a
`SessionInfo` in `creating` state, but the current handler awaits runtime
startup before completing. Startup failure commits the claim, transitions the
session to `error`, and returns `500`; the existing startup-failure test
asserts this. Detached/pre-spawn resume success is a future
application/controller target, not current compatibility.

`report_attention` accepts the valid observed statuses and the active aliases
`thinking`/`reasoning` only when the harness advertises active-work support.
Invalid status or malformed `observedAt` is `400`. A stale report is ignored
with `200`; accepted and ignored reports both return `200`. Unknown sessions
return `404`; metadata persistence failure returns `500`; a lost concurrent
session returns `409`. The accepted merge updates the existing handle and
metadata path, preserves/updates `planPath` according to
`metadata::PlanPathUpdate` (omitted preserves, `null` clears), records an
optional harness cwd, and clears only non-descriptive pending input. A
`working` Claude signal can clear latched capacity state after timestamp
checks.

Plan selection is intentionally split. `report_session_plan_path` canonicalizes
and stores a hook-reported path, but does not change attention. A prior
user-selected plan wins over a hook report. `select_terminal_plan` requires
the open-plan token, resolves the exact printed text inside the session's
worktree family, and stores a user-selected reference. Missing/invalid token
maps to `503`/`401`; resolution failures map to the current conflict/error
responses. `get_session_plan_content` and `request_session_plan_review` are
also token-protected; review injection is explicitly user-approved.

`delete_session` requires a live in-memory handle: unknown/no-handle is `404`.
It synchronously sends the kill signal before its await point, then awaits the
existing status transition to `killed`, clears ended-session tracking, and
returns `200`. It does not remove the metadata record. The runtime exit path
may repeat the same terminal transition safely.

`forget_session` rejects a live/creating/running handle with `409` and body
`Cannot forget a live session. Kill it first.` It requires a workspace and a
metadata file (both missing cases are `409`/`404` respectively), deletes the
session metadata and events, clears matching last-active memory, removes the
handle and runtime tracking, and returns `200`. A corrupt-but-present metadata
file is forgettable. Session/event deletion failure is `500` for the session
file; event cleanup is best-effort after the session deletion.

## Concurrency, ordering, and side effects

The existing `Mutex`-protected `workspace` and `sessions` state is authoritative.
Handlers must not cache a second map or construct a competing runtime owner.
Admission is generation-aware: deletion, resume, startup, and runtime exit
must not let an older task overwrite a newer claim. Blocking metadata/Git and
integration work is kept off the async worker where the current code already
uses `spawn_blocking`; detached PTY startup remains asynchronous.

The required ordering is: validate; synchronously claim or signal in-memory
state where a later await could otherwise lose the action; persist the
corresponding metadata transition; then start/continue detached runtime work.
Failed startup compensates by committing the claim and publishing `error` via
the generation-guarded runtime transition. Resume clears stale terminal-size
state before the new runtime starts. Forget removes durable state before
removing the in-memory handle. For `create_session`, success means only that
the pre-spawn `creating` record was accepted; it does not mean PTY startup
completed. `resume_session` currently has the synchronous behavior above.

## HTTP mapping table

| Operation | Success | Current failures |
| --- | --- | --- |
| `POST /workspace` | `200` JSON workspace | `400` non-directory; `500` no home |
| `POST /workspace/active-session` / `PUT /workspace/active-harnesses` | `200` empty | `409` no workspace |
| `POST /sessions` | `200` JSON `SessionInfo` in `creating` | `400` retired/invalid launch |
| `POST /sessions/:id/resume` | `200` JSON `SessionInfo` in `creating` | `400` no resume; `404` unknown; `409` admission conflict; `500` startup failure |
| `POST /sessions/:id/attention` | `200` empty for accepted/ignored | `400` invalid input; `404` unknown; `409` lost claim; `500` persistence |
| `POST /sessions/:id/plan-path` | `200` empty | `404` unknown; `409` lost claim; `500` persistence |
| `POST /sessions/:id/select-terminal-plan` | `200` empty | `401` invalid token; `503` token unavailable; current resolution/conflict errors |
| `GET /sessions/:id/plan-content` | `200` `{ content }` | `401` invalid token; `503` token unavailable; current session/plan errors |
| `POST /sessions/:id/request-plan-review` | `200` empty | `401` invalid token; `503` token unavailable; current session/plan/runtime errors |
| `DELETE /sessions/:id` | `200` empty | `404` no live handle |
| `DELETE /sessions/:id/forget` | `200` empty | `404` missing file; `409` live/no workspace; `500` delete failure |

## Renderer controller contract

`openWorkspace(path: string)` is the required controller-level operation even
though the current preload picker is `openWorkspace(): Promise<unknown>` and
the current `setWorkspace(baseUrl, path)` wrapper owns the HTTP call. The
future controller must preserve this seam rather than allowing renderer code
to discover filesystem paths or call Electron APIs directly.

Current behavior: `App.tsx` owns `refreshSessions` through
`startSessionPolling` in `sessionPolling.ts`; poll failures are silent and
retry, and disposal stops the polling timer. Generation/cancellation,
stale-result rejection, and post-disposal no-op behavior are future controller
obligations for every async operation.

Current behavior: workspace switching clears sessions, refreshes the new
workspace's session list, and then restores `lastActiveSessionId`, but does
not validate presence or dead state. Current deletion/forgetting clears the
active ID before refresh. Future controller obligations are to reject
absent/dead restored IDs and define intended post-operation deletion ordering.

Create responses are correlated by the exact returned session ID through
`trackPendingCreate`/`resolvePendingCreates`. A pending ID remains pending for
`creating`, resolves silently for normal `running` (and any non-error status),
is removed silently if absent from a poll, and produces one error notification
only when that exact ID reaches `error`. Unrelated session errors never notify.

Polling prunes terminal attachments in `pruneTerminals` before publishing the
new session snapshot. Notification suppression is intentional for background
poll failures, normal create resolution, missing/deleted pending creates, and
the currently active session's unread state. Post-disposal callbacks must be
no-ops, including late poll responses and late provider/settings responses.

## Settings draft/commit contract

Electron-main is the authority for defaults and persistence in
`electron/settingsMemory.ts`; the renderer receives a normalized
`AppSettings` plus a cloned `defaultHotkeys` copy. The current preload symbols
are `getSettings`, `saveHotkeys`, `saveRetention`, `saveDebugSettings`,
`saveProviderSettings`, `verifyOllama`, `getProviderModels`, and provider
label/integration methods. The settings controller contract names the
operations `load`, `updateDraft`, `discard`, `verifyOllama`, `resetHotkey`, and
`commit`.

`load` reads Electron settings (missing/corrupt files return fresh defaults),
normalizes hotkeys/retention/debug/providers, and initializes the renderer
draft from the committed result. Hotkeys are trimmed and syntax-normalized on
read; invalid persisted values fall back to defaults, duplicate persisted
accelerators fall back to defaults, and `resetLayout` may be `null`.
`updateDraft` is local-only. `discard` drops all draft edits and reloads the
last committed value. `resetHotkey(action)` replaces only that action with
Electron's canonical default; it does not invent renderer-side defaults.

`verifyOllama(baseUrl)` calls `POST /settings/providers/ollama/verify` through
Electron-main using the draft URL. It is a read/diagnostic operation and must
not write `settings.json`, mutate saved provider settings, or replace the
draft. Verification failure is surfaced as a result/error while the draft is
retained.

Current behavior: `SettingsModal` saves domains independently and has no
unified discard/commit transaction. The future controller's `commit` validates
and persists the selected hotkeys, retention, debug,
providers, and integrations through their existing Electron handlers. A
successful domain save returns `{ ok: true, settings: rendererSettings(...) }`
and updates the committed settings/menu. Retention/provider saves also push
the normalized values to the sidecar. A durable Electron save can succeed
while that sidecar push fails; the failure is logged and represented as
pending/stale application for retry. A failed domain save retains the renderer
draft so the user can retry or discard it. Provider verification is never part
of the provider commit and cannot mutate saved settings.

## Existing characterization coverage

Existing coverage is narrower than the future contract: Rust tests cover
workspace reconciliation, resume admission/rollback/startup failure,
attention validation/staleness/persistence, delete/forget behavior, create's
pre-spawn response, and plan handling; desktop tests cover API response
errors, workspace restoration ordering, polling ownership, exact pending-create
ID correlation, settings defaults/normalization, and Ollama verification's
preload contract. Tasks 2–4 must add tests for generation/cancellation and
post-disposal behavior, absent/dead restoration and deletion ordering, unified
settings draft/discard/commit and sidecar-failure retention, and exact wire
compatibility including omitted empty `activeHarnessIds`, plan routes, and
`SessionInfo` serialization.
