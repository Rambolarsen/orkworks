# Electron Console Event API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Electron's deprecated positional `console-message` listener arguments while preserving metadata-only renderer diagnostics.

**Architecture:** Keep the existing listener in the Electron main process. Read `level`, `lineNumber`, and `sourceId` from Electron's event details, map the string severity to the existing numeric diagnostic contract, and continue omitting the renderer message payload.

**Tech Stack:** Electron 39, TypeScript, Node test runner.

## Global Constraints

- Preserve the Electron-main/renderer boundary.
- Do not log renderer console message payloads from the main process.
- Use pnpm-managed desktop tooling.

---

### Task 1: Migrate the console-message listener

**Files:**
- Modify: `apps/desktop/electron/main.ts:145-151`
- Modify: `apps/desktop/tests/errorBoundaryWiring.test.ts:35-48`

**Interfaces:**
- Consumes: Electron `WebContentsConsoleMessageEventParams` through the event object supplied to the `console-message` listener.
- Produces: The existing `rendererConsoleDiagnostic(level, sourceId, line)` metadata record, with no `message` field.

- [x] **Step 1: Write the failing source assertion**

  Add a test asserting the listener uses `details.level`, `details.sourceId`, and `details.lineNumber`, and does not destructure or accept the deprecated positional fields.

- [x] **Step 2: Run the focused test and verify it fails**

  Run `node --experimental-strip-types --test tests/errorBoundaryWiring.test.ts` from `apps/desktop/`.
  Expected: FAIL because the current listener uses positional parameters.

- [x] **Step 3: Implement the minimal listener migration**

  Change the listener to receive `details`, map its string level to the existing numeric levels (`info` → 1, `warning` → 2, `error` → 3, `debug` → 0), and pass `details.sourceId`, `details.lineNumber` to the existing sanitizer/diagnostic helper. Keep the handler free of `details.message` logging.

- [x] **Step 4: Run focused and desktop validation**

  Run `node --experimental-strip-types --test tests/errorBoundaryWiring.test.ts` and `npx tsc --noEmit` from `apps/desktop/`.
  Expected: PASS and a clean type-check.

- [x] **Step 5: Run repository checks**

  Run `bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh` from the repository root.
