# Task 3 Report: Preserve the field across Electron settings persistence

## Status

Implemented and committed on `fix/provider-model-selection`.

## Files changed

- `apps/desktop/src/providerTypes.ts`
  - Added `model: string | null` to the renderer `ProviderSettingsEntry` interface.
- `apps/desktop/electron/providerTypes.ts`
  - Added the mirrored Electron `ProviderSettingsEntry.model` field.
- `apps/desktop/electron/settingsMemory.ts`
  - Added `model: null` to every `DEFAULT_PROVIDER_SETTINGS.providers` entry.
  - Normalized persisted provider models with Rust-compatible trim/empty/non-string semantics.
  - Left `normalizePeonModel` and legacy top-level `peonModel` migration separate and unchanged.
- `apps/desktop/tests/electronSettingsMemory.test.ts`
  - Added complete-entry persistence coverage for trimmed, whitespace-only, non-string, save/read, missing-field, and legacy `peonModel` behavior.
  - Updated complete normalized-entry expectations with `model: null`.

## Commits

- `eae98d8 feat: persist provider-specific Peon models`
- `413692d docs: record task 3 verification report`

## Commands and results

- `pnpm install --frozen-lockfile` — PASS; installed existing desktop dependencies. No manifests or lockfiles changed.
- `pnpm exec tsx --test tests/electronSettingsMemory.test.ts` — BLOCKED: `tsx` is not declared in this branch, so pnpm reported `Command "tsx" not found`.
- `pnpm exec node --experimental-strip-types --test tests/electronSettingsMemory.test.ts` before implementation — FAIL, 26 passed / 5 failed, confirming the new persistence assertions failed for the missing model field.
- `pnpm exec node --experimental-strip-types --test tests/electronSettingsMemory.test.ts` after implementation — PASS, 31 passed / 0 failed.
- `pnpm test` — BLOCKED by repository configuration: no `test` script exists in `apps/desktop/package.json`.
- `pnpm exec node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` — PASS, 397 passed / 0 failed.
- `pnpm exec tsc --noEmit` — PASS.
- `git diff --check` — PASS.
- `bash .claude/hooks/doc-check.sh` — PASS.
- `bash .claude/hooks/worktree-check.sh` — PASS.

## Concerns

- The exact brief commands using `tsx` and `pnpm test` cannot currently run because this branch has neither the `tsx` dependency nor a `test` package script. Equivalent documented Node test commands pass.
- Node test execution emits existing `MODULE_TYPELESS_PACKAGE_JSON` warnings; no warning-related files were changed.
