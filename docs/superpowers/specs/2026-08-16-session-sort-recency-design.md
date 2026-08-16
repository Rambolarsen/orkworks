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

This is a deliberate tradeoff, not a fix for the old symptom by another
means: pure recency does **not** surface action-needed sessions (capped,
blocked, failed) above more-recently-active working sessions. The old
"bump" rule let such sessions sit indefinitely below working sessions by
accident; the new rule lets them sit there by design when other sessions
are more recently active. The owner has confirmed this matches intent —
attention is surfaced by row badge and color, not by sort position.

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

`mergeSessionsById` becomes a **pure function returning a tuple**
`[SessionInfo[], Date]` — the next ordered list, and the next
`lastResortAt` value. It mutates no external state. This is required
for React 19 strict mode, which double-invokes state-updater functions:
a ref mutated inside an updater would diverge between the two
invocations (the first invocation writes a new `lastResortAt`, the
second reads it and throttles differently). Returning the next
`lastResortAt` from the pure function and writing it outside the
updater avoids that trap.

App.tsx holds the throttle state in two refs:

- `lastResortAtRef = useRef<Date>(new Date(0))` — epoch so the very
  first merge always re-sorts.
- `sessionsRef = useRef<SessionInfo[]>([])` — mirrored to `sessions`
  state via `useEffect(() => { sessionsRef.current = sessions; },
  [sessions])`, so async callbacks (`refreshSessions`,
  `handleConfirmNewSession`) can read the latest sessions without a
  functional `setSessions` updater.

Both merge call sites (refresh poll at line 116, session creation at
line 214) restructure from the current functional-updater form into:

```ts
const [next, nextLastResortAt] = mergeSessionsById(
  sessionsRef.current,
  incomingList,
  lastResortAtRef.current,
  new Date(),
);
sessionsRef.current = next;
lastResortAtRef.current = nextLastResortAt;
setSessions(next);
```

`setSessions(next)` is a **plain value** (not an updater function),
which strict mode never double-invokes. The two synchronous ref writes
after the compute run outside any React updater, so they don't risk
the trap.

**Merge logic (inside the pure function):**

- If `now - lastResortAt < 30s` **and** the incoming set of session IDs
  equals the existing set: merge updated fields into existing rows **in
  their existing visual order**. Do not re-sort. Return the same
  `lastResortAt` (unchanged) so subsequent merges keep throttling until
  the tick.
- Otherwise (≥30s elapsed, **or** the ID set changed, **or** the
  existing list is empty): do a full `sortSessions` pass and return
  `now` as the next `lastResortAt`.

An ID-set change (a new session appears in the polled snapshot, or an
existing session disappears from it) bypasses the throttle and triggers
a full re-sort immediately. Lifecycle transitions within an unchanged
ID set do *not* bypass the throttle — they update row fields in place
and wait for the 30s tick for any visual reordering. New sessions
naturally land near the top of a re-sort because their
`lastActivityTimestamp` falls back to `created_at = now`.

**Other `setSessions` call sites in App.tsx stay unchanged:**

- Line 163 (`setSessions([])` on workspace switch) is a plain reset.
- Line 309 (`setSessions((prev) => prev.map(s => s.id === id ? session : s))`,
  single-session replace on resume — `handleForgetSession` and
  `handleKillSession` go through `refreshSessions` and merge at line 116,
  not this site) doesn't go through merge. The next 2s poll reflects the
  resumed session's updated fields via the mirror effect (it's the same ID
  set, so the throttle preserves visual order on that merge); the resumed
  session's eventual rise to its recency-sorted position fires at the 30s
  tick per the throttle rule above. A transient race where
  `sessionsRef.current` is stale for one poll tick is bounded and
  self-corrects on the next merge.

### Removed code

From `sessionSort.ts`:

- `ATTENTION_PRIORITY` constant
- `isAtLeastOneMinuteOld` helper
- `newestUpdatedAliveSession` helper
- The bump-to-top block in `mergeSessionsById` (lines 60–71 of the current
  file). The full `mergeSessionsById` body lines 52–71 is rewritten by
  the Throttle section above — the partition into `alive`/`nonAlive`
  (lines 60–62) and the final return (line 71) are also restructured by
  the new tuple-returning form, not left stranded.

### Kept (unchanged scope)

- `needsAttention` and `sessionAttentionStatus` — the UI uses these for row
  badges and color. They are ordering-agnostic and stay.
- `apps/desktop/src/domain/session.ts` `sortSessions` — feeds a different
  view, unchanged in this change.
- Row badges, attention colors, the detail panel, and all non-sort behavior.

### Touched files

- `apps/desktop/src/sessionSort.ts` — comparator rewrite; `mergeSessionsById`
  signature change to a pure tuple `[SessionInfo[], Date]`; removed helpers.
- `apps/desktop/src/App.tsx` — add `lastResortAtRef` and `sessionsRef`
  (plus the mirroring effect) and restructure both `mergeSessionsById`
  call sites (refresh poll at line 116, session creation at line 214) per
  the Throttle section. The other two `setSessions` call sites (lines 163
  and 309) are unchanged.
- `apps/desktop/tests/sessionSort.test.ts` — see below.

### Tests

`sessionSort.test.ts` rewritten. All existing `mergeSessionsById` call
sites in the tests pick up the new tuple return and a `lastResortAt`
argument (default `new Date(0)` so the first call always re-sorts,
mirroring App.tsx's initial ref value).

- **Delete:** "sortSessions ranks actionable alive sessions before
  working, idle, and dead" — category ordering no longer exists.
- **Delete:** "mergeSessionsById does not promote a session that just
  became alive" and "mergeSessionsById promotes fresh activity only
  after the top session is quiet for one minute" — the bump rule is
  gone.
- **Keep verbatim:** `needsAttention` and `sessionAttentionStatus`
  behavior tests (those helpers are unchanged).
- **Keep, update regex:** the test asserting App.tsx combines a
  creation response with the current snapshot before merging (see
  Test-update note below for the new shape).
- **Keep, rename:** "sortSessions breaks attention ties by most recent
  activity, not label" → "sortSessions orders by most recent activity
  when timestamps differ". The "attention tie" framing no longer applies
  (there is no attention category to tie on); the test now verifies the
  primary sort key directly. Setup and assertion unchanged.
- **Keep verbatim:** "sortSessions uses recent output when it is newer
  than meaningful activity" — still pins the `max(lastOutputAt,
  lastActivityAt)` behavior of `lastActivityTimestamp`. Unchanged.
- **Keep verbatim:** "mergeSessionsById drops existing rows absent from
  an authoritative polling snapshot" — drops via the ID-set-change
  branch (full re-sort). Call signature updated; assertion unchanged.
- **Keep verbatim:** "mergeSessionsById drops every existing row when
  the polling snapshot is empty" — same family. Call signature updated;
  assertion unchanged.
- **Keep verbatim:** "mergeSessionsById sorts an initial polling
  snapshot deterministically" — empty existing list always re-sorts.
  Call signature updated; assertion unchanged.
- **Add:** throttle holds visual order for 29s under rapid data churn
  (multiple updates within the tick produce no re-sort); re-sort fires
  at 30s; a new session triggers an immediate re-sort (ID-set change
  bypasses throttle); an ID set change (session disappears from poll)
  triggers an immediate re-sort; full re-sort produces the deterministic
  recency order (newest activity first, ties to `created_at` desc,
  then `label` asc).

The redundant "initial-empty merge still sorts deterministically" Add
item from the prior draft of this spec is dropped — the behavior is
already covered verbatim by the existing test at
`tests/sessionSort.test.ts:95`.

**Test-update note on the regex test:** the App.tsx regex test at
`sessionSort.test.ts:105` asserts a `mergeSessionsById` invocation of
the form `mergeSessionsById(\1, [...\1, session])`. After this change
both call sites become `const [next, nextLastResortAt] = mergeSessionsById(…,
…, lastResortAtRef.current, new Date())`. The regex must be updated to
match the new shape; the test's intent (App.tsx still combines the
creation response with the current snapshot before merging) is
preserved by matching on the `sessionsRef.current` and
`[...sessionsRef.current, session]` arguments rather than the old
`prev`-based functional updater form.

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