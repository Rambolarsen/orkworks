# Session list sort by recency, throttled — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the session list's attention-category sort with a single-key recency sort, throttled to 30s, so the list visually holds still between re-sort ticks.

**Architecture:** `sortSessions` becomes a 3-key comparator (`lastActivityTimestamp` desc → `created_at` desc → `label` asc). `mergeSessionsById` becomes a pure function returning `[SessionInfo[], Date]` — the next ordered list plus the next `lastResortAt`. App.tsx holds `lastResortAt` and a `sessions` mirror in `useRef`s, calls `mergeSessionsById` from outside the `setSessions` updater, and writes the plain value back. This avoids the React 19 strict-mode double-invoke trap that ref-mutation-inside-an-updater would hit.

**Tech Stack:** TypeScript, React 19, Vite, Electron, `node:test` + `node:assert/strict`.

## Global Constraints

- **Branch + PR required.** Every touched file is under `apps/desktop/` (`apps/desktop/src/sessionSort.ts`, `apps/desktop/src/App.tsx`, `apps/desktop/tests/sessionSort.test.ts`). Per repo `AGENTS.md`, this work goes on a dedicated branch (or worktree if parallel agents are running), not directly on `main`. Invoke the `starting-work` skill (under repo `skills/`) before Task 1 to create the working branch/worktree. The plan document itself is docs-only and was committed to `main` directly.
- **Renderer/electron boundary stays intact.** `src/` only. No edits to `electron/`.
- **TDD.** Every step writes/updates a test first, runs it to confirm red, then changes production code, runs again to confirm green.
- **Validation commands** (run from `apps/desktop/`):
  - Type check: `npx tsc --noEmit`
  - Tests: `node --experimental-strip-types --test tests/sessionSort.test.ts`
  - Full test suite (final gate): `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
- **Pure function contract for `mergeSessionsById`.** Returns a tuple, mutates no external state. App.tsx owns the ref writes outside the React state updater.
- **30s throttle** is a single named constant `THROTTLE_MS = 30_000` in `sessionSort.ts`. Tunable in one place without touching the comparator.
- **No comments in production code** unless an existing comment is being preserved verbatim.

## Spec

This plan implements `docs/superpowers/specs/2026-08-16-session-sort-recency-design.md` (the spec is the source of truth; read it before starting). It supersedes `docs/superpowers/specs/2026-07-31-session-list-stability-design.md`.

### File structure

| File | Responsibility |
| ---- | --------------- |
| `apps/desktop/src/sessionSort.ts` | Recency comparator + pure tuple-returning merge with throttle. |
| `apps/desktop/src/App.tsx` | Two `useRef` declarations, one mirroring `useEffect`, two restructured `mergeSessionsById` call sites. No other `setSessions` site changes. |
| `apps/desktop/tests/sessionSort.test.ts` | Pin the new behavior; delete obsolete category / bump-rule tests; update call signatures; add throttle tests; update the App.tsx regex test. |

---

### Task 1: Rewrite `sortSessions` to single-key recency

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts:4-12` (remove `ATTENTION_PRIORITY`), `apps/desktop/src/sessionSort.ts:32-45` (rewrite `sortSessions`)
- Test: `apps/desktop/tests/sessionSort.test.ts:49` (delete), `apps/desktop/tests/sessionSort.test.ts:60` (rename)
- Test: `apps/desktop/tests/sessionSort.test.ts` (add new test pinning recency-only behavior)

**Interfaces:**
- Consumes: `lastActivityTimestamp` from `apps/desktop/src/labels.ts:226` (unchanged).
- Produces: `sortSessions(list: SessionInfo[]): SessionInfo[]` — signature unchanged; behavior changes to 3-key recency comparator. `mergeSessionsById` (this task) still uses `sortSessions` at line 52 in its initial-empty branch — TypeScript signature stays compatible.

- [ ] **Step 1: Add the failing recency-only test**

In `apps/desktop/tests/sessionSort.test.ts`, insert this test after the existing "sortSessions ranks actionable alive sessions before working, idle, and dead" test at line 49:

```ts
test("sortSessions orders purely by lastActivityTimestamp descending, ignoring attention", () => {
  const olderButNeedsYou = {
    ...session("older-needs-you", "alive", "needs_you"),
    lastActivityAt: "2026-08-01T10:00:00.000Z",
  };
  const newerButIdle = {
    ...session("newer-idle", "alive", "idle"),
    lastActivityAt: "2026-08-01T11:00:00.000Z",
  };

  const ordered = sortSessions([olderButNeedsYou, newerButIdle]);

  assert.deepEqual(ordered.map((item) => item.id), ["newer-idle", "older-needs-you"]);
});
```

- [ ] **Step 2: Run the new test — verify it fails**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts` (from repo root; or `cd apps/desktop && node --experimental-strip-types --test tests/sessionSort.test.ts`)
Expected: FAIL. The current comparator puts `needs_you` above `idle` via `ATTENTION_PRIORITY`, so the assertion order (`["newer-idle", "older-needs-you"]`) reverses.

- [ ] **Step 3: Rewrite `sortSessions` and remove `ATTENTION_PRIORITY`**

In `apps/desktop/src/sessionSort.ts`:

1. Delete lines 4–12 (the `const ATTENTION_PRIORITY: Record<string, number> = { … };` block).
2. Replace the body of `sortSessions` (lines 32–45) with:

```ts
export function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const ta = Date.parse(lastActivityTimestamp(a) ?? "");
    const tb = Date.parse(lastActivityTimestamp(b) ?? "");
    if (!Number.isNaN(ta) && !Number.isNaN(tb) && ta !== tb) return tb - ta;
    const ca = Date.parse(a.created_at ?? "");
    const cb = Date.parse(b.created_at ?? "");
    if (!Number.isNaN(ca) && !Number.isNaN(cb) && ca !== cb) return cb - ca;
    return a.label.localeCompare(b.label);
  });
}
```

- [ ] **Step 4: Run the new test — verify it passes**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: The new test PASSES. The existing test "sortSessions ranks actionable alive sessions before working, idle, and dead" (line 49) now FAILS — that's expected: those sessions have no `lastActivityAt`, fall back to `created_at: "now"` (identical for all), then break ties by `label` asc, producing alphabetical order which doesn't match the test's expected `["needs-you", "failed", "working", "idle", "dead"]`. We delete that test next.

- [ ] **Step 5: Delete the obsolete attention-category test**

In `apps/desktop/tests/sessionSort.test.ts`, delete the entire test "sortSessions ranks actionable alive sessions before working, idle, and dead" (currently lines 49–58).

- [ ] **Step 6: Rename the tie-break test**

In `apps/desktop/tests/sessionSort.test.ts`, rename the test "sortSessions breaks attention ties by most recent activity, not label" (currently around line 60) to "sortSessions orders by most recent activity when timestamps differ". The test setup and assertion stay unchanged — the assertion is already correct under the new recency comparator (newer `lastActivityAt` first).

- [ ] **Step 7: Run the full test file — verify all pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: PASS for every test in the file (including the kept `needsAttention`, `sessionAttentionStatus`, `sortSessions uses recent output when it is newer`, and all `mergeSessionsById` tests — those still pass because the merge path's bump rule still uses `sortSessions`'s recency order via its initial-empty branch, and the bump rule's "don't promote" path doesn't depend on the removed `ATTENTION_PRIORITY`).

- [ ] **Step 8: Type-check**

Run: `npx tsc --noEmit` (from `apps/desktop/`)
Expected: No errors. Verify that removing `ATTENTION_PRIORITY` didn't leave any references in `sessionSort.ts` (it was only used inside `sortSessions`).

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/src/sessionSort.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "refactor(session-sort): order sessions by recency only

Drop attention-category ordering from the sortSessions comparator.
Order is now lastActivityTimestamp desc, then created_at desc, then
label asc. Removes ATTENTION_PRIORITY constant; the UI still surfaces
attention via row badges and color, not via sort position.

Deletes the 'ranks actionable alive sessions before working, idle, and
dead' test (the removed behavior) and renames the former attention-tie
break test to reflect the new single-key order. Adds a test pinning
recency-over-attention behavior."
```

---

### Task 2: Transform `mergeSessionsById` to pure tuple-returning form (always re-sort — no throttle yet)

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts:47-72` (rewrite `mergeSessionsById` body), `apps/desktop/src/sessionSort.ts:74-102` (delete `isAtLeastOneMinuteOld` and `newestUpdatedAliveSession`)
- Test: `apps/desktop/tests/sessionSort.test.ts` — update tests at lines 80, 91, 95 to destructure tuple; delete tests at lines 111, 126; add a new test pinning the tuple return for an initial-empty merge.

**Interfaces:**
- Produces: `mergeSessionsById(existing: readonly SessionInfo[], incoming: readonly SessionInfo[], lastResortAt: Date = new Date(0), now: Date = new Date()): [SessionInfo[], Date]` — pure function returning `[nextSessions, nextLastResortAt]`. The third param's default is `new Date(0)` (epoch) so the very first merge always re-sorts; the fourth param defaults to `new Date()`. Old 2-arg calls still typecheck via the defaults, but their old single-return-shape consumers must destructure.
- Consumes: `sortSessions` from Task 1.

- [ ] **Step 1: Add the failing tuple-return test**

In `apps/desktop/tests/sessionSort.test.ts`, add this test (place it right before the existing "mergeSessionsById drops existing rows absent..." test, currently around line 80):

```ts
test("mergeSessionsById returns a [list, nextLastResortAt] tuple for an initial empty list, with nextLastResortAt === now", () => {
  const now = new Date("2026-08-15T12:00:00.000Z");
  const lastResortAt = new Date("2026-08-15T11:59:55.000Z"); // 5s before now
  const incoming = [session("a", "alive"), session("b", "alive")];

  const [merged, nextLastResortAt] = mergeSessionsById([], incoming, lastResortAt, now);

  assert.equal(merged.length, 2);
  assert.equal(nextLastResortAt, now); // referential equality; initial-empty branch always re-sorts and returns now
});
```

- [ ] **Step 2: Run — verify red**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: FAIL. The new test fails because the current `mergeSessionsById` returns `SessionInfo[]` (not a tuple) and its third param is named `now`, not `lastResortAt` — destructuring produces `[merged, undefined]` and `assert.equal(merged.length, 2)` would actually pass on the array's `length` against undefined-wrapper behavior; the cleaner assertion `assert.equal(nextLastResortAt, now)` is what fails clearly. Other existing tests (80, 91, 95) also start to fail at the assertion `merged.map((item) => item.id)` because `merged` is now `SessionInfo[]` in their destructuring `const [merged] = mergeSessionsById(...)` — but they haven't been updated yet, so they keep passing until we update them. The red signal for this step is specifically the new test.

- [ ] **Step 3: Rewrite `mergeSessionsById` to a pure tuple-returning form (always re-sort, no throttle yet)**

In `apps/desktop/src/sessionSort.ts`, replace lines 47–102 (the existing `mergeSessionsById`, `isAtLeastOneMinuteOld`, and `newestUpdatedAliveSession`) with:

```ts
export function mergeSessionsById(
  existing: readonly SessionInfo[],
  incoming: readonly SessionInfo[],
  lastResortAt: Date = new Date(0),
  now: Date = new Date(),
): [SessionInfo[], Date] {
  if (existing.length === 0) {
    return [sortSessions([...incoming]), now];
  }
  const existingIds = new Set(existing.map((session) => session.id));
  const incomingMap = new Map(incoming.map((session) => [session.id, session]));
  const updated = existing.map((session) => incomingMap.get(session.id) ?? session);
  const added = [...incomingMap.values()].filter((session) => !existingIds.has(session.id));
  return [sortSessions([...updated, ...added]), now];
}
```

Always-return-`now` (no throttle yet) — the throttle lands in Task 3. `isAtLeastOneMinuteOld` and `newestUpdatedAliveSession` are now removed; no other code references them.

- [ ] **Step 4: Update the existing `mergeSessionsById` tests to destructure the tuple**

In `apps/desktop/tests/sessionSort.test.ts`, update three tests to destructure the tuple return. The assertions are unchanged; only the call shape changes.

For "mergeSessionsById drops existing rows absent from an authoritative polling snapshot" (currently around line 80):

Before:
```ts
const merged = mergeSessionsById([existing, polledNew], [createdNew]);
```
After:
```ts
const [merged] = mergeSessionsById([existing, polledNew], [createdNew]);
```

For "mergeSessionsById drops every existing row when the polling snapshot is empty" (currently around line 91):

Before:
```ts
assert.deepEqual(mergeSessionsById([session("forgotten", "dead")], []), []);
```
After:
```ts
const [merged] = mergeSessionsById([session("forgotten", "dead")], []);
assert.deepEqual(merged, []);
```

For "mergeSessionsById sorts an initial polling snapshot deterministically" (currently around line 95):

Before:
```ts
assert.deepEqual(mergeSessionsById([], incoming), sortSessions(incoming));
```
After:
```ts
const [merged] = mergeSessionsById([], incoming);
assert.deepEqual(merged, sortSessions(incoming));
```

- [ ] **Step 5: Run — verify the destructive-regex test passes; the bump-rule tests still need fixing**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: The tuple-return test, the drop tests, and the initial snapshot test now PASS. The two bump-rule tests at lines 111 and 126 FAIL — they assert the old "no promotion" and "promotion after 1 minute" behavior, which no longer exists (we always re-sort, so `starting` with newer `lastOutputAt` jumps to the top in test 111, and `updated` jumps in the first assertion of test 126).

- [ ] **Step 6: Delete the two obsolete bump-rule tests**

In `apps/desktop/tests/sessionSort.test.ts`, delete the entire test "mergeSessionsById does not promote a session that just became alive" (currently around lines 111–124) and the entire test "mergeSessionsById promotes fresh activity only after the top session is quiet for one minute" (currently around lines 126–146).

- [ ] **Step 7: Run the full test file — verify all pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: all tests PASS.

- [ ] **Step 8: Type-check**

Run: `npx tsc --noEmit` (from `apps/desktop/`)
Expected: FAIL. The current `App.tsx` calls `mergeSessionsById` at lines 116 and 214 in the form `setSessions((prev) => mergeSessionsById(prev, …))` — `mergeSessionsById` now returns a tuple, so the `setSessions` updater would store a tuple in `SessionInfo[]` state. That's a real type mismatch we'll fix in Task 4. Note this as the predicted failure but DO NOT modify App.tsx in this task; the fix is its own task. Restated: this TypeScript error is expected to persist until Task 4 lands. Do not commit if tsc reports OTHER errors (those would be unanticipated).

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/src/sessionSort.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "refactor(session-sort): mergeSessionsById returns [list, nextLastResortAt]

Pure tuple-returning form so React 19 strict-mode double-invocation of
the state updater can't diverge via ref mutation. Always re-sorts in
this commit (no throttle yet); throttle lands in the next commit.

Removes isAtLeastOneMinuteOld and newestUpdatedAliveSession helpers —
the bump-to-top rule they implemented is gone; the list converges to
recency order naturally via sortSessions.

Updates three existing mergeSessionsById tests to destructure the
tuple (assertions unchanged). Deletes the two bump-rule tests
('does not promote a session that just became alive' and 'promotes
fresh activity only after the top session is quiet for one minute')
whose assertions no longer hold under always-re-sort."
```

---

### Task 3: Add the 30s throttle (with ID-set-change bypass)

**Files:**
- Modify: `apps/desktop/src/sessionSort.ts` (add `THROTTLE_MS` constant + throttle branch in `mergeSessionsById`)
- Test: `apps/desktop/tests/sessionSort.test.ts` (add 4 new throttle tests)

**Interfaces:**
- Produces: `mergeSessionsById` signature is unchanged from Task 2 — still `(existing, incoming, lastResortAt = new Date(0), now = new Date()): [SessionInfo[], Date]`. Behavior changes only internally: the throttle branch preserves existing visual order and returns the unchanged `lastResortAt`.
- Consumes: `THROTTLE_MS = 30_000` constant (new, local to this file).

- [ ] **Step 1: Add the four failing throttle tests**

In `apps/desktop/tests/sessionSort.test.ts`, append these four tests at the end of the file:

```ts
test("mergeSessionsById throttle holds visual order for 29s under rapid data churn", () => {
  const start = new Date("2026-08-15T12:00:00.000Z");
  const a = { ...session("a", "alive", "working"), lastActivityAt: "2026-08-15T12:00:00.000Z" };
  const b = { ...session("b", "alive", "working"), lastActivityAt: "2026-08-15T11:00:00.000Z" };

  const [first, lastResortAt1] = mergeSessionsById([], [a, b], new Date(0), start);
  assert.deepEqual(first.map((item) => item.id), ["a", "b"]);

  // 29s later, b emits output that would otherwise move it to top
  const churnedB = { ...b, lastActivityAt: "2026-08-15T12:00:29.000Z" };
  const [second, lastResortAt2] = mergeSessionsById(
    first,
    [churnedB, a],
    lastResortAt1,
    new Date("2026-08-15T12:00:29.000Z"),
  );
  assert.deepEqual(second.map((item) => item.id), ["a", "b"]); // visual order preserved
  assert.equal(lastResortAt2, lastResortAt1); // throttle unchanged
  assert.equal(second.find((item) => item.id === "b")?.lastActivityAt, "2026-08-15T12:00:29.000Z");
});

test("mergeSessionsById throttle fires a full re-sort at 30s", () => {
  const start = new Date("2026-08-15T12:00:00.000Z");
  const a = { ...session("a", "alive", "working"), lastActivityAt: "2026-08-15T12:00:00.000Z" };
  const b = { ...session("b", "alive", "working"), lastActivityAt: "2026-08-15T11:00:00.000Z" };

  const [first, lastResortAt1] = mergeSessionsById([], [a, b], new Date(0), start);

  const churnedB = { ...b, lastActivityAt: "2026-08-15T12:00:30.000Z" };
  const at30s = new Date("2026-08-15T12:00:30.000Z");
  const [second, lastResortAt2] = mergeSessionsById(first, [churnedB, a], lastResortAt1, at30s);
  assert.deepEqual(second.map((item) => item.id), ["b", "a"]); // re-sort fires
  assert.equal(lastResortAt2, at30s);
});

test("mergeSessionsById bypasses throttle when a new session appears in the poll", () => {
  const start = new Date("2026-08-15T12:00:00.000Z");
  const a = { ...session("a", "alive", "working"), lastActivityAt: "2026-08-15T11:00:00.000Z" };

  const [first, lastResortAt1] = mergeSessionsById([], [a], new Date(0), start);

  const b = { ...session("b", "alive", "working"), lastActivityAt: "2026-08-15T12:00:05.000Z" };
  const at5s = new Date("2026-08-15T12:00:05.000Z"); // throttle not expired
  const [second] = mergeSessionsById(first, [a, b], lastResortAt1, at5s);
  assert.deepEqual(second.map((item) => item.id), ["b", "a"]); // ID-set change re-sorts
});

test("mergeSessionsById bypasses throttle when a session disappears from the poll", () => {
  const start = new Date("2026-08-15T12:00:00.000Z");
  const a = { ...session("a", "alive", "working"), lastActivityAt: "2026-08-15T11:00:00.000Z" };
  const b = { ...session("b", "alive", "working"), lastActivityAt: "2026-08-15T12:00:00.000Z" };

  const [first, lastResortAt1] = mergeSessionsById([], [a, b], new Date(0), start);

  const at5s = new Date("2026-08-15T12:00:05.000Z"); // throttle not expired
  const [second] = mergeSessionsById(first, [a], lastResortAt1, at5s);
  assert.deepEqual(second.map((item) => item.id), ["a"]); // dropped + re-sorted
});
```

- [ ] **Step 2: Run — verify red**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: FAIL on all 4 new tests. The current `mergeSessionsById` (Task 2's always-re-sort implementation) re-sorts on every call, so the "holds visual order for 29s" test's assertion `["a", "b"]` reverses to `["b", "a"]`; the 30s test passes already (re-sort always happens) but its `lastResortAt2 === at30s` assertion would also pass since `now` is always returned. Predicted red: at least the 29s test and the new-session-appears test fail.

- [ ] **Step 3: Add the throttle branch to `mergeSessionsById`**

In `apps/desktop/src/sessionSort.ts`, above the `mergeSessionsById` declaration, add the constant:

```ts
const THROTTLE_MS = 30_000;
```

Replace the body of `mergeSessionsById` with:

```ts
export function mergeSessionsById(
  existing: readonly SessionInfo[],
  incoming: readonly SessionInfo[],
  lastResortAt: Date = new Date(0),
  now: Date = new Date(),
): [SessionInfo[], Date] {
  if (existing.length === 0) {
    return [sortSessions([...incoming]), now];
  }
  const existingIds = new Set(existing.map((session) => session.id));
  const incomingMap = new Map(incoming.map((session) => [session.id, session]));
  const incomingIds = new Set(incomingMap.keys());
  const idsChanged =
    existingIds.size !== incomingIds.size ||
    [...existingIds].some((id) => !incomingIds.has(id));
  const updated = existing.map((session) => incomingMap.get(session.id) ?? session);
  const added = [...incomingMap.values()].filter((session) => !existingIds.has(session.id));
  if (idsChanged || now.getTime() - lastResortAt.getTime() >= THROTTLE_MS) {
    return [sortSessions([...updated, ...added]), now];
  }
  return [updated, lastResortAt];
}
```

The throttle branch preserves existing visual order (returns `updated` in existing order) and returns the unchanged `lastResortAt` so subsequent merges keep throttling until a tick or an ID-set change.

- [ ] **Step 4: Run — verify all 4 new tests pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: all tests in the file PASS (4 new + all previously-passing tests).

- [ ] **Step 5: Type-check**

Run: `npx tsc --noEmit` (from `apps/desktop/`)
Expected: FAIL only on `apps/desktop/src/App.tsx` at the two `mergeSessionsById` call sites (lines 116 and 214) — the same Task-2-pending App.tsx type mismatch. This is expected; Task 4 fixes it.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/sessionSort.ts apps/desktop/tests/sessionSort.test.ts
git commit -m "feat(session-sort): throttle re-sorts to 30s, bypass on ID-set change

When the incoming ID set equals the existing set and <30s has elapsed
since the last re-sort, mergeSessionsById preserves existing visual
order and returns the unchanged lastResortAt. An ID-set change (new
session appears or existing session disappears) bypasses the throttle
and triggers an immediate re-sort. Initial-empty always re-sorts.

THROTTLE_MS is a single named constant tunable without touching the
comparator.

Adds 4 tests: throttle holds at 29s; throttle fires at 30s; new
session bypasses; disappeared session bypasses."
```

---

### Task 4: Wire App.tsx to the tuple-returning `mergeSessionsById`

**Files:**
- Modify: `apps/desktop/src/App.tsx:49-50` (add the two refs near the existing refs), `apps/desktop/src/App.tsx` (add the mirror `useEffect`), `apps/desktop/src/App.tsx:116` (restructure the refresh-poll merge), `apps/desktop/src/App.tsx:214` (restructure the session-creation merge)
- Test: `apps/desktop/tests/sessionSort.test.ts:105-109` (update the regex test)

**Interfaces:**
- Consumes: `mergeSessionsById` from Task 3 (tuple return).
- Produces: App.tsx's two refs (`lastResortAtRef`, `sessionsRef`), one mirror `useEffect`, and two restructured merge sites. The mirror keeps `sessionsRef.current` synchronized with the `sessions` state so async callbacks can read the latest sessions without a functional `setSessions` updater.

- [ ] **Step 1: Update the regex test first (TDD: red)**

In `apps/desktop/tests/sessionSort.test.ts`, replace the body of the test "App combines a creation response with the current snapshot before merging" (currently lines 105–109) with:

```ts
test("App combines a creation response with the current snapshot before merging", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  // Ref declarations required for the new pure-function shape
  assert.match(source, /const\s+lastResortAtRef\s*=\s*useRef<Date>\s*\(\s*new Date\s*\(\s*0\s*\)\s*\)/);
  assert.match(source, /const\s+sessionsRef\s*=\s*useRef<SessionInfo\[\]>\s*\(\s*\[\s*\]\s*\)/);
  // The mirror effect keeps sessionsRef.current in sync with the sessions state
  assert.match(source, /useEffect\s*\(\s*\(\s*\)\s*=>\s*\{\s*sessionsRef\.current\s*=\s*sessions\s*;\s*\}\s*,\s*\[\s*sessions\s*\]\s*\)/);
  // The creation-path merge site uses the tuple-destructure pattern with refs and new Date()
  assert.match(
    source,
    /const\s+\[\s*\w+\s*,\s*\w+\s*\]\s*=\s*mergeSessionsById\s*\(\s*sessionsRef\.current\s*,\s*\[\s*\.\.\.sessionsRef\.current\s*,\s*session\s*\]\s*,\s*lastResortAtRef\.current\s*,\s*new Date\s*\(\s*\)\s*\)/,
  );
});
```

- [ ] **Step 2: Run the regex test — verify red**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: this test FAILS. App.tsx currently uses `setSessions((prev) => mergeSessionsById(prev, [...prev, session]))` and has no `lastResortAtRef` or `sessionsRef`, so none of the four regexes match.

- [ ] **Step 3: Add the two ref declarations in App.tsx**

In `apps/desktop/src/App.tsx`, find the block of refs around line 49–50 (currently `dockviewApiRef` and `sessionsHiddenLayoutRef`). Add these two new refs immediately after `sessionsHiddenLayoutRef`:

```ts
  const lastResortAtRef = useRef<Date>(new Date(0));
  const sessionsRef = useRef<SessionInfo[]>([]);
```

`useRef` is already imported at line 1; `SessionInfo` is already imported at line 14. No new imports needed.

- [ ] **Step 4: Add the mirror `useEffect` to keep `sessionsRef` in sync**

In `apps/desktop/src/App.tsx`, immediately after the new refs (or in the next natural `useEffect` slot — e.g., right after the existing renderer-health effect block ends on line 77), add:

```ts
  useEffect(() => {
    sessionsRef.current = sessions;
  }, [sessions]);
```

`useEffect` is already imported at line 1. This effect runs after every commit where `sessions` changed, so the ref always reflects the latest committed state — including the unchanged `setSessions([])` at line 163 and the `setSessions((prev) => prev.map(…))` at line 309.

- [ ] **Step 5: Restructure the refresh-poll merge site (line 116)**

In `apps/desktop/src/App.tsx`, find the `refreshSessions` callback (around lines 111–123). Replace:

```ts
      setSessions((previous) => mergeSessionsById(previous, list));
```

with:

```ts
      const [next, nextLastResortAt] = mergeSessionsById(
        sessionsRef.current,
        list,
        lastResortAtRef.current,
        new Date(),
      );
      sessionsRef.current = next;
      lastResortAtRef.current = nextLastResortAt;
      setSessions(next);
```

`refreshSessions` is wrapped in `useCallback(async () => { … }, [])`. The new body still returns `true`/`false` and runs in `useCallback` scope — verify the existing `try` / `return true` / `catch` structure is intact after the edit.

- [ ] **Step 6: Restructure the session-creation merge site (line 214)**

In `apps/desktop/src/App.tsx`, find the `handleConfirmNewSession` callback (around lines 209–225). Replace:

```ts
        setSessions((prev) => mergeSessionsById(prev, [...prev, session]));
```

with:

```ts
        const [next, nextLastResortAt] = mergeSessionsById(
          sessionsRef.current,
          [...sessionsRef.current, session],
          lastResortAtRef.current,
          new Date(),
        );
        sessionsRef.current = next;
        lastResortAtRef.current = nextLastResortAt;
        setSessions(next);
```

The regex test (Step 1) targets this creation site specifically — it expects `sessionsRef.current` and `[...sessionsRef.current, session]`.

- [ ] **Step 7: Run the regex test — verify green**

Run: `node --experimental-strip-types --test apps/desktop/tests/sessionSort.test.ts`
Expected: the regex test now PASSES.

- [ ] **Step 8: Type-check the full project**

Run: `npx tsc --noEmit` (from `apps/desktop/`)
Expected: PASS. The App.tsx type mismatches from Tasks 2 and 3 are now resolved by the tuple destructure + plain-value `setSessions(next)`. No new type errors introduced by the new refs or the mirror effect.

- [ ] **Step 9: Run the full desktop test suite**

Run: `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` (from `apps/desktop/`)
Expected: PASS for all test files. In particular, `sessionProviderContext.test.ts`, `sessionPolling.test.ts`, `sessionGroups.test.ts`, and `sessionUnread.test.ts` should all pass — these are neighboring test files that assert App.tsx-adjacent behavior. If any regress, the issue is likely the `sessionsRef` mirror not seeing the `setSessions([])` reset at line 163 or the `prev.map` replace at line 309 — both are covered by the mirror effect added in Step 4 (`useEffect(() => { sessionsRef.current = sessions; }, [sessions])`), so the next render's effect should restore sync. Investigate any failure before proceeding.

- [ ] **Step 10: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/tests/sessionSort.test.ts
git commit -m "refactor(app): wire session merge to tuple return via refs

App.tsx now holds lastResortAt and a sessions mirror in useRef so the
merge function stays pure (React 19 strict mode double-invokes state
updaters; mutating refs inside an updater would diverge). The two
mergeSessionsById call sites (refresh poll + session creation) now
destructure [next, nextLastResortAt], write the refs, and pass a
plain value to setSessions. The other two setSessions call sites
(workspace reset at line 163, single-session replace at line 309)
are unchanged; the sessionsRef mirror effect picks up their changes.

Updates the App.tsx regex test to assert the new shape: ref
declarations, mirror effect, and tuple destructure at the creation
path."
```

---

### Task 5: Verification before completion

**Files:** None modified; verification only.

- [ ] **Step 1: Re-run full desktop test suite**

Run: `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` (from `apps/desktop/`)
Expected: all tests PASS.

- [ ] **Step 2: Re-run type-check**

Run: `npx tsc --noEmit` (from `apps/desktop/`)
Expected: PASS with no errors.

- [ ] **Step 3: Run the doc currency check**

Run: `bash .claude/hooks/doc-check.sh` (from repo root)
Expected: no flagged docs require updating. (This change is internal sort behavior; the spec doc is its own documentation. AGENTS.md mentions session sorting only in passing and doesn't need an update. If the script flags anything, address it before declaring complete.)

- [ ] **Step 4: Run the worktree currency check**

Run: `bash .claude/hooks/worktree-check.sh` (from repo root)
Expected: only this branch/worktree shows as active and unmerged. Follow up only on branches you own per the AGENTS.md stranded-branches rule.

- [ ] **Step 5: Inspect the final diff vs. `main`**

Run: `git diff main...HEAD -- apps/desktop/src/sessionSort.ts apps/desktop/src/App.tsx apps/desktop/tests/sessionSort.test.ts`
Expected: the diff shows the comparator collapse, the tuple-returning merge with throttle, the removed helpers, the ref declarations + mirror effect in App.tsx, the restructured merge sites, and the test file rewrite. No surprise files in the diff.

- [ ] **Step 6: Open a PR**

Per AGENTS.md review gate: PRs touching `apps/desktop/src/` must have a `/code-review` run before merge. Default to lightweight review unless the diff grew past ~8 code files (it didn't — three files) or 500 lines (unlikely). Open the PR:

```bash
gh pr create --title "Sort session list by recency, throttled" --body "Implements docs/superpowers/specs/2026-08-16-session-sort-recency-design.md.

Supersedes the 2026-07-31 one-minute bump rule. The session list is now sorted by a single key (lastActivityTimestamp descending, ties to created_at desc, then label asc) and re-sorts at most every 30s. ID-set changes (new session appears or existing disappears) bypass the throttle and re-sort immediately.

mergeSessionsById is now a pure tuple-returning function [list, nextLastResortAt] so React 19 strict mode's double-invoke of state updaters can't diverge via ref mutation. App.tsx holds lastResortAt and a sessions mirror in useRef and writes them outside the setSessions updater.

Removes ATTENTION_PRIORITY, isAtLeastOneMinuteOld, newestUpdatedAliveSession, and the bump-to-top block. Attention still surfaces via row badges and color, but no longer via sort position — confirmed as intent during spec review.

Test changes: 3 obsolete tests deleted (category ordering + 2 bump-rule tests); 4 throttle tests added; tuple-return call signature updated across the kept merge tests; App.tsx regex test updated for the new shape."
```

Replace the body with whatever matches your PR style; the above matches the repo's recent commit-message tone.

- [ ] **Step 7: Run `/code-review` on the PR**

Per AGENTS.md, this PR touches `apps/desktop/src/` and must have a `/code-review` run before merge. Address findings or note why each is intentional in the PR description.

- [ ] **Step 8: After merge, clean up the worktree (if one was opened)**

If a worktree was created for this work, after the branch merges, remove it:

```bash
git worktree remove <path-to-worktree>
git worktree prune
git branch -d <branch-name>
```

Per AGENTS.md: clean up your worktrees when done.

---

## Self-Review

**1. Spec coverage:**
- Sort key (3-key comparator: `lastActivityTimestamp` desc, `created_at` desc, `label` asc)? Task 1, Step 3.
- Pure tuple-returning `mergeSessionsById`? Task 2, Step 3.
- Throttle (30s, keep visual order on update, return unchanged `lastResortAt`)? Task 3, Step 3.
- ID-set change bypasses throttle? Task 3, Step 3 (`idsChanged || …`).
- Initial-empty always re-sorts? Task 2, Step 3 (and preserved in Task 3).
- `THROTTLE_MS` single named constant? Task 3, Step 3.
- Removed helpers (`ATTENTION_PRIORITY`, `isAtLeastOneMinuteOld`, `newestUpdatedAliveSession`, bump-to-top block)? Task 1 Step 3, Task 2 Step 3.
- Kept `needsAttention` and `sessionAttentionStatus`? Yes — neither is touched by any task.
- App.tsx `lastResortAtRef = useRef<Date>(new Date(0))`? Task 4, Step 3.
- App.tsx `sessionsRef = useRef<SessionInfo[]>([])` + mirror `useEffect`? Task 4, Steps 3–4.
- App.tsx restructures both `mergeSessionsById` call sites (lines 116 and 214)? Task 4, Steps 5–6.
- App.tsx other two `setSessions` call sites (163 reset, 309 single-session replace) stay unchanged? Yes — no task touches them; the mirror effect catches them.
- Tests deleted: `sortSessions ranks actionable alive sessions before working, idle, and dead`, `mergeSessionsById does not promote a session that just became alive`, `mergeSessionsById promotes fresh activity only after the top session is quiet for one minute`? Task 1 Step 5, Task 2 Step 6.
- Tests renamed: `sortSessions breaks attention ties by most recent activity, not label` → `sortSessions orders by most recent activity when timestamps differ`? Task 1 Step 6.
- Tests kept verbatim: `needsAttention`, `sessionAttentionStatus`, `sortSessions uses recent output when it is newer than meaningful activity`, drop tests, empty-poll test, initial-snapshot deterministic test? Yes — assertion bodies unchanged; only call signatures updated for the kept merge tests (Task 2 Step 4) and the regex test is updated for the new App.tsx shape (Task 4 Step 1).
- Tests added: throttle holds 29s, fires 30s, new session bypasses, disappeared session bypasses? Task 3 Step 1.
- Branch + PR per AGENTS.md? Task 1 preamble (global constraint) + Task 5 Steps 6–7.
- Doc currency check + worktree currency check before completion? Task 5 Steps 3–4.

**2. Placeholder scan:** No "TBD", "TODO", "implement later", "fill in details", "add appropriate error handling", "similar to Task N", or unscoped "write tests for the above" in any step. Every code step shows the code; every command step shows the command and expected output.

**3. Type consistency:**
- `mergeSessionsById` signature: `(existing: readonly SessionInfo[], incoming: readonly SessionInfo[], lastResortAt: Date = new Date(0), now: Date = new Date()): [SessionInfo[], Date]` — used identically in Task 2 (introduces it), Task 3 (extends the body), Task 4 (consumes it from App.tsx). Same param names (`lastResortAt`, `now`) and tuple return order (`[list, nextLastResortAt]`) across all references.
- `sortSessions(list: SessionInfo[]): SessionInfo[]` — signature unchanged from Task 1 through Task 4; only the body changes in Task 1 and stays stable afterwards.
- `lastResortAtRef = useRef<Date>(new Date(0))` and `sessionsRef = useRef<SessionInfo[]>([])` — names and types identical in Task 4 (`apps/desktop/src/App.tsx` declarations + tests/regex-test-pattern).
- `THROTTLE_MS = 30_000` — declared once in Task 3 Step 3; not referenced in any earlier task.
- All `next`, `nextLastResortAt`, `lastResortAt1`, `lastResortAt2` test variable names consistently reference the `Date` half of the tuple; `first` / `merged` / `second` / `next` consistently reference the `SessionInfo[]` half.
- App.tsx call sites: `const [next, nextLastResortAt] = mergeSessionsById(sessionsRef.current, …, lastResortAtRef.current, new Date())` — forms match between the spec's example shape (spec lines 68–78) and the plan's Task 4 Steps 5–6, and between the plan body and the regex test's assertions (Task 4 Step 1).
- Regex test anchor strings: `'lastResortAtRef'`, `'sessionsRef'`, `'sessionsRef.current'`, `'lastResortAtRef.current'`, `'[...sessionsRef.current, session]'`, `'new Date()'` — match the production code Task 4 introduces.

No type/name mismatches found.