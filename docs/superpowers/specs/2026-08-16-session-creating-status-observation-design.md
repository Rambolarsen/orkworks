# Session-Creating Status Observation Design

## Goal

Make the New Session dialog's create flow actually observe the `"creating"` status
before the harness has spawned, so the `terminal-starting-overlay` / non-interactive
stdin guard added in PR #299 covers the primary create path, not just resume and
daemon-restart reconciliation.

## Evidence

`create_session` (session_handlers.rs) persists a `"creating"` metadata record, then
`.await`s `start_session_runtime` to completion before returning the HTTP response.
`start_session_runtime` flips the in-memory status to `"running"` (session_runtime.rs)
almost immediately after `spawn_command` succeeds, with no meaningful await in
between. So the client's `createSession()` response body never actually contains
`status: "creating"` — `App.handleConfirmNewSession` adds the session to `sessions`
state and selects it only once it is already `"running"`.

Resume (`resume_session_handler`) and daemon-restart reconciliation already persist
`"creating"` and let the client observe it via the poll-driven `GET /sessions`
(`list_sessions`, which reads live `state.sessions`), because those paths can leave
an *already-selected* session showing `"creating"` mid-request. Create has no such
existing session in client state to display in the interim, so the client never
sees the interval at all — this is what #299's own review comment (discussion
`r3772750041`) identified.

## Options

1. **Client-side optimistic placeholder.** Add a placeholder session to `sessions`
   state before the create POST resolves, swap it for the real session on success.
   Doesn't touch the actual timing bug server-side; needs new rollback logic for
   requests that fail validation before any session exists server-side (e.g. the
   retired-harness 400), and needs a way to reconcile a client-invented id against
   the server's real one.
2. **Return `POST /sessions` as soon as the `"creating"` record is persisted, spawn
   `start_session_runtime` as a detached background task, surface `running`/`error`
   via the existing poll/metadata mechanism.** Fixes the actual root cause; reuses
   the exact status-transition infrastructure the resume and daemon-restart paths
   already rely on, including the generation guards (`owns_spawned_generation`,
   `startup_generation_is_ending`) that already make detached startup safe against
   concurrent deletion — proven by resume already doing this. **Chosen.**

## Design

### Backend (`crates/orkworksd/src/http/session_handlers.rs::create_session`)

- Unchanged through persisting the `"creating"` metadata record.
- Return `Json(info)` (still `status: "creating"`) immediately after that persist,
  instead of after `start_session_runtime` completes.
- Move the `start_session_runtime(...).await` call, its `Ok`/`Err` handling (on
  error: set status `"error"` for the generation, schedule the existing ending
  finalization — unchanged), and the success-only `session.created` event append
  into a `tokio::spawn(async move { ... })` block, mirroring the shape the resume
  handler already uses for its `startup_task`.
- No new synchronous 500 path: a spawn failure becomes an async `status: "error"`
  transition, observed by the client on its next poll — the same signal resume and
  daemon-restart reconciliation already produce for this exact failure mode.

### Frontend (`apps/desktop/src/App.tsx`)

Once the response actually contains `"creating"`, the rest of the chain already
works unmodified: `TerminalPanel` derives `starting={session.status === "creating"}`,
and `CenterPanel`'s `computeTerminalInteractivity` already reacts to it. The one gap
is the error toast: `handleConfirmNewSession`'s `catch` block currently fires
"Couldn't start a new session." on the synchronous 500, which no longer happens for
spawn failures.

- Track ids of sessions just added via the New Session dialog in a ref-backed set,
  scoped only to this creation window.
- In the existing poll-driven `sessions` update, when a tracked id's status resolves
  away from `"creating"`: if `"error"`, fire the same toast and drop the id; anything
  else (e.g. `"running"`), drop the id silently.
- Scoping to dialog-created ids only means an unrelated crash later in a long-running
  session can never misfire this toast.

## Tests

1. **Rust, real create flow:** `create_session` returns `200` with `status:
   "creating"` before the background task can plausibly finish; a follow-up read of
   `state.sessions` observes the transition to `"running"` for a normal spawn, and to
   `"error"` for a broken one.
2. **Rust, unchanged fast-fail path:** `create_session_rejects_a_retired_harness`
   (still-synchronous 400, before any session or metadata record exists) stays green
   unchanged.
3. **Frontend, pending-tracking logic:** a pure-function test (mirroring the existing
   `sessionUnread.test.ts` style — this repo's tests are Node's built-in test runner
   over pure logic plus source-pattern checks, not a DOM harness) driven by a mock
   response shaped like the real `"creating"` server payload, asserting it flows
   through the real `computeTerminalInteractivity`, not a synthetic `starting: true`
   prop.

## Non-goals

- No change to `computeTerminalInteractivity` or the overlay's visual design (already
  correct, already unit-tested in #299).
- No change to the resume or daemon-restart-reconciliation paths.
- No client API (`api.ts`) or IPC contract changes.
