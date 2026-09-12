# Coding Tool Settings Implementation Plan

> **For agentic workers:** Use executing-plans to implement this plan task-by-task.

**Goal:** Restore settings disclosure and resolve the reported Windows integration retry failure.

**Architecture:** Keep the mounted React subsection and existing integration orchestration. Correct confirmed defects at their owner, preserving install confirmation and status provenance.

**Tech Stack:** React, TypeScript, CSS, Electron, Node test runner; Rust only if the installation failure is traced there.

## Global Constraints

- Use pnpm; add no dependencies.
- Keep Electron and renderer imports separate.
- An enabled or detected coding tool is not evidence of installed hooks.
- Preserve unrelated hook entries and unsaved path drafts.

### Task 1: Restore disclosure

Files: `apps/desktop/src/App.css`, `apps/desktop/tests/providersPanel.test.ts`.

- [x] Add a regression assertion that the subsection's hidden state overrides its flex display; run `node --experimental-strip-types --test tests/providersPanel.test.ts` from `apps/desktop` and confirm failure.
- [x] Add `.settings-config-item-subsection[hidden] { display: none; }`.
- [x] Rerun the focused tests and verify expanded/collapsed layout in Chromium.

### Task 2: Diagnose and fix installation retry

Files: existing settings, preload, orchestration, and integration owners as indicated by reproduction.

- [x] Compare packaged bridge calls with current source and read the live sidecar's grouped Claude status.
- [x] Reproduce the failing boundary with its actual error/result, using temporary reporter directories for mutation checks.
- [x] Add a failing regression for that confirmed failure, correct the owning code, and rerun the focused regression.
- [x] Verify that installation becomes healthy only after successful installation and that failures are visible at the affected row.

Implementation: `write_new_file_atomically` drops the flushed file before replacement. `resultForGroup` converts nominal success plus registration `error` to failed, preserving diagnostics through the existing renderer failure map. Tests cover immediate enable, per-group reconciliation, batched save, and display after a later absent-status probe.

### Task 3: Verify and review

- [x] Run desktop tests, type checking, and build; run `bash scripts/verify-repo.sh` and report any environment or baseline failures accurately.
- [x] Perform `/code-review low` on the diff and address actionable findings.
- [ ] Run doc and worktree currency checks, then commit and open the focused PR with validation evidence.

## Verification record

- 97 focused desktop tests passed; 17 Rust integration/publication tests passed on Windows.
- The new native reporter-update test failed before the fix with Windows error 32 and passed after closing the source handle.
- Hidden Electron smoke: collapsed display `none`, expanded `flex`, off/on called the scoped install callback and rendered healthy after success.
- Desktop type check, desktop build, docs build, Rust formatting/build, and diff check passed.
- Full desktop run: 682 passed, 3 failed. All three failures reproduced against an untouched archive of base commit `cc80152`: concurrent settings writers, preload path expectations, and the Peon diagnostics source assertion.
- Consolidated verification stopped at the full Windows Rust suite: 931 passed, 96 failed. Existing POSIX reporter/path assumptions also failed in the pre-fix integration-suite run. This is not a clean full-suite result.
- Review: correctness pass found no issues; coverage suggestions added immediate-wrapper and refresh-retention assertions. Broad Windows CI coverage is outside this focused fix; PR Rust CI remains Linux-only, so the native replacement regression is verified locally on Windows.
- Doc currency check passed. Worktree currency flags the unrelated `merge-conflict-fix` worktree as 16 days stale; it is not owned by this task.
