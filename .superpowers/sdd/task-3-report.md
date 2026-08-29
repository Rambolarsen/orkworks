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
