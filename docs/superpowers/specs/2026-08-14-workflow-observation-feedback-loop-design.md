# Workflow observation feedback loop design

## Context

Peon currently produces a one-line `summary` describing concrete session work.
The latest value is persisted on `SessionMetadata`, and accepted changes are
also stored as durable summary checkpoints. The desktop uses the latest value
as one fallback for the selected session's situation headline and renders the
checkpoint log as "Task history."

That history does not advance OrkWorks' agentic development workflow. A prose
activity summary cannot reliably represent the recurring actions, obstacles,
missing context, assumptions, corrections, workarounds, and verification gaps
that should lead to better repository instructions, skills, tests, tooling, or
documentation.

OrkWorks needs two deliberately separate records:

- current session situation, used for coordination; and
- durable workflow evidence, used for improvement recommendations.

This design creates that split. It follows the existing product ownership rule:
Peon observes individual sessions and writes normalized evidence; Taskmaster
reads workspace state and recommends what should happen next. Neither worker
silently changes the repository.

## Goals

- Give the latest session summary a concrete consumer in Taskmaster's session
  coordination and future handoff decisions.
- Capture workflow friction as structured, immutable, workspace-local evidence.
- Accept both explicit agent reports and Peon inference through one recording
  seam while preserving source and confidence.
- Let Taskmaster correlate recurring observations and react to singular,
  high-impact observations.
- Present evidence-backed improvement recommendations without taking the
  recommended action.
- Stop treating summary history as workflow evidence.

## Responsibility split

Peon produces two independent outputs:

1. `SessionSituation` is the existing normalized session state, including the
   latest `summary`. It answers "what is happening now?"
2. `WorkflowObservation` is an immutable record of workflow friction. It
   answers "what made this work harder than necessary?"

Taskmaster reads both, for different purposes:

- It uses `SessionSituation` to understand current work across sessions,
  prioritize attention, judge review or handoff readiness, and avoid proposing
  transitions from stale or irrelevant context.
- It uses `WorkflowObservation` records to propose improvements to the
  development workflow.

Taskmaster never parses activity-summary prose to manufacture workflow
evidence. Peon never writes recommendations.

### Current-summary projection

`summary` becomes a first-class snapshot rather than text whose provenance is
borrowed from unrelated record-wide metadata fields. The session contract adds:

```text
summary
summarySource        agent | peon
summaryConfidence    confidence that the summary accurately describes the work
summaryObservedAt    timestamp of the accepted summary
```

An accepted non-empty Peon summary or agent attention message replaces all four
fields together. An attention report without a message leaves all four fields
unchanged. A newly submitted descriptive user instruction and an accepted
session-label reset command clear all four fields synchronously, preventing the
previous turn's activity from appearing current while new work starts.
Non-descriptive confirmations and hotkeys do not clear the snapshot.

Taskmaster uses only summaries carrying the dedicated source, confidence, and
timestamp fields. Legacy sessions that contain only the flat `summary` remain
displayable in the selected-session headline but do not become Taskmaster
handoff evidence until a new accepted summary populates the dedicated fields.

The Taskmaster workspace snapshot maps a valid snapshot to each session's
`currentWork` input. Session-transition recommendations that create a handoff
(`start_review_session`, `start_verification_session`, `start_fix_session`, or
`start_fresh_handoff_session`) include `Current work: <summary>` in their
generated prompt and cite its source and observed time in evidence. A missing
snapshot simply omits that line; it never blocks a recommendation. The snapshot
does not expire by wall-clock age: clearing on the next descriptive instruction
defines the turn boundary, and the last summary of an ended session remains
useful handoff context. It is never treated as workflow-friction evidence.

## Observation model

The persisted record has the following logical shape. Rust uses snake_case and
the JSON representation uses the camelCase names shown below.

```text
WorkflowObservation
  id                  stable occurrence identity
  sequence            durable monotonic workspace append order
  sessionId           originating OrkWorks session
  observedAt          accepted timestamp
  kind                repetition | obstacle | missing_context | assumption |
                      correction | workaround | verification_gap
  description         concise statement of the friction
  evidence            concrete action, missing fact, correction, or outcome
  reportedImpact      low | medium | high
  source              agent | peon
  confidence          confidence that the observation is accurate
  fingerprint         versioned, server-derived correlation key
  idempotencyKeyHash  server-derived durable retry identity; not API-exposed
```

The containing workspace metadata directory supplies workspace identity; a
caller cannot select another workspace. `description` is limited to 500 Unicode
scalar values and `evidence` to 2,000. Observations reference the useful fact
without storing large terminal excerpts, complete transcripts, or hidden model
reasoning.

`source` describes how the observation entered OrkWorks:

- `agent` means a coding agent explicitly reported the observation. The
  authenticated reporting adapter assigns confidence `0.9`; the caller cannot
  set it. This policy treats a direct report from the session agent as strong
  but fallible evidence.
- `peon` means Peon inferred the observation from terminal evidence. The
  candidate carries its own required confidence in the strict inference schema;
  one inference may therefore emit observations with different confidences.

Higher confidence makes evidence more useful; it does not make the claim
unquestionably true. Taskmaster remains responsible for applying deterministic
eligibility rules, and every resulting recommendation remains dismissible.

An observation is immutable while retained; bounded storage and explicit
session deletion may remove it. The `correction` kind means a human or reviewer
had to correct the coding agent in a way that reveals workflow friction; it is
not a mechanism for amending or retracting a stored observation. Observation
amendment/retraction is outside this first version. A false or unhelpful cluster
is handled by dismissing its passive recommendation.

## Recording module and adapters

A workflow-evidence module owns validation, normalization, fingerprinting,
deduplication, persistence, retention, and retrieval. Its external interface is
kept small:

```text
record_observation(session_id, origin, idempotency_key, candidate)
  -> accepted observation, duplicate identity, or rejection
workspace_observations(workspace_id) -> observations in append order
delete_session_observations(session_id) -> deletion outcome
```

Two adapters cross this seam:

- the explicit agent-report HTTP adapter; and
- the Peon inference adapter.

`origin` is a server-owned enum selected by the adapter, never a request field.
The module owns the confidence policy: it assigns `0.9` to an authenticated
agent report and requires a bounded per-candidate confidence for Peon input.
Neither adapter implements storage, fingerprinting, or deduplication rules.
Taskmaster reads through the module rather than opening metadata files directly.
Tests exercise the same interface as production callers.

Fingerprint version 1 is the string `v1:<kind>:<normalized-description>`, where
normalization trims the description, lowercases it, and collapses every run of
Unicode whitespace to one ASCII space. Evidence is deliberately excluded so
separate occurrences with different proof can correlate.

The module contains one workspace-scoped mutex. Idempotency lookup, durable
write, bounded-file trimming, and cache publication happen while holding that
mutex, so the Peon and HTTP adapters cannot race through a check-then-write
sequence. Before accepting a new occurrence, the module durably advances a
workspace counter and assigns that value to `sequence`; a crash may leave a gap
but cannot reuse an accepted sequence. An under-limit write appends one complete
JSON line, flushes it, and calls `sync_data` before publishing the cache entry
or returning success. A
write that would cross either segment bound instead writes the newest allowed
complete records, including the new one, to a temporary file, syncs it,
atomically replaces the segment, and syncs the parent directory. A failed
durable write does not publish the idempotency key. Ordering and dismissal
watermarks use `sequence`, never timestamps or random IDs.

An observation line carries the key hash and canonical request hash needed for
idempotency; raw caller keys are never persisted. When bounded trimming removes
an observation less than 15 minutes
after acceptance, the same atomic replacement retains a compact tombstone with
the key hash, payload hash, observation ID, sequence, and acceptance time. The
module caps all accepted observations for one session at 60 per rolling minute,
so reserving up to 1,024 tombstones inside the segment's byte bound guarantees
the 15-minute retry window. A matching retry within that window returns the
same observation identity even if its evidence record was trimmed; a
same-key/different-payload retry still conflicts. After 15 minutes the key is
expired for the explicit agent-report adapter, and reuse is treated as a new
logical occurrence with a new ID and sequence. Agents are instructed not to
retry older reports. Peon never resubmits a completed revision range, so its
unchanged-window guarantee does not depend on the tombstone lifetime.
Idempotency state is rebuilt from retained observations and unexpired
tombstones after restart.

On read, one malformed final line is treated as a crash tail: the store reports
a corruption diagnostic and truncates that final fragment under the writer
mutex before the next append. A malformed interior line is skipped but marks
workflow analysis degraded in diagnostics; Taskmaster may use later valid
records but the UI must disclose that evidence history is incomplete.

## Explicit agent reporting

The sidecar exposes a harness-neutral, session-scoped route:

```text
POST /sessions/:id/workflow-observations
```

Every live session receives an independent 256-bit random reporting capability
in `ORKWORKS_REPORT_TOKEN`. The token lives on `SessionHandle`, is not persisted,
and is replaced on resume. The route requires
`Authorization: Bearer <ORKWORKS_REPORT_TOKEN>` and rejects missing, malformed,
or wrong capabilities without recording an observation. Here, `source: agent`
means "reported by a holder of that live session's reporting capability," not a
verified statement about which harness process authored the text.

The JSON request contains only:

- `kind`;
- `description`;
- `evidence`; and
- `reportedImpact`.

The request also requires an `Idempotency-Key` header containing 1–128 visible
ASCII characters. Reusing a key for the same session and payload returns the
previous `{ observationId, sequence, acceptedAt }` identity with
`duplicate: true`; the initial response returns the same identity with
`duplicate: false`. Reusing the key with a different payload during the
15-minute idempotency window returns `409 Conflict`. After that window the key
is treated as a new report, as described by the recording contract.

The request cannot provide workspace identity, source, confidence, fingerprint,
recommendation text, or recommendation lifecycle. The sidecar requires a known
live session in the active workspace, limits the complete request body to 8 KiB,
applies the 500/2,000-character field limits, validates the fixed vocabulary,
derives server-owned fields, and rejects empty or malformed input. Each session
may attempt at most 30 reports in a rolling 60-second window; excess requests
return `429 Too Many Requests` without reaching persistence.

Every spawned session already receives `ORKWORKS_SESSION_ID` and
`ORKWORKS_PORT`; the new token completes the reporting capability. Coding agents
can use the route without a harness-specific protocol. Harness integrations may
add convenient wrappers later without changing the recording interface.

The route reports evidence only. It cannot directly create or mutate a
Taskmaster recommendation.

An authenticated explicit report with `reportedImpact: high` intentionally
meets the single-event eligibility threshold because it is direct evidence from
the live coding session. The fixed `0.9` remains below certainty, and the only
possible consequence is a dismissible passive recommendation.

## Peon inference

Peon's inference schema gains an optional collection of workflow-observation
candidates. Each candidate contains `kind`, `description`, `evidence`,
`reportedImpact`, and its own `confidence`. Peon is prompted to emit a candidate
only when terminal evidence supports one of the fixed kinds. It must not turn
ordinary progress, terminal redraws, or speculative advice into workflow
friction.

The existing session-situation inference remains independent. A single Peon
pass may update the current situation, report workflow observations, do both,
or do neither.

Each live `SessionRuntime` receives a random runtime-instance ID. The Peon
adapter derives its idempotency key from that ID, the session ID, current input
generation, first and last ring-buffer revisions in the analyzed snapshot, and
candidate index. Those bounds extend the existing Peon output-revision contract;
they are captured with the snapshot before inference. A Peon pass has a hard
two-minute deadline, including provider and persistence work. After every
candidate in a range is accepted, deduplicated, or permanently rejected, the
runtime advances `min_peon_output_revision` past that range before another pass
can be scheduled. A transient persistence failure does not advance the cursor
and remains eligible for retry within the deadline. Therefore the adapter never
submits the same completed range after the 15-minute tombstone window, even if a
test clock advances; retrying an in-flight range returns the same occurrence,
while a resumed runtime cannot collide with the prior runtime's revision range.
A genuinely repeated action produces a later range and another occurrence,
even when its normalized fingerprint is unchanged.

## Persistence and compatibility

Accepted observations are workspace-scoped but segmented by session under the
existing global metadata root:

```text
~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson
~/.orkworks/workspaces/<hash>/workflow-observations/sequence
```

The store aggregates these segments for Taskmaster. Segmentation keeps
workspace-wide correlation behind the module interface while making session
forgetting and configured retention exact. Append-oriented NDJSON matches the
existing event-log durability model and permits new optional fields without a
destructive migration; bounded trimming means a segment is not an unbounded
historical archive.

Each session segment is bounded to the newest 1,000 observations and 2 MiB,
including the reserved compact idempotency tombstones. It trims complete oldest
evidence records by atomic replace when a write would cross either limit and
removes expired tombstones during the same rewrite. Workspace reconstruction
reads at most the newest 10,000 evidence records across all retained segments,
ordered by `sequence`; older retained records remain on disk but do not
participate in the active recommendation pass. IDs are server-generated UUIDs
for identity only. The counter file is advanced by atomic replace and directory
sync before the observation write. On a fresh workspace it starts above the
maximum retained sequence; a malformed existing counter degrades workflow
analysis and rejects new observations instead of guessing and reusing an order
value. Segment readers distinguish public observation records from internal
idempotency tombstones; tombstones never reach Taskmaster or the desktop API.

Ordinary size trimming does not invalidate an existing recommendation. At
creation or update, the recommendation embeds immutable snapshots of every
cited observation field needed to explain the proposal. Its observation IDs
preserve lineage, while its expandable evidence does not depend on the source
segment remaining within the active storage window.

`DELETE /sessions/:id/forget` and automatic session retention delete that
session's observation segment in the same cleanup path as session metadata and
events. They also delete every derived recommendation referencing the removed
session; Taskmaster may recreate a recommendation only when the remaining
retained evidence still independently qualifies. This prevents orphaned links
and prevents recommendation prose from retaining evidence the user asked
OrkWorks to forget. The cleanup coordinator serializes recommendation and
observation deletion against reads, reports failure until both stores have been
updated, and runs the same orphan scrub during startup reconstruction. An
orphaned recommendation is never returned by the Taskmaster API while cleanup
is pending or after a crash.

The latest activity `summary` remains on `SessionMetadata` so it survives a
sidecar or desktop restart and can be consumed with the rest of the normalized
session situation.

New summary checkpoints are no longer appended to
`events/<session-id>.ndjson`. The desktop removes its summary-log fetch and
"Task history" surface, and the dedicated summary-log route is removed. The
generic historical event reader remains tolerant of old events containing
`summary` and `source`; existing files are not rewritten or deleted.

Workflow observations do not participate in `SessionMetadata` source-priority
overwrites. They are independent evidence occurrences, so an agent report does
not erase a Peon observation and a later Peon observation does not erase an
agent report.

## Taskmaster correlation and eligibility

Taskmaster evaluates accepted observations within the current workspace only.
It reacts five seconds after the latest accepted observation so a burst of
related records can be considered together, and it reconstructs its view from
persisted observations after a restart.

A deterministic evaluator considers:

- a fingerprint cluster containing at least two distinct observations whose
  individual confidence is at least `0.6`; or
- one high-impact observation whose confidence is at least `0.8`.

Two inference results over the same unchanged evidence window count as one
observation. Repeated actions remain distinct observations and therefore can
establish recurrence. Recurrence can occur within one session or across
sessions; the recommendation must state which happened.

Version 1 does not semantically combine different fingerprints. This deliberate
limit keeps the first evaluator deterministic and aligned with the accepted
Taskmaster rule engine. Equivalent observations whose wording normalizes to
different descriptions remain separate until a later, separately specified
model-assisted correlation phase exists. Peon prompts and explicit-report
guidance should therefore favor short, concrete problem statements.

The evaluator maps observation kinds to a default target and recommendation
template:

| Observation kind | Default target | Proposed-improvement template |
| --- | --- | --- |
| `repetition` | `tooling` | Automate or remove repeated work: `<description>` |
| `obstacle` | `tooling` | Remove or document the obstacle: `<description>` |
| `missing_context` | `instructions` | Add missing repository context: `<description>` |
| `assumption` | `instructions` | Make the required assumption explicit: `<description>` |
| `correction` | `instructions` | Prevent this recurring correction: `<description>` |
| `workaround` | `tooling` | Replace the workaround with a supported path: `<description>` |
| `verification_gap` | `test` | Add reliable verification for: `<description>` |

The template is presentation, not evidence. Every recurrence, session, impact,
and confidence claim is computed from the cited observations. Model-generated
correlation, target selection, and recommendation prose are non-goals for this
version.

Observations below `0.6` confidence may be stored for later supporting context
but do not count toward recurrence eligibility. A high-impact observation below
`0.8` does not qualify on its own.

## Recommendation model

Workflow improvement is the passive `improve_workflow` variant of Taskmaster's
canonical recommendation contract, not a parallel recommendation type or
store. It carries all required shared fields and the full shared lifecycle:

```text
Recommendation
  id
  workspaceId
  chainId
  chainDepth
  type                  improve_workflow
  status                proposed | accepted | executing | completed | dismissed |
                        superseded | expired | failed
  priority              derived from highest cited impact
  title
  summary
  reason                plain-language strings
  evidence              immutable workflow-observation snapshots
  sourceSessionIds
  targetSessionId       null
  suggestedHarnessId    null
  suggestedModel        null
  suggestedWorkingDirectory null
  suggestedPrompt       null
  confidence            low | medium | high
  requiresApproval      false
  dedupeKey
  createdAt
  updatedAt
  expiresAt
  workflowImprovement
    proposedImprovement
    targetSurface       instructions | skill | test | tooling | documentation
    observationIds
    recurrenceCount
    affectedSessionIds
    impact
    expectedBenefit
    supersedesRecommendationId null or dismissed predecessor ID
    dismissalWatermark null or dismissed evidence watermark
```

Recommendations use the existing workspace recommendation persistence area and
Taskmaster list/dismiss interfaces. They remain derived proposals, not
authoritative facts. `requiresApproval: false` means there is no executable
action to approve; it does not authorize OrkWorks to apply the improvement.
For this passive variant, only `proposed` and `dismissed` are reachable in
the first version. Other canonical statuses, including `superseded`, remain
valid for shared deserialization but cannot be produced by its evaluator.
When a dismissed recommendation's evidence later qualifies for a resurfaced
successor, the dismissed record remains immutable history and the successor's
`supersedesRecommendationId` records the lineage; the predecessor's status is
never rewritten. Priority is the
highest cited impact. Recommendation confidence is conservative: `high` only
when every qualifying cited observation is at least `0.8`; otherwise it is
`medium`. Ineligible observations are not cited or counted. A proposed
recommendation may be updated with later qualifying evidence while retaining
its identity and lifecycle history. Each canonical evidence entry contains the
observation ID, sequence, session ID, kind, description, evidence text, impact,
source, confidence, and observed time. This bounded duplication is deliberate:
it keeps proposed and dismissed recommendations explainable after ordinary
observation trimming. Explicit session forgetting or retention still deletes
the whole recommendation because the snapshot retains that session's evidence.

The dedupe family is
`improve_workflow:v1:<target-surface>:<observation-fingerprint>`. It never uses
generated prose. Dismissal stores an evidence watermark containing
`dismissedAt`, `dismissedThroughSequence`, qualifying observation IDs and
count, highest impact, and affected session IDs. The evaluator compares later
durable observations with that fixed watermark without mutating the dismissed
record or resurfacing it unless either:

- the highest impact increases; or
- at least two qualifying observations have a sequence greater than
  `dismissedThroughSequence`, including one from a session not represented in
  the watermark.

When either condition holds, Taskmaster creates one new `proposed`
recommendation with the same dedupe family and the dismissed ID in
`supersedesRecommendationId`. The dismissed record remains immutable history.
Unchanged evidence cannot create a duplicate, and only one proposed member of a
dedupe family may exist at a time.

## Presentation

The Taskmaster surface presents one card per active workflow recommendation.
The card shows:

- the proposed improvement and target surface;
- why Taskmaster is suggesting it now;
- recurrence count and affected sessions;
- impact, confidence, and expected benefit;
- expandable supporting observations with source and timestamp; and
- a `Dismiss` action.

The first version presents recommendations only. It does not create a GitHub
issue, start an implementation session, or edit repository files. Dismissal is
the only recommendation action introduced by this design.

The selected-session detail panel continues to use the latest summary as part
of its situation headline. The removed "Task history" list is not replaced by
workflow observations; improvement evidence belongs to Taskmaster, not the
session-detail activity presentation.

## Failure behavior

Observation and recommendation work is always secondary to the coding session:

- Peon inference failure cannot block or terminate a session.
- Observation persistence failure rejects that occurrence, logs a scoped error,
  and leaves session state unchanged.
- Invalid explicit reports receive a clear client error and are not partially
  persisted.
- A recoverable crash tail is truncated with a diagnostic; interior corruption
  leaves later valid observations readable but marks analysis degraded.
- Taskmaster failure does not lose accepted observations. Analysis can retry
  from the durable workspace log.
- Recommendation persistence failure does not mutate observation history.
- Low-confidence or ineligible evidence remains quiet rather than generating a
  speculative recommendation.

## Verification strategy

### Workflow-evidence module

- Accept every valid observation kind and reject unknown, empty, or oversized
  candidates.
- Derive source, confidence, timestamp, ID, workspace sequence, and fingerprint
  server-side.
- Keep fingerprint normalization stable for equivalent candidates.
- Return the original identity when either adapter retries the same durable
  idempotency key within 15 minutes, including after its evidence record was
  trimmed and after a store reconstruction.
- Reject reuse of one explicit idempotency key with a different payload during
  that window, then treat reuse after expiry as a new occurrence.
- Suppress repeated Peon inference over one unchanged revision range.
- Keep a completed Peon revision range suppressed after the idempotency clock
  advances beyond 15 minutes; only a later revision range can create another
  occurrence.
- Preserve a genuinely repeated action as a new occurrence with the same
  fingerprint.
- Append and reload observations in order across a fresh store instance.
- Preserve workspace append order across equal timestamps, segment boundaries,
  deletion, restart, and deliberate counter gaps.
- Serialize concurrent Peon and HTTP writers so check-and-append cannot race.
- Recover a partial final record and surface an interior-corruption diagnostic.
- Enforce the per-session record/byte bounds and the workspace reconstruction
  bound.
- Delete a forgotten or retention-removed session's observations and every
  recommendation that references them.

### Input adapters

- Verify the explicit route accepts a valid report for a known session and
  rejects unknown/dead sessions, missing or wrong capabilities, forged browser
  origins without the capability, caller-owned provenance fields, oversized
  bodies, and rate-limit excess.
- Verify explicit reporting and Peon inference both write through the same
  recording interface.
- Verify a Peon pass can update session situation without an observation, emit
  an observation without changing the summary, or do both.
- Verify each Peon candidate retains its own confidence and revision-bound
  idempotency key.

### Taskmaster

- Two distinct matching observations qualify for analysis; one ordinary
  observation does not.
- One sufficiently confident high-impact observation qualifies immediately.
- Low-confidence evidence cannot independently qualify.
- Different fingerprints never combine in version 1.
- Every observation kind maps to the specified deterministic target and text
  template.
- A recommendation cannot claim more recurrences or sessions than its evidence
  contains.
- `improve_workflow` recommendations serialize through the canonical contract,
  use the shared store, and expose no accept/execute action.
- Dismissal persists across restart.
- A mere count increase remains suppressed; increased impact or the defined
  two-observation/new-session threshold creates one linked successor.
- After a segment's 1,001st observation trims cited source evidence, proposed
  and dismissed recommendations still expose their immutable evidence snapshots.

### Desktop and compatibility

- The latest summary continues to drive the selected session's situation
  headline and is available to Taskmaster session coordination.
- Summary text, source, confidence, and timestamp update and clear atomically;
  Taskmaster ignores provenance-less legacy summaries.
- The desktop no longer fetches or renders summary checkpoint history.
- Historical metadata containing summary checkpoint fields remains readable.
- Recommendation cards render rationale, confidence, recurrence, session links,
  evidence expansion, and dismissal.
- Unavailable Taskmaster analysis does not degrade session switching or terminal
  interaction.

The end-to-end acceptance scenario is two sessions encountering the same
missing-context problem, recording two observations through either adapter,
and Taskmaster presenting one evidence-backed recommendation to improve the
relevant repository guidance.

## Documentation and decision impact

This changes the authoritative meaning of Peon summaries, durable event
history, and Taskmaster inputs. Implementation requires, before code:

- updating `specs/orkworks-mvp.md` and `specs/taskmaster.md`;
- adding ADR 0041, which supersedes ADR 0024 and restates its surviving bounded
  terminal-replay decision while replacing durable summary checkpoints with
  workflow observations;
- having ADR 0041 also supersede ADR 0029 while restating its surviving stable
  one-shot label decision and defining the current-summary snapshot;
- updating the metadata protocol and domain-entity documentation; and
- creating or updating the corresponding GitHub implementation issue before
  implementation begins.

## Delivery slices

The implementation plan should preserve one feedback-loop design while landing
it in reviewable slices:

1. Update the authoritative specs and ADRs, define the protocol types, and sync
   the metadata/domain documentation.
2. Add the workflow-evidence module, authenticated explicit-report adapter,
   Peon adapter, bounded segmented persistence, retention integration, and
   current-summary provenance; stop producing and displaying summary
   checkpoints.
3. Add deterministic Taskmaster eligibility, the passive canonical
   `improve_workflow` recommendation variant, dismissal watermarks, and restart
   reconstruction.
4. Add the Taskmaster recommendation cards and end-to-end desktop verification.

Each slice must keep the app usable and metadata backward-compatible. The issue
board may represent these as separate deliverable-sized issues under one
feature, consistent with the repository's one-logical-unit-per-PR rule.

## Non-goals

- Automatically editing instructions, skills, tests, tooling, or documentation.
- Creating GitHub issues or starting coding, review, or handoff sessions.
- Correlating evidence across workspaces or exporting it to a global learning
  system.
- Capturing audio, hidden model reasoning, complete transcripts, or large raw
  terminal excerpts.
- Treating explicit agent reports as unquestionable facts.
- Replacing the current session summary with workflow observations.
- Using activity-summary prose as Taskmaster's workflow-improvement evidence.
- Expanding Peon's observer-only authority or Taskmaster's explicit-approval
  model.
