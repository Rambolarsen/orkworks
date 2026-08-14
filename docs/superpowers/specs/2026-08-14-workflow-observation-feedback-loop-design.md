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

## Observation model

The persisted record has the following logical shape. Rust uses snake_case and
the JSON representation uses the camelCase names shown below.

```text
WorkflowObservation
  id                  stable occurrence identity
  sessionId           originating OrkWorks session
  observedAt          accepted timestamp
  kind                repetition | obstacle | missing_context | assumption |
                      correction | workaround | verification_gap
  description         concise statement of the friction
  evidence            concrete action, missing fact, correction, or outcome
  reportedImpact      low | medium | high
  source              agent | peon
  confidence          source-derived confidence; never caller-selected
  fingerprint         versioned, server-derived correlation key
```

The containing workspace metadata directory supplies workspace identity; a
caller cannot select another workspace. `description` is limited to 500 Unicode
scalar values and `evidence` to 2,000. Observations reference the useful fact
without storing large terminal excerpts, complete transcripts, or hidden model
reasoning.

`source` describes how the observation entered OrkWorks:

- `agent` means a coding agent explicitly reported the observation. The
  sidecar assigns confidence `1.0` to the provenance fact that the agent made
  the report; the caller cannot set it. This does not assert that the report's
  interpretation or proposed significance is objectively correct.
- `peon` means Peon inferred the observation from terminal evidence. The
  accepted inference confidence is retained.

Higher-confidence provenance makes evidence more useful; it does not make the
reported claim unquestionably true. Taskmaster remains responsible for judging
whether the evidence supports a recommendation.

Observations are append-only. Corrections do not rewrite history; a later
observation can clarify or contradict an earlier one, and Taskmaster must retain
both provenance trails.

## Recording module and adapters

A workflow-evidence module owns validation, normalization, fingerprinting,
deduplication, persistence, and retrieval. Its external interface is kept small:

```text
record_observation(session_id, origin, candidate)
  -> accepted observation or rejection
workspace_observations(workspace_id) -> observations in append order
```

Two adapters cross this seam:

- the explicit agent-report HTTP adapter; and
- the Peon inference adapter.

`origin` is a server-owned enum selected by the adapter, never a request field.
Neither adapter implements storage, confidence, fingerprint, or deduplication
rules. Taskmaster reads through the module rather than opening metadata files
directly. Tests exercise the same interface as production callers.

Fingerprint version 1 is the string `v1:<kind>:<normalized-description>`, where
normalization trims the description, lowercases it, and collapses every run of
Unicode whitespace to one ASCII space. Evidence is deliberately excluded so
separate occurrences with different proof can correlate. Taskmaster's semantic
pass handles related observations whose deterministic fingerprints differ.

## Explicit agent reporting

The sidecar exposes a harness-neutral, session-scoped route:

```text
POST /sessions/:id/workflow-observations
```

The request contains only:

- `kind`;
- `description`;
- `evidence`; and
- `reportedImpact`.

It cannot provide workspace identity, source, confidence, fingerprint,
recommendation text, or recommendation lifecycle. The sidecar requires a known
session in the active workspace, applies the 500/2,000-character limits,
validates the fixed vocabulary, derives server-owned fields, and rejects empty
or malformed input.

Every spawned session already receives `ORKWORKS_SESSION_ID` and
`ORKWORKS_PORT`, so coding agents can use the route without a harness-specific
protocol. Harness integrations may add convenient wrappers later without
changing the recording interface.

The route reports evidence only. It cannot directly create or mutate a
Taskmaster recommendation.

## Peon inference

Peon's inference schema gains an optional collection of workflow-observation
candidates. Peon is prompted to emit a candidate only when terminal evidence
supports one of the fixed kinds. It must not turn ordinary progress, terminal
redraws, or speculative advice into workflow friction.

The existing session-situation inference remains independent. A single Peon
pass may update the current situation, report workflow observations, do both,
or do neither.

Repeated inference over the same unchanged evidence window must not create new
occurrences. A genuinely repeated action must create another occurrence, even
when its normalized fingerprint matches an earlier record. The recording module
therefore suppresses identical candidates tied to the same evidence window,
not all consecutive observations with the same fingerprint.

## Persistence and compatibility

Accepted observations are appended to a workspace-scoped NDJSON file under the
existing global metadata root:

```text
~/.orkworks/workspaces/<hash>/workflow-observations.ndjson
```

Workspace scope makes correlation direct while `sessionId` retains provenance.
Append-only NDJSON matches the existing event-log durability model and permits
new optional fields without a destructive migration.

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

A deterministic eligibility pass limits semantic analysis to:

- a fingerprint cluster containing at least two distinct observations whose
  individual confidence is at least `0.6`; or
- one high-impact observation whose confidence is at least `0.8`.

Two inference results over the same unchanged evidence window count as one
observation. Repeated actions remain distinct observations and therefore can
establish recurrence. Recurrence can occur within one session or across
sessions; the recommendation must state which happened.

Taskmaster may semantically combine closely related fingerprints after the
eligibility pass. It must preserve all contributing observation IDs and cannot
claim recurrence or impact without traceable supporting records. Explicit agent
reports carry more evidentiary weight than Peon inference, but neither source
automatically wins a semantic disagreement.

Observations below `0.6` confidence may be stored for later supporting context
but do not count toward recurrence eligibility. A high-impact observation below
`0.8` does not qualify on its own.

## Recommendation model

Taskmaster produces a workspace-local `WorkflowRecommendation` with:

```text
WorkflowRecommendation
  id
  title
  proposedImprovement
  targetSurface        instructions | skill | test | tooling | documentation
  rationale
  observationIds
  recurrenceCount
  affectedSessionIds
  impact
  confidence
  expectedBenefit
  status               proposed | dismissed | superseded
  createdAt
  updatedAt
```

Recommendations use the existing workspace recommendation persistence area.
They remain derived proposals, not authoritative facts. Updating a
recommendation must retain its evidence references and lifecycle history.

Dismissal is persisted. Each recommendation has an internal versioned
fingerprint derived from its normalized proposed improvement, target surface,
and impact. Taskmaster does not resurface a dismissed recommendation while that
fingerprint remains unchanged. An additional matching occurrence updates the
stored evidence count and affected sessions without creating or resurfacing a
card. A changed proposed improvement, target surface, or impact creates a new
fingerprint and may produce a new recommendation linked to the superseded one.

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
- A malformed historical NDJSON line is skipped with a warning; later valid
  observations remain readable.
- Taskmaster failure does not lose accepted observations. Analysis can retry
  from the durable workspace log.
- Recommendation persistence failure does not mutate observation history.
- Low-confidence or ineligible evidence remains quiet rather than generating a
  speculative recommendation.

## Verification strategy

### Workflow-evidence module

- Accept every valid observation kind and reject unknown, empty, or oversized
  candidates.
- Derive source, confidence, timestamp, ID, workspace, and fingerprint
  server-side.
- Keep fingerprint normalization stable for equivalent candidates.
- Suppress repeated inference over one unchanged evidence window.
- Preserve a genuinely repeated action as a new occurrence with the same
  fingerprint.
- Append and reload observations in order across a fresh store instance.
- Skip a malformed NDJSON line without losing later valid entries.

### Input adapters

- Verify the explicit route accepts a valid report for a known session and
  rejects unknown sessions and caller-owned provenance fields.
- Verify explicit reporting and Peon inference both write through the same
  recording interface.
- Verify a Peon pass can update session situation without an observation, emit
  an observation without changing the summary, or do both.

### Taskmaster

- Two distinct matching observations qualify for analysis; one ordinary
  observation does not.
- One sufficiently confident high-impact observation qualifies immediately.
- Low-confidence evidence cannot independently qualify.
- Semantic grouping retains every contributing observation ID.
- A recommendation cannot claim more recurrences or sessions than its evidence
  contains.
- Dismissal persists across restart.
- Equivalent new evidence updates a dismissed recommendation's history without
  resurfacing it; materially changed impact or scope may resurface it.

### Desktop and compatibility

- The latest summary continues to drive the selected session's situation
  headline and is available to Taskmaster session coordination.
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
- superseding or amending ADR 0024's durable-summary-checkpoint decision with a
  new ADR rather than silently diverging from it;
- reconciling ADR 0029's statement that summary checkpoint history is the sole
  home for current task detail while retaining its stable-label decision;
- updating the metadata protocol and domain-entity documentation; and
- creating or updating the corresponding GitHub implementation issue before
  implementation begins.

## Delivery slices

The implementation plan should preserve one feedback-loop design while landing
it in reviewable slices:

1. Update the authoritative specs and ADRs, define the protocol types, and sync
   the metadata/domain documentation.
2. Add the workflow-evidence module, explicit-report adapter, Peon adapter, and
   observation persistence; stop producing and displaying summary checkpoints.
3. Add Taskmaster eligibility, semantic correlation, recommendation persistence,
   dismissal, and restart reconstruction.
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
