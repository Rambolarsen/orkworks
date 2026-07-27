# Narrow helper for integration-handler lock/revalidate/await

## Context

Issue context: <https://github.com/Rambolarsen/orkworks/issues/235>, split from
PR #229 review:
<https://github.com/Rambolarsen/orkworks/pull/229>.

`crates/orkworksd/src/http/integration_handlers.rs::run_integration_action`
already centralizes the integration-handler choreography:

1. lock `state.workspace`, snapshot the workspace path, drop the guard;
2. lock `state.harness_catalog`, clone the resolved harness, drop the guard;
3. await the version probe with no `!Send` guard alive;
4. re-read `state.harness_catalog` and reject if the harness definition changed;
5. re-read `state.workspace` and reject if the workspace path changed;
6. proceed with the request.

Issue #235 was triggered because this pattern now exists alongside other
`std::sync` + async adjacency shapes in the crate. The integration path is the
only shape that does **drop guard -> await -> reacquire and revalidate identity**
before acting on the result.

Survey summary (from grep + file review):

- Integration TOCTOU shape: `http/integration_handlers.rs::run_integration_action`.
- `spawn_blocking` workspace reads/writes (different shape, no post-await
  identity revalidation): `runtime/terminal_http.rs`,
  `runtime/session_runtime.rs`, `http/harness_handlers.rs`,
  `http/session_handlers.rs`, `http/provider_handlers.rs`,
  `runtime/peon_runtime.rs`, `runtime/terminal_runtime.rs`.

## Decision

Extract a small private helper **from** `run_integration_action` for the
integration-handler pattern only. The helper boundary is:

- snapshot workspace path and harness definition identity;
- await tool probing with no `!Send` guard held;
- revalidate harness and workspace identity;
- then execute a caller-provided closure while the final workspace guard is
  still held (so the existing `IntegrationContext` borrow lifetime remains
  intact).

This is intentionally closure-based rather than value-returning for workspace
state, so the helper does not reopen a TOCTOU window between workspace
revalidation and `IntegrationContext` use.

The helper stays narrow on purpose:

- it keeps the `workspace` + `harness_catalog` identity checks together;
- it preserves the current `409 Conflict` behavior and error priority order;
- it does **not** try to absorb the runtime `spawn_blocking` readers, because
  they are a different blocking-read shape with no revalidation step.

The three public integration entrypoints (`get_integration_status`,
`install_integration`, `uninstall_integration`) already share
`run_integration_action`; this change carves an internal helper out of that
function. Runtime `spawn_blocking` sites remain ad hoc.

## Alternatives

- **Broader generic helper**: one abstraction for the integration path and the
  runtime blocking reads. Rejected for now because it mixes two different safety
  problems and would hide the TOCTOU boundary that matters here.
- **Keep everything ad hoc**: lowest code churn, but it leaves the third copy in
  place and keeps the subtle revalidation order easy to drift.

## ADR deliverable

This work also produces an ADR in `docs/adr/` that records:

- why the helper is integration-specific;
- why the closure-based shape was chosen;
- why broader `spawn_blocking` unification was rejected for now.

The ADR will be indexed in `docs/adr/README.md`.

## Data flow

1. Capture workspace and harness identity under lock.
2. Drop both guards.
3. Probe the tool version outside the locks.
4. Revalidate both identities.
5. Reacquire workspace guard, build `IntegrationContext`, and run the action
   closure under that guard.

If revalidation fails, return the same conflict response the handler already
uses today.

## Tests

- keep the existing integration-status polling regression;
- add a regression for harness-definition edits during the probe window;
- add a regression for workspace switches during the probe window;
- add helper-focused unit tests that pin the primitive behavior independently
  (snapshot/await/revalidate success + both conflict paths), in addition to
  handler-level regressions;
- keep the call-site survey explicit in implementation notes so runtime
  `spawn_blocking` readers are documented as intentionally out of scope.

## Scope

This change is limited to the integration handlers and their tests. It does not
alter the runtime metadata readers, the version-probe cache, or the conflict
semantics.
