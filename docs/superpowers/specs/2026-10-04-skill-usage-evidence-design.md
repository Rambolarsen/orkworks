# Skill delivery and usage evidence protocol

> **Status:** Proposed component contract for written review. This document does
> not authorize runtime implementation or claim any coding-tool capability is
> verified.
>
> **Issue:** [#743](https://github.com/Rambolarsen/orkworks/issues/743) in the
> [agent hierarchy initiative (#738)](https://github.com/Rambolarsen/orkworks/issues/738).
> **Depends on:** [role configuration (#741)](https://github.com/Rambolarsen/orkworks/issues/741),
> [coding-tool capability research (#740)](https://github.com/Rambolarsen/orkworks/issues/740),
> and the [ordinary-child orchestration contract (#610)](https://github.com/Rambolarsen/orkworks/issues/610).
> **Product design:** [Agent hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md).
> **Baseline:** [Taskmaster orchestration](../../../specs/taskmaster.md#proposed-ordinary-child-orchestration-extension)
> and [accepted ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md).

## Scope and status

Define bounded, assignment-bound evidence for which skills were selected for a
child, confirmed delivered to it, reported as used by the child, and observed as
invoked by a verified coding-tool adapter. The protocol also defines unknown
coverage, replay handling, retention/deletion, recovery, and the renderer's
evidence projections.

The records are independent of existing workflow-friction observations. They
are inputs to later assignment evaluation and repository-scoped configuration
learning; they do not score work, establish causality, imply user acceptance, or
grant authority. Role/profile and skill identities are consumed from the
proposed immutable assignment contract in #741. All launch, exact-plan approval,
parent-only delegation, worktree, manual-integration, and single-selected-
terminal constraints in #610 remain binding.

There is no verified initial adapter. The #740 register currently marks all six
Copilot profiles no-go and names no verified substitute. A source session,
adapter, or skill must not be described as supported until the version-specific
delivery, permission, and event-coverage evidence required by #740 has passed
review. Until then the structures below are design contracts and the UI
projects capability as unavailable or unknown.
All JSON examples below are synthetic contract fixtures; they do not claim that
Copilot or any other adapter is eligible.

## Terms and evidence semantics

Evidence is keyed by the approved #741/#742 identity tuple: `runId`, `planId`,
`planRevision`, `taskId`, `taskVersion`, `reservationId`, `childSessionId`,
`parentSessionId`, `configurationId`, `configurationDigest`, `sidecarGeneration`,
`launchGeneration`, `adapterGeneration`, and `evidenceStoreGeneration`. These
existing identities do not
introduce an `assignmentId`, `assignmentRevision`, or `taskAttempt`.
`configurationId` and its digest bind the selected skills; plan/task/reservation/
session identities bind the exact launched child. Retries or changed
assignments require a newly approved plan revision and configuration. A
logical skill name alone is not an identity. `skillSnapshotId` is exactly the
`SkillSnapshot.id` from #741, with no second ID namespace. That snapshot binds
skill version, canonical content digest, source reference, and the selected
content bytes/delivery form.

Evidence is historical per launch generation. The current projection is keyed
by `(configurationId, configurationDigest, skillSnapshotId,
sidecarGeneration, launchGeneration, adapterGeneration,
evidenceStoreGeneration)`. A resumed runtime
gets a new launch generation even when its session/configuration IDs stay the
same. Older events remain historical and cannot confirm that a skill is loaded
or used in the new runtime; the new launch needs its own delivery and usage
evidence. Context recall is not proof of continued loading.

| Term | Meaning | Producer and strength |
| --- | --- | --- |
| `selected` | The exact skill snapshot appears in the approved assignment configuration. | Derived by the sidecar from the approved configuration; not a report. |
| `loaded` | A verified adapter receipt proves the exact skill snapshot content was made available to the child through a reviewed delivery route. | Sidecar accepted delivery receipt from a capability-verified adapter; proves delivery only, never reading or use. |
| `reported_used` | An authenticated child report claims it invoked or followed the named skill. | Child self-report; retained as reported evidence, not promoted to observed evidence. |
| `observed_used` | A verified adapter emitted a recognized invocation event for the exact skill snapshot, and the event passed the reviewed observer contract. | Native adapter event with exact assignment/skill binding and documented coverage; stronger source provenance, not proof of useful application or quality. |
| `unknown` | Delivery or usage cannot be established for this skill and interval, including unsupported adapters, incomplete coverage, missing reports, or interrupted runs. | Derived state; absence of evidence is not evidence of non-use. |

`selected`, `loaded`, and usage are separate dimensions. A selected skill can
remain delivery-unconfirmed. A use report without a delivery receipt remains a
reported claim and cannot confirm delivery. An observation event cannot imply
delivery; loading requires a separate `delivery_receipt` event. Neither
`reported_used` nor inferred text may be relabeled
`observed_used`.

Usage records are append-only evidence events. The effective projection for a
skill retains separate `reportedUsed` and `observedUsed` summaries, each with
source, time, event count, and coverage. If evidence sources disagree, preserve
both. Do not collapse them into one confidence score or infer “unused” from a
zero count. A definitive negative-use result is outside this protocol.

## Assignment and reporter binding

An accepted report is bound to all of the following server-resolved values,
using the #741 configuration and #742 plan/task identity vocabulary:

- repository/workspace identity and its current local registration generation;
- approved `runId`, `planId`, and `planRevision`;
- `taskId`, `taskVersion`, `reservationId`, `parentSessionId`, and launched
  `childSessionId`;
- `configurationId`/`configurationDigest`, including role, coding-tool
  executable/version identity, model binding, and effective permission profile;
- skill snapshot ID, version, and content digest;
- adapter identity, version, capability-register revision, and coverage declaration;
- `sidecarGeneration`, `launchGeneration`, `adapterGeneration`, and current
  `evidenceStoreGeneration`.

The child cannot choose or override those authority-bearing fields. The
sidecar derives the reporting session from the same session-scoped bearer
capability used by the workflow-reporting pattern, then resolves that session
to its active immutable assignment. A `reported_use` event is therefore an
authenticated self-report from the assigned OrkWorks session. The bearer does
not prove which same-user process emitted the request: under #610 and ADR 0077,
same-user processes may inspect or replay environment bearers. Preserve this
limitation in provenance and UI copy; do not describe self-reports as
process-authenticated. A report is accepted only while the assignment is
admitted and its plan/run authority is current. The capability is not
serialized in configuration, report bodies, logs, or evidence records.

Every request carries expected values for the plan revision, task version,
reservation, child session, configuration identity/digest, and generation
identities, including `parentSessionId` and `evidenceStoreGeneration`.
`planRevision` uses #742's
1–64 range; `taskVersion` uses its
1–2^31−1 range. `sidecarGeneration` and `launchGeneration` are lowercase
64-character hex strings. `adapterGeneration` is #741's bounded ASCII identity
(1–128 bytes), not a numeric counter. New events must match the durable plan,
assignment and current live runtime binding. Exact replays are checked against
the immutable stored launch binding, so they remain verifiable after the child
runtime ends. A stale, superseded, foreign-workspace or mismatched tuple is
rejected without persistence. A completed assignment accepts only an exact
replay of an already committed event during its replay window. Reports never
reopen a task, resume a session, change a plan, or create launch authority.

The proposed report API is workspace-local and authenticated:

```http
POST /sessions/{childSessionId}/orchestration/skill-evidence
Authorization: Bearer <session-scoped report capability>
Content-Type: application/json
```

The child route accepts only `reported_use` and a correction of that same
session's self-reports. It rejects adapter-only event kinds. Adapter evidence
uses a separate sidecar-internal integration call bound to a server-held
adapter capability. That capability is never injected into the coding-tool
environment, prompt, terminal, or child report token. The sidecar assigns
adapter provenance only after checking the compiled integration identity,
exact adapter/version binding, capability evidence, launch generation, and
evidence-store generation. If an adapter cannot submit through this separated
route, it cannot report delivery or native invocation evidence and remains
unsupported. The #740 register currently verifies no adapter. The server
assigns provenance from the authenticated route/integration, never from
caller-supplied `source`, `confidence`, or `observed` fields.

## Event contract

The following JSON is illustrative. Server-owned IDs and bindings are returned
in the receipt and are not trusted from the request.

```json
{
  "schemaVersion": 1,
  "expected": {
    "runId": "run-123",
    "planId": "plan-456",
    "planRevision": 4,
    "taskId": "task-789",
    "taskVersion": 2,
    "reservationId": "reservation-abc",
    "parentSessionId": "session-parent",
    "childSessionId": "session-def",
    "configurationId": "config-ghi",
    "configurationDigest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "sidecarGeneration": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "launchGeneration": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "adapterGeneration": "copilot-cli-1.0.90-profile-v1",
    "evidenceStoreGeneration": 3
  },
  "producerStreamId": "rs_01J9EXAMPLE7N8R3Y6K2M4P0Q1A",
  "producerSequence": 18,
  "eventId": "rs_01J9EXAMPLE7N8R3Y6K2M4P0Q1A:18",
  "skillSnapshotId": "skill-snapshot-graph-planning-7f2a",
  "kind": "reported_use",
  "occurredAt": "2026-10-07T14:23:10Z",
  "evidenceRef": {
    "kind": "source_metadata_digest",
    "digest": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
  }
}
```

`producerStreamId` is an opaque ID issued by the sidecar for one configuration,
launch generation, evidence-store generation, producer kind, and adapter
generation; the authenticated route selects the
authoritative stream, and callers cannot create or reset it. `producerSequence`
is a positive, monotonically increasing integer in that stream. `eventId` is
the deterministic string `<producerStreamId>:<producerSequence>`, validated by
the server. The adapter and session reporter must persist and retry the same
sequence/event ID until acknowledged. A sequence gap is recorded as incomplete
coverage. The server records its own `receivedAt` and workspace-monotonic
`workspaceSequence`, distinct from the producer's sequence. `occurredAt` is
advisory; it must use `YYYY-MM-DDTHH:mm:ss[.sss]Z` UTC RFC3339 syntax with at
most 32 ASCII bytes, no earlier than
the bound launch's `startedAt` minus five minutes, and no later than server
receipt time plus five minutes. The UI
orders by server `receivedAt`, never by this producer-supplied value.
`evidenceRef` is an optional opaque reference/digest to a retained event or
approved source-metadata digest, never arbitrary transcript text. The server
returns the accepted event ID, workspace sequence,
effective provenance, and whether it was newly accepted or replayed.

The child event kinds are closed:

- `reported_use`: authenticated child reports usage;
- `correction`: authorized producer or user invalidates a specific accepted event.

The separate adapter integration can emit `delivery_receipt`,
`observed_invocation`, and `coverage_update`; those kinds are rejected by the
child HTTP route. The sidecar's Electron-authorized evidence-management action
can emit a user `correction`; renderer code cannot call it directly.

`correction` has this compact illustrative form (actor, source, and all
assignment fields remain server-resolved):

```json
{
  "schemaVersion": 1,
  "producerStreamId": "rs_01J9EXAMPLE7N8R3Y6K2M4P0Q1A",
  "producerSequence": 19,
  "eventId": "rs_01J9EXAMPLE7N8R3Y6K2M4P0Q1A:19",
  "kind": "correction",
  "targetEventId": "rs_01J9EXAMPLE7N8R3Y6K2M4P0Q1A:18",
  "reason": "misbound"
}
```

`correction` contains `targetEventId` and one closed reason: `duplicate`,
`misbound`, `source_invalidated`, or `user_correction`. It has no free-form
explanation. The assigned session may correct only its own `reported_use`; the
adapter integration may correct only its own adapter-origin events; an
Electron-main-authorized user action may correct any event in the current
repository. A correction cannot target another correction, cannot itself be
corrected, and cannot create or promote evidence. Each event may be corrected
once. The target remains in bounded history with its provenance; the active
projection and aggregates exclude it and mark it corrected.

Unknown event kinds and unknown schema versions fail closed. A delivery receipt
must include the exact delivery mechanism and content digest. An observed event
must identify the adapter's documented invocation event type and a stable
source event reference. Adapter source sequence maps to `producerSequence`;
coverage `fromSequence`, `throughSequence`, and gap ranges all use this same
stream-local sequence, while the returned sequence is workspace-local storage
order. One stream is scoped to one configuration, launch generation, producer
kind, and adapter generation. Sequences start at 1 and are strictly increasing. A restart creates
a new generation and stream; it cannot reuse the old stream or claim coverage
across the restart. Producers submit in sequence order. The server rejects a
lower sequence unless it is an exact replay with a retained matching event or
tombstone; a higher sequence is accepted and its skipped interval is marked as
a gap.

Skill mentions in prompts, terminal output, shell
history, logs, summaries, or arbitrary tool arguments do not qualify as an
invocation event. Adapters that cannot distinguish invocation from mention
must report no invocation coverage.

## Capability and coverage gates

An adapter's capability entry must bind exact coding-tool version and launch
configuration to reproducible evidence of:

1. how selected skill bytes are delivered and how the receipt binds those exact
   bytes to the launched child;
2. which native event identifies an actual skill invocation, where it is
   emitted, and how it binds to the child, skill version, and assignment;
3. observer coverage start/end, missing-event behavior, duplicate/gap handling,
   and runtime-generation fencing;
4. the adapter's ability to exclude full prompts, hidden reasoning, and
   transcript content from evidence;
5. a separated adapter-event ingress whose authority the child report route
   cannot invoke or impersonate;
6. the role-specific permission profile and any route that could widen it.

Until the evidence register verifies all requirements for an assignment's
requested role/profile, that adapter cannot produce `loaded` or
`observed_used` evidence for launch eligibility. A prompt instruction or
successful process start is not a delivery receipt. Unsupported usage
observation leaves usage `unknown`; it is not replaced with terminal parsing,
model inference, or child claims.

Coverage is explicit per assignment and adapter generation:

```json
{
  "coverage": {
    "delivery": "confirmed",
    "usageObservation": "partial",
    "fromSequence": 1,
    "throughSequence": 18,
    "gaps": [{ "from": 9, "through": 10, "reason": "adapter_restart" }]
  }
}
```

Allowed delivery coverage is `confirmed`, `unconfirmed`, or `unsupported`.
Allowed usage coverage is `complete`, `partial`, `unsupported`, or
`interrupted`. `complete` means the reviewed adapter contract covered the
entire launched assignment interval; it still does not make a missing event
proof of non-use. Any restart, dropped sequence, unavailable observer, or
unexplained interval marks coverage partial/interrupted and keeps the
projection unknown where evidence is absent.

## Replay, ordering, and conflicts

The pair `(producerStreamId, producerSequence)` determines a unique `eventId`.
The server hashes the canonical validated payload and stores the hash with the
event:

- same event ID and same payload hash returns the original receipt as an
  idempotent replay; it does not append a second event or increment counts;
- same event ID with different payload returns `409 idempotency_conflict`;
- the same producer sequence with a different event ID or payload returns
  `409 producer_sequence_conflict` while its record/tombstone is retained;
- stale revision/generation returns `409 stale_assignment`; neither persists
  the event;
- a terminally closed assignment rejects any new producer sequence with
  `410 assignment_closed`;
- a lower producer sequence with no matching retained event/tombstone returns
  `410 producer_sequence_expired`; it cannot be accepted as a new event;
- a higher sequence is accepted with an explicit gap marker; producers cannot
  submit out of order or move a stream backwards.

Idempotency tombstones survive ordinary record trimming and assignment
completion for 24 hours. The durable per-stream high-water sequence survives
for the assignment's retention period, even after an individual tombstone
expires. Thus an old sequence whose tombstone has expired is rejected as
`410 producer_sequence_expired`, not accepted as a new event; the receipt is
available only while its matching event/tombstone remains. A new sequence
represents a new report even if its prose or evidence resembles an earlier
report. For an already committed event, authenticate and validate the request,
then compare its event/tombstone payload before checking whether the assignment
still accepts new writes. An exact same-payload retry returns the original
receipt for 24 hours even after assignment completion; a changed payload
conflicts. Keep only a hashed report-capability verification record for this
24-hour replay-only period. It cannot authorize a new sequence. Run cancellation
does not restore plan authority; workspace deletion/evidence clear revoke the
record immediately, and sidecar restart revokes old-generation capabilities.
A stale-generation retry then receives a fenced response while any already
committed event remains visible in the projection. A failed response after
durable commit is safe to retry with the same event ID and sequence. Persist
event, tombstone, high-water mark, and aggregate update atomically or recover
them from the durable event log before serving projections.

## Bounds, rate limits, and retention

These are proposed v1 contract limits, subject to reviewed implementation
capacity evidence. Requests over any limit are rejected before persistence;
the server must not truncate fields into a different accepted payload.

| Resource | Proposed bound |
| --- | ---: |
| JSON request body | 16 KiB |
| Event ID / opaque reference ID | 128 UTF-8 bytes |
| `producerStreamId` | ASCII `[A-Za-z0-9_-]`, 1–64 bytes; colon is reserved for the event ID separator |
| `producerSequence` | Positive canonical decimal JSON integer, no leading zeros |
| `occurredAt` | UTC RFC3339 `YYYY-MM-DDTHH:mm:ss[.sss]Z`, at most 32 ASCII bytes, launch start −5 minutes through receipt time +5 minutes |
| Skill snapshot ID | 128 ASCII bytes, matching #741's `SkillSnapshot.id` |
| Evidence digest | 128 ASCII bytes |
| Evidence reference object | 512 bytes |
| Retained assignment identity/binding row | 2 KiB |
| `planRevision` | Integer `1..=64`, matching #742 |
| `taskVersion` | Integer `1..=2,147,483,647`, matching #742 |
| `sidecarGeneration` / `launchGeneration` | Opaque lowercase 64-character hex strings, matching #742 |
| `adapterGeneration` | Nonempty ASCII identity, at most 128 bytes, matching #741 |
| `evidenceStoreGeneration` | Positive integer `1..=2,147,483,647` |
| Producer sequence | Integer `1..=2,147,483,647`; stream exhaustion requires a new launch/plan binding, never wraparound |
| Retained assignment bindings per workspace | 1,000 |
| Producer streams per workspace | 2,000 |
| Events in one request | 1 |
| Skills in one configuration | 16, matching #741 |
| Evidence events per assignment | 256 |
| Retained event bytes per assignment | 256 KiB |
| Retained events per workspace | 10,000 |
| Retained event bytes per workspace | 10 MiB |
| Per-skill aggregates per workspace | 10,000 |
| Aggregate bytes per workspace | 2 MiB |
| Accepted reports per workspace | 60 per rolling minute |
| Attempts per valid capability | 120 per rolling minute, charged before JSON parsing and assignment lookup; includes replay and rejected requests |
| Attempts per workspace | 600 per rolling minute, charged before JSON parsing and assignment lookup |
| Idempotency tombstones per workspace | 100,000 |

Count a report attempt after validating its bearer capability but before JSON
decoding, assignment/configuration lookup, or event/tombstone hashing. Charge
new events, exact retries, malformed payloads, conflicts, stale generations,
and other rejected authenticated requests. Keep rolling attempt counters in
bounded short-lived memory and return `429 rate_limited` with `Retry-After`;
attempt counters are not evidence history. The workspace accepted-event limit
counts only newly persisted evidence/correction events. Enforce the body byte
limit while reading the request, before buffering the complete payload.

On assignment/workspace count or byte pressure, trim oldest raw event payloads
first and preserve compact aggregates, per-stream high-water marks, and
event-ID tombstones through the 24-hour replay window. Raw history has a
30-day maximum TTL, not a minimum residency guarantee: workspace/assignment
byte or count pressure may evict raw events sooner. Do not trim the approved
assignment/skill identity, latest coverage state, deletion marker, active
high-water marks, or tombstones still within the replay window. The 180-day
aggregate/history TTL is also an upper bound; the 1,000 assignment and 10,000
aggregate caps take precedence. When no expired binding/aggregate can be
evicted, reject new evidence with `429 evidence_capacity`; do not silently drop
an unexpired aggregate or weaken a replay guarantee. If tombstone capacity
cannot preserve the full replay window under the rate cap, reject new events
with `429 evidence_capacity` before accepting them.
The workspace rate cap admits at most 86,400 IDs in a 24-hour window; the
100,000 compact-tombstone bound preserves headroom for clock and cleanup lag.
Apply this cap before allocating a tombstone or event record.

Retain raw bounded evidence events for at most 30 days after the assignment
reaches a terminal state, subject to earlier eviction under the stated storage
caps; then delete event payloads and evidence references. Retain bounded
per-skill aggregates and assignment/configuration identity for at most 180 days
after terminal state for local comparison, subject to the stated caps and
repository deletion. If assignment/aggregate capacity is full and all records
are still within their retention period, reject new evidence until capacity is
available or the user clears repository evidence history. Repository evidence
clear is an Electron-main-authorized operation scoped to the registered
repository. It atomically increments `evidenceStoreGeneration`, revokes all
report and adapter capabilities/streams, removes evidence records and
aggregates, and writes a 24-hour deletion fence. Existing live assignments
become evidence-disabled and cannot repopulate the cleared store; they may
continue under their existing execution authority. New evidence requires a
later approved assignment bound to the new evidence-store generation. This
operation leaves ordinary session history untouched. Repository/workspace
deletion performs the same evidence revocation and deletion, then removes the
rest of workspace history under its own contract. Retention is local to the
selected repository and does not synchronize across workspaces or installations.

Startup recovery validates schema versions, checksummed records, per-file and
aggregate bounds, tombstone deadlines, sequence counters, and assignment
bindings before loading projections. Malformed or over-limit state is quarantined
and reported as degraded; startup must not guess sequence numbers, silently
relabel evidence, or resume acceptance with reset replay state. Recovery may
rebuild projections from valid event records and their protected compact
aggregates. Writes remain disabled for the degraded evidence store until a
reviewed repair path restores a consistent sequence and tombstone set.

## Privacy and evidence references

Store only event kind, opaque skill/configuration references, source/coverage,
timestamps, sequence/generation bindings, bounded digests, and aggregate
counts. Never store bearer tokens, secrets, hidden reasoning, full prompts,
complete transcripts, arbitrary tool arguments, or unredacted terminal output
as skill telemetry. `evidenceRef` may point only to another retained event in
this same evidence store or a digest of source metadata already present in the
approved assignment snapshot. It cannot contain a path, URL, blob, transcript,
or pointer to separately stored content. References are workspace- and
assignment-scoped, require the same authorization as their source event, and
are removed when the referenced event/snapshot is deleted. No external evidence
artifact store is introduced by this contract. Hashes do not make sensitive
source material safe to retain.

## HTTP outcomes

| Status | Meaning |
| --- | --- |
| `201` | New event durably accepted. |
| `200` | Exact idempotent replay; includes the original receipt. |
| `400` | Malformed JSON, unsupported schema/kind, or invalid field. |
| `401` | Missing, invalid, or revoked report capability. |
| `403` | Authenticated session is not the assigned producer for this event kind. |
| `404` | Assignment or skill snapshot is not visible in this workspace. |
| `409` | Idempotency conflict, producer-sequence conflict, or stale revision/generation. |
| `410` | Assignment closed, producer sequence expired, or workspace deletion fenced the report. |
| `413` | Request exceeds a byte/count bound. |
| `429` | Rate or evidence capacity limit; includes bounded retry guidance. |
| `503` | Store degraded or durable commit unavailable; retry only with the same event ID. |

Responses contain no bearer data, prompt text, or raw evidence. Errors use a
stable machine code plus a short safe message. They do not expose filesystem
paths or on-disk storage details.

## Recovery and deletion examples

- A child submits `reported_use`, the server commits it, and the response is
  lost. Retrying the same event ID and payload returns `200` and the original
  sequence; the count remains one.
- An adapter stream reports sequence 12 after sequence 8. The server accepts
  sequence 12, records 9–11 as a gap, and marks that interval unknown; it does
  not infer no usage in the gap. The restarted adapter uses a new stream and
  cannot claim continuity from sequence 8.
- A parent proposes a revised plan after research. Reports tied to the prior
  assignment/plan revision fail stale-generation validation. The new assignment
  gets new IDs and cannot inherit prior reports.
- Workspace deletion revokes report capabilities before deleting records.
  Late retries receive `410`; they cannot recreate the assignment or aggregate.
- Startup finds a corrupt sequence counter. The evidence store enters degraded
  mode, preserves valid readable records for diagnostics, and rejects new
  evidence until explicit repair; it does not restart at sequence one.

## UI projections

The renderer receives a typed, read-only projection from the sidecar; it does
not inspect raw event files or authenticate producers. For each skill, expose:

```json
{
  "skillSnapshotId": "skill-snapshot-graph-planning-7f2a",
  "label": "orchestrating-task-graphs",
  "delivery": { "state": "selected_unconfirmed", "source": null },
  "usage": {
    "reported": { "state": "recorded", "source": "assigned_session_self_report", "processOriginVerified": false, "count": 1, "lastAt": "2026-10-07T14:23:10Z" },
    "observed": { "state": "unknown", "source": "adapter_native_event", "count": null, "coverage": "unsupported" }
  }
}
```

Badge behavior follows the parent design's refinement:

- selected badges are outlined;
- confirmed-loaded badges are filled;
- a recorded reported or observed use briefly highlights the badge, with a
  visible source label in details; self-report copy says “reported by assigned
  session” and does not imply same-user process identity verification;
- a usage report alone can highlight an outlined badge but cannot confirm load;
- delivery-unconfirmed has an accessible text label, not color alone;
- missing use remains `unknown`, never “unused” or numeric zero;
- reduced-motion settings replace the brief highlight with a static indicator.

The sidecar derives `lastAt` from server `receivedAt`, never `occurredAt`. It
may return null counts when coverage is unsupported/incomplete.
Only display zero for an explicitly counted event stream in a stated coverage
interval, and still label that interval's coverage. The UI does not convert
that local interval into a lifetime no-use claim. Loading and usage evidence
remain separate from quality/completeness scoring (#744) and from user
acceptance.

## Contract verification and implementation gates

Before runtime planning, written review must resolve field compatibility with
#741, obtain eligible exact-version delivery and invocation evidence from #740,
and confirm the report/session-generation seam against the ordinary-child
authority model in #610. The initial supported adapter set may remain empty.
No automatic fallback, terminal scraping, model-inferred invocation, prompt-only
permission substitute, or new authority is introduced to fill that gap.

The future implementation plan must cover at least these contract cases:

- selected but unconfirmed delivery; confirmed delivery without usage;
- reported use without delivery receipt; observed event with valid coverage;
- invocation-looking terminal mention that must be rejected as observed use;
- unsupported, partial, interrupted, and complete adapter coverage;
- wrong workspace/session/assignment/skill/configuration digest;
- stale plan, task attempt, runtime, and adapter generations;
- same-key same-payload replay and same-key different-payload conflict;
- rate, request, per-assignment, and workspace bounds before persistence;
- trim with tombstone replay, expired replay, deletion, late retry, restart,
  malformed state, and durable-commit response loss;
- read-only renderer projection, source labels, delivery badge distinctions,
  unknown values, and reduced motion.

This remains specification-only until the component contract, capability
register, authoritative specification alignment, and a scoped implementation
plan are reviewed and approved. It does not enable launches or describe any
coding-tool profile as verified.
