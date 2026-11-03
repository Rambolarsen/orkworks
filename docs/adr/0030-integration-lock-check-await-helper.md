---
type: "Architecture Decision"
title: "Integration lock-check-drop-await-relock helper"
description: "Architecture decision record: Integration lock-check-drop-await-relock helper."
tags: ["orkworks", "architecture"]
---

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

## Amendment — owned Codex filesystem execution (2026-11-02)

The approved [ordinary-session storage scheduling contract](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md#interface-scheduling-and-publication)
requires external effects to release input-critical application guards. Codex
Settings mutation/cleanup operationalizes that contract at the same post-probe
validation cutoff: capture one owned action under the existing checks, then run
its adapter/configuration/observation-clear work on a blocking task outside the
projection/workspace guards. A narrow integration-file handle retains the exact
observation owner and the action retains the original workspace lease. Same-path
owner/lease replacement is rejected before admission; grouped document/selection
revision checks still apply. Bounded fail-fast admission prevents an unbounded
queue behind a stalled clear.

The accepted identity revalidation decision stands. Once admitted, an action is
definitive and may complete on its original workspace after waiter cancellation;
its result cannot authorize a replacement workspace. Clear failure is surfaced
even when hook configuration already committed. This does not imply rollback.
Other harnesses retain the synchronous prompt-revocation sequencing from the
application-ownership amendment. Inspection retains its existing scheduling.
The separate legacy JSON observation follow-up subsequently adopts the external
file claim/lease boundary; its compatibility and remaining synchronous identity
validation are recorded in the [adapter validation record](../validation/session-store-adapter.md#legacy-json-identity-observation-follow-up).


The first identity-target metadata lookup also uses the existing capture/await/
revalidate rule through a bounded owned session reader. Its immutable file view
shares the legacy decoder without constructing a second mutation owner. It retains
its original read slot and workspace lease through file completion after cancellation,
and rejects stale workspace/reader/runtime results before continuation. The
transactional JSON merge and later identity reads remain synchronous; the
[validation record](../validation/session-store-adapter.md#owned-json-identity-target-lookup)
bounds this source qualification explicitly.

Identity follow-ups now consume the transaction's exact identity result instead
of rereading JSON for live resume publication or label reservation. A private
completion retains original application/workspace/reader/runtime context, and label
reservation revalidates its original bearer inside the existing memory gate.
The mutating transaction retains its synchronous owner until conflicting lifecycle
and metadata reservations are qualified; this does not introduce a second JSON
worker. See [transaction identity completion](../validation/session-store-adapter.md#transaction-identity-completion).
