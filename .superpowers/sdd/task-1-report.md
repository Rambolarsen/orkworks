# Task 1: Atomic resume admission hardening

## Scope completed

- Added the runtime-only `SessionHandle.resume_in_progress: bool` field and initialized it to `false` at every construction site.
- Made the stale-handle resume admission critical section mutable under `AppState.sessions`:
  - an existing claim conflicts;
  - the claim is set only after attachment, persisted-ended metadata, and tracked-PID checks permit replacement;
  - the replacement runtime handle begins unclaimed (`false`).
- Corrected the stale endpoint fixture so its in-memory handle is active/running while persisted metadata is ended.
- Added the concurrent endpoint regression. It starts two resumes for the same PID-free stale handle and asserts exactly one `200 OK`, one `409 CONFLICT`, and one final registered, unclaimed handle.

## Test evidence

Red (before implementation):

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
error[E0609]: no field `resume_in_progress` on type `SessionHandle`
```

Green (elevated verification requested because the shared target directory lock was unavailable in the sandbox):

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
PASS

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_replaces_unattached_ended_stale_handle
PASS

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_admits_only_one_concurrent_stale_handle_resume
PASS; observed statuses [200, 409]
```

`rtk git diff --check` also exited successfully.

## Formatting and concerns

- `rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` was not clean because it reports broad, pre-existing formatting drift in unrelated sidecar sources. I did not reformat unrelated code.
- No metadata schema, external API shape, or lifecycle vocabulary changed.
- The requested replacement handle begins with `resume_in_progress: false`; metadata is synchronously reset to `creating` immediately afterward, before any awaited startup work.

## Critical review correction: claimed replacement lifetime

- Replaced the validate/claim-then-replace sequence with `try_install_claimed_resume_handle`, which constructs the replacement first and installs it with `resume_in_progress: true` while holding `AppState.sessions`.
- The same atomic installer covers both stale-handle replacement and no-handle admission. An already claimed replacement conflicts before another runtime can be installed.
- The runtime ownership claim now stays set through startup and the live runtime. `complete_session_ending` clears it only after terminal metadata and the in-memory terminal state are finalized; the existing startup-error path enters terminal finalization and therefore releases the claim there too.
- The stale endpoint regression now uses an active/running in-memory handle, keeps the fake resumed process live long enough to assert its claim, kills it, and observes the claim clearing only after finalization.
- Added a barrier-controlled admission regression where two callers both hold the same ended-metadata observation before admission; exactly one installs a runtime, one conflicts, and the sole installed runtime is claimed.

Fresh verification:

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_admission_installs_one_claimed_runtime
PASS: 1

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_replaces_unattached_ended_stale_handle
PASS: 1

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml finalize_session_ending_times_out_to_fallback_snapshot
PASS: 1

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_handle_conflicts
PASS: 1

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_session_rejects
PASS: 2

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
PASS: 590

rtk cargo build --manifest-path crates/orkworksd/Cargo.toml
PASS with one existing dead-code warning covering two unused methods

rtk git diff --check
bash .claude/hooks/doc-check.sh
bash .claude/hooks/worktree-check.sh
PASS
```

Formatting remains subject to the same pre-existing repository-wide `cargo fmt --check` drift recorded above; unrelated files were not reformatted.

## Final claim review: admission and finalization edges

- Made persisted-ended metadata and an empty tracked-PID slot unconditional resume-admission prerequisites, including when `AppState.sessions` has no handle for the ID.
- Added separate no-handle regressions for active metadata and a tracked PID. Both failed before the guard moved outside the handle lookup and pass afterward.
- Made an existing `ending` handle conflict until its terminal finalizer completes. The deterministic lifecycle regression rejects premature replacement, completes the old finalizer, then admits a claimed replacement and verifies it remains non-ended.
- Added a real resume-endpoint startup-failure regression using an overridden nonexistent command. It observes HTTP 500, waits for terminal finalization, and verifies the claim clears with final `error` / `ended` state. Removing the claim clear made this test time out and fail; restoring it returned the test to green.
- No public API, metadata schema, or lifecycle vocabulary changed.

Final verification:

```text
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml resume_
PASS: 26

rtk cargo build --manifest-path crates/orkworksd/Cargo.toml
PASS with the existing dead-code warning for two unused metadata methods

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
PASS: 594

rtk bash .claude/hooks/doc-check.sh
rtk bash .claude/hooks/worktree-check.sh
PASS
```

`rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` still reports the pre-existing broad formatting drift in unrelated sidecar code, so no bulk formatting rewrite was applied.
