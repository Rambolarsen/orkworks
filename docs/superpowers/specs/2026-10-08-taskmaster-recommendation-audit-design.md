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
- No new statuses, no transport-level `Recommendation` shape changes beyond
  the additions listed below.
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

Each proposed (and `executing`-reserved, if any) card is classified by every
criterion it matches; criteria are independent and all labels are kept:

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
   (accepted/completed/dismissed/superseded/failed/expired) whose successor
   lineage the proposed card does not extend.
4. **`stale`** — the newest `observedAt` across the card's cited evidence is
   older than `STALE_AFTER` (constant, 14 days; no runtime config in v1).

Cards matching none of the criteria are `healthy` and are never proposed for
dismissal. Terminal records are never classified.

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
- `requiresApproval: true` — unlike `improve_workflow`, this card mutates
  state on accept, so it must render the approval affordance.
- `priority` and `confidence` are informational (`medium`/`medium`); the
  reason string reports counts per criterion.
- `proposed_improvement` text: "Dismiss N proposed recommendations audited
  as under-eligible/noise/duplicate/stale."
- The desktop panel renders it like any card but with the entry list
  (title + criteria badges) expandable, and its `Accept` action reads
  "Run cleanup" instead of "Fix with AI" (no prompt is submitted anywhere).

## Accept execution

A new sidecar route: `POST /taskmaster/recommendations/:id/accept` already
exists for `improve_workflow` (session-scoped prompt delivery). The cleanup
card reuses the same route with type-specific execution:

- Only a `proposed` cleanup card is acceptable; accept is atomic within the
  existing recommendation store lock.
- For every entry in the card's `audit.entries` that still exists and is
  still `proposed`: transition to `dismissed` via the existing `store::dismiss`
  path, with `dismissalWatermark` populated as a normal dismissal and a
  recorded reason `audit:<criterion>` (first matching criterion, deterministic
  order under-eligible → noise → duplicate → stale). The audit card id is
  appended to the watermark's record as part of the reason string so the
  lineage is auditable from either side.
- Entries that changed state between audit and approval are skipped and
  reported in the accept response (`skipped: [{ id, status }]`); they are
  not errors.
- The cleanup card itself transitions to `accepted`, then `completed` via a
  synthetic completion recorded in the same transaction, so it leaves the
  active set immediately. If any entry dismiss fails mid-batch, the batch
  stops, already-dismissed entries stay dismissed, and the cleanup card
  rolls back to `proposed` (mirroring the improve_workflow accept rollback).
- Dismissing the cleanup card discards the proposal; no other card changes.

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
delivery window). Store gains a type-specific accept path guarded by
`RecommendationType`.

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
- Store tests: accept executes bulk dismissal atomically, skips
  concurrently-changed entries, rolls back on mid-batch failure, completes
  the cleanup card, one-active-card invariant, replace-in-place on re-audit.
- HTTP tests: route returns the card; accept round-trip.
- Desktop: panel rendering test for the cleanup card shape.
- Evaluator tests: cleanup cards do not block brain analyses and are not
  treated as exact families.

## Verification

`cargo test --manifest-path crates/orkworksd/Cargo.toml`, desktop tests, and
`bash scripts/verify-repo.sh` per the scoped AGENTS instructions.
