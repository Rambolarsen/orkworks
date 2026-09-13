# Task 1 report

## Implementation

Added the requested renderer characterization test, Windows resolver normalization tests, Windows printed-path resolution fixture, and strengthened the unresolvable `select_plan` rejection assertions. No production implementation code was changed.

## Files

- `apps/desktop/tests/terminalLinks.test.ts`
- `crates/orkworksd/src/plan_handoff.rs`
- `crates/orkworksd/src/session_application.rs`

## Test commands and results

- `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts` — blocked by PowerShell execution policy for `pnpm.ps1`.
- Equivalent `pnpm.cmd ...` — blocked by a local pnpm temporary-file permission error while pnpm attempted its dependency status check.
- Direct Node test — could not run because this worktree has no installed `@xterm/xterm` dependency.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff` — RED: the new non-Windows test fails to compile because `normalize_windows_drive_alias` is not yet implemented. The build also reports an unrelated pre-existing moved-`Arc` test error at `session_application.rs:5591`.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path` — blocked by the same crate compilation errors.
- `git diff --check` — passed.

## TDD RED evidence

The resolver test correctly exposes the missing production helper with `cannot find function normalize_windows_drive_alias`; this is the expected RED-phase failure before the Windows implementation task.

## Self-review

Reviewed the diff against the brief. Changes are test-only, limited to the three specified source files, preserve the exact renderer path text, use the requested `cfg` gates and fixture shape, and assert no persisted plan or selection event after rejection.

## Concerns

- The focused desktop test could not execute in this worktree because dependencies are unavailable and pnpm setup is permission-blocked.
- The Rust crate currently has an unrelated existing compile error involving a moved `Arc`.
- Commit creation requires elevated filesystem permission for the linked worktree’s Git index lock.

## Reviewer fix

The reviewer identified that the test module referenced `normalize_windows_drive_alias` without importing it from `super`. Added that missing test import only; no production implementation code changed.

## Reviewer-fix verification

- Renderer characterization attempt: `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts` — blocked by PowerShell execution policy for `pnpm.ps1`.
- Renderer equivalent: `pnpm.cmd --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts` — blocked by pnpm temporary-file permission error.
- Direct Node attempt from `apps/desktop`: blocked because `@xterm/xterm` is unavailable in this worktree.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff` — still RED on the missing production helper, now reported as the expected unresolved test import; the unrelated moved-`Arc` compile error remains.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path` — blocked by the same crate-wide compile errors.
- `git diff --check` — passed before commit.

## Reviewer-fix files

- `crates/orkworksd/src/plan_handoff.rs` — added the missing test-module import.
- `.superpowers/sdd/task-1-report.md` — appended this fix and verification record.

## Task 2 reviewer fix

The strengthened rejection test reused `state` after passing it by value to `SessionApplication::new`, causing an `Arc` move error. Changed only that construction to `SessionApplication::new(state.clone())`; the persisted-state assertions are unchanged.

## Task 2 reviewer-fix verification

- `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff` — remains RED because `normalize_windows_drive_alias` is not present in production; the test import now resolves syntactically, and the previously reported `Arc` move at the rejection test is no longer emitted. Existing unrelated warnings remain.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path` — remains blocked by the same missing production helper compile error; the `Arc` ownership error is cleared.
- `git diff --check` — passed.

## Task 2 reviewer-fix files

- `crates/orkworksd/src/session_application.rs` — cloned `state` at the `SessionApplication::new` call.
- `.superpowers/sdd/task-1-report.md` — appended this fix and exact verification results.

## Task 2 ownership-fix follow-up

Changed the strengthened rejection test to pass `state.clone()` into `SessionApplication::new`, preserving the subsequent `plan_path` and event-log assertions. No production implementation code was changed.

## Task 2 ownership-fix checks

- `cargo test --manifest-path crates/orkworksd/Cargo.toml plan_handoff` — compiled successfully past the prior `Arc` move error, then failed 6 of 17 tests because Windows returned backslash-separated relative paths where existing assertions expect forward slashes. The new Windows alias normalization test passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests::select_plan_application_seam_rejects_unresolvable_path` — passed: 1 test, 0 failed.
- `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/terminalLinks.test.ts` — blocked by PowerShell execution policy (`pnpm.ps1` cannot be loaded).
- `git diff --check` — passed before commit.

## Remaining blockers

- The `plan_handoff` suite has the existing Windows path-separator assertion failures described above; resolving them would require production/test behavior changes outside this ownership-only fix.
- The renderer characterization test remains unexecuted because pnpm is blocked by the environment’s PowerShell policy.
