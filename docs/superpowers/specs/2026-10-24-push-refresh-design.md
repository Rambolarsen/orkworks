---
type: Design
title: Session and recommendation push refresh
description: Workspace-scoped push synchronization with changed session rows and authoritative recovery.
tags: [orkworks, desktop, sidecar, sessions, taskmaster]
status: stable
workflow_status: owner-approved
---

# Session and recommendation push refresh

## Intent and evidence

Replace repeated renderer session-list downloads with workspace-scoped change
updates. Keep REST queries authoritative for recovery and existing actions.
This is the owner-approved source design. Source implementation does not imply
availability in published installers.

The owner agreed with investigating change notifications and slower recovery
polling after asking why sessions and recommendations are repeatedly fetched.
The owner approved the detailed design and its 30-second derived-field freshness
tradeoff on 2026-10-24 with “go ahead”.

[Issue #22](https://github.com/Rambolarsen/orkworks/issues/22) deferred session
push until measured cost or latency justified it. On 2026-10-24, six sequential
GET samples against the running development sidecar, spaced by two seconds
after both queries completed, found:

| Query | Records | Bytes per response | Median latency | Maximum latency |
| --- | --- | --- | --- | --- |
| `/sessions` | 655 | 1,323,570 | 1,264 ms | 1,295 ms |
| `/taskmaster/recommendations` | 0 | 39 | 20 ms | 22 ms |

These are client wall-clock measurements with the existing app still running,
not isolated CPU measurements. All six session responses differed. A separate
two-response comparison found only `lastOutputAt` changed, on one session.
No payload contents, session IDs, or credentials were retained in this record.
This supports reducing full-list transfers; it does not establish the cost of
individual projection stages or a CPU improvement from this design.

## Boundaries

Preserve session identity, organization, attention authority, pending creation,
unread tracking, terminal pruning and workspace restoration. Preserve Taskmaster
approval gates and access validation. Notification delivery grants no action
authority. Terminal sockets and PTY lifetime stay independent.

This increment covers the session list, shell Actions recommendation source
(`useActionsRecommendations`) and mounted recommendations panel, including its
analysis status refresh. Both recommendation consumers currently poll
independently; replace them with one shared workspace-scoped list snapshot and
one invalidation-driven refresh. Keep focused recommendation details panel-local. Session detail history, Settings polling,
terminal transport, pagination and historical-record caching are outside scope.
Do not introduce a second authoritative session registry.

## Transport and ownership

Add one read-only workspace update WebSocket per renderer, owned by the workspace
session controller. Use the existing localhost WebSocket stack and connection
boundary. No new browser credentials, URL tokens or persistent event log.
The endpoint must enforce the existing renderer connection/origin policy; audit
that policy before implementing rather than assuming CORS protects WebSockets.
No session reporting capability is accepted as a renderer subscription token.

Each subscription binds to the current sidecar instance and workspace generation.
Workspace closing, replacement, or backend lifecycle loss closes the subscription
and rejects all late work. Electron lifecycle remains the authority for adoption.
A socket message cannot switch or select a workspace.

Use versioned messages: initial `snapshot`, subsequent `sessions-delta`, and
`recommendations-invalidated`. Include a server instance identifier, workspace
generation and monotonic stream sequence. The initial snapshot contains the full
existing session projection. Deltas contain complete changed SessionInfo rows
and removed IDs. Preserve snapshot ordering with an explicit ordered ID list when
membership/order changes. Every data message, including recommendation
invalidation, advances one global cursor by exactly one. The initial snapshot
establishes that cursor; each later message carries its expected base and next
value. Heartbeats never advance it. A missed invalidation therefore causes the
same resynchronization as a missed session delta. Never derive deletion from a failed projection.

One controller applies a delta to its canonical list and passes the resulting
list through the existing pending-create, merge, pruning and publication path.
Reuse those semantics; a removed ID must have exactly the same effect as absence
from a successful authoritative list, including pending-create protection.
Recommendation messages contain only invalidation, without evidence or report
contents. The shared recommendation source refetches through the existing access-validated
REST query; the mounted panel refreshes analysis-status IPC and any focused
details on the same signal. A successful list refresh updates both consumers. Its current stale-workspace guards remain binding.

## Producing updates

Maintain a bounded in-process invalidation signal, scoped to the active workspace.
Signals coalesce; they are hints to rebuild, not durable facts or action commands.
After successful visible changes, lifecycle actions, runtime exit, accepted
metadata writes, Peon publication, capacity/provider changes and Taskmaster
recommendation/run-state transitions mark their affected domain dirty. Include
retention, archive/restore/delete, settings changes and harness catalog changes.
Document the concrete producer coverage in the implementation plan before code.

External metadata writers remain supported. Add a nonrecursive metadata-directory
watcher for sessions, organization state, recommendations and capacity; atomic
replacement, deletion and directory recreation invalidate the corresponding
domain. Watcher overflow/error invalidates everything and triggers a rearm;
failures keep reconciliation active. Ignore raw terminal/event log writes.
The implementation may choose the maintained cross-platform watcher package;
record its version and platform coverage in the ADR before adding it.

Raw PTY output only marks session data dirty. Coalesce projection requests with
a minimum two-second interval between starts, with at most one in flight.
A dirty signal received during projection schedules a later pass; never clear a
new dirty revision when acknowledging an older captured revision. Every pass
reuses the entire existing `/sessions` query pipeline, including
SessionProjection and the live Peon diagnostics/workflow-observation overlay
currently added in `http/session_handlers.rs`, then compares complete serialized
rows by ID against the last successfully published snapshot. Extract this shared
query behind both transports rather than duplicating it. Peon diagnostic and
workflow-observation changes must invalidate session rows too. This avoids missing fields
and never omits an attention change because only output was expected.

Git/cwd, clock-based capacity expiry, external coding-tool saved state and missed
filesystem events may lack a reliable producer. Reconcile the full existing
projection and recommendation/status invalidation at least every 30 seconds
while subscribed. This deliberately permits up to 30 seconds of staleness for
those derived fields, compared with today's faster list polling; owner approval
must cover that tradeoff. Resume/focus and explicit user actions request an
immediate reconciliation. Persistent projection cost during active output is
not solved here: the initial improvement is fewer full transfers and fewer
idle projections, not a promised CPU reduction under continuous activity.

Avoid an invalidation feedback loop from projection write-back: publish writes
only when persisted semantic values change, and verify a completed projection
converges without repeatedly invalidating itself. Use current lock ordering and
release locks before socket/network I/O. Slow clients cannot block mutations.
Bound output buffering to one pending authoritative snapshot; on overflow close
the stream and require resynchronization rather than silently dropping deltas.

## Initial synchronization and recovery

Register the dirty signal before capturing the initial projection. Changes during
capture require another pass, so there is no subscribe-after-read gap. Send the
snapshot before deltas. Deltas have a base sequence and next sequence; a client
applies only a matching base in the matching instance/workspace generation.
Any gap, malformed message or unsupported version discards the stream baseline
and reconnects for a full authoritative snapshot. No partial replay protocol.

Only a complete query can advance the baseline or remove rows. Directory scan,
metadata read, or decode failures must be distinguished from confirmed absence
for the streamed query, even where a legacy reader currently skips bad records.
On an incomplete query, retain the prior snapshot, mark source availability
unavailable, and retry without publishing deltas. A complete recovery publishes
a fresh snapshot. REST and stream must share this failure contract; do not let
their deletion semantics diverge. Read-only optional diagnostic failures keep
their current nullable-field behavior and are not evidence of session removal.

Refresh the shared recommendation list and mounted analysis status immediately
after initial connection/reconnection;
a delayed panel mount also fetches immediately. Coalesce panel invalidations with
one list request in flight and a trailing refresh if dirtied during the request.
Local mutations invalidate the shared list immediately, even before their stream
notification arrives. Late panel mounts reuse a current workspace snapshot;
mounting a second consumer cannot start an independent polling loop.

Use heartbeat detection and bounded exponential reconnect delay with jitter
(1 second initially, capped at 30 seconds). A lost stream restores the existing
2-second session and 5-second recommendation polling until a valid new snapshot
has been adopted. Do not run two session refresh authorities concurrently:
serialize stream adoption with REST refresh and invalidate earlier operations.
While the stream is healthy, remove those renderer polling loops. The server's
30-second reconciliation invalidation replaces the panel's periodic refresh.

Backend shutdown/switch/disposal cancels polling, subscriptions, retry timers,
watchers and pending work for the old generation. Unsupported older sidecars
retain current polling. Stream failure must not mark the backend unavailable
when REST is healthy. Subscription loss never kills a coding session.

## Acceptance and verification

- Verify initial snapshot, changed rows, removals and ordering produce the same
  renderer list as the authoritative REST response.
- Test dirty changes before/during capture, producer bursts, slow clients,
  sequence gaps, old-generation responses, repeated reconnect and disposal.
- Exercise live attention, session creation/exit, external atomic metadata
  writes, archive/restore/delete and recommendation completion independently.
- Verify empty recommendations cause no five-second renderer query while the
  stream is healthy; status transitions still refresh the mounted panel.
- Verify watcher failure and time-derived fields recover within 30 seconds,
  and REST fallback restores current polling while disconnected.
- Show an idle connected renderer makes no recurring `/sessions` REST requests;
  active output sends changed rows rather than the 655-row snapshot each cycle.
- Repeat the baseline workload, measuring bytes and projection frequency as well
  as latency. Report active-output CPU limits rather than assuming improvement.
- Run scoped controller/protocol and sidecar integration tests, full repository
  verification, documentation checks and the required code review before merge.

## Implementation preparation

Before product code, approve this design, update #22's stale watcher assumptions
and expanded recommendation scope, record the transport decision in a proposed
ADR and update the MVP/Taskmaster specifications. The existing watcher was
removed; no dormant implementation can simply be wired up. Write a concise
implementation plan with the producer inventory, compatibility checks and the
recovery tests above. Follow the repository's plan review and code review gates.

An implementation can be reverted to the existing polling loops without changing
stored metadata. Rollout must preserve that fallback until protocol and platform
coverage is verified. No data migration is required.

## Design review

A bounded fresh-context builder check identified cursor semantics and partial
projection/deletion handling. A separate source correctness check identified
the live Peon overlay and the shell's independent recommendations hook. This
record incorporates those findings. The implementation plan records producer
coverage; the read-only socket rejects remote browser origins while supporting
loopback development, packaged opaque origins and native clients.
