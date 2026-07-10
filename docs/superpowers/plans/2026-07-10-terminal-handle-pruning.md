# Terminal Handle Pruning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the desktop renderer from retaining stale terminal handles after workspace switches or session-list churn, without regressing live-session switching.

**Architecture:** Keep terminal caching for live session switching, but make it bounded. `TerminalPanel` will refuse to create terminals for non-live sessions, `App.tsx` will build a keep-set from live sessions and prune against it after refreshes, and `terminalStore.ts` will expose a narrow `pruneTerminals(keepLiveSessionIds)` helper plus full-cache disposal for workspace changes.

**Tech Stack:** React, TypeScript, xterm.js, Node built-in test runner

## Global Constraints

Keep the existing behavior where switching between still-live sessions in the same workspace does not dispose inactive terminals.
Terminal creation must be gated to live sessions only; non-live selected sessions render the terminal empty state instead of creating a new renderer terminal.
`pruneTerminals(keepLiveSessionIds: ReadonlySet<string>)` only removes cached ids absent from the keep set.
`App.tsx` is responsible for building `keepLiveSessionIds` from sessions whose `memoryState === "live"`.
Clear all cached terminals on workspace changes before resetting the session list.
Session refresh results must not repopulate state or prune against a previous workspace after a workspace switch.
Do not change terminal protocol, PTY lifetime, or backend session lifetime.

---

### Task 1: Lock the leak fix in source-based regression tests

**Files:**
- Modify: `apps/desktop/tests/dockview.test.ts`
- Modify: `apps/desktop/tests/terminalDetachSource.test.ts`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/terminalDetachSource.test.ts`

**Interfaces:**
- Consumes: `TerminalPanel` source, `App.tsx` source, `terminalStore.ts` source
- Produces: Failing assertions for live-only terminal creation, live-session keep-set pruning, workspace-switch disposal, and prune helper presence

- [ ] **Step 1: Write the failing source tests**

```ts
test("TerminalPanel only opens CenterPanel for live sessions", () => {
  const source = readFileSync(new URL("../src/components/TerminalPanel.tsx", import.meta.url), "utf8");

  assert.match(source, /session\.memoryState !== "live"/);
  assert.match(source, /<EmptyState message="Select a live session to open its terminal\." \/>/);
  assert.match(source, /return <CenterPanel backendStatus=\{backendStatus\} sessionId=\{session\.id\} \/>/);
});

test("App clears cached terminals on workspace switch before resetting session state", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  const handleOpenWorkspace =
    source.match(/const handleOpenWorkspace = useCallback\(async \(\) => \{([\s\S]*?)\n  \}, \[\]\);/)?.[1] ?? "";

  assert.match(source, /import \{[^}]*disposeAllTerminals[^}]*\} from "\.\/terminalStore"/);
  assert.match(handleOpenWorkspace, /refreshGenerationRef\.current \+= 1;/);
  assert.match(handleOpenWorkspace, /disposeAllTerminals\(\);/);
  assert.match(handleOpenWorkspace, /setSessions\(\[\]\);/);
});

test("App prunes cached terminals against the current live session ids", () => {
  const source = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  const refreshSessions =
    source.match(/const refreshSessions = useCallback\(async \(\) => \{([\s\S]*?)\n  \}, \[\]\);/)?.[1] ?? "";

  assert.match(source, /import \{[^}]*pruneTerminals[^}]*\} from "\.\/terminalStore"/);
  assert.match(source, /refreshGenerationRef = useRef\(0\)/);
  assert.match(refreshSessions, /const generation = refreshGenerationRef\.current;/);
  assert.match(refreshSessions, /if \(generation !== refreshGenerationRef\.current\) return;/);
  assert.match(refreshSessions, /filter\(\(s\) => s\.memoryState === "live"\)/);
  assert.match(refreshSessions, /map\(\(s\) => s\.id\)/);
  assert.match(refreshSessions, /new Set\(/);
  assert.match(refreshSessions, /pruneTerminals\(liveSessionIds\)/);
  assert.match(refreshSessions, /setSessions\(sortSessions\(list\)\)/);
});
```

```ts
test("terminalStore exposes pruneTerminals and full-cache disposal", () => {
  assert.match(source, /export function pruneTerminals\(keepLiveSessionIds: ReadonlySet<string>\): void \{/);
  assert.match(source, /for \(const id of \[\.\.\.terminals\.keys\(\)\]\) \{/);
  assert.match(source, /if \(!keepLiveSessionIds\.has\(id\)\) disposeTerminal\(id\);/);
  assert.match(source, /export function disposeAllTerminals\(\): void \{/);
});

test("pruneTerminals and disposeAllTerminals stay idempotent", () => {
  assert.match(source, /const handle = terminals\.get\(id\);\s*if \(!handle\) return;/);
  assert.match(source, /for \(const id of \[\.\.\.terminals\.keys\(\)\]\) disposeTerminal\(id\);/);
});
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/terminalDetachSource.test.ts`
Expected: FAIL because `TerminalPanel` still opens terminals for any selected session, `App.tsx` does not prune or clear the cache, and `terminalStore.ts` does not expose `pruneTerminals`.

### Task 2: Add bounded terminal-cache cleanup

**Files:**
- Modify: `apps/desktop/src/components/TerminalPanel.tsx`
- Modify: `apps/desktop/src/terminalStore.ts`
- Modify: `apps/desktop/src/App.tsx`
- Test: `apps/desktop/tests/dockview.test.ts`
- Test: `apps/desktop/tests/terminalDetachSource.test.ts`

**Interfaces:**
- Consumes: `SessionInfo.memoryState`, `listSessions(baseUrl): Promise<SessionInfo[]>`, `disposeTerminal(id): void`
- Produces: `pruneTerminals(keepLiveSessionIds: ReadonlySet<string>): void`, live-only terminal rendering, workspace-switch disposal, and post-refresh pruning

- [ ] **Step 1: Implement the live-only terminal gate**

```tsx
function TerminalPanel({ backendStatus, session }: TerminalPanelProps) {
  if (!session || session.memoryState !== "live") {
    return <EmptyState message="Select a live session to open its terminal." />;
  }
  return <CenterPanel backendStatus={backendStatus} sessionId={session.id} />;
}
```

- [ ] **Step 2: Implement the prune helper in the terminal store**

```ts
export function pruneTerminals(keepLiveSessionIds: ReadonlySet<string>): void {
  for (const id of [...terminals.keys()]) {
    if (!keepLiveSessionIds.has(id)) disposeTerminal(id);
  }
}
```

- [ ] **Step 3: Handle workspace switch atomically**

```ts
const handleOpenWorkspace = useCallback(async () => {
  try {
    const info = await window.orkworks.openWorkspace();
    if (info) {
      refreshGenerationRef.current += 1;
      disposeAllTerminals();
      setWorkspaceState(info);
      setActiveHarnessIds(info.activeHarnessIds ?? []);
      setBackendStatus("connecting…");
      setSessions([]);
      setActiveSessionId(info.lastActiveSessionId ?? null);
    }
  } catch {
    pushToast("error", "Couldn't open workspace.");
  }
}, []);
```

- [ ] **Step 4: Prune terminal handles after each successful session refresh**

```ts
const refreshGenerationRef = useRef(0);

const refreshSessions = useCallback(async () => {
  const generation = refreshGenerationRef.current;
  try {
    const baseUrl = await window.orkworks.getBackendUrl();
    const list = await listSessions(baseUrl);
    if (generation !== refreshGenerationRef.current) return;
    const liveSessionIds = new Set(list.filter((s) => s.memoryState === "live").map((s) => s.id));
    pruneTerminals(liveSessionIds);
    setSessions(sortSessions(list));
  } catch {
    // Silent: polled every 2s; transient failures are reflected by the
    // backendStatus badge, not by spamming toasts.
  }
}, []);
```

- [ ] **Step 5: Bump the refresh generation on initial workspace load before refresh**

```ts
if (!cancelled && info) {
  refreshGenerationRef.current += 1;
  disposeAllTerminals();
  setWorkspaceState(info);
  setActiveHarnessIds(info.activeHarnessIds ?? []);
  await refreshSessions();
  if (info.lastActiveSessionId) {
    setActiveSessionId(info.lastActiveSessionId);
  }
}
```

- [ ] **Step 6: Run the focused tests to verify they pass**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/dockview.test.ts tests/terminalDetachSource.test.ts`
Expected: PASS

### Task 3: Run full desktop verification and doc guardrails

**Files:**
- Verify only: `apps/desktop/tests/*.test.ts`
- Verify only: `apps/desktop/tests/*.test.mjs`
- Verify only: `apps/desktop/src/`
- Verify only: `.claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: Desktop test suite, TypeScript compiler, repo doc-check hook
- Produces: Fresh verification evidence for the leak fix branch

- [ ] **Step 1: Run the full desktop test suite**

Run: `cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
Expected: PASS with `0` failures

- [ ] **Step 2: Run the desktop typecheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: exit `0` with no TypeScript diagnostics

- [ ] **Step 3: Run the doc currency check**

Run: `bash .claude/hooks/doc-check.sh`
Expected: exit `0`, or a list of doc files to review and explicitly confirm as unchanged for this renderer-only bugfix

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/components/TerminalPanel.tsx \
        apps/desktop/src/terminalStore.ts \
        apps/desktop/src/App.tsx \
        apps/desktop/tests/dockview.test.ts \
        apps/desktop/tests/terminalDetachSource.test.ts \
        docs/superpowers/specs/2026-07-10-terminal-handle-pruning-design.md \
        docs/superpowers/plans/2026-07-10-terminal-handle-pruning.md
git commit -m "fix(desktop): prune stale terminal handles"
```
