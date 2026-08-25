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
pub(crate) fn list(&self) -> Result<Vec<SessionInfo>, SessionProjectionError>
```

The module may borrow the existing `Arc<AppState>`, but it must not create a
second session registry, runtime owner, or metadata authority. `AppState.sessions`
remains the only live-session map and `WorkspaceState.metadata` remains the
source of persisted session state.

The HTTP handler becomes a thin adapter that invokes the operation and maps the
result to the existing JSON response and error behavior. No endpoint, field,
status vocabulary, or serialization shape changes are in scope.

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
- No second live-session map or metadata authority exists.
- Existing Rust behavior and tests remain green.
- The full Rust suite passes, and doc/worktree currency checks are run before
  handoff.
