# Taskmaster Recommendation Rollups

- Status: proposed
- Deciders: OrkWorks owner and Codex
- Date: 2026-09-13

## Context

The current Taskmaster workflow-improvement evaluator creates one proposed
recommendation for each exact observation fingerprint. A fingerprint is the
observation kind plus a normalized description. This makes repeated identical
observations useful, but it treats nearby descriptions as unrelated even when
they describe the same underlying problem:

- `Model detection`
- `Model detection issue`
- `Peon model detection`
- `Fixing peon model detection`

The current evaluator also permits one high-impact observation to create a
recommendation. That is appropriate for urgent evidence, but it means a noisy
or overly broad Peon output can create many single-observation cards. The
desktop panel currently renders every proposed card, so the user sees a list
of related fragments instead of a concise set of actionable problems.

The existing contract is still valuable: observations are immutable evidence,
their source sessions are retained, and exact recommendation families provide
stable dismissal and completion history. The change must improve signal
quality without replacing that audit trail or allowing a model to invent
relationships or evidence.

## Decision

Taskmaster will use two layers of identity:

1. **Exact evidence families** collect observations with the same structured
   problem identity. They remain the durable, deterministic audit units.
2. **Recommendation rollups** combine related proposed families into one
   actionable recommendation when bounded evidence supports that relationship.

The rollup is a persisted recommendation with the union of its member
families' evidence. Its member recommendations remain persisted as internal
`rolled_up` history, including their original IDs, fingerprints, evidence, and
source sessions. The API and desktop panel expose the rollup as the single
actionable card.

### Structured observation identity

Peon workflow-observation candidates gain a `problemArea` field. It is a short,
neutral description of the recurring problem, separate from `description`,
which continues to describe what happened and remains user-visible evidence.
For example:

```json
{
  "kind": "obstacle",
  "description": "Fixing peon model detection",
  "problemArea": "Peon model detection",
  "evidence": "The selected model was not detected",
  "reportedImpact": "high",
  "confidence": 0.8
}
```

The sidecar validates and normalizes `problemArea`; it never accepts an opaque
model-generated fingerprint as authority. New records use a versioned
fingerprint derived from `kind` and normalized `problemArea`. Legacy records
without `problemArea` remain readable and retain their v1 fingerprint; their
problem area is derived in memory from the legacy description only. Legacy
files are not rewritten merely because they are read.

The field is bounded to a non-empty, non-control string of at most 120
characters. Peon is instructed to keep it under eight words and to omit task
verbs, session-specific IDs, PR numbers, timestamps, and transient evidence
when those details are not the problem identity. The original `description`
and `evidence` remain unchanged.

Agent-origin reports use the same field and validation. During the compatibility
window, an omitted `problemArea` falls back to the supplied description so
older reporters remain accepted; new reporters should send it explicitly.

### Exact evidence families

The deterministic evaluator continues to group qualifying observations by
fingerprint and applies the existing confidence and impact thresholds. The
structured identity makes new observations more consistent, but exact family
identity remains conservative and does not attempt semantic matching.

The one-high-impact-observation rule remains available for urgent evidence.
The recommendation records that result as one occurrence from one source
session; it must not describe the observation as recurring.

### Rollup evaluator

After exact families are evaluated, a separate rollup pass considers only
currently proposed exact families. It never reads raw terminal replay and it
cannot cite evidence outside the supplied family snapshots.

When a configured Taskmaster model is available, the rollup pass receives a
bounded list of family IDs, target surfaces, structured problem areas,
descriptions, summaries, source sessions, and representative evidence. The
model may return candidate clusters containing only supplied family IDs. The
sidecar validates every member ID, rejects duplicate or empty clusters, and
requires at least two distinct families in a rollup. It computes the rollup
identity from the sorted member recommendation IDs and a server-owned
generation, never from generated prose.

Rollups must preserve a coherent target surface. The model must select one of
the supported target surfaces, and the sidecar rejects a cluster when its
members cannot be represented by that one scoped action. A family can belong
to at most one active rollup. The model may suggest concise rollup title and
summary text, but those fields do not determine identity and cannot add
evidence, recurrence counts, source sessions, or permissions.

Without a configured or available Taskmaster model, exact recommendations
continue to work. No semantic rollup is created from an unavailable, malformed,
stale, or invalid model result. This failure is isolated to rollups and does
not discard observations or exact recommendations.

### Rollup lifecycle and persistence

`RecommendationStatus` gains `rolled_up`. A member recommendation transitions
to that status only when a rollup parent has been durably written. The member
stores the parent rollup ID. Accepted, completed, dismissed, or otherwise
terminal recommendations are never silently merged into a rollup.

A rollup recommendation contains:

- a stable rollup ID derived from sorted member IDs and generation;
- the sorted member recommendation IDs and their dedupe keys;
- the union of member workflow evidence;
- the union of member source-session IDs;
- the selected target surface, priority, confidence, and bounded generated
  title/summary;
- a `supersedesRecommendationId` when it replaces a dismissed or superseded
  rollup.

Each member recommendation gains a nullable `rolledUpBy` field containing the
parent rollup ID. The field is written together with the member's `rolled_up`
status and is cleared only if a failed, uncommitted transition is recovered
before the parent becomes visible.

The rollup's recurrence count, affected sessions, impact, confidence, and
evidence are computed by the sidecar from member evidence. Generated prose
cannot change them. The rollup itself follows the existing explicit
`proposed`, `executing`, `accepted`, `completed`, and `dismissed` actions.

If later evidence changes the membership set, the old proposed rollup becomes
`superseded` and a new rollup is created with the old ID as its predecessor.
If later evidence belongs to the same member set, the existing proposed
rollup is updated in place. After a rollup is dismissed or reaches a terminal
state, a successor requires new qualifying evidence and a new generation; the
old rollup and its members remain immutable history apart from the explicit
rollup relationship.

The store writes parent and member transitions under the existing workspace
lock. If any part of the transition fails, no member is hidden and the prior
recommendation state remains visible. Re-running the same model result is
idempotent because member IDs and generation determine identity.

### API and desktop behavior

The existing recommendation list endpoint remains the desktop contract. It
returns active rollup parents and proposed exact families that are not members
of an active rollup. Rolled-up members remain available through the individual
recommendation endpoint for audit and handoff, but are not rendered as cards
in the normal list.

The single rollup card shows:

- the bounded proposed improvement and target surface;
- why it is suggested now;
- combined recurrence count and affected sessions;
- impact, confidence, and expected benefit;
- expandable member-family evidence with each observation's source and
  timestamp;
- the existing `Dismiss` and `Fix with AI` actions.

The generated fix prompt includes the rollup ID, member recommendation IDs,
combined evidence, and the existing `working-on-recommendation` handoff. It
continues to target only the user's active session and never starts or edits a
session in the background.

### Migration and compatibility

Existing v1 observation and recommendation JSON remains deserializable. Legacy
observations are not rewritten during migration. On the first rollup pass,
compatible existing proposed recommendations may become rollup members; their
original JSON records remain as `rolled_up` history. Accepted, completed, and
dismissed recommendations are left untouched and are excluded from initial
rollup membership.

The authoritative Taskmaster specification will be updated to replace the
v1-only “never combines different fingerprints” rule with this two-layer
contract. The metadata protocol documentation will describe the new
`problemArea`, rollup membership, and lifecycle fields. A numbered ADR is
required before implementation code because this changes the recommendation
identity and lifecycle protocol.

## Consequences

Positive consequences:

- New Peon evidence has a problem-oriented identity instead of using a task
  summary as its only dedupe input.
- Related exact families can become one actionable card while retaining every
  original observation and source session.
- Rollup IDs are stable and evidence-derived, so generated model prose cannot
  create duplicate identities or inflate claims.
- Existing deterministic behavior remains available when Taskmaster inference
  is not configured.
- Dismissal, acceptance, completion, and audit links remain explicit.

Costs and trade-offs:

- The recommendation schema gains versioned identity, rollup membership, and a
  new internal lifecycle state.
- Semantic rollups depend on the configured Taskmaster model and must handle
  malformed or stale output without affecting the deterministic path.
- Existing noisy proposed recommendations require a one-time rollup
  projection; terminal history is intentionally not merged.
- The evaluator and store must coordinate parent/member writes atomically under
  the workspace lock.
- A conservative target-surface compatibility rule may leave some related
  families separate when one scoped fix cannot represent them safely.

## Non-goals

- No autonomous acceptance, session creation, terminal input, file editing, or
  Git workflow action.
- No deletion of observations or recommendation history as part of grouping.
- No raw terminal replay added to model context.
- No general-purpose vector database, embedding index, or external similarity
  service.
- No multi-terminal or parallel-session presentation surface.
