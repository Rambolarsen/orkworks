---
type: Design
title: Responsive terminal input during session projection
description: Approved capacity-analysis separation and cached terminal observations without weakening input ownership.
tags: [terminal, performance, concurrency, sessions]
status: draft
workflow_status: approved
---

# Responsive terminal input during session projection

Restore responsive typing and alternate-screen interaction in long coding sessions.
Track delivery in [#1222](https://github.com/Rambolarsen/orkworks/issues/1222).
The owner approved this concrete concurrency/sensitivity design and inline
implementation on 2026-11-01 ("go ahead, implement", then "keep going").

## Evidence and existing contracts

Live sampling found `workspace_updates -> query_sessions -> project_capacity ->
CapacityState::observe` consuming the busy sidecar worker. A complete session
request took 2.157 seconds for 719 rows. `SessionProjection::list` holds
`projection_lock` across this computation; input preparation and bookkeeping
acquire the same gate. The user confirms burst/freeze typing. The native sample
did not capture an individual input waiter, so no measured per-key wait is claimed.

The same session's retained raw tail contained 64 KiB newline-free continuation
records and tens of thousands of ANSI escapes. Its persisted tail is not a
measurement of the in-memory logical buffer. Per-key sensitivity detection also
normalizes variable-sized text while holding the sessions mutex.

Preserve [ADR 0075](../../adr/0075-responsive-pty-input-delivery.md),
[ADR 0041](../../adr/0041-session-runtime-generation-ownership.md),
[ADR 0092](../../adr/0092-workspace-update-stream.md),
[ADR 0065](../../adr/0065-ingestion-time-hard-wrap-reassembly.md), and
[MVP usage-limit awareness](../../../specs/orkworks-mvp.md#usage-limit-awareness).
The [proposed decision](../../adr/0102-terminal-projection-input-fence.md)
changes projection lock scope, not PTY delivery authority.

## Selected approach

Keep the existing input owner, ordered per-runtime gate, two owner slots,
projection fence, control sender, delivery acknowledgement and stop path. Do
not fix latency by allowing input to bypass identity checks or by releasing its
owner before accepted/rejected bookkeeping finishes.

Move only pure capacity analysis outside the projection fence. Keep metadata,
organization, plan/cwd/Git projection and their current write/read ordering under
the existing fence in this repair. This avoids introducing a second revision
system for every metadata writer. It targets the measured dominant hotspot;
remaining fenced I/O must be measured honestly after the repair, not assumed fast.

### Capture, compute and commit

Use a fail-fast process-local analysis admission permit, independent of
`projection_lock`, to bound one active capacity computation. Never wait for that
permit while holding a state lock. Under a short capture fence, retain workspace
ownership identity (lease/store semantics, not just path), each selected runtime
generation/control identity, resolved capability fingerprint, capacity state,
output revision, counters and immutable shared output references. Release the
fence before scanning. Preserve owned snapshot interfaces for other consumers.

Then acquire the existing projection fence and run the ordinary metadata and
organization projection. Use this fresh authoritative view to decide current
harness/lifecycle/attention. Apply prepared capacity observations only after
validating workspace ownership, runtime/control identity, current resolved
capability fingerprint and a process-local capacity control revision. Do not
move metadata/organization reads out of their current critical section.
Workspace replacement invalidates the whole pass and uses the existing
unavailable path, never an authoritative empty/removal set. Individual stale
capacity observations do not invalidate the complete list: use the current
committed capacity state for those sessions and let the next throttled pass
observe them again. Never retry within the request or scan under the fence as
fallback. Concurrent callers without the analysis permit take the existing
unavailable/recovery path, with no unbounded blocking-pool waiter queue.

### Output churn and capacity authority

Add a monotonically increasing, process-local capacity control revision. It
advances for every accepted nonempty input frame, hook/reset clear, latch or
recheck-origin change and pending-check visibility transition. Resume also
changes the runtime generation. **Ordinary output-counter advancement does not
advance this control revision.** Existing `arm_usage_limit_recheck` may be a
no-op for an unlatched session; input must nevertheless invalidate older analysis.
Centralize accepted-input invalidation in its existing post-delivery application
path, not in UI input parsing or before delivery acceptance. Preserve counter
exhaustion as a fail-closed analysis invalidation, never silently reuse an epoch.

Before applying an observation, nonblockingly try to acquire that runtime's
existing `input_gate` while holding the projection fence. If an input owner
holds the gate, skip this session's observation and use current committed
capacity. Never wait for the input gate while holding `projection_lock`.
Otherwise keep the temporary gate guard through that session's validation and
capacity mutation; the projection fence still excludes subsequent input
preparation/enqueue. This closes the interval from preparation through PTY
acknowledgement and post-delivery bookkeeping: the ordinary input owner already
holds `input_gate` throughout, and accepted nonempty input advances the control
revision before releasing it. Failed delivery does not advance the revision or
arm a recheck; skipping analysis during its pending attempt is conservative.
Initial configured/startup prompt delivery retains ADR 0075's existing driver
semantics; the new user-input observation fence applies to the shared ordinary
and approved-input owner path, without claiming new startup authority.

At commit, an available input gate and unchanged control revision permit applying
an observation of the captured output prefix even if newer output has arrived. Do not assign captured
counters back to the handle: current physical-line/raw-byte counters remain
monotonic. An observation's `origin_update` advances only through its captured
scan position, leaving newer output eligible for the next scoped check. Stamp a
new latch from the captured output timestamp, not a later unrelated chunk.
This is the liveness rule: continuous output alone cannot reject the whole list
or prevent a completed prefix scan from contributing capacity state. Input,
resume, hook clear or capability changes reject that session's older observation.

Derive provider aggregation under the existing fence from accepted observations
and the current committed states of rejected sessions; a stale candidate cannot
clear another session/provider. Preserve peer-cap propagation, provenance and
reset-hint selection policy; [#1083](https://github.com/Rambolarsen/orkworks/issues/1083)
remains separate. A rejected observation cannot advance pending visibility or
origins; those effects require accepted current analysis or existing current state.

Investigated mutation sites: accepted input reaches
`SessionApplication::arm_usage_limit_recheck` from
`record_terminal_input_impl`; hook clearing reaches
`CapacityState::clear_latch_and_rearm`; projection applies
`CapacityState::apply_observation` and `advance_pending_visibility`; creation
and resume install `CapacityState::pending` with a fresh runtime generation.
These state-machine/application chokepoints own revision advancement. Tests must
exercise a no-op input recheck, change-then-restore state to catch ABA reuse, and
a paused owner after PTY acknowledgement but before accepted-input bookkeeping.
A capacity commit in that interval must skip that session without blocking input
or publishing a stale latch/provider transition; the older candidate remains
invalid after bookkeeping advances the revision and releases the gate.
No metadata-file or organization-store writer instrumentation is introduced.

### Pure scan reuse

Cache line analysis by runtime generation, immutable logical-line revision and
resolved pattern fingerprint, with exact range identity where a scoped window
uses a partial view. Store only matched/not-matched and bounded reset-hint
results; normalize an unchanged retained line once. Retain entries only for
currently retained lines and the current pattern set; one active analysis may
hold its immutable inputs until completion. Raw scan caching uses its exact byte
position/content revision and capability fingerprint; that raw window keeps its
existing 8 KiB bound and incomplete-ANSI allowance.

Recompose each capacity observation through `CapacityState::observe`'s existing
full/fresh-window rules, using cached line evidence rather than a cached mutation.
A whole-observation cache may additionally reuse one exact current state/output
key, but is not sufficient by itself: continuous output must not cause unchanged
large lines to be normalized again on every list pass. A cache hit must not clone
retained text. Resume/removal/workspace replacement release per-runtime caches.

Preserve detector ordering: detection may find any matching row; line-based
hint extraction prefers newer logical lines. Preserve raw split-ANSI/cursor
handling, colocated hints, false-positive protections, physical counters and
fresh-window origins. Latch/provider writes and pending-check visibility are
never cached or replayed as effects.

### Password sensitivity before input

Capture immutable references to the same five logical lines currently selected
by `snapshot_input_context`, plus their output revision. Normalize and classify
all five selected lines outside the sessions mutex and projection fence. This
conservative coverage preserves prompts previously checked after the old
newest-first snapshot was reversed, including prompts followed by ordinary
output, and also covers the newest selected lines. Cache the
boolean by runtime identity and exact selected-line revision. Revalidate both
before enqueue while retaining ADR 0075's preparation/enqueue transaction.
Never move the sensitivity observation to after delivery or inspect echoed input.

To bound first-key work, if the selected observation exceeds 64 KiB of UTF-8
text, classify it sensitive without scanning; use checked/saturating byte sums.
If the output revision changes before enqueue, classify sensitive for that frame
rather than rescanning under the fence. This fail-closed choice suppresses
label/topic/last-user-input recording for a benign oversized or racing frame;
it does not drop its terminal bytes, alter attention authority, or infer a reset
from sensitive input. Identity/sender/workspace validation still decides whether
the command can be delivered. A later stable, in-budget observation uses normal
detection again. Cache misses never weaken password protection.

## Boundaries and recovery

No wire schema, persisted metadata, terminal history, replay size, session
lifecycle, user approval or native attention capability changes. No dependencies.
Do not restart or hot-replace the running sidecar that owns the user's session.
Caches are process-local and disposable; rollback is a normal code revert and
owner-controlled application restart, with no data repair or migration.

Absolute byte-retention changes to `RingBuffer` and `pending_wrap_prefix`, and
reinterpretation of raw continuation chunks as wrapped physical rows, are
explicitly deferred: they change ADR 0065's observation window and deserve a
separate bounded design. This repair must make input independent of slow
projection scans even when existing retained output is large. The oversized
password path above prevents that known buffer issue from imposing unbounded
pre-enqueue normalization work.

## Acceptance

- A deterministic test pauses the real projection's compute phase. Ordinary
  terminal input reaches its captured PTY sender and completes bookkeeping
  while compute remains paused. The test fails on the current monolithic fence.
- Continuous output during a paused computation still allows a complete list
  and captured-prefix capacity commit without losing newer counters. Input,
  hook clear, resume/workspace replacement or capability change during compute
  produces no stale capacity/provider write, no input to a replacement sender
  and no authoritative removal set. Stop remains independent; cancellation cannot authorize duplicate input.
- Repeated listing with unchanged relevant inputs performs no additional
  normalization/scans; every cache-key input has an invalidation regression.
- Scoped capacity rechecks, split ANSI/cursor sequences, reset hints and
  multi-session provider aggregation retain their current results.
- Password prompts, split/wrapped text, oversized observation, changing output,
  sensitive submit and reset commands cannot persist protected input. Oversized
  and racing observations exercise the fail-closed metadata path.
- Full repository verification, current-diff review and required CI pass.
- After owner-controlled restart, compare the same long-session workload:
  input admission timing, projection compute/commit timing and CPU separately.
  Automated tests use synchronization/counters; wall-clock sampling supports
  the live check, not a flaky CI latency threshold.

## Approval and review

Approved for the scoped inline implementation on 2026-11-01.
The one required owner decision covers the changed lock scope, captured-prefix
capacity commit rule and conservative sensitivity policy together. The implementation
plan is [terminal input lag](../plans/2026-10-31-terminal-input-lag.md).

Independent builder/plan review completed on 2026-10-31: Ready for owner approval.
The plan records the output-liveness, writer-coverage and delivery/bookkeeping
findings and their resolutions. Owner acceptance was subsequently given on 2026-11-01.
