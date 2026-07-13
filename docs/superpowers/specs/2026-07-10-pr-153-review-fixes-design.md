# PR 153 Review Fixes Design

## Scope

Address the three unresolved PR 153 review threads without changing the debug
session state injection feature's public API or adding new injection scenarios.

## Decisions

- Compute normal capacity and latch state before projecting a temporary debug
  overlay onto the session response. The overlay must not become persisted
  runtime state or affect sibling/provider capacity state.
- While clearing expired debug injection metadata, collect persistence work
  under the existing locks and write session metadata after releasing the
  session lock. Filesystem latency must not extend the session lock's hold
  time.
- Treat Electron IPC arguments as untrusted. Validate both identifiers as
  non-empty strings and encode the session identifier before placing it in the
  sidecar URL path.

## Verification

- Add Rust regression coverage that applies `running-capped`, polls
  `list_sessions` twice, and proves the debug value neither latches runtime
  capacity nor propagates capped/reset-hint state to a sibling session or
  provider. Cover lock-safe metadata clearing behavior within existing handler
  tests where practical.
- Add Electron main-process coverage for rejecting malformed IPC payloads and
  for encoded session request paths.
- Run focused Rust and desktop tests, type-check the desktop project, and run
  the repository documentation currency check.
