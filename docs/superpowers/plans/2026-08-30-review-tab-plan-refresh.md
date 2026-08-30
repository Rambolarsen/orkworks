# Review Tab Plan File Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh the Review tab's plan file content on demand via a tab-header action and on "Review plan" clicks, updating smoothly without layout flashes or stale workspace state.

**Architecture:** A Dockview header action in `DockviewApp.tsx` renders a `RotateCw` refresh button when the Review tab is active and the selected session has an openable plan. An incrementing `reviewTick` state in `App.tsx` coordinates manual refreshes and "Review plan" button / terminal-link activations. `ReviewPanel.tsx` uses `reviewTick` with `lastTickRef` to fetch fresh content in the background without clearing the rendered view on same-session refreshes, protected by request ID guards and memoization.

**Tech Stack:** React 19, TypeScript, Dockview (`dockview-react`), Lucide React (`RotateCw`), Node.js test runner.

## Global Constraints

- The selected session's `planPath` remains the source of truth; no speculative file-tree scanning.
- Existing authenticated IPC bridge `getPlanContent(sessionId)` and sidecar route `/sessions/:id/plan-content` are reused as-is.
- No background polling thread or file watchers are added.
- Electron-main and renderer boundary invariant is strictly preserved.
- `ReviewPanel` must remain memoized (`export default memo(ReviewPanel)`) to avoid re-parsing markdown on session polling ticks.

---

### Task 1: Add Review Tab Refresh Header Action & reviewTick Plumbing in DockviewApp

**Files:**
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: `DockviewAppData` interface, `PANEL_DEFAULTS`, `SessionInfo.hasOpenablePlan`
- Produces: `DockviewAppData.reviewTick: number`, `DockviewAppData.onRefreshReview: () => void`, and `DockviewHeaderActions` rendering the refresh icon button when `review` panel is active

- [ ] **Step 1: Write and update header action tests in `apps/desktop/tests/dockview.test.ts`**

Update the existing `SessionsHeaderActions` tests to match `DockviewHeaderActions` and add the new review tab header action test:

```ts
test("DockviewApp exposes header actions for Sessions and Review panels", () => {
  const source = readFileSync(
    new URL("../src/components/DockviewApp.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /rightHeaderActionsComponent=\{DockviewHeaderActions\}/);
  assert.match(source, /PANEL_DEFAULTS\.sessions\.component/);
  assert.match(source, /PANEL_DEFAULTS\.review\.component/);
  assert.match(source, /dockview-header-action/);
});

test("DockviewHeaderActions renders Refresh plan button with RotateCw for review tab when session has an openable plan", () => {
  const source = readFileSync(new URL("../src/components/DockviewApp.tsx", import.meta.url), "utf8");

  assert.match(source, /RotateCw/);
  assert.match(source, /title="Refresh plan"/);
  assert.match(source, /aria-label="Refresh plan"/);
  assert.match(source, /onClick=\{\(\)\s*=>\s*ctx\.onRefreshReview\(\)\}/);
  assert.match(source, /reviewTick=\{ctx\.reviewTick\}/);
});
```

- [ ] **Step 2: Run tests to verify failure**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: FAIL with assertion error on `DockviewHeaderActions` or `RotateCw`.

- [ ] **Step 3: Implement header actions and reviewTick plumbing in `DockviewApp.tsx`**

1. Import `RotateCw` from `"lucide-react"`.
2. Update `DockviewAppData` interface:
   ```ts
   reviewTick: number;
   onRefreshReview: () => void;
   ```
3. Update `SessionsHeaderActions` (rename/generalize to `DockviewHeaderActions`):
   ```tsx
   function DockviewHeaderActions(props: IDockviewHeaderActionsProps) {
     const ctx = useContext(DockviewContext);

     if (props.activePanel?.id === PANEL_DEFAULTS.sessions.component) {
       if (!ctx.workspace) return null;
       return (
         <button
           className="dockview-header-action"
           type="button"
           title="New session"
           aria-label="New session"
           onClick={() => ctx.onCreateSession()}
         >
           +
         </button>
       );
     }

     if (props.activePanel?.id === PANEL_DEFAULTS.review.component) {
       const session = ctx.sessions.find((s) => s.id === ctx.activeSessionId);
       if (!session?.hasOpenablePlan) return null;
       return (
         <button
           className="dockview-header-action"
           type="button"
           title="Refresh plan"
           aria-label="Refresh plan"
           onClick={() => ctx.onRefreshReview()}
         >
           <RotateCw size={14} />
         </button>
       );
     }

     return null;
   }
   ```
4. Pass `rightHeaderActionsComponent={DockviewHeaderActions}` to `<DockviewReact />`.
5. In `ReviewTab`:
   ```tsx
   function ReviewTab() {
     const ctx = useContext(DockviewContext);
     const session = ctx.sessions.find((s) => s.id === ctx.activeSessionId);
     return (
       <ReviewPanel
         sessionId={session?.hasOpenablePlan ? ctx.activeSessionId : null}
         reviewTick={ctx.reviewTick}
       />
     );
   }
   ```

- [ ] **Step 4: Run tests to verify pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/DockviewApp.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat(desktop): add review tab header refresh action and reviewTick plumbing"
```

---

### Task 2: Implement Smooth In-Place Plan Refresh in ReviewPanel

**Files:**
- Modify: `apps/desktop/src/components/ReviewPanel.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: `{ sessionId: string | null; reviewTick?: number }`, `window.orkworks.getPlanContent(sessionId)`
- Produces: `ReviewPanel` with background refresh preserving rendered content on `reviewTick` changes, single unified effect using `lastTickRef`, and preserved `memo` wrapper

- [ ] **Step 1: Write tests in `apps/desktop/tests/dockview.test.ts`**

```ts
test("ReviewPanel supports reviewTick prop with lastTickRef and retains memoization", () => {
  const source = readFileSync(new URL("../src/components/ReviewPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /reviewTick\?: number/);
  assert.match(source, /lastSessionIdRef/);
  assert.match(source, /lastTickRef/);
  assert.match(source, /window\.orkworks\.getPlanContent\(sessionId\)/);
  assert.match(source, /export default memo\(ReviewPanel\);/);
});
```

- [ ] **Step 2: Run tests to verify failure**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: FAIL with assertion error on `reviewTick?: number` or `lastTickRef`.

- [ ] **Step 3: Update `ReviewPanel.tsx`**

```tsx
interface ReviewPanelProps {
  sessionId: string | null;
  reviewTick?: number;
}

function ReviewPanel({ sessionId, reviewTick }: ReviewPanelProps) {
  const [content, setContent] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const requestId = useRef(0);
  const lastSessionIdRef = useRef<string | null>(null);
  const lastTickRef = useRef(reviewTick);

  const load = useCallback((isExplicitRefresh = false) => {
    if (!sessionId) {
      setContent(null);
      setError(false);
      lastSessionIdRef.current = null;
      return;
    }
    const currentRequest = ++requestId.current;
    if (!isExplicitRefresh && lastSessionIdRef.current !== sessionId) {
      setContent(null);
    }
    setError(false);
    lastSessionIdRef.current = sessionId;

    void window.orkworks.getPlanContent(sessionId)
      .then((value) => {
        if (currentRequest === requestId.current) setContent(value);
      })
      .catch(() => {
        if (currentRequest === requestId.current) setError(true);
      });
  }, [sessionId]);

  useEffect(() => {
    const isTickChange = reviewTick !== undefined && reviewTick !== lastTickRef.current;
    lastTickRef.current = reviewTick;
    load(isTickChange);
  }, [sessionId, reviewTick, load]);

  if (!sessionId) return <EmptyState message="Select a session with a plan to review it." />;
  if (error) return <EmptyState message="This plan is no longer available." action={{ label: "Retry", onClick: load }} />;
  if (content === null) return <EmptyState message="Loading plan…" />;
  return (
    <div className="review-plan-content">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>{content}</ReactMarkdown>
    </div>
  );
}

export default memo(ReviewPanel);
```

- [ ] **Step 4: Run tests to verify pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/components/ReviewPanel.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat(desktop): implement smooth in-place refresh in ReviewPanel"
```

---

### Task 3: Wire Up reviewTick State and onReviewPlan Coordination in App.tsx

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`

**Interfaces:**
- Consumes: `handleReviewPlan`, `DockviewApp` props
- Produces: `reviewTick` state incremented on `handleReviewPlan` and `onRefreshReview`, passed to `DockviewApp`

- [ ] **Step 1: Write failing tests in `apps/desktop/tests/dockview.test.ts`**

```ts
test("App.tsx increments reviewTick on handleReviewPlan and passes reviewTick to DockviewApp", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

  assert.match(source, /const \[reviewTick, setReviewTick\] = useState\(0\);/);
  assert.match(source, /setReviewTick\(\(?t\)?\s*=>\s*t\s*\+\s*1\);/);
  assert.match(source, /onRefreshReview=\{/);
  assert.match(source, /reviewTick=\{reviewTick\}/);
});
```

- [ ] **Step 2: Run tests to verify failure**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: FAIL with assertion error.

- [ ] **Step 3: Implement `reviewTick` state and coordination in `App.tsx`**

1. Add `const [reviewTick, setReviewTick] = useState(0);`
2. In `handleReviewPlan`:
   ```ts
   const handleReviewPlan = useCallback(() => {
     const api = dockviewApiRef.current;
     if (!api) return;
     const panel = api.getPanel("review") ?? api.addPanel({
       id: "review", component: "review", title: "Review",
       position: { referencePanel: "terminal" },
     });
     panel?.api.setActive();
     setReviewTick(t => t + 1);
   }, []);
   ```
3. Pass `reviewTick={reviewTick}` and `onRefreshReview={() => setReviewTick(t => t + 1)}` to `<DockviewApp ... />`.

- [ ] **Step 4: Run tests to verify pass**

Run: `node --experimental-strip-types --test apps/desktop/tests/dockview.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/tests/dockview.test.ts
git commit -m "feat(desktop): wire reviewTick and refresh review handler in App"
```

---

### Task 4: Full Suite Verification & Type Check

**Files:**
- Test: all desktop tests (`apps/desktop/tests/`)
- Type check: `apps/desktop/`

- [ ] **Step 1: Run TypeScript type check**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: No type errors.

- [ ] **Step 2: Run all desktop tests**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: All tests pass.

- [ ] **Step 3: Run doc check and git diff check**

Run: `./scripts/doc-check.sh && git diff --check`
Expected: Clean output.
