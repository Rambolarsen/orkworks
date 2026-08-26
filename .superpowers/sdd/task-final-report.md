# Final Whole-Branch Review Fix Report

Date: 2026-08-26
Branch: `fix-runtime-recovery`

## Status

All actionable findings from the final whole-branch review are addressed. The fixes preserve the existing Electron wiring coverage and are committed on the current branch.

## Findings addressed

1. Renderer console diagnostics now log only allowlisted metadata: severity level, source origin, and line number. The arbitrary Chromium renderer message is discarded, preventing prompts, workspace paths, and error payloads from reaching logs. Pure diagnostic and source-wiring tests assert the absence of the payload.

2. Terminal attachment now runs through a pure cancellation-aware seam. A rejected backend URL/readiness promise calls the shared unavailable callback without producing an unhandled rejection; stale effects cannot update terminal state after cancellation. Tests cover success, rejection, and cancellation, with component wiring assertions.

3. Sidecar retry scheduling now uses an explicit retry-delay index independent of launch-attempt counting. Stable readiness resets both counters, so the first post-reset failure receives `retryDelaysMs[0]`; subsequent failures receive the expected increasing/capped delays. Tests assert the delay sequence before and after stability and the correct exhaustion budget.

4. `docs/agents/architecture.md` now documents the provider runner accurately: plain `Command::spawn()` with piped stdin/stdout/stderr and no Unix fork callback, `setsid`, or inherited-descriptor sweep.

5. `providers.rs` includes a macOS-only regression test that invokes the real `ProcessRunner` while a sibling parent thread is active. The cross-platform real-runner spawn-error test remains intact. The targeted macOS test passed on this Darwin host.

6. Renderer lifecycle coverage was strengthened with pure tests for attach readiness, fallback document construction, recovery retry behavior, allowlisted diagnostics, and lifecycle wiring. No Electron GUI dependency was introduced.

The unused injected `fetch` dependency was removed from the sidecar lifecycle contract and tests. The plan wording now uses `location.replace` and no longer describes the removed `fetch` injection. No EOF whitespace errors were present (`git diff --check` passed).

## TDD evidence

- RED: new renderer seam tests initially failed because the diagnostic export, recovery-document module, and attach seam did not yet exist.
- RED: the retry-delay regression initially observed the first post-stability delay as `0` instead of the required first configured delay.
- GREEN: focused lifecycle, renderer diagnostic, recovery-document, attach, wiring, and Dockview tests pass after the implementation and assertion corrections.

## Verification

- Focused desktop tests: 96 passed, 0 failed.
- Full desktop tests: 451 passed, 0 failed.
- TypeScript: `npx tsc --noEmit` passed.
- Desktop production build: passed; Vite transformed 2,095 modules.
- Full Rust suite: 819 passed, 0 failed.
- Rust build: passed with 4 pre-existing warnings.
- macOS ProcessRunner regression: 1 passed, 818 filtered out.
- `bash .claude/hooks/doc-check.sh`: passed.
- `bash .claude/hooks/worktree-check.sh`: passed.
- `git diff --check`: passed.

## Remaining concerns

- The Rust build retains four pre-existing unused/dead-code warnings.
- Node reports the repository's existing `MODULE_TYPELESS_PACKAGE_JSON` warning during direct test execution.
- Vite retains the existing warning for the 1.2 MB minified renderer chunk.
- Electron GUI execution was intentionally not added to CI-sensitive verification; behavior is covered through pure seams and source wiring tests.
