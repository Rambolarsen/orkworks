# Recorded Terminal-Size Cue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explain fixed-grid dead-session replay with its recorded terminal size without changing terminal output or scaling.

**Architecture:** Keep `HistoricalTerminal` as the sole renderer for dead-session output. It will retain the successful replay's optional dimensions in component state and render an informational sibling before the existing measured terminal container. The Rust sidecar response, raw-replay format, and scale calculation stay untouched.

**Tech Stack:** React, TypeScript, xterm.js, Node's built-in test runner.

## Global Constraints

- Preserve the recorded `cols × rows` grid; never reflow or normalize terminal replay records.
- Render the cue only after a non-empty replay has loaded successfully and only when both dimensions are present.
- Keep the cue outside `.terminal-container`; that element remains the flex child and scale-measurement target.
- Use no new dependencies, no new API fields, and no persisted UI state.

---

### Task 1: Show the recorded-size cue for successful fixed-grid replays

**Files:**

- Modify: `apps/desktop/src/components/HistoricalTerminal.tsx:12-131`
- Test: `apps/desktop/tests/dockview.test.ts:51-57`

**Interfaces:**

- Consumes: `loadTerminalReplay(...): Promise<"loaded" | "empty" | "error" | "stale">` and its `createTerminal({ cols?, rows? })` callback.
- Produces: `HistoricalTerminal` rendering `Recorded at {cols} × {rows}` only for a successfully loaded fixed-grid replay.

- [ ] **Step 1: Write the failing source-level regression test**

  Add this test beside `HistoricalTerminal loads output without opening an interactive terminal transport` in `apps/desktop/tests/dockview.test.ts`:

  ```ts
  test("HistoricalTerminal labels successful fixed-grid replays with their recorded size", () => {
    const source = readFileSync(new URL("../src/components/HistoricalTerminal.tsx", import.meta.url), "utf8");

    assert.match(source, /Recorded at \{recordedSize\.cols\} × \{recordedSize\.rows\}/);
    assert.match(source, /recordedSize && state === "loaded"/);
    assert.match(source, /<div className="terminal-shell">\s*\{recordedSize/);
  });
  ```

- [ ] **Step 2: Run the focused test to verify it fails**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/dockview.test.ts
  ```

  Expected: FAIL because `HistoricalTerminal` does not yet expose `recordedSize` or the cue.

- [ ] **Step 3: Add the minimum replay-size state and cue**

  In `apps/desktop/src/components/HistoricalTerminal.tsx`, add state for the dimensions that produced the currently loaded replay:

  ```ts
  const [recordedSize, setRecordedSize] = useState<{ cols: number; rows: number } | null>(null);
  ```

  At the start of the `useEffect`, clear it with `setRecordedSize(null)`. Capture valid dimensions in a local effect variable from the `createTerminal` callback, but only publish them after `loadTerminalReplay` resolves to `"loaded"`:

  ```ts
  let loadedSize: { cols: number; rows: number } | null = null;
  // inside createTerminal:
  if (cols && rows) loadedSize = { cols, rows };
  // inside the result handler, after the stale guard:
  if (result === "loaded") setRecordedSize(loadedSize);
  ```

  Replace the final one-line shell return with this sibling structure:

  ```tsx
  return (
    <div className="terminal-shell">
      {recordedSize && state === "loaded" && (
        <div className="historical-terminal-size">Recorded at {recordedSize.cols} × {recordedSize.rows}</div>
      )}
      <div
        ref={containerRef}
        className="terminal-container"
        aria-label={state === "loading" ? "Loading saved terminal output" : "Saved terminal output"}
      />
    </div>
  );
  ```

  Do not add CSS unless the existing inherited terminal-shell typography makes the text unreadable; if needed, use one local class rule only and retain `.terminal-container` as the flex child.

- [ ] **Step 4: Run focused test and type-check**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/dockview.test.ts
  npx tsc --noEmit
  ```

  Expected: both exit 0.

- [ ] **Step 5: Run relevant replay tests**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/terminalReplay.test.ts tests/terminalReplayScale.test.ts tests/dockview.test.ts
  ```

  Expected: all tests pass.

- [ ] **Step 6: Commit the implementation**

  ```bash
  git add apps/desktop/src/components/HistoricalTerminal.tsx apps/desktop/tests/dockview.test.ts
  git commit -m "fix: label recorded terminal replay size"
  ```

## Final Verification

- [ ] Run `bash .claude/hooks/doc-check.sh` from the repository root and address any relevant documentation drift.
- [ ] Run `bash .claude/hooks/worktree-check.sh` from the repository root; act only on worktrees owned by this branch.
- [ ] Recheck `git diff --check` and `git status --short` before handoff.
