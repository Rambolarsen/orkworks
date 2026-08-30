# Task 1 report

## Status

Task 1 is complete in `/Users/froomiebot/workspace/orkworks-active-coding-tool-hook-toggle`.

## Requirements coverage

### `apps/desktop/src/harnessIntegrationPresentation.ts`

- Added `IntegrationDisplayState` with the required semantic appearances:
  `off`, `neutral`, `healthy`, `needs-you`, `error`, and `in-progress`.
- Added `ActiveHarnessIntegrationResult` and `ActiveHarnessSaveResult`.
- Added `deriveIntegrationDisplayState(...)` with the Task 1 precedence:
  in-progress overlay, operation failure, status-unavailable error, current
  diagnostic, trust/ownership/registration conditions, then healthy or
  unsupported outcomes.
- Preserved the existing `isAttentionSignal(...)` and
  `shouldShowInstalledConfirmation(...)` helpers.

### `apps/desktop/src/orkworksWindow.d.ts`

- Added the typed `ActiveHarnessSaveResult` window contract.
- Added
  `saveActiveHarnessesWithIntegrations(ids: string[]): Promise<ActiveHarnessSaveResult>`
  to `window.orkworks`.

### `apps/desktop/electron/preload.ts`

- Added the duplicated preload-side `ActiveHarnessSaveResult` contract.
- Added the IPC bridge:
  `saveActiveHarnessesWithIntegrations(ids) =>
  ipcRenderer.invoke("save-active-harnesses-with-integrations", ids)`.
- Kept the existing direct integration status/install/uninstall methods in
  place, as required by the brief.

### Tests

- `apps/desktop/tests/harnessIntegrationSection.test.ts`
  now covers:
  - healthy installed full coverage
  - enabled absent integration
  - Codex `needs_trust`
  - disabled owned cleanup remaining
  - unsupported enabled tool
  - limited Aider coverage
  - status unavailable
  - operation failure precedence over status diagnostics
  - neutral in-progress state
  - explicit assertion that the semantic state is the literal `needs-you`
- `apps/desktop/tests/providersPanel.test.ts`
  now pins:
  - the preload bridge name and IPC channel
  - the typed `ActiveHarnessSaveResult` declaration in `orkworksWindow.d.ts`

### Documentation

- Updated `docs/agents/architecture.md` because the preload contract gained a
  new `window.orkworks` method, which the repo instructions treat as a
  required architecture-doc update.

## TDD record

### Red

Command run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
```

Observed failure:

```text
SyntaxError: The requested module '../src/harnessIntegrationPresentation.ts' does not provide an export named 'deriveIntegrationDisplayState'
```

Also failed source assertions:

```text
✖ preload exposes the combined active-harness save IPC bridge
✖ orkworksWindow declares the typed combined active-harness save result
```

### Green

Command run from `apps/desktop/`:

```bash
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
```

Observed result:

```text
ℹ tests 29
ℹ pass 29
ℹ fail 0
```

## Verification run on August 29, 2026

### Focused tests

```bash
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
```

Observed result:

```text
ℹ tests 29
ℹ pass 29
ℹ fail 0
```

### Type-check

```bash
npx tsc --noEmit
```

Observed result:

```text
exit 0
```

### Repo closeout checks

```bash
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Observed result:

```text
doc-check: no output, exit 0
[worktree-check] Repo-wide worktrees needing a decision (not necessarily yours — only act on branches you own):
  • active-coding-tool-hook-toggle (/Users/froomiebot/workspace/orkworks-active-coding-tool-hook-toggle): merged into main — remove worktree + delete branch
```

## Commit

- `2771fde` — `feat: define coding tool integration toggle states`

## Concerns

- `git commit` initially failed with a transient lock at
  `/Users/froomiebot/workspace/orkworks/.git/worktrees/orkworks-active-coding-tool-hook-toggle/index.lock`;
  the lock disappeared on re-check and the retry succeeded without manual
  cleanup.
- `worktree-check` reports this branch/worktree as already merged into `main`,
  so after the broader handoff it likely needs the usual remove-worktree and
  delete-branch cleanup by the owner.

## Current Review-Fix Report - August 29, 2026

### Scope

- Fixed Task 1 review findings only.
- Modified:
  - `apps/desktop/src/harnessIntegrationPresentation.ts`
  - `apps/desktop/tests/harnessIntegrationSection.test.ts`
  - `apps/desktop/tests/providersPanel.test.ts`

### Review Findings Addressed

1. Failed integration operations now take precedence over `status unavailable`.
2. `stale_workspace` is ignored by the display contract instead of surfacing as user-actionable `needs-you`.
3. Tests now cover both precedence cases and use a stricter preload/window contract assertion for `ActiveHarnessSaveResult`.

### Verification

Red run after adding regressions, before the production fix:

```bash
cd apps/desktop
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
```

Observed result:

```text
exit 1
fail 2
- deriveIntegrationDisplayState keeps failed operations actionable even when status refresh is unavailable
- deriveIntegrationDisplayState ignores stale_workspace outcomes in the display contract
```

Green run after the production fix:

```bash
cd apps/desktop
node --experimental-strip-types --test tests/harnessIntegrationSection.test.ts tests/providersPanel.test.ts
npx tsc --noEmit
```

Observed result:

```text
focused tests: exit 0, pass 31, fail 0
tsc: exit 0, TypeScript: No errors found
```

Repo closeout checks:

```bash
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```

Observed result:

```text
doc-check: exit 0, no output
worktree-check: exit 0, no output
```
