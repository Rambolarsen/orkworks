# PR #351 Review Fix Report

## Review finding

`ProviderSettingsSection` reset every provider draft when the parent `providerSettings` object identity changed. That discarded an unblurred model edit when an unrelated provider setting committed.

## Fix

- Added pure `synchronizeProviderModelDrafts` synchronization logic in `apps/desktop/src/providerPresentation.ts`.
- `ProviderSettingsSection` now tracks the previously committed provider models and preserves only drafts that differ from those committed values.
- Committed provider model changes are synchronized, and additions/removals follow the provider list.
- Added focused regression tests in `apps/desktop/tests/providersPanel.test.ts`.
- No Rust files changed.

## Verification results

All commands were run from `apps/desktop/` unless noted.

| Check | Result |
| --- | --- |
| `node --experimental-strip-types --test tests/providersPanel.test.ts tests/peonModelPicker.test.ts` | PASS — 23 tests, 23 passed, 0 failed |
| `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs` | PASS — 404 tests, 404 passed, 0 failed |
| `npx tsc --noEmit` | PASS — exit code 0 |
| `git diff --check` | PASS — exit code 0 |
| `bash .claude/hooks/doc-check.sh` | PASS — exit code 0 |
| `bash .claude/hooks/worktree-check.sh` | PASS — exit code 0 |

The Node test runner emitted existing `MODULE_TYPELESS_PACKAGE_JSON` and `FORCE_COLOR` warnings; no test failures resulted.
