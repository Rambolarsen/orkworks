# Terminal Trackpad Bursts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop tiny trackpad events from becoming excessive full-screen terminal scroll commands (#720).

**Architecture:** A per-terminal renderer filter uses the public xterm wheel API.
It immediately forwards each gesture's first movement and accumulates later
pixels by rendered line height. The existing dependency patch remains intact.

**Tech Stack:** TypeScript, xterm 6.0.0, Node tests, Electron runtime fixtures.

**Spec:** `specs/orkworks-mvp.md` (Terminal Sessions / Milestone 2), approved #720 approach, and ADR 0074.

## Global Constraints

- Keep Electron main and renderer imports separate; use pnpm.
- Preserve one active terminal, terminal lifetime, ordinary scrollback `0.1`, and effective fast scrolling.
- Preserve mouse tracking, shift-wheel, zero-delta guards, and both shipped xterm bundles.

## Investigated uncertainties

Least confidence: gesture boundaries and rendered cell measurement. Use a 150ms
idle boundary, immediate direction/modifier changes, and public screen height /
rows with a font-size fallback. No timer means no queued input after disposal.
The missed case in previous tests was a burst of small events, so verify both
mouse-tracked reports and passive arrow sequences, not only isolated packets.
Native device feel remains an explicitly reported validation limitation.

### Task 1: Filter alternate-buffer trackpad bursts

**Files:** Create `apps/desktop/src/terminalWheel.ts`; modify `terminalStore.ts`,
`tests/xtermAltBufferWheelRuntime.test.mjs`; create `tests/terminalWheel.test.ts`.
ADR 0074, its predecessor, index, and root guide are synchronized first.

**Interfaces:** `createTrackpadWheelFilter()` produces `filter(event, alternate,
rowHeight): boolean` and `reset(): void`. `installTerminalWheelHandler(term)`
installs the filter on a real terminal and resets it on buffer transitions.

- [x] Extend the existing Electron fixture with realistic bursts. With a 20px
  rendered line, fifty 1px events must produce three commands, not fifty;
  first movement and immediate reverse each produce one command.
- [x] Run the wheel fixture and observe the excess-command assertion fail.
- [x] Add deterministic Node cases for pauses, modifier changes, zero/horizontal
  input, normal buffer, line/page mode, and independent terminal accumulators.
- [x] Implement the filter and install it in `ensureTerminal` before opening.
  Cancel suppressed events; keep one command per incoming event maximum.
- [x] Run focused Node/Electron tests and type-check. The Electron fixture must
  use the production installation function, and preserve normal-buffer checks.
- [ ] Run `bash scripts/verify-repo.sh`, review `/code-review low`, inspect the
  diff, and commit the verified implementation.
- [ ] Open one PR closing #720; inventory CI/reviews using babysitting skill,
  disposition feedback, check Taskmaster tie-off, merge under repository policy,
  and clean the matching worktree with `scripts/finish-pr.sh`.

## Verification record

- RED: Electron burst fixture failed with 50 commands before production changes.
- GREEN: 13 focused wheel/filter tests and renderer type-check passed. Expanded
  Electron cases also passed trusted Chromium pixel input in both tracking modes.
- Consolidated repository verification: 1,586 Rust tests, 9 OpenCode reporter
  tests, 1,103 desktop tests; desktop/docs builds and formatting passed.
- Manual `/code-review low`: no diff-changing findings; spec and code quality approved.
- Physical trackpad feel remains for the user to validate in the updated app.
