## Task 3 local implementation report

Implemented capability-derived integration participation in `SettingsModal`:

- Removed the hard-coded `INTEGRATION_HARNESS_IDS` allowlist.
- Mounted integration status for every selectable harness with a non-null resolved `integration` capability, regardless of active state; this includes Aider and leaves unsupported tools neutral through the existing status path.
- Updated the source-contract tests to pin capability-derived participation and reject per-harness ID exceptions.

Verification:

- `node --experimental-strip-types --test tests/providersPanel.test.ts tests/newSessionDialogState.test.ts`: 24 passed, 0 failed.
- `pnpm exec tsc --noEmit`: passed.
- `git diff --check`: passed.

Note: the planned subagent implementation could not start because the platform reported the subagent usage limit. This scoped change was completed locally.

## Review-fix follow-up — 2026-08-29

Addressed the Task 3 review finding in the existing capability-derived UI:

- `HarnessIntegrationSection.tsx` now gates install/reinstall actions on the saved integration status being enabled.
- Disabled integration-capable tools stay visible, but disabled `absent` and disabled non-owned `drifted` states no longer offer install or reinstall actions.
- Disabled OrkWorks-owned non-absent registrations stay on the uninstall cleanup path with explicit cleanup copy.
- `providersPanel.test.ts` adds focused source-contract coverage for disabled install/reinstall suppression and disabled owned cleanup retention.

Verification:

- `node --experimental-strip-types --test tests/providersPanel.test.ts`: 19 passed, 0 failed.
- `pnpm exec tsc --noEmit`: passed.
- `git diff --check`: passed.
- `bash scripts/doc-check.sh`: passed.
- `bash .claude/hooks/worktree-check.sh`: passed.
