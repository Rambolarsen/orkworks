# Taskmaster recommendation audit (bulk review/cleanup) — design

Date: 2026-10-08
Status: draft for review
Owner decisions locked in session: one-click bulk approval; audit criteria
= under-current-eligibility + low-evidence noise + duplicate shapes + stale
inactivity; one-off manual trigger (re-runnable, no scheduler); merge =
dismiss duplicates and keep the newest card in each family; approval surfaces
as a persistent `cleanup` card in the recommendations panel.

## Problem

After the recurrence-floor change (PR #775), this workspace's recommendation
store still holds ~180 `proposed` high-priority cards left over from the old
single-observation escape hatch, plus assorted dismissed/rolled-up noise.
Only the user can dismiss today, one card at a time
(`POST /taskmaster/recommendations/:id/dismiss`). Taskmaster has no way to
audit its own backlog and propose bulk maintenance, so stale cards sit
forever and the panel stays noisy.

## Goals

- One deterministic audit pass over live Taskmaster recommendations that
  classifies each `proposed` card against explicit criteria and produces a
  single reviewable cleanup proposal.
- The user approves or dismisses the cleanup proposal with one click;
  Taskmaster then executes the bulk decision.
- No new scheduler, no autonomous mutation, no change to per-observation
  confidence gates or the improve_workflow evaluator's qualification rules.

## Non-goals

- No automatic (unapproved) dismissal — the existing "explicit user approval
  for every action" rule is preserved; the audit only ever *proposes*.
- No migration of already-terminal records (dismissed/completed/expired stay
  immutable history).
- Transport-level shape changes are limited to exactly these additive
  items: the `Cleanup` discriminant on `RecommendationType`, the optional
  `audit` field on `Recommendation`, the optional `reason` field on
  `DismissalWatermark`, and the accept route's `sessionId` becoming
  optional (type-validated). Nothing else in the shared contract changes.
- No scheduling; the audit runs only when triggered.

## The audit pass

New sidecar module `crates/orkworksd/src/taskmaster/audit.rs` with a pure,
deterministic function:

```
audit_recommendations(
    recommendations: &[Recommendation],
    workspace_id: &str,
    now: &str,
) -> Option<Recommendation>   // the cleanup card, or None
```

Trigger: `POST /taskmaster/audit/recommendations` (sidecar HTTP route, same
localhost port; no auth beyond the existing workspace-scoped sidecar binding).
Only one active (non-terminal) cleanup card may exist per workspace. Running
the audit while one is `proposed` replaces it in place (same id, refreshed
list and counts), like the evaluator updates a proposed family. Running it
after the previous card reached a terminal state creates a new generation
(`chain_id` = the cleanup dedupe family).

### Classification criteria

Each `proposed` card is classified by every criterion it matches; criteria
are independent and all labels are kept:

1. **`under_eligible`** — the card's cited evidence contains fewer than two
   distinct qualifying observations under the *current* eligibility rule
   (confidence ≥ `0.6`; a `reportedImpact: high` citation additionally
   requires confidence ≥ `0.8`). This is exactly the leftover class from the
   removed single-observation escape hatch.
2. **`noise`** — every cited observation is `source: peon` and none carries a
   `problemArea` (the deterministic proxy for the over-detection class
   AGENTS.md describes: title-like, artifact-free Peon inference). A card
   with any agent-reported citation or any problem area is never `noise`.
3. **`duplicate`** — the same dedupe family already contains a newer
   `proposed` card (keep-newest rule), or a terminal sibling
   (accepted/completed/dismissed/superseded/failed/expired) that the proposed
   card does not extend: the proposed card's `supersedesRecommendationId`
   (directly or through any chain of predecessors) references neither that
   sibling nor any record the sibling itself supersedes.
4. **`stale`** — the newest `observedAt` across the card's cited evidence is
   older than `STALE_AFTER` (constant, 14 days; no runtime config in v1).

Cards matching none of the criteria are `healthy` and are never proposed for
dismissal. Terminal records are never classified. Rollup parents (cards with
non-empty `rollupMemberIds`) are excluded from classification entirely —
dismissing one would strand its `RolledUp` members; they count toward
`scanned` but are never entries.

### The cleanup card

`RecommendationType` gains a `Cleanup` discriminant (serde `cleanup`). The
card:

- dedupe key: `cleanup:v1` (workspace-scoped; one family, generational via
  `chain_depth` like exact families).
- carries a new optional `audit` field on `Recommendation`
  (`skip_serializing_if = "Option::is_none"`): `{ entries: [{ id,
  criteria: [..], title }], scanned: usize, healthy: usize, staleAfterDays }`.
  It cites no workflow-observation evidence and fabricates no recurrences
  (`evidence: []`, `recurrenceCount: 0`).
- `WorkflowImprovement` remains a required field on every recommendation, so
  the cleanup card carries a placeholder value:   `targetSurface:
  Documentation`, `impact: medium`, empty observation/affected-session lists,
  and improvement text "Dismiss N proposed recommendations audited as
  under-eligible/noise/duplicate/stale." The placeholder is presentation
  only — no accept/dismiss flow reads it for a cleanup card.
- `requiresApproval: true` — unlike `improve_workflow`, this card mutates
  state on accept, so it must render the approval affordance.
- `priority` and `confidence` are informational (`medium`/`medium`); the
  reason string reports counts per criterion.
- The desktop panel renders it like any card but with the entry list
  (title + criteria badges) expandable, and its `Accept` action reads
  "Run cleanup" instead of "Fix with AI" (no prompt is submitted anywhere).

## Accept execution

The existing accept route `POST /taskmaster/recommendations/:id/accept`
becomes type-dispatched: the `sessionId` request field (currently required
for `improve_workflow`'s prompt delivery) becomes optional — `improve_workflow`
accepts require it present exactly as today, and cleanup accepts require it
absent (a present `sessionId` on a cleanup card is a parse rejection, not an
ignored field). The handler branches on the recommendation's type:

- Only a `proposed` cleanup card is acceptable. The bulk dismissal runs
  through the existing per-record CAS write discipline: each entry's
  `proposed` → `dismissed` transition is a compare-and-swap on its current
  status, so an entry changed between audit and approval fails its CAS, is
  skipped, and is reported in the accept response (`skipped: [{ id, status }]`)
  rather than aborting the batch.
- For every entry in the card's `audit.entries`: transition to `dismissed`
  via the existing `store::dismiss` path (guard widened to admit the
  `cleanup` target type — see below), with `dismissalWatermark` populated as
  a normal dismissal.
- Watermark lineage: `DismissalWatermark` gains an additive optional
  `reason` field (serde default `None`, never written by the ordinary user
  dismiss route) — the audit batch writes `audit:<criterion>@<cleanup-card-id>`
  (first matching criterion in deterministic order under-eligible → noise →
  duplicate → stale) so the lineage is auditable from either side. This is
  the only `DismissalWatermark` change.
- The cleanup card itself transitions `proposed` → `accepted` → `completed`
  inside the same batch, using the existing rollup transaction manifest
  (`store.rs` batch machinery) so either all entries plus the card's own
  lifecycle persist or none do. If persistence fails mid-batch, the manifest
  aborts and the cleanup card stays `proposed`.
- Dismissing the cleanup card discards the proposal; no other card changes.
  The `store::dismiss` type guard is widened from `ImproveWorkflow`-only to
  `ImproveWorkflow | Cleanup` so the card's own Dismiss action works; audited
  target cards are still dismissed through the same widened guard.

## Evaluator interaction

- `evaluate_workflow_improvements` and the rollup evaluator are untouched.
  The cleanup card is not an exact family: its dedupe key never collides
  with `improve_workflow:v1:` families, and evaluators ignore cards of type
  `cleanup` when scanning for active workflow recommendations.
- Dismissed-by-audit exact-family cards follow the existing dismissal-watermark
  resurfacing rules unchanged: new qualifying evidence after the watermark
  (impact increase, or two later observations including a new session) still
  creates a fresh proposed successor. The audit never blocks legitimate
  resurfacing; it only clears what no longer qualifies.
- `active_workflow_recommendation` (brain-analysis blocking) must ignore
  cleanup-type cards.

## Lifecycle and statuses

Existing statuses only: the cleanup card uses `proposed` → (`accepted` →
`completed` | `dismissed`). `executing` is not used by the audit (no prompt
delivery window). The store's type guards are widened for `Cleanup` on both
the accept and dismiss paths (see "Accept execution").

## Desktop UI

- `api.ts` / `taskmaster.ts`: `RecommendationType` union gains `cleanup`;
  origin classification in `recommendationOrigin` maps the `cleanup:v1`
  prefix to a third origin (`cleanup`) that the panel filter shows under
  "all" and its own filter chip; `alwaysVisible` handling unchanged.
- `RecommendationsPanel.tsx`: cleanup card rendering — counts summary,
  expandable entry list (title + criteria badges), `Run cleanup` /
  `Dismiss` actions; disabled accept while any entry list is empty (empty
  audit produces no card).
- The panel's existing status filter only shows `proposed` cleanup cards.

## Docs to update

- `specs/taskmaster.md` — new "Recommendation audit" section after the
  rollup section (authoritative contract: criteria, card shape, accept
  execution, immutability guarantees).
- `docs/agents/domain-entities.md` — `RecommendationType` vocabulary.
- `docs/agents/architecture.md` — route table row + one-paragraph summary.
- Root `AGENTS.md` metadata-protocol bullet list gains a one-line pointer.

## Testing

- Pure-function tests in `audit.rs`: each criterion (under-eligible leftover
  card, peon-noise card with problem area excluded, duplicate family, stale
  window, healthy card, terminal records skipped, criteria multiplicity).
- Store tests: accept executes bulk dismissal atomically via the batch
  manifest, skips concurrently-changed entries (CAS miss), aborts cleanly on
  mid-batch persistence failure, completes the cleanup card, one-active-card
  invariant, replace-in-place on re-audit, watermark `reason` lineage.
- HTTP tests: route returns the card; type-dispatched accept round-trips for
  cleanup (sessionId rejected when present) and improve_workflow (sessionId
  still required).
- Desktop: panel rendering test for the cleanup card shape.
- Evaluator tests: cleanup cards do not block brain analyses and are not
  treated as exact families.

## Verification

`cargo test --manifest-path crates/orkworksd/Cargo.toml`, desktop tests, and
`bash scripts/verify-repo.sh` per the scoped AGENTS instructions.
