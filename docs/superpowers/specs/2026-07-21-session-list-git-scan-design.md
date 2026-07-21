# Session List Git Scan Performance Design

Date: 2026-07-21
Status: approved

## Problem

`GET /sessions` refreshes Git context by calling `git::detect` once for every
returned session. Historical sessions remain visible by design, so the cost of
one poll grows with total session history even when every session belongs to
the same working directory.

The live reproduction contained 132 sessions and one unique working directory.
Each two-second renderer poll therefore performed 132 identical full Git status
scans. A response took roughly four to six seconds, allowing interval-driven
polls to overlap and keeping `orkworksd` near 100% CPU.

## Goals

- Preserve the complete visible live and historical session list.
- Preserve current Git context fields and their polling freshness.
- Make Git detection work proportional to unique working directories, not
  session count.
- Prevent slow session requests from accumulating overlapping renderer polls.
- Add regression coverage for both failure modes.

## Non-goals

- Changing session retention or hiding historical sessions.
- Adding a cross-request Git cache with expiry or invalidation rules.
- Replacing session polling with WebSocket push; that remains tracked by issue
  #22.
- Changing the Git context contract or recommendation behavior.

## Design

### Request-local Git context reuse

During `list_sessions`, build a request-local map keyed by the session `cwd`.
For each session, look up its Git context in the map and call `git::detect` only
when that working directory has not yet been seen in the current request. Clone
the resulting `GitContext` into each session projection before calculating the
existing recommendation and conflict fields.

The map is discarded after the response. This preserves the current behavior
where every successful poll observes fresh Git state, while reducing the live
reproduction from 132 scans to one. Different working directories continue to
receive independent detection even if they belong to the same repository,
because worktree identity, branch, and dirty state may differ by directory.

### Single-flight renderer polling

Replace interval behavior that can invoke `refreshSessions` while its previous
request is unresolved with a self-scheduling poll loop. Schedule the next poll
only after the current refresh settles, and cancel future scheduling when the
effect is cleaned up or the backend disconnects.

The first refresh remains immediate. Subsequent refreshes wait two seconds
after the preceding request completes. Manual refreshes triggered by session
actions remain unchanged; the guard applies to the background polling loop.

## Error handling

Git detection retains its existing fallback behavior for non-repository or
unreadable directories. A failed renderer refresh remains silent as today, and
the next background attempt is still scheduled after the normal delay. Cleanup
must prevent a completed request from scheduling another timer after the effect
has been cancelled.

## Testing

- Add a sidecar regression test with multiple session projections sharing one
  working directory and an injectable/countable Git-context resolver. Assert
  one resolution per unique directory and unchanged per-session Git fields.
- Add a renderer polling test using a controllable unresolved refresh. Advance
  the timer beyond multiple poll intervals and assert that a second background
  request does not start until the first settles.
- Run the focused Rust and frontend tests, then the complete Rust and frontend
  suites plus TypeScript checking.

## Documentation impact

The change does not alter product behavior, metadata fields, architecture
boundaries, dependencies, or user-facing documentation. No ADR is required;
this is a bounded performance correction inside the existing polling and Git
context design.
