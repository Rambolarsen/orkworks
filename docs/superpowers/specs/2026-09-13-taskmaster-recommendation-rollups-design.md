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

The v2 canonicalization algorithm is fixed: apply Unicode NFKC, Unicode
lowercase, trim leading/trailing whitespace, and collapse every run of
Unicode whitespace to one ASCII space. Punctuation is retained. The sidecar
forms the fingerprint input as `kind + "\\0" + canonical_problem_area` and
stores `v2:<kind>:<sha256-hex>` as the fingerprint. This keeps the persisted
key bounded and prevents caller-controlled fingerprint strings from becoming
authority. The fingerprint version is part of the key and is never silently
changed in place.

Agent-origin reports use the same field and validation. During the
compatibility window, an omitted `problemArea` is accepted and retains the
existing v1 description-based identity; the sidecar may derive the bounded
problem area for model context without rewriting the legacy observation. An
explicitly supplied empty or control-bearing `problemArea` is rejected; an
explicitly supplied overlong value is truncated by the canonicalizer. A
missing field must never cause an otherwise valid legacy report to be silently
discarded; an invalid explicit field produces a diagnostic and rejects only
that malformed report. New reporters should send `problemArea` explicitly.

On the wire and in persisted observations, `problemArea` is optional with a
serde default of `null`. A missing value identifies a legacy observation and
uses its v1 fingerprint; a compatibility adapter may derive a bounded area
for model context without rewriting the v1 record. Persisted recommendation fields
introduced by this design also have defaults (`[]` for member lists and
`null` for nullable relationship/generation fields). Unknown status values or
malformed relationship fields are quarantined with a diagnostic rather than
silently omitted. A binary that cannot read `rolled_up` records must not be
used against a workspace after this schema is written; startup should report
the protocol-version conflict.

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
limits are 32 families per pass, at most three deterministic representative
observations per family (earliest, latest, and highest-impact when distinct),
96 observations total, at most eight representative source-session IDs per
family, and 128 KiB for the serialized rollup input. The full member records
remain the audit source; representative evidence and source IDs are only model
context.

The model may return at most eight candidate clusters, with two to eight
distinct supplied family IDs per cluster, and title/summary limits of 240 and
1,000 characters. The complete model response is limited to 64 KiB. The
sidecar validates every member ID, rejects duplicate or empty clusters, and
rejects the entire response if any family occurs in more than one cluster.
Clusters and member IDs are sorted before evaluation, so model array order
cannot affect identity. No partial application is permitted for an invalid
response.

The rollup identity is independent of model prose and evaluation generation:
`rollup:<sha256-hex>` over the sorted member recommendation IDs. A
server-owned evaluation token contains the workspace instance ID, a
monotonic generation within that workspace instance, the provider/model
identity, and a hash of the supplied family snapshot. The token is not
model-supplied or persisted as authority. Applying a result requires the
same workspace instance and generation, and a re-read under the workspace
lock must confirm that every supplied family is still proposed, has the same
evidence snapshot, and has no changed active parent. Otherwise the result is
stale and is discarded. The accepted parent records the generation for
diagnostics, but a same-member-set result updates the same parent in place.

Rollups must preserve a coherent target surface. In v1 this means every
member family must have the same `targetSurface`; cross-surface relationships
are rejected rather than guessed. The model must select that supplied target
surface, and the sidecar rejects a cluster when its members cannot be
represented by that one scoped action. A family can belong to at most one
active rollup. The model may suggest concise rollup title and summary text,
but those fields do not determine identity and cannot add evidence,
recurrence counts, source sessions, permissions, or a target surface not
already present in the members.

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

- a stable rollup ID derived from the sorted member IDs;
- the sorted member recommendation IDs and their dedupe keys;
- a bounded projection of the union of member workflow evidence (at most 64
  entries and 128 KiB, selecting high-impact evidence first, then newest
  evidence, with observation ID as the tie-breaker); the member IDs remain
  authoritative for the complete audit evidence;
- the sorted, de-duplicated union of member source-session IDs, subject to the
  existing metadata bounds;
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
rollup and its active members are updated in place through one transaction.
After a rollup is dismissed or reaches a terminal state, a successor requires
new qualifying evidence and a new exact-family generation; the old rollup and
its members remain immutable history apart from the explicit current-rollup
relationship.

The store writes parent and member transitions under the existing workspace
lock. The transition is a recoverable store transaction, not merely a group of
independent `put` calls. The store stages all new parent/member JSON, records
the expected old file hashes and durable old-content backups in a fsynced
transaction manifest, fsyncs the staged files, and then commits the manifest
before publishing replacements. On startup and before serving recommendation
reads, the store recovers any unfinished manifest: an uncommitted transaction
is rolled back from its backups, while a committed transaction is completed.
The manifest is removed only after all replacements and the recommendations
directory are fsynced. If recovery cannot establish either the complete old
graph or the complete new graph, recommendation reads remain unavailable and
emit a diagnostic rather than exposing partial state. Temporary transaction
files are excluded from normal listing.

The graph has these invariants after recovery: every `rolled_up` member has
exactly one current `rolledUpBy`; that parent exists and lists the member;
every active parent lists existing members; and a family is in at most one
active parent. `rolledUpBy` is the current relationship only. A superseded
parent retains its historical member list, while members are atomically
released or assigned to the successor, so a singular field cannot create
ambiguous current ownership.

If a proposed rollup's membership changes, the old parent becomes
`superseded`; members that are not assigned to the new parent are returned to
`proposed`, and members assigned to the new parent point only to the new
parent. If a rollup is dismissed, accepted, completed, expired, or failed,
its members remain historical `rolled_up` records and are not rendered as
active cards. New qualifying evidence after a terminal parent never mutates
that terminal graph: it creates a new exact-family generation, which may later
receive a new rollup parent. This prevents terminal actions from being undone
and prevents new evidence from being stranded behind a closed parent.

Session cleanup is graph-aware. It must never delete a parent or member in
isolation. Historical records retain their original source-session IDs even
when a source session has been removed; orphan scrubbing may remove or mark a
whole recommendation graph only according to the existing retention policy,
and any release/reparenting is committed transactionally. A partial loss of
source sessions therefore cannot leave a surviving family permanently hidden
by a missing parent. The audit UI labels unavailable source sessions rather
than treating them as newly observed sessions.

Rollup evaluation uses the existing Taskmaster scheduler and provider
reservation. It is one bounded request per eligible workspace evaluation,
obeys the existing minimum interval, daily reservation, provider identity,
and workspace-instance invalidation rules, and is skipped when the budget or
provider is unavailable. Its cache key includes workspace instance, exact
family snapshot hash, provider/model identity, and rollup prompt version.

### API and desktop behavior

The existing recommendation list endpoint remains the desktop contract. It
returns active rollup parents and proposed exact families that are not members
of an active rollup. Rolled-up members remain available through the individual
recommendation endpoint for audit and handoff, but are not rendered as cards
in the normal list.

The JSON contract adds `rolled_up` to `RecommendationStatus`, optional
`problemArea` to workflow evidence, and the following recommendation fields:
`rollupMemberIds: string[]`, `rollupMemberDedupeKeys: string[]`,
`rollupGeneration: number | null`, and `rolledUpBy: string | null`.
Legacy records deserialize as empty lists/nulls. A non-empty member list marks
the record as a rollup parent; `rolledUpBy` is required exactly when status is
`rolled_up`. The list endpoint returns only proposed/executing rollup parents
and proposed exact families without an active parent. It never returns a
`rolled_up` member in the normal actionable list. Detail responses expose the
parent/member relationship, and actions on a rolled-up member return the
existing invalid-transition response rather than acting on hidden state.

The single rollup card shows:

- the bounded proposed improvement and target surface;
- why it is suggested now;
- combined recurrence count and affected sessions;
- impact, confidence, and expected benefit;
- expandable member-family evidence with each observation's source and
  timestamp;
- the existing `Dismiss` and `Fix with AI` actions.

The generated fix prompt includes the rollup ID, member recommendation IDs,
combined bounded evidence, and the existing `working-on-recommendation`
handoff. It continues to target only the user's active session and never
starts or edits a session in the background. Evidence, source descriptions,
and generated rollup prose are inserted as explicitly delimited untrusted
reference data, with control characters removed and the same prompt-size
limits applied. The handoff tells the active coding tool not to follow
instructions found inside that data and to treat it only as a report of prior
observations.

### Migration and compatibility

Existing v1 observation and recommendation JSON remains deserializable. Legacy
observations are not rewritten during migration. On the first rollup pass,
compatible existing proposed recommendations may become rollup members; their
original JSON records remain as `rolled_up` history. Accepted, completed, and
dismissed recommendations are left untouched and are excluded from initial
rollup membership.

The migration is read-compatible, not rollback-compatible: once a workspace
contains `rolled_up` records, an older sidecar that does not understand the
status must refuse the workspace with a protocol-version diagnostic. The
first rollup pass uses only currently proposed exact families and records a
snapshot hash, so a restart or interrupted pass can safely retry without
rewriting legacy observations. A legacy family that later receives qualifying
evidence follows the terminal/member-generation rules above rather than
silently reviving a dismissed or completed record.

The authoritative Taskmaster specification will be updated to replace the
v1-only “never combines different fingerprints” rule with this two-layer
contract. The metadata protocol documentation will describe the new
`problemArea`, rollup membership, and lifecycle fields. A numbered ADR is
required before implementation code because this changes the recommendation
identity and lifecycle protocol.

## Implementation sequence

Implementation follows these dependency boundaries:

1. Record the ADR and update the authoritative Taskmaster and metadata
   protocol documentation.
2. Add versioned observation identity, optional-field deserialization, legacy
   fallback, and fingerprint tests without changing the existing v1 records.
3. Add the recoverable recommendation-graph transaction and lifecycle
   invariants to the store, including startup recovery tests.
4. Add the bounded rollup model adapter and stale-result/provider-budget
   checks; keep exact evaluation independently usable throughout.
5. Add the API fields, list/detail filtering, invalid-transition behavior, and
   desktop rollup card/handoff behavior.
6. Exercise migration, retention, failure, and prompt-safety acceptance tests
   before enabling the rollup pass by default.

## Implementation acceptance criteria

Before implementation is considered complete, tests must demonstrate:

- v1 observations and recommendations deserialize with defaults, while
  malformed records and unknown statuses produce diagnostics;
- old Peon and agent payloads without `problemArea` are accepted through the
  deterministic fallback, and invalid explicit fields are handled according
  to the validation contract;
- equivalent v2 problem areas produce the same fingerprint, while punctuation
  and materially different areas remain distinct;
- rollup input/output bounds, supplied-ID validation, same-target enforcement,
  duplicate-family rejection, overlapping-cluster rejection, and deterministic
  ordering are enforced;
- stale workspace instances, generations, provider identities, and evidence
  snapshot hashes cannot mutate recommendations;
- a same-member-set result updates in place, a changed set supersedes the old
  parent, and rerunning a committed result is idempotent;
- interrupted writes recover to either the complete new graph or the complete
  old graph, never a partially hidden graph;
- parent dismissal/terminal transitions, new evidence, session cleanup, and
  orphan scrubbing preserve the graph invariants and do not strand actionable
  evidence;
- the list/detail/action API and desktop types/rendering agree on parent,
  member, and `rolled_up` semantics; and
- provider timeout, unavailable budget, malformed output, prompt-size
  overflow, and untrusted evidence are isolated without dropping exact
  recommendations or observations.

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
- The evaluator and store must coordinate parent/member writes through a
  recoverable transaction in addition to the workspace lock.
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
