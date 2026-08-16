# Session list sort by recency, throttled

Supersedes `2026-07-31-session-list-stability-design.md`, whose one-minute
"bump newest-updated alive session to top" rule produced inconsistent order:
sessions that needed action (capped / blocked / failed) could sit indefinitely
below working sessions because nothing in the merge path re-sorted past them.
The bump rule only ever moved one session per update, and only after the
current top was quiet for a full minute.

## Decision

Sort the visible session list by **a single key**: most recent activity,
descending. Drop attention-category ordering entirely from the comparator.
Throttle re-sorts so the list holds visually still between re-sort ticks,
eliminating the jumpiness that pure recency ordering would otherwise produce
on every polled update (every ~2s with chatty PTY output).

Order within the list is determined solely by `lastActivityTimestamp`
(`max(lastOutputAt, lastActivityAt)`, falling back to `peonLastInference`
then `created_at`). Ties break to `created_at` desc, then `label` asc.

## Implementation

### Sort key

`sessionSort.ts` `sortSessions` collapses to a three-key comparator:

1. `lastActivityTimestamp` descending
2. `created_at` descending
3. `label` ascending

No attention priority, no lifecycle grouping. The comparator is the only
ordering logic in this file.

### Throttle

`mergeSessionsById` gains a `lastResortAt: Date` parameter. App.tsx holds
`lastResortAt` in a `useRef` so the merge function stays pure. Merge logic:

- If `now - lastResortAt < 30s` **and** the incoming set of session IDs
  equals the existing set: merge updated fields into existing rows **in their
  existing visual order**. Do not re-sort. Update `lastResortAt` only on a
  re-sort.
- Otherwise (≥30s elapsed, **or** the ID set changed, **or** the existing
  list is empty): do a full `sortSessions` pass and set `lastResortAt = now`.

An ID-set change (a new session appears in the polled snapshot, or an
existing session disappears from it) bypasses the throttle and triggers a
full re-sort immediately. Lifecycle transitions within an unchanged ID set
do *not* bypass the throttle — they update row fields in place and wait
for the 30s tick for any visual reordering. New sessions naturally land
near the top of a re-sort because their `lastActivityTimestamp` falls back
to `created_at = now`.

### Removed code

From `sessionSort.ts`:

- `ATTENTION_PRIORITY` constant
- `isAtLeastOneMinuteOld` helper
- `newestUpdatedAliveSession` helper
- The bump-to-top block in `mergeSessionsById` (lines 60–71 of the current
  file)

### Kept (unchanged scope)

- `needsAttention` and `sessionAttentionStatus` — the UI uses these for row
  badges and color. They are ordering-agnostic and stay.
- `apps/desktop/src/domain/session.ts` `sortSessions` — feeds a different
  view, unchanged in this change.
- Row badges, attention colors, the detail panel, and all non-sort behavior.

### Touched files

- `apps/desktop/src/sessionSort.ts` — comparator rewrite; `mergeSessionsById`
  signature change; removed helpers.
- `apps/desktop/src/App.tsx` — add `lastResortAtRef` and pass it through to
  both `mergeSessionsById` call sites (refresh poll at line 116, session
  creation at line 214).
- `apps/desktop/tests/sessionSort.test.ts` — see below.

### Tests

`sessionSort.test.ts` rewritten:

- **Delete:** "sortSessions ranks actionable alive sessions before working,
  idle, and dead" — category ordering no longer exists.
- **Delete:** "mergeSessionsById does not promote a session that just became
  alive" and "mergeSessionsById promotes fresh activity only after the top
  session is quiet for one minute" — the bump rule is gone.
- **Keep verbatim:** `needsAttention` and `sessionAttentionStatus` behavior
  tests (those helpers are unchanged).
- **Keep:** the regex test asserting App.tsx combines a creation response
  with the current snapshot before merging.
- **Add:** throttle holds visual order for 29s under rapid data churn;
  re-sort fires at 30s; a new session triggers an immediate re-sort; an ID
  set change (session disappears from poll) triggers an immediate re-sort;
  full re-sort produces the deterministic recency order; initial-empty merge
  still sorts deterministically.

## Why 30s

30s is longer than a human's typical dwell time on a single row but short
enough that an actively-working session converges to the top within one
attention cycle. A shorter tick (e.g. 10s) reintroduces visible jitter; a
longer tick (e.g. 60s) lets the list lag too far behind where activity
actually is. 30s is the first guess; the `lastResortAt` interval is a single
named constant, tunable in one place without touching the comparator.

## Scope

Single change to the renderer's session sort and merge. No setting, no
persistence, no sidecar/API, no manual ordering, no `domain/session.ts`
sort. No ADR — this is a UX behavior tweak inside an already-decided
component, not a new architectural boundary. The prior design doc
(`2026-07-31-session-list-stability-design.md`) stays in place as
historical record; this one supersedes its rule.