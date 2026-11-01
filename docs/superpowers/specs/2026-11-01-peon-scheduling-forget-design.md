---
type: Design
title: Peon scheduling across session deletion
description: Proposed serialization of Peon scheduling writes with session deletion without extending provider lock scope.
tags: [peon, concurrency, sessions, cleanup]
status: draft
workflow_status: approved
---

# Peon scheduling across session deletion

After a session is forgotten, work selected earlier must not recreate its Peon
tracking. Delivery is tracked in [#403](https://github.com/Rambolarsen/orkworks/issues/403).
The owner approved this concrete concurrency design with “go ahead” on
2026-11-01. Delivery follows #403 and the
[repository preparation policy](../../agents/development-workflow.md#plan-complexity-and-quality).

## Evidence and scope

[PR #1261](https://github.com/Rambolarsen/orkworks/pull/1261) made
`SessionApplication::clear_forgotten_session_tracking` the shared deletion
cleanup owner. At baseline `c6df8aad23ec8c0506870b45545d867e1606325d`, the
scheduler in `runtime/peon_runtime.rs` still retains local ID lists outside the
deletion boundary:

| Selected work | Later write that can outlive deletion |
| --- | --- |
| Output candidates | `mark_candidate`, `in_flight` insertion, `mark_idle`, `begin_attempt` |
| Drained label IDs | Reinsertion into `label_pending`, hint consumption and attempt admission |
| Running sessions missing a timer origin | Reinsertion into `last_output` |

A temporary deterministic probe used the real `forget_session` API, verified
that metadata and diagnostics were gone, then invoked `mark_candidate` and
`mark_idle` for a locally retained ID. The assertion that diagnostics remained
absent failed. This proves the callback ordering is unsafe; it does not measure
its production frequency or reproduce a whole concurrent scheduler tick.
The label requeue and timer cases are supported by their source paths, not yet
by equivalent runtime probes. Diagnostics remain bounded to 1,024 sessions;
unbounded diagnostic growth is not claimed.

The clean baseline passed 45 existing cleanup regressions. Fresh local Sonar
analysis of that baseline completed with no scanner errors or analysis warnings
(source fingerprint `57b777171bb5b2f4c801e34a584e23804c7edfc99c55eb21a07816a9949c6f0e`).
Its report is retained locally at `/tmp/orkworks-peon-stale-sonar-baseline.json`,
alongside `/tmp/orkworks-peon-stale-probe.rs` and its `.log` result. The probe was
removed and source restored exactly. No changed-source comparison or coverage
claim is made for this design-only proposal.

Preserve [runtime contracts](../../agents/runtime-contracts.md#metadata-protocol),
[runtime ownership](../../adr/0041-session-runtime-generation-ownership.md),
[responsive input](../../adr/0102-terminal-projection-input-fence.md), and
[contextual titles](../../adr/0098-contextual-session-titles.md).
[#402](https://github.com/Rambolarsen/orkworks/issues/402) completed the separate
live/persisted transition work; [#181](https://github.com/Rambolarsen/orkworks/issues/181)
remains the typed-state assessment. Neither transition ownership nor public
status vocabulary changes here.

## Recommended boundary

Use the existing `AppState::projection_lock` for synchronous scheduler admission
and timer-origin repair. Manual forget, session-action Delete, retention and
workspace replacement already use this fence. Introduce no tombstone registry,
new session epoch, new lock or second lifecycle authority.

Move scheduler selection/admission into one private synchronous owner in
`runtime/peon_runtime.rs`. It acquires the projection fence before draining
labels or reading candidates and retains it through all associated scheduling
mutations: candidate/idle diagnostics, requeue, hint consumption, `in_flight`
reservation and attempt/lease creation. It returns owned prepared attempts,
never unadmitted IDs that a later stage may use to create tracking.

The returned private record carries the existing `AttemptContext`, inference
mode, captured label epoch/hint, output snapshot/columns and output-boundary
identity. A separate launch stage consumes these records after the synchronous
owner returns. Keep hard-wrap reassembly, prompt preparation, provider selection,
provider execution and waits outside the projection fence. Do not move provider
or metadata work into the synchronous admission owner as an extraction side
effect. Snapshot copying is limited to the existing bounded Peon output buffer;
do not introduce a new full terminal-history scan under this fence.

Move timer selection and missing-`last_output` repair into a second synchronous
section using the same projection fence. It returns the existing
`(session_id, RuntimeGeneration)` idle selections. Release its fence before
calling `SessionApplication::apply_idle_timeout`; preserve that owner's current
generation, lifecycle, prompt-authority and persistence checks.

Admission after deletion sees no live handle, drops local pending-output state
for that ID, and performs no candidate/idle/in-flight/label insertion. A retained
but inactive session keeps ordinary end behavior; do not change label retention,
provider admission policy or end cleanup as part of this fix.

## Ordering and settlement

There are two supported orders:

1. Admission finishes first. The deletion owner subsequently acquires the fence,
   removes the session and invalidates its existing attempt/lease and maps.
   Already-admitted provider work may finish, but its existing runtime/attempt
   checks reject stale writes and settlement cannot recreate tracking.
2. Deletion finishes first. Scheduler admission subsequently acquires the fence,
   observes the missing session and cannot create tracking for it.

Deletion failure before metadata removal preserves the existing session and
retry behavior. If metadata was removed before a later evidence-cleanup error,
the existing deletion owner clears tracking while still holding the fence.
Retained sessions and a resumed/replacement runtime must remain unaffected.
The guarantee concerns scheduler writes after successful forgetting. Before
launching a prepared attempt, check its existing current-attempt guard and skip
one already invalidated by deletion. That check is not an atomic provider-spawn
fence: deletion can race a subsequent external launch. Do not promise synchronous
cancellation or that no provider process can start after deletion returns.

The projection fence is acquired before Peon/session locks, with no await,
provider wait, input-gate wait or filesystem/Git scan under it. Do not retain
`last_output` or `in_flight` guards while acquiring `sessions`: current result
processing can hold `sessions` while writing output tracking, and invalidation
can hold diagnostic leases while waiting for `in_flight`. Capture small map
snapshots and release those guards before inspecting sessions. Likewise release
the sessions guard before diagnostic/lease/in-flight mutation; the projection
fence, rather than nested map locks, supplies deletion exclusion. Review these
affected lock edges against existing settlement and end cleanup before coding.

This deliberately does not serialize every end/status transition through
`projection_lock`. Runtime identity, input generation, title epochs and existing
attempt guards still decide whether admitted work is current.

## Alternatives considered

- **Only stop `mark_idle` from creating entries:** leaves candidate admission,
  label requeue and timer repair able to recreate state.
- **Check session existence immediately before each insertion:** a check and
  write separated from deletion still race. Holding sessions across arbitrary
  Peon writes conflicts with existing lock edges.
- **Keep forgotten-ID tombstones or add a new cancellation generation:** creates
  another retained authority and a new boundedness/ID-reuse policy for a boundary
  already coordinated by an existing fence.
- **Fence provider execution or the entire asynchronous loop:** risks blocking
  deletion/input on external work. Only synchronous state preparation is fenced.

## Acceptance and recovery

Deterministic tests must cover deletion before admission, deletion after
admission but before settlement, label requeue and timer repair across deletion,
replacement runtime preservation, repeated deletion cleanup, and evidence
failure before versus after metadata removal. Test both orderings with a
controlled barrier at the real preparation boundary; use no timing sleeps as
the synchronization proof. Late success, failure, timeout and drop must not
recreate diagnostic/in-flight state for an invalidated attempt. Existing
end-label, provider isolation, output-boundary, title-epoch and shutdown tests
remain required.

Keep unrelated-session scheduling live. Demonstrate that blocked provider work
does not retain the projection fence or prevent forgetting, and that shutdown
still drains/aborts according to its existing deadline. Report preparation
fence duration separately from provider time; no latency improvement is claimed.

There is no storage/schema migration or new persisted state. Reverting this
change restores the existing race without requiring data repair. A later
implementation records the approved rationale in the next available ADR before
changing production code, updates current contract prose only when implemented,
and follows the [implementation plan](../plans/2026-11-01-peon-scheduling-forget.md).
