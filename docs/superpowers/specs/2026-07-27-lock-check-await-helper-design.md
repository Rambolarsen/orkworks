# Narrow helper for integration-handler lock/revalidate/await

## Context

`crates/orkworksd/src/http/integration_handlers.rs::run_integration_action`
currently hand-implements a subtle choreography:

1. lock `state.workspace`, snapshot the workspace path, drop the guard;
2. lock `state.harness_catalog`, clone the resolved harness, drop the guard;
3. await the version probe with no `!Send` guard alive;
4. re-read `state.harness_catalog` and reject if the harness definition changed;
5. re-read `state.workspace` and reject if the workspace path changed;
6. proceed with the request.

The issue was triggered by a third copy of this exact safety pattern. A crate
grep also finds two other `std::sync`/async shapes: the `spawn_blocking`
workspace metadata readers in `runtime/terminal_http.rs` and the post-persist
trim tasks in `runtime/session_runtime.rs`. Those are intentionally different:
they read metadata off the async path, but they do not perform post-await
identity revalidation.

## Decision

Extract a small private async helper for the integration-handler pattern only.
The helper will own the snapshot/await/revalidate choreography and return the
validated `ResolvedHarness` plus the detected tool result to the caller.

The helper stays narrow on purpose:

- it keeps the `workspace` + `harness_catalog` identity checks together;
- it preserves the current `409 Conflict` behavior and error priority order;
- it does **not** try to absorb the runtime `spawn_blocking` readers, because
  they are a different blocking-read shape with no revalidation step.

The three public integration entrypoints (`get_integration_status`,
`install_integration`, `uninstall_integration`) will call through the helper.
The runtime metadata readers in `runtime/terminal_http.rs` and
`runtime/session_runtime.rs` remain ad hoc.

## Alternatives

- **Broader generic helper**: one abstraction for the integration path and the
  runtime blocking reads. Rejected for now because it mixes two different safety
  problems and would hide the TOCTOU boundary that matters here.
- **Keep everything ad hoc**: lowest code churn, but it leaves the third copy in
  place and keeps the subtle revalidation order easy to drift.

## Data flow

1. Capture workspace and harness identity under lock.
2. Drop both guards.
3. Probe the tool version outside the locks.
4. Revalidate both identities.
5. Build `IntegrationContext` and run the requested action.

If revalidation fails, return the same conflict response the handler already
uses today.

## Tests

- keep the existing integration-status polling regression;
- add a regression for harness-definition edits during the probe window;
- add a regression for workspace switches during the probe window;
- keep the call-site survey explicit in the implementation notes so the runtime
  `spawn_blocking` readers are documented as intentionally out of scope.

## Scope

This change is limited to the integration handlers and their tests. It does not
alter the runtime metadata readers, the version-probe cache, or the conflict
semantics.
