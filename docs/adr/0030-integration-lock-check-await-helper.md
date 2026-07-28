# Integration lock-check-drop-await-relock helper

- Status: accepted
- Date: 2026-07-27
- Deciders: Copilot CLI

## Context

Issue #235 (split from PR #229 review) flagged drift risk in the integration
TOCTOU choreography in
`crates/orkworksd/src/http/integration_handlers.rs::run_integration_action`.
That path snapshots identity under lock, probes outside `!Send` guards, then
revalidates identity before action. This crate has many `std::sync` + async
adjacent shapes, but this is the only path that requires post-await identity
revalidation.

## Decision

Extract one private helper from `run_integration_action` to own the common
lock-check-drop-await-relock flow:

1. Snapshot workspace path and harness definition identity under lock.
2. Drop both guards.
3. Run tool detection/version probing outside `!Send` guards.
4. Revalidate harness definition and workspace path.
5. Execute caller logic with the revalidated workspace reference.

The helper remains integration-specific and is reused by the three integration
entrypoints through `run_integration_action`.

## Call-site survey checklist

- [x] `http/integration_handlers.rs::run_integration_action` (target pattern)
- [x] `spawn_blocking` lock readers in `runtime/terminal_http.rs`, `runtime/session_runtime.rs`, `http/harness_handlers.rs`, `http/session_handlers.rs`, `http/provider_handlers.rs`, `runtime/peon_runtime.rs`, `runtime/terminal_runtime.rs` (out-of-scope by design)

## Rejected alternatives

- **Broader helper for all `std::sync` + async adjacency shapes**:
  rejected because it mixes two different safety problems (TOCTOU revalidation
  vs. simple blocking metadata reads) and obscures lock ownership boundaries.
- **Leave choreography ad hoc**:
  rejected because additional call sites increase drift risk in conflict
  semantics and revalidation ordering.

## Consequences

- The highest-risk lock/await/revalidate choreography now has one owner.
- Existing `409 Conflict` semantics and identity keys remain unchanged.
- Runtime `spawn_blocking` workspace readers stay explicit and unchanged.
