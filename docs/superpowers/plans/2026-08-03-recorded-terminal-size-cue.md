# Recorded Terminal-Size Cue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explain fixed-grid dead-session replay with its recorded terminal size without changing terminal output or scaling.

**Architecture:** Keep `HistoricalTerminal` as the sole renderer for dead-session output. Session-keyed component state retains the successful replay's optional dimensions and renders an informational sibling before the existing measured terminal container. The Rust sidecar response, raw-replay format, and scale calculation stay untouched.

**Tech Stack:** React, TypeScript, xterm.js, Node's built-in test runner.

## Global Constraints

- Preserve the recorded `cols × rows` grid; never reflow or normalize terminal replay records.
- Render the cue only after a non-empty replay has loaded successfully and only when both dimensions are present.
- Keep the cue outside `.terminal-container`; that element remains the flex child and scale-measurement target.
- Use no new dependencies, no new API fields, and no persisted UI state.
- Use the existing `--text-xs`, `--text-muted`, and `--space-2` design tokens for the cue.

---

### Task 0: Create the tracked implementation workspace

**Files:**

- Create: GitHub issue titled `Explain recorded terminal dimensions in dead-session replay`
- Create: linked worktree `../orkworks-recorded-terminal-size-cue` on branch `recorded-terminal-size-cue`

**Interfaces:**

- Consumes: the approved design at `docs/superpowers/specs/2026-08-03-recorded-terminal-size-cue-design.md`.
- Produces: an issue-backed, isolated checkout for the desktop change.

- [ ] **Step 1: Invoke `starting-work` and inspect checkout ownership**

  Run from the repository root:

  ```bash
  git worktree list --porcelain
  git status --short
  ```

  Expected: recognize the primary checkout's unrelated edits and choose an isolated worktree.

- [ ] **Step 2: Create the tracking issue**

  Create an issue that links the design and has these acceptance criteria:

  ```md
  - [ ] Sized, successfully loaded dead-session replays show `Recorded at {cols} × {rows}`.
  - [ ] Legacy, empty, error, stale, and newly selected sessions do not show a stale cue.
  - [ ] The cue is above—not inside or over—the measured terminal container.
  - [ ] Replay output, recorded geometry, API, and persistence format are unchanged.
  ```

- [ ] **Step 3: Create and prepare the worktree**

  ```bash
  git worktree add ../orkworks-recorded-terminal-size-cue -b recorded-terminal-size-cue
  cd ../orkworks-recorded-terminal-size-cue/apps/desktop
  pnpm install
  ```

  Expected: a clean agent-owned checkout with desktop dependencies installed.

---

### Task 1: Show the recorded-size cue for successful fixed-grid replays

**Files:**

- Modify: `apps/desktop/src/components/HistoricalTerminal.tsx:12-131`
- Modify: `apps/desktop/src/App.css:528-548`
- Test: `apps/desktop/tests/dockview.test.ts:51-57`

**Interfaces:**

- Consumes: `loadTerminalReplay(...): Promise<"loaded" | "empty" | "error" | "stale">` and its `createTerminal({ cols?, rows? })` callback.
- Produces: `HistoricalTerminal` rendering `Recorded at {cols} × {rows}` only for a successfully loaded fixed-grid replay.

- [ ] **Step 1: Write the failing source-level regression test**

  Add this test beside `HistoricalTerminal loads output without opening an interactive terminal transport` in `apps/desktop/tests/dockview.test.ts`:

  ```ts
  test("HistoricalTerminal labels successful fixed-grid replays with their recorded size", () => {
    const source = readFileSync(new URL("../src/components/HistoricalTerminal.tsx", import.meta.url), "utf8");

    assert.match(source, /Recorded at \{replay\.size\.cols\} × \{replay\.size\.rows\}/);
    assert.match(source, /replay\.sessionId === sessionId && replay\.state === "loaded" && replay\.size/);
    assert.match(source, /setReplay\(\{ sessionId, state: result, size: result === "loaded" \? loadedSize : null \}\)/);
    assert.match(source, /<div className="terminal-shell">\s*\{replay\.sessionId/);
  });
  ```

- [ ] **Step 2: Run the focused test to verify it fails**

  Run from `apps/desktop/`:

  ```bash
  node --experimental-strip-types --test tests/dockview.test.ts
  ```

  Expected: FAIL because `HistoricalTerminal` does not yet expose session-keyed replay state or the cue.

- [ ] **Step 3: Add the minimum replay-size state and cue**

  In `apps/desktop/src/components/HistoricalTerminal.tsx`, replace the standalone status state with state keyed to the session that produced it:

  ```ts
  type ReplayState = "loading" | "empty" | "error" | "loaded";
  const [replay, setReplay] = useState<{
    sessionId: string;
    state: ReplayState;
    size: { cols: number; rows: number } | null;
  }>({ sessionId, state: "loading", size: null });
  ```

  At the start of the `useEffect`, set `{ sessionId, state: "loading", size: null }`. Capture valid dimensions in a local effect variable from the `createTerminal` callback, but publish them only after `loadTerminalReplay` resolves. The render condition must compare `replay.sessionId` to the current prop, which prevents an old cue from flashing before the effect for a newly selected session runs:

  ```ts
  let loadedSize: { cols: number; rows: number } | null = null;
  // inside createTerminal:
  if (cols && rows) loadedSize = { cols, rows };
  // inside the result handler, after the stale guard:
  setReplay({ sessionId, state: result, size: result === "loaded" ? loadedSize : null });
  // inside the catch handler, after the current guard:
  setReplay({ sessionId, state: "error", size: null });
  ```

  Replace the final one-line shell return with this sibling structure:

  ```tsx
  return (
    <div className="terminal-shell">
      {replay.sessionId === sessionId && replay.state === "loaded" && replay.size && (
        <div className="historical-terminal-size">Recorded at {replay.size.cols} × {replay.size.rows}</div>
      )}
      <div
        ref={containerRef}
        className="terminal-container"
        aria-label={replay.sessionId === sessionId && replay.state === "loading" ? "Loading saved terminal output" : "Saved terminal output"}
      />
    </div>
  );
  ```

  Add this sibling style next to `.terminal-shell`; leave `.terminal-container` unchanged as the flex child and scale target:

  ```css
  .historical-terminal-size {
    flex: 0 0 auto;
    padding: 0 var(--space-2);
    color: var(--text-muted);
    font-size: var(--text-xs);
  }
  ```

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

- [ ] **Step 6: Manually verify the layout in the desktop app**

  Run from `apps/desktop/`:

  ```bash
  pnpm dev
  ```

  Open one dead session with a `.terminal-size` sidecar and one legacy dead session without it. Verify the sized replay shows its cue above the terminal, remains fully visible when the panel is resized, and preserves its historic hard wraps; verify the legacy replay has no cue.

- [ ] **Step 7: Commit the implementation**

  ```bash
  git add apps/desktop/src/components/HistoricalTerminal.tsx apps/desktop/src/App.css apps/desktop/tests/dockview.test.ts
  git commit -m "fix: label recorded terminal replay size"
  ```

## Final Verification

- [ ] Run `bash .claude/hooks/doc-check.sh` from the repository root and address any relevant documentation drift.
- [ ] Run `bash .claude/hooks/worktree-check.sh` from the repository root; act only on worktrees owned by this branch.
- [ ] Recheck `git diff --check` and `git status --short` before handoff.
