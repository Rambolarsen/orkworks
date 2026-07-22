# Read-only Terminal Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Display persisted output for dead sessions without reopening their terminal runtime.

**Architecture:** Add a small replay loader that fetches output and suppresses stale writes. A focused renderer component owns a disposable read-only xterm; `TerminalPanel` selects it only for `lifecycle === "dead"`.

**Tech Stack:** React, TypeScript, xterm.js, Node test runner.

## Global Constraints

- Creating, alive, and stopping sessions preserve the current interactive WebSocket path.
- Dead-session replay is read-only and uses only `GET /sessions/:id/terminal-output`.
- Raw replay remains bounded by the existing persistence policy.

### Task 1: Replay loader

**Files:** Create `apps/desktop/src/terminalReplay.ts`; create `apps/desktop/tests/terminalReplay.test.ts`.

- [ ] Write tests for loaded, empty, failed, and stale replay responses.
- [ ] Run the test and observe failure because the module is absent.
- [ ] Implement the minimal loader around `getTerminalOutput` with a current-generation guard.
- [ ] Re-run the focused test and observe success.

### Task 2: Read-only terminal panel

**Files:** Create `apps/desktop/src/components/HistoricalTerminal.tsx`; modify `apps/desktop/src/components/TerminalPanel.tsx`; modify `apps/desktop/tests/dockview.test.ts`.

- [ ] Write source regression tests: only dead sessions use historical replay; transitional sessions retain the live terminal path; the historical component does not create a WebSocket or call `ensureTerminal`.
- [ ] Run the focused tests and observe failure.
- [ ] Implement the disposable read-only xterm component and lifecycle selection.
- [ ] Re-run focused tests and observe success.

### Task 3: Verification

- [ ] Run TypeScript checking and all frontend tests.
- [ ] Run the repository doc and worktree currency checks.
