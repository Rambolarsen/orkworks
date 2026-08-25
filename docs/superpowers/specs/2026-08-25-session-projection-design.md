# Session Projection Design

## Goal

Move the session-listing projection behind a narrow, testable module interface
without changing the renderer-facing `SessionInfo` contract or the ownership of
live session state.

## Context

`crates/orkworksd/src/http/session_handlers.rs::list_sessions` currently owns
both HTTP response construction and the complete session read model. The
operation merges live `SessionHandle` state with persisted metadata, projects
remembered sessions, detects capacity state, propagates harness capacity to
provider rows, resolves effective process working directories, enriches Git
context, and computes shared-workspace conflict warnings. Some of those steps
also update live latches and provider state.

This is a real seam: the HTTP adapter needs one operation, while the behavior
has its own state invariants and an existing Git-detection test seam.

## Interface

Introduce a `SessionProjection` module whose caller-facing operation is:

```rust
pub(crate) fn list(&self) -> Vec<SessionInfo>
```

Projection is intentionally non-failing at the public seam. Missing,
unreadable, or corrupt per-session metadata follows the current behavior: the
record is omitted or the available live/persisted fields are used. Git and
process-cwd probing also degrade to the existing fallback values. There is no
new HTTP error mapping in this slice.

The module may borrow the existing `Arc<AppState>`, but it must not create a
second session registry, runtime owner, or metadata authority. `AppState.sessions`
remains the only live-session map and `WorkspaceState.metadata` remains the
source of persisted session state.

The HTTP handler is an async adapter that clones the `Arc<AppState>`, invokes
`SessionProjection::list` through `tokio::task::spawn_blocking`, and maps the
returned vector to the existing JSON response. It may retain only state-clone,
blocking-task orchestration, join-error handling consistent with current
behavior, and JSON serialization. It must not perform session-policy decisions
or capacity/provider write-back. No endpoint, field, status vocabulary, or
serialization shape changes are in scope.

## Behavior and invariants

The implementation must preserve:

- merging live and remembered sessions by session ID;
- resolved harness capabilities and resume options;
- capacity detection from both bounded output and raw scan text;
- generation-safe capacity latch and reset write-back;
- propagation of live harness capacity to provider state;
- effective cwd precedence: reported harness cwd, live process cwd, then launch cwd;
- one Git detection per unique effective cwd;
- Git context and shared-workspace conflict warnings;
- current behavior when no workspace is open;
- all existing lock ordering and blocking-work behavior.

Live records take precedence over remembered records with the same session ID.
Live runtime fields are projected from the `SessionHandle`; persisted metadata
supplies durable fields and fills fields that are not runtime-owned. The
existing canonical field mapping remains authoritative.

The projection uses an optimistic two-stage snapshot. It clones the live
handle observation under `state.sessions`, reads the workspace metadata under
`state.workspace`, then releases both locks before process-cwd and Git work.
The snapshot boundary is the consistency point for the returned list; the
operation does not promise a globally atomic view across concurrent runtime
events. Capacity-latch write-back is compare-before-write: it reacquires the
sessions lock and writes only when the live counters and origin still equal the
values observed in the snapshot. Provider capacity updates are derived from
that same projection and delegated to the provider manager's existing
serialization; concurrent listings may recompute the same idempotent state but
must not introduce a second provider-state authority.

The projection must not hold `state.sessions` or `state.workspace` while doing
filesystem reads, process inspection, or Git detection. Any blocking work
required to assemble the snapshot remains inside the `spawn_blocking` task;
the async worker is used only for orchestration.

The module may retain internal helper seams for Git detection and pure
projection calculations. Those seams must not be exposed through the external
interface merely to make tests convenient.

## Testing

Add characterization tests before moving production logic. Tests should cross
the projection interface where possible and assert observable `SessionInfo`
results plus required state write-backs. Existing focused tests for Git
deduplication and session listing behavior should be moved or adapted rather
than duplicated.

The test set must cover:

1. live plus remembered session projection;
2. capacity detection, latching, reset baselines, and provider propagation;
3. effective cwd selection and one Git probe per unique cwd;
4. shared-workspace conflict warnings;
5. empty/no-workspace behavior and compatibility-sensitive fields.

Also pin:

6. live-over-remembered precedence and stable response ordering;
7. missing/corrupt metadata fallback and the non-failing projection contract;
8. concurrent live-handle mutation during projection, including rejected stale
   capacity write-back;
9. provider-state behavior with and without an open workspace.

## Out of scope

- changing the REST protocol or `SessionInfo` JSON;
- introducing a generic `SessionContext` or `AppState` wrapper;
- moving PTY lifecycle ownership;
- changing Peon inference or Taskmaster evaluation;
- splitting `SessionApplication` or `metadata.rs` further;
- changing Git/worktree control behavior.

## Acceptance criteria

- `list_sessions` is an HTTP adapter with no session-projection policy logic.
- The projection module exposes one primary caller-facing operation.
- The HTTP adapter contains only orchestration, error/join handling, and JSON
  serialization; all session policy and write-back lives behind the seam.
- No second live-session map or metadata authority exists.
- Existing Rust behavior and tests remain green.
- The new module is reflected in the Rust module-layout documentation.
- The full Rust suite passes, and doc/worktree currency checks are run before
  handoff.
