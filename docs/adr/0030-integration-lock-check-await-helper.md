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

## Amendment — application ownership (2026-10-10)

The accepted lock-check-drop-await-relock decision remains in force.
[Issue #816](https://github.com/Rambolarsen/orkworks/issues/816) moves its owner
from HTTP handlers to `harness_integration_application.rs`, following the
[approved extraction design](../superpowers/specs/2026-10-10-harness-integration-application-design.md).
The concrete application coordinates existing state and adapters; it does not
introduce another integration registry, session map or authority store.

Legacy harness and grouped-key operations retain separate private helpers
because grouped operations also revalidate document and active-selection
revisions under the projection lock. Both preserve probe-outside-lock ordering
and revalidate identity before configuration access. Application errors carry
domain facts; HTTP handlers retain status codes, request parsing and serialization.

Inspection can demote stale prompt authority. Mutation finalizes required
revocation synchronously after configuration work, with no added asynchronous
gap. Workspace listing retains snapshot collection order and revocation-failure
precedence. Cleanup and the synchronous launch-readiness query retain their
existing policies; readiness does not revoke authority.

This amendment changes ownership and interface visibility, not observable
behavior, failure semantics, storage or authorization. Adapter internals and
unrelated blocking workspace readers remain outside this decision.
