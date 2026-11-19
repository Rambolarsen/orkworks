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
  captured location inputs and invalidate known changes conservatively. Adaptive five/30-second
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

## Adaptive follow-up

The owner requested continuation after the adaptive-refresh recommendation on
2026-11-19. Keep this work in #1910. Revise periodic enrichment to five seconds
when an existing row's output timestamp changed in the past ten seconds,
otherwise 30 seconds. Keep explicit refresh immediate, status coalescing at two
seconds, five-second stale presentation and fresh action validation. No new
configuration surface or persisted activity state is needed.

Checks: demonstrate the old idle scan in a failing publisher test; verify idle,
active, quiet-again and resumed deadlines deterministically; freeze the
output-only counter test's clock to separate scheduled work from output-driven
invalidations on slower Windows runners. Repeat identical idle/busy fixtures,
then full repository checks and current-diff review. Compare results before
marking the PR ready. The initial fixed-cadence measurements above remain
historical evidence, not the adaptive implementation's qualification.

### Adaptive measurement result

Repeated the identical 35-second socket workload twice with the final adaptive
scheduler, reversing mode order on the second run. Compilation and other checks
from this task were stopped during measurement. User + system CPU seconds:

| Workload | Legacy, runs 1 / 2 | Adaptive, runs 1 / 2 |
| --- | --- | --- |
| Idle | 0.18 / 0.18 | 0.18 / 0.18 |
| Busy | 1.28 / 1.26 | 0.99 / 1.01 |

Idle CPU was equal at the timer's 0.01-second reporting precision; busy CPU
improved by 19.8–22.7% (21.3% comparing means). Adaptive idle runs started three
shared inspections versus nine for both busy runs; 350 output writes did not
increase inspection frequency. One legacy run had 351 writes because the final
100 ms tick coincided with the measurement deadline. This small count difference
does not establish precise percentage accuracy.

The adaptive cadence removes the observed fixed-cadence idle penalty in this
fixture and supports source enablement under the amended design. It remains
unqualified for installed-app/release performance: no live daemon was replaced,
and empty-repository results do not establish whole-app savings.

The idle publisher test failed against fixed cadence, then the final focused
suite passed 17 tests (two manual benchmarks ignored). Pure deadline cases cover
idle, repeated output, quiet-again and resumed activity; unchanged metadata and
initial history do not count as output. The Windows counter regression now waits
for both subscriptions' fresh baseline and fixes virtual time throughout startup
and status capture, avoiding false failures when a slow runner crosses a valid
periodic deadline.

Adaptive amendment review: completed current-session `/code-review medium` for
source and tests under `apps/desktop/src`, `apps/desktop/tests` and
`crates/orkworksd`, relative to the original design commit `c9d5d994`.
Code-diff SHA-256:
`7ae28e9b78cdfcdd6caca5f01987944e0493ac42e28d66a47fd91a5be57fc3cb`.
Checked deadline anchoring, no queued periodic work, activity expiration,
explicit-refresh recovery, initial-history classification, watcher behavior and
unchanged legacy transport. No remaining correctness findings. The production
build and 17 focused stream tests pass; full repository validation is running.
The earlier review fingerprint describes the superseded fixed-cadence code.
