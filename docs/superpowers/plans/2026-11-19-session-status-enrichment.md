---
type: Implementation Plan
title: Session status and enrichment delivery
description: Implement the owner-approved split with projection, scheduler and mixed-version regression checks.
tags: [orkworks, sessions, performance]
status: draft
workflow_status: active
---

# Session status and enrichment delivery

Owner approval: 2026-11-19. Implements [the approved design](../specs/2026-11-18-session-status-enrichment-design.md)
for [#1895](https://github.com/Rambolarsen/orkworks/issues/1895). Execute inline.

## Deliverables

- [x] Separate status projection and typed enrichment fields in `session_projection.rs`
  and a scoped stream module. Status must perform zero repository/document inspections,
  including legacy plan availability. Preserve full REST projection and final validation.
- [x] Add one workspace-owner publisher for negotiated subscribers in `workspace_updates/`.
  Status, enrichment and recovery use separate single-flight work and one ordered cursor.
  Output and persisted-output watcher events must not schedule extra enrichment; compare
  captured location inputs and invalidate known changes conservatively. Five-second
  enrichment, two-second status and 30-second recovery timers must not queue missed ticks.
- [x] Extend the desktop reducer/transport and freshness presentation. Keep old server/client
  behavior, atomic optional-field clearing, stale timers, reader invalidation and backpressure.
- [ ] Synchronize accepted spec/ADR prose; verify focused regression tests, synthetic workload,
  full repository checks, review and CI before merge. No running daemon replacement.

## Verification and review focus

Write failing tests first for status-with-zero-opens, shared subscriptions, status during a
blocked enrichment, persisted output, late worker results, malformed/gapped messages,
optional-field clearing and unsupported/legacy negotiation. Reuse existing projection
branch/location/pin/ownership regressions. Test deletion/replacement, explicit unavailable
bindings and whole-directory disappearance without destructive empty-list publication.
Measure shared and distinct worktree counts with one busy and many idle rows; compare
idle and busy work. Record actual results, including limits on whole-app CPU conclusions.

Plan review: Ready for implementation within the approved design. Dependencies 3,
blast radius 3, state changes 2, reversibility 2, uncertainty 3 (13/25). Runtime
qualification remains unmeasured. Keep legacy transport available for rollback; no migration.

## Implementation evidence

The complete local `scripts/verify-repo.sh` run passed: Rust build, 3,608 Rust
unit tests (23 ignored), integrations, reporter checks, TypeScript, 1,787 desktop
tests, 2,056 documentation tests and documentation build. The final recovery
changes followed that Rust run; the focused stream suite then passed 14 tests
(two manual benchmarks ignored). Current-head CI remains required before merge.

Completed current-session `/code-review medium` covers the cross-component protocol and
publisher lifecycle. Two findings were reproduced and fixed: an HTTP upgrade
could attach to another workspace's existing publisher, and a reconnect could
adopt a cached snapshot before its refresh completed. Regression tests now
require matching workspace ownership and a completed inspection at or after the
subscription's requested enrichment revision. Failed inspection closes the
stream for REST recovery; it never publishes an authoritative empty list.
Document-directory replacement also rearms the existing metadata watcher.

The README drift warning needs no edit: the public top-level product and
milestone summary are unchanged. Updated architecture, ADR, accepted review
specification and session guide carry the scoped behavior. Worktree warnings
refer to other owners' checkouts; none were modified.

## Synthetic performance results

Measured on 2026-11-19 with the same debug test executable, 32 remembered rows,
two WebSocket subscribers, one empty Git repository and a 35-second window.
Busy writes one row's output timestamp every 100 ms (350 writes); idle writes
none. `/usr/bin/time -p` measures user plus system CPU for the process and its
children, including fixture setup, without compilation. Own build/test jobs
were stopped during measurements; unrelated Mac processes remained active.

| Workload | Legacy CPU seconds | Split CPU seconds | Difference |
| --- | --- | --- | --- |
| Idle | 0.23 | 0.53 | +0.30 seconds |
| Busy | 1.54 | 1.30 | -0.24 seconds (15.6%) |

Both split workloads started nine shared inspections, independent of the 350
output writes. Wall durations were 35.36–35.46 seconds. The idle penalty exceeds
the busy saving in this small fixture; the approved design's enabling condition
is therefore unresolved. Owner input was requested on an opt-in rollout versus
an activity-dependent cadence. Do not represent this as qualified default
behavior or a measured improvement to the running application.

A separate warmed projection benchmark ran five rounds of 20 queries, with
32 rows sharing one repository or distributed over four distinct repositories.
Median round time divided by 20:

| Repository layout | Full projection | Status projection |
| --- | --- | --- |
| One shared repository | 14.126 ms | 6.241 ms |
| Four distinct repositories | 27.269 ms | 6.016 ms |

These empty-repository fixtures show the status path avoids repository cost;
they do not model the owner's populated worktrees, live Peon scheduling, release
build, renderer CPU or total Mac load. The running daemon was not replaced.

Review conclusion: no remaining correctness findings in the current code diff;
rollout qualification remains the explicit gate above. Reviewed code-diff SHA-256
(relative to this branch's original design commit):
`35dedded43ecf6dc9a5a1575d78d31564662e97f6e8e4191e3f9834770955b37`.
Medium effort is justified by the cross-component protocol, concurrency and
lifecycle changes. This was a current-session review, not an independent review.
