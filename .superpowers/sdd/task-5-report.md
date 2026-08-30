# Task 5 Report

## Files

- Updated `docs/superpowers/specs/2026-08-14-provider-settings-migration-design.md` only.
- Added this focused verification report.
- The dated design was referenced but not rewritten.

## Commit

- This report is included in the final commit: `docs: define provider-specific model precedence`.

## Verification

- `git diff --check` — passed.
- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` — failed on pre-existing formatting drift across unrelated Rust files; no formatter changes were applied.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml` — passed: 827 passed, 0 failed.
- `cd apps/desktop && pnpm test` — failed because `apps/desktop/package.json` has no `test` script; pnpm exited 1 without test output.
- Underlying desktop command `pnpm exec node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — passed: 401 passed, 0 failed.
- `cd apps/desktop && pnpm exec tsc --noEmit` — passed.
- `cd apps/desktop && pnpm run build` — passed; Vite completed with the existing large-chunk warning.
- `bash .claude/hooks/doc-check.sh` — passed with no output.
- `bash .claude/hooks/worktree-check.sh` — passed with no output.
- Final `git diff --check` — passed.

## Concerns

- The required `pnpm test` command is not runnable as written because the package has no `test` script; the underlying test command passes.
- Rust format check remains non-clean due to unrelated existing drift.
- The manual `/code-review` gate was not run because this task changes documentation only and no code-changing PR was produced.

## Active coding tool hook toggle Task 5 — 2026-08-29

### Status

Implemented in `/Users/froomiebot/workspace/orkworks-active-coding-tool-hook-toggle`.

### Changes

- Added `apps/desktop/src/components/HarnessCommandPathControl.tsx` and moved the custom command-path draft, absolute-path validation, Save/Clear IPC calls, local error handling, and refresh callback into that standalone control.
- Updated `apps/desktop/src/components/SettingsModal.tsx` to mount the new control for every `command-template` tool, keep it out of `platform-shell` rows, and keep integration status ownership in the modal.
- Removed the old inline hook section by deleting `apps/desktop/src/components/HarnessIntegrationSection.tsx`; the toggle state stays driven by the existing per-harness status map in Settings.
- Added layout styling for the standalone path control in `apps/desktop/src/App.css`.
- Added focused source regressions in `apps/desktop/tests/harnessCommandPathControl.test.ts` and updated `apps/desktop/tests/providersPanel.test.ts`.

### TDD evidence

- RED: `rtk node --experimental-strip-types --test tests/harnessCommandPathControl.test.ts tests/providersPanel.test.ts` failed 7 checks: the new component file did not exist, `SettingsModal` still imported and mounted `HarnessIntegrationSection`, and the command-path refresh/disabled wiring was absent.
- GREEN: the same focused suite passed after the extraction and remounting changes.

### Verification

- `cd apps/desktop && rtk node --experimental-strip-types --test tests/harnessCommandPathControl.test.ts tests/providersPanel.test.ts` — PASS, 22 passed, 0 failed.
- `cd apps/desktop && rtk pnpm exec tsc --noEmit` — PASS.
- `rtk bash scripts/doc-check.sh` — PASS.
- `rtk bash .claude/hooks/worktree-check.sh` — PASS.
- `rtk git diff --check` — PASS.

### Concerns

- The focused test coverage remains source-level because the current Node test setup cannot import `.tsx` modules directly under the required `node --experimental-strip-types --test ...` command.
- The desktop test commands continue to emit the repository's existing `MODULE_TYPELESS_PACKAGE_JSON` warning.

### Commit

The Task 5 implementation commit hash is reported at handoff because this appended report is part of that commit.
