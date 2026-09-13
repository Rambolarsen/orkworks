# Taskmaster recommendation rollups

- Status: proposed
- Deciders: OrkWorks owner and Codex
- Date: 2026-09-13

## Context

Taskmaster's workflow-improvement evaluator currently creates one proposed
recommendation for each exact observation fingerprint. Exact fingerprints are
valuable deterministic audit units, but nearby descriptions can represent the
same underlying problem. Showing every exact recommendation independently can
therefore produce a noisy set of actionable cards.

The recommendation history and immutable workflow-observation evidence must be
retained. Any semantic grouping must also be bounded and must not let model
output invent evidence, identity, recurrence, source sessions, permissions,
or target surfaces.

## Decision

Taskmaster uses two layers of identity:

1. Exact evidence families group observations with the same structured problem
   identity and remain the durable, deterministic audit units.
2. Recommendation rollups combine related, currently proposed exact families
   into one actionable recommendation when bounded model output passes
   sidecar validation.

Peon workflow-observation candidates and agent reports may supply an optional
`problemArea`, a short neutral description of the recurring problem separate
from the user-visible `description`. New records use a versioned fingerprint
derived from `kind` and normalized `problemArea`. The sidecar owns validation
and normalization: Unicode NFKC, Unicode lowercase, trimmed edges, and every
run of Unicode whitespace collapsed to one ASCII space. Punctuation is
retained. The fingerprint is `v2:<kind>:<sha256-hex>` over
`kind + "\0" + canonical_problem_area`.

The field is bounded to a non-empty, non-control string of at most 120
characters. Explicit overlong values are truncated by the canonicalizer;
explicit empty or control-bearing values are rejected. Missing values remain a
compatible legacy input: the record keeps its v1 description-based
fingerprint, and any derived problem area is in-memory context only. Legacy
observations are not rewritten merely because they are read. Persisted and
wire values default to `null`.

The deterministic evaluator continues to apply the existing exact-family
rules: at least two distinct observations sharing a fingerprint, each with
confidence at least `0.6`, or one `high`-impact observation with confidence at
least `0.8`. The one-observation high-impact result remains one occurrence and
must not be described as recurring. A five-second debounce after the latest
accepted observation still allows a burst to be evaluated together.

After exact evaluation, a separate rollup pass considers only proposed exact
families. A configured Taskmaster model receives no raw terminal replay and
only receives bounded family snapshots: at most 32 families, three
representative observations per family (earliest, latest, and highest-impact
when distinct), 96 observations total, eight source-session IDs per family,
and 128 KiB serialized input. The model may return at most eight clusters of
two to eight supplied family IDs, with a title of at most 240 characters, a
summary of at most 1,000 characters, and a response of at most 64 KiB.

The sidecar validates every supplied ID, rejects empty or duplicate clusters,
overlapping families, invalid bounds, mismatched target surfaces, and any
generated text outside its bounds. Invalid output is rejected as a whole; no
partial rollup is applied. All cluster and member ordering is normalized
before identity is computed. The stable parent identity is
`rollup:<sha256-hex>` over sorted member recommendation IDs. Generated prose
does not determine identity or derived claims. If the provider is unavailable,
malformed, or stale, exact recommendations and observations remain unchanged.

`RecommendationStatus` gains `rolled_up`. A rollup parent stores sorted
`rollupMemberIds`, sorted `rollupMemberDedupeKeys`, `rollupGeneration`, and a
bounded projection of member evidence; a rolled-up member stores
`rolledUpBy`. The parent projection is limited to 64 evidence entries and 128
KiB. The sidecar computes the parent's evidence, recurrence count, affected
sessions, impact, confidence, and target surface from its members. In v1 all
members must share one target surface. A member can belong to at most one
active parent.

The parent/member transition is owned by the sidecar and written under the
workspace lock as one recoverable transaction. It stages all replacement JSON,
durable old-content backups, expected old-file hashes, and a fsynced manifest;
it fsyncs staged files, commits the manifest, publishes replacements, fsyncs
the recommendations directory, and removes the manifest only after complete
publication. Startup and recommendation reads recover unfinished manifests:
uncommitted transactions roll back and committed transactions complete. Reads
remain unavailable with a diagnostic if neither a complete old graph nor a
complete new graph can be established. After recovery, every rolled-up member
has exactly one existing parent, every active parent lists existing members,
and no family belongs to more than one active parent.

If membership changes, the old proposed parent becomes `superseded`; members
not assigned to the successor return to `proposed`, while assigned members
point only to the successor. A same-member-set proposed result updates in
place. Dismissed, accepted, completed, expired, failed, or otherwise terminal
recommendations are never silently merged. Their member records remain
historical `rolled_up` records and are not rendered as active cards. New
qualifying evidence after a terminal parent creates a new exact-family
generation and may later receive a new parent.

The normal recommendation list returns active rollup parents and proposed
exact families without an active parent. It never returns a `rolled_up` member
as an actionable card. Individual detail responses retain parent/member
relationships for audit and handoff, while actions on a rolled-up member use
the existing invalid-transition response. No autonomous session creation,
focus, terminal input, file edit, Git action, or acceptance is introduced.

## Consequences

Related exact families can become one actionable card without losing their
original IDs, fingerprints, evidence, or source sessions. New structured
problem areas make exact identity more stable while preserving readable v1
records. Rollup identity and all substantive claims remain sidecar-owned.

The recommendation schema and lifecycle store become more complex, and model
availability can affect only semantic grouping. The workspace lock and
recoverable graph transaction are required to prevent partially hidden
recommendations during membership changes or process failure.
