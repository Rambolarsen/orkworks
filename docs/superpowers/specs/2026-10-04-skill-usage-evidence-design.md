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

## Terms and evidence semantics

Evidence is keyed by the immutable pair `(assignmentId, skillSnapshotId)` from
the approved #741 assignment configuration. A logical skill name alone is not
an identity. The snapshot binds skill ID, version, canonical content digest,
source reference, and the selected content bytes/delivery form. The
configuration digest and assignment revision pin the complete approved set.

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
delivery unless that adapter event independently includes a valid delivery
receipt. Neither `reported_used` nor inferred text may be relabeled
`observed_used`.

Usage records are append-only evidence events. The effective projection for a
skill retains separate `reportedUsed` and `observedUsed` summaries, each with
source, time, event count, and coverage. If evidence sources disagree, preserve
both. Do not collapse them into one confidence score or infer “unused” from a
zero count. A definitive negative-use result is outside this protocol.

## Assignment and reporter binding

An accepted report is bound to all of the following server-resolved values:

- repository/workspace identity and its current local registration generation;
- approved run ID, plan ID/revision, and immutable assignment ID/revision;
- task ID/attempt and launched child OrkWorks session ID;
- assignment configuration digest, including role, coding-tool executable/version
  identity, model binding, and effective permission profile;
- skill snapshot ID, version, and content digest;
- adapter identity, version, capability-register revision, and coverage declaration;
- sidecar runtime generation and report-capability generation.

The child cannot choose or override those authority-bearing fields. The
sidecar derives the reporting session from the same session-scoped bearer
capability used by the workflow-reporting pattern, then resolves that session
to its active immutable assignment. A report is accepted only while the
assignment is admitted and its plan/run authority is current. The capability is
not serialized in configuration, report bodies, logs, or evidence records.

Every request also carries the expected plan revision, assignment revision,
task attempt, runtime generation, and adapter generation. The server compares
them to its current durable assignment and live runtime. A stale, completed,
cancelled, superseded, foreign-workspace, or mismatched assignment is rejected
without persistence. Reports never reopen a task, resume a session, change a
plan, or create launch authority.

The proposed report API is workspace-local and authenticated:

```http
POST /taskmaster/assignments/{assignment_id}/skill-evidence
Authorization: Bearer <session-scoped report capability>
Content-Type: application/json
```

The exact route and capability wiring require implementation review against
the existing workflow observation handler. Authentication alone does not make
the caller an eligible native observer. Only a capability-verified adapter may
submit `delivery_receipt` or `observed_invocation` event kinds; the child
reporter may submit `reported_use`. The server assigns provenance from the
authenticated route/adapter registration, never from a caller-supplied
`source`, `confidence`, or `observed` flag.

## Event contract

The following JSON is illustrative. Server-owned IDs and bindings are returned
in the receipt and are not trusted from the request.

```json
{
  "schemaVersion": 1,
  "eventId": "01J9EXAMPLE7N8R3Y6K2M4P0Q1A",
  "expected": {
    "planRevision": 4,
    "assignmentRevision": 2,
    "taskAttempt": 1,
    "runtimeGeneration": 3,
    "adapterGeneration": 1
  },
  "skillSnapshotId": "skill-snapshot-graph-planning-7f2a",
  "kind": "reported_use",
  "occurredAt": "2026-10-07T14:23:10Z",
  "evidenceRef": {
    "kind": "skill_invocation_receipt",
    "digest": "sha256:5b0d..."
  }
}
```

`eventId` is an opaque client-generated idempotency key with at least 128 bits
of randomness. `occurredAt` is advisory and bounded; the server records its
own `receivedAt` and monotonic sequence. `evidenceRef` is an optional opaque
reference/digest to a bounded, redacted evidence object, never arbitrary
transcript text. The server returns the accepted event ID, assigned sequence,
effective provenance, and whether it was newly accepted or replayed.

The event kinds are closed:

- `delivery_receipt`: adapter proves delivery of the exact skill snapshot;
- `reported_use`: authenticated child reports usage;
- `observed_invocation`: adapter reports a recognized invocation event;
- `coverage_update`: adapter identifies the covered interval and any gaps.

Unknown event kinds and unknown schema versions fail closed. A delivery receipt
must include the exact delivery mechanism and content digest. An observed event
must identify the adapter's documented invocation event type and a stable
source event reference. Skill mentions in prompts, terminal output, shell
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
5. the role-specific permission profile and any route that could widen it.

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

The event ID is unique within an assignment and retained through the replay
window. The server hashes the canonical validated payload and stores the hash
with the event:

- same event ID and same payload hash returns the original receipt as an
  idempotent replay; it does not append a second event or increment counts;
- same event ID with different payload returns `409 idempotency_conflict`;
- stale revision/generation returns `409 stale_assignment`; a terminally closed
  assignment returns `410 assignment_closed`; neither persists the event;
- out-of-order sequence references are accepted only when the adapter contract
  defines reorder behavior; the server records received order and marks a gap.

Idempotency tombstones survive ordinary record trimming and assignment
completion for 24 hours. After that window an event may be rejected as
`410 replay_window_expired`; it must not be treated as a new event. Deleting an
assignment or workspace revokes its report capability immediately, so a late
retry cannot resurrect deleted evidence. A failed response after durable
commit is safe to retry with the same event ID. Persist event, tombstone, and
aggregate update atomically or recover them from the durable event log before
serving projections.

## Bounds, rate limits, and retention

These are proposed v1 contract limits, subject to reviewed implementation
capacity evidence. Requests over any limit are rejected before persistence;
the server must not truncate fields into a different accepted payload.

| Resource | Proposed bound |
| --- | ---: |
| JSON request body | 16 KiB |
| Event ID / opaque reference ID | 128 UTF-8 bytes |
| Skill snapshot ID | 256 UTF-8 bytes |
| Evidence digest | 128 ASCII bytes |
| Evidence reference object | 512 bytes |
| Events in one request | 1 |
| Skills in one assignment | 32 |
| Evidence events per assignment | 256 |
| Retained event bytes per assignment | 256 KiB |
| Retained events per workspace | 10,000 |
| Retained event bytes per workspace | 10 MiB |
| Accepted reports per workspace | 60 per rolling minute |
| Idempotency tombstones per workspace | 100,000 |

On assignment/workspace count or byte pressure, trim oldest non-protected event
payloads first and preserve a compact aggregate plus event-ID tombstone through
the 24-hour replay window. Do not trim the approved assignment/skill snapshot,
latest coverage state, deletion marker, or tombstones still within the replay
window. If tombstone capacity cannot preserve the full replay window under the
rate cap, reject new events with `429 evidence_capacity` before accepting them.
The workspace rate cap admits at most 86,400 IDs in a 24-hour window; the
100,000 compact-tombstone bound preserves headroom for clock and cleanup lag.
Apply this cap before allocating a tombstone or event record.

Retain raw bounded evidence events for 30 days after the assignment reaches a
terminal state, then delete event payloads and evidence references. Retain
bounded per-skill aggregates and assignment/configuration identity for up to
180 days after terminal state for local comparison, subject to repository
deletion. On repository/workspace deletion, remove events, aggregates,
references, and assignment-linked history; retain only a minimal deletion
watermark sufficient to reject late writes for 24 hours, then remove it. Users
must be able to clear evidence history for a repository without deleting
ordinary session history. Retention is local to the selected repository and
does not synchronize across workspaces or installations.

Startup recovery validates schema versions, checksummed records, per-file and
aggregate bounds, tombstone deadlines, sequence counters, and assignment
bindings before loading projections. Malformed or over-limit state is quarantined
and reported as degraded; startup must not guess sequence numbers, silently
relabel evidence, or resume acceptance with reset replay state. Recovery may
rebuild projections from valid event records and their protected compact
aggregates. Writes remain disabled for the degraded evidence store until a
reviewed repair path restores a consistent sequence and tombstone set.

## Privacy and evidence references

Store only event kind, opaque skill/assignment references, source/coverage,
timestamps, sequence/generation bindings, bounded digests, and aggregate
counts. Never store bearer tokens, secrets, hidden reasoning, full prompts,
complete transcripts, arbitrary tool arguments, or unredacted terminal output
as skill telemetry. Evidence references point to separately bounded,
redacted artifacts with independent access checks; if an artifact cannot be
redacted and authorized, omit the reference and retain the evidence event
without it. Hashes do not make sensitive source material safe to retain.

## HTTP outcomes

| Status | Meaning |
| --- | --- |
| `201` | New event durably accepted. |
| `200` | Exact idempotent replay; includes the original receipt. |
| `400` | Malformed JSON, unsupported schema/kind, or invalid field. |
| `401` | Missing, invalid, or revoked report capability. |
| `403` | Authenticated session is not the assigned producer for this event kind. |
| `404` | Assignment or skill snapshot is not visible in this workspace. |
| `409` | Idempotency conflict, stale revision/generation, or gap-policy conflict. |
| `410` | Assignment closed, replay window expired, or workspace deletion fenced the report. |
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
- An adapter restarts and reports sequence 12 after sequence 8. Unless its
  verified contract provides bounded reordering, the server marks the interval
  unknown and records partial coverage; it does not infer no usage in the gap.
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
    "reported": { "state": "recorded", "count": 1, "lastAt": "2026-10-07T14:23:10Z" },
    "observed": { "state": "unknown", "count": null, "coverage": "unsupported" }
  }
}
```

Badge behavior follows the parent design's refinement:

- selected badges are outlined;
- confirmed-loaded badges are filled;
- a recorded reported or observed use briefly highlights the badge, with a
  visible source label in details;
- a usage report alone can highlight an outlined badge but cannot confirm load;
- delivery-unconfirmed has an accessible text label, not color alone;
- missing use remains `unknown`, never “unused” or numeric zero;
- reduced-motion settings replace the brief highlight with a static indicator.

The sidecar may return null counts when coverage is unsupported/incomplete.
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
