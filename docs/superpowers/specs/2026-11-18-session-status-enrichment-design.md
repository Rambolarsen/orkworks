---
type: Design
title: Separate session status updates from repository enrichment
description: Approved status-only stream updates with adaptive, freshly validated Git and document summaries.
tags: [orkworks, performance, sessions, architecture]
status: draft
workflow_status: approved-for-implementation
---

# Separate session status updates from repository enrichment

Proposed for [#1895](https://github.com/Rambolarsen/orkworks/issues/1895).
The owner approved this design for implementation on 2026-11-19. This is not
a claim about installed behavior. The owner requested continued CPU work after
[scan reuse #1909](https://github.com/Rambolarsen/orkworks/pull/1909) merged.

## Outcome and evidence

A session producing terminal output should update its visible activity without
triggering Git/document inspection for every idle or remembered session.
Status delivery must continue while a slow repository inspection is running.

At source `ac522955`, `workspace_updates::run` calls the complete
`query_sessions` for session invalidations, at most once every two seconds per
subscriber. `SessionProjection::list_with_capacity_analysis` then resolves
locations, enriches documents and independently validates locations again.
The existing stream reduces bytes, but still pays complete projection cost.

A passive follow-up of the running debug daemon on 2026-11-18 measured 20.43%
of one core over 10.3279 seconds. A concurrent five-second sample again contained
session projection/Git inspection; Ollama was idle at the initial process check.
That daemon executable was in the primary checkout, whose source remained at
`abb7262d`; the exact compiled revision was not established. This is not a
controlled before/after measurement of #1909. Sampling and changing workload
also limit comparison. No daemon was replaced or session restarted.

## Proposed decision

Split the stream's display projection into status and enrichment. Keep complete
REST listing, explicit refresh, selected-document reads and action validation
as authoritative operations. Do not extend existing five-second scan cache TTLs.

| Path | Work and cadence |
| --- | --- |
| Status | Existing metadata/runtime/capacity/organization projection, without Git/document inspection; coalesce at the existing two-second cadence. |
| Enrichment | One in-flight worker per workspace owner, shared by opted-in subscribers; inspect current locations and Git/documents independently of status. |
| Periodic enrichment | Start a pass every five seconds during recent output activity, otherwise every 30 seconds, while at least one opted-in subscription exists. Never overlap passes or queue missed ticks. Freshness still expires after five seconds; a slow worker does not imply current data. |
| Explicit refresh | Request a fresh full validation pass immediately, subject to the single-flight bound; preserve refresh request identity until it completes or fails. |
| Reconciliation | Keep the existing 30-second complete metadata reconciliation, watcher rearm and reconnect recovery. |

### Adaptive cadence amendment

After the initial fixed-cadence fixture showed an idle regression, the owner
requested continuation after the adaptive-refresh recommendation on 2026-11-19.
Use five-second enrichment during output activity and 30-second enrichment when
quiet. The implementation defines recent activity as a changed output timestamp
on an existing row within the last ten seconds, measured with a monotonic clock.
Open/alive session count and ordinary metadata changes do not establish activity.
A resumed output stream advances the idle deadline to five seconds after the last
inspection start; it does not schedule a pass for each output event. Ten seconds
without a new observed output timestamp returns to the idle interval. This
window is an implementation choice, not a new session lifecycle state.

Known location/document mutations and explicit refresh remain immediate subject
to single-flight execution. Thirty-second metadata reconciliation does not start
an additional repository inspection if the independent cadence already covers
it. No scan cache TTL, action-validation rule or five-second display freshness
limit changes. Idle summaries can therefore be visibly stale between scans.
No recursive worktree watchers are introduced.

Status-only events do not reset the inspection-start clock or invalidate repository
caches. Only changed output timestamps select the faster periodic cadence; repeated
events cannot create extra passes inside that interval. The status projection may still inspect
all rows for harness-wide capacity and grouping correctness. This phase bounds
repository work independently of output rate; it does not promise constant-time
whole-list status projection or remove history scaling.

## Fields, identity and invalidation

Define explicit typed field groups rather than accepting arbitrary partial
`SessionInfo` objects. Enrichment owns `cwd`, `worktreeBinding`,
`gitContextSource`, `repoRoot`, `branch`, `dirty`, `changedFiles`, `lineChanges`,
`isWorktree`, `conflictWarning`, `recommendation`, `hasOpenablePlan` and `planDocuments`.
Implementation inspection identified `recommendation` as Git-derived in
`enrich_sessions_with_git_context`, so it follows enrichment rather than status.
The remaining current fields belong to the status projection, including capacity,
attention, organization and provenance. Review newly added fields explicitly.

A status update replaces the complete status group for each changed row, so
absent optional values clear previous values. It cannot overwrite enrichment.
Enrichment replaces its entire group and cannot roll back a newer status.
New rows initially have pending enrichment; deletions remove both groups.
A worker captured before deletion or runtime replacement cannot resurrect a row.

Capture workspace owner/generation, session runtime identity and location inputs
before inspection. Reuse current capture/inspect/recheck logic for enrichment;
all optimistic publication boundaries still perform fresh validation. Sharing
work is confined to that workspace owner and a current inspection pass. Group
matching worktree requests within the pass; independently revalidate each
session binding before publishing. Do not share handles across validation passes.

Known location/binding changes, accepted report/selection, runtime replacement
and branch-document mutations invalidate the affected enrichment immediately
and request a coalesced pass. A known canonical worktree/branch key invalidates
all its associated rows; before resolution, invalidate the session's old group
and new location rather than assuming a new branch. Unknown mutation scope,
watcher overflow or metadata replacement invalidates all enrichment. Existing
coarse mutation signals remain conservative until a producer has a tested
classification. Terminal-output producers alone get the status-only signal.

External branch, HEAD, path removal/replacement and unreported process-directory
changes are discovered by periodic fresh inspection, not inferred from unchanged
metadata. An unavailable explicit binding stays unavailable; it never falls
back to a different root. Preserve detached/unborn branch, failed-scan pin and
containment behavior. A failed pass publishes unavailable/stale enrichment,
not an empty authoritative session list or an automatic selection winner.

## Display freshness and action authority

Every enrichment group carries an owner-scoped revision, check time and
`pending | ready | stale | unavailable` state. Status updates never advance its
check time. The client marks it stale after five seconds without a successful
check, including while a worker is stalled. Old summaries may remain visibly
labelled as last checked; known location changes immediately clear old branch
and document summaries. No stale value is represented as a newly validated path.

Selected readers and all document/review/mutation actions retain their existing
fresh server-side identity, containment and runtime admission checks. A displayed
openable hint is never permission. Reader-byte caches keep their current invalidation
rules; status-only updates do not become reader-validation events. Known location
invalidation closes/invalidates affected reader state immediately. Opening or
refreshing a reader always uses its existing validated endpoint.

## Wire compatibility and recovery

Keep stream version 1's existing complete-row semantics for legacy subscribers.
New clients request an explicit `status-enrichment-v1` capability in their
subscription. A new server acknowledges it in the first snapshot; without that
acknowledgement, the client stays on the existing version-1 reducer and polling
fallback. A legacy request must never receive the new message types.

For negotiated clients, the initial authoritative snapshot establishes both
field groups and their freshness; initial synchronization can wait for the full
query. Thereafter `session-status-delta` and `session-enrichment-delta` share the
existing instance/workspace/generation/global sequence cursor with recommendation
invalidations. Each message is validated atomically before updating state.
Status messages own membership/removal/order; enrichment messages may only update
existing rows matching the captured row/location revision. Unknown IDs or
mismatched revisions cause resynchronization, never implicit row insertion.

A single workspace publisher orders status and enrichment publications. Workers
return prepared results to it and cannot write directly to sockets. Keep one
bounded pending message per consumer and the existing send timeout; a lagging
consumer reconnects to an authoritative snapshot. Dropped/gapped messages,
owner replacement or unsupported capability close the subscription and restore
REST fallback. Publication failure cannot delete valid rows. Tear down the
publisher when its owner is replaced or its last subscriber leaves; late worker
results are discarded. Cancellation need not interrupt a synchronous Git call.

[Session storage](../../../specs/session-storage.md) and
[ADR 0103](../../adr/0103-sqlite-session-storage.md) retain their separate
format-2/version-2 activation gates. This capability does not reserve version 2,
activate SQLite, change retention, or implement history pagination. Future
version-2 integration must explicitly define the capability for its live rows;
this change does not assume version-1 history payloads remain authoritative then.

## Required contract changes after approval

Amend the branch identity section of
[session plan review](../../../specs/session-plan-review.md) to distinguish
status-only display updates from enrichment refreshes. Proposed wording:

> Resolve the branch from the current validated worktree on every enrichment
> refresh, explicit full refresh, document read and action. Negotiated status-only
> updates do not refresh Git/document information or establish its validity;
> retain its separate freshness state and revalidate before every read/action.

Preserve all existing scan budgets, cache bounds and final validation clauses.
Amend [ADR 0092](../../adr/0092-workspace-update-stream.md): its consequences
already reserve active projection CPU as a separate optimization. Document the
opt-in field-group messages, workspace publisher, adaptive enrichment cadence
and independent status delivery, retaining legacy complete-row behavior and
ADR 0103's scoped supersession. Synchronize the desktop stream contract and
runtime guide in that implementation PR. Approval of this proposal precedes
those accepted-contract changes and dependent runtime implementation.

## Delivery and checks

1. After owner approval, update the above contracts and introduce typed status /
   enrichment projections. Retain the complete REST path as composition of both.
2. Add the workspace-owned scheduler and typed invalidation; connect the terminal
   output producer to status only. Add deterministic fake-clock and inspection
   counters before runtime implementation.
3. Implement negotiated stream messages and desktop reducer/freshness UI together,
   preserving Electron-main/renderer boundaries and old-server fallback.
4. Verify one busy plus many idle sessions, shared and distinct worktrees: any
   number of status ticks between enrichment deadlines causes zero additional
   Git/document opens. Two subscribers share one enrichment pass. A blocked scan
   cannot block status, input admission or heartbeat delivery. Count complete
   inspection work, not only the scoped open seam used in #1909.
5. Exercise branch switches, removed/replaced worktrees, explicit unavailable
   binding, selections/pins, detach, owner/runtime replacement, deletion during
   a scan, stale publications, missed events, slow consumers and reconnects.
   Check old/new client-server combinations and exact optional-field clearing.
6. Run required repository checks and current-diff review. Compare idle and busy
   CPU/latency with fixed debug/release build, workspace/session population,
   subscriber count and workload. Live daemon replacement remains a separate
   owner-controlled experiment; local synthetic results do not qualify it.

Rollback disables capability negotiation and returns clients to existing full
projection/recovery; no persistent migration or stored ownership state exists.
Keep legacy code until mixed-version tests pass. If measurements show idle cost
regression outweighs busy savings, revise the cadence before enabling the change.

## Design review

Current-session design review completed; owner approved implementation on 2026-11-19. Checked
scope, freshness, late-worker rejection, mixed-version recovery and storage
separation. Material concerns addressed in this draft: a five-second schedule
is not a freshness guarantee for slow scans; full worker results cannot replace
newer status; timers and workers are shared per owner, not per socket; unknown
producer signals remain conservative. No independent reviewer was used.

Risk assessment: dependencies 3 (sidecar and desktop protocol), blast radius 3
(shared list display), state changes 2 (opt-in transient protocol), reversibility
2 (legacy fallback, no migration), uncertainty 3 (live CPU benefit unmeasured).
Total 13/25; these ratings do not replace owner approval or runtime tests.
