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

The implementation lives in a new `crates/orkworksd/src/session_projection.rs`
module. `session_view.rs` remains the home for pure view helpers and field
projections; it is not made responsible for locks, filesystem reads, or live
state write-back. `SessionProjection` and its constructor are `pub(crate)`;
the Git detector injection used by tests remains private to the module.

Projection is intentionally non-failing at the public seam. Missing,
unreadable, or corrupt per-session metadata follows the current behavior: a
remembered record is omitted; a live record remains projected from its live
handle with no persisted overlay. A missing sessions directory produces no
remembered records. Git and process-cwd probing degrade to the existing
fallback values. There is no new projection error mapping in this slice.

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

“Non-failing” describes recoverable data-source failures only: missing or
malformed metadata, process-cwd failures, and Git failures degrade to the
existing fallbacks. A poisoned lock or panic remains a daemon failure. If the
blocking task returns a `JoinError`, the adapter returns HTTP 500 with an
empty body; it does not fabricate a partial list. This is the only new
failure mapping introduced by moving the synchronous work into
`spawn_blocking`.

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
handle observation under `state.sessions`, then captures the immutable
workspace metadata root/path and workspace identity under `state.workspace`.
It releases both locks before constructing a metadata reader and performing
filesystem reads, process-cwd inspection, or Git work. The metadata reader is
created from the captured root; it must not borrow `WorkspaceState` across
blocking I/O. Before applying any write-back, the projection rechecks that the
workspace identity is still current. If the workspace changed, it discards
the snapshot's write-backs and returns an empty list; the next poll projects
the newly current workspace. If no workspace is current, live sessions are
still returned, remembered sessions are absent, and provider propagation uses
the live-session snapshot exactly as it does today.

Capacity-latch write-back is compare-before-write: it reacquires the sessions
lock and writes only when every input used by the write-back still matches the
snapshot, including the run generation, latch, pending flag, visible-once flag,
output counters, and resume-scan origin. A per-handle runtime generation is the
preferred identity check; field-by-field comparison is acceptable only when it
covers that complete set.

Concurrent listings are serialized by one shared `std::sync::Mutex<()>`
projection lock added to `AppState` and held for the complete projection
operation, including provider capacity update. Workspace replacement acquires
the same lock before taking `state.workspace`; therefore a workspace switch
cannot occur between identity validation and the projection commit. The lock
order for this seam is: projection lock, then `state.workspace` or
`state.sessions`, then the provider manager's internal locks. No code may take
`state.workspace` or `state.sessions` and then wait for the projection lock.
The coordination lock is not a session or provider-state authority; the
existing `ProviderManager` remains the provider-state authority.

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

6. live-over-remembered precedence and the existing response ordering contract
   (live `HashMap` iteration followed by remembered metadata order; no new
   ordering guarantee);
7. missing/corrupt metadata fallback and the non-failing projection contract;
8. concurrent live-handle mutation during projection, including rejected stale
   capacity write-back;
9. provider-state behavior with and without an open workspace.

The extraction order is fixed: first move the live/remembered snapshot and
pure `SessionInfo` assembly; next move capacity detection and complete
compare-before-write state updates; then move cwd/Git enrichment and conflict
calculation; finally move provider propagation and replace the HTTP body with
the blocking-task adapter. Each step must leave the existing handler tests
green before the next step begins.

Provider propagation is keyed by resolved `harness_id`, not
`model_provider_id`. A live harness is capped when any live session for that
harness is capped; checking state masks the capped display for that harness;
the first available reset hint wins; remembered sessions never inherit live
capacity flags. The provider manager receives the complete recomputed maps
from the committed projection only.

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
- The shared projection lock is documented as coordination state, not a second
  session or provider-state authority.
- Existing Rust behavior and tests remain green.
- The new module is reflected in the Rust module-layout documentation.
- `set_workspace` and the projection module use the documented projection-lock
  order without introducing a lock cycle.
- The full Rust suite passes, and doc/worktree currency checks are run before
  handoff.
