# PR 764 Review Cycle 6 Implementation Plan

> **For agentic workers:** Execute inline in the authorized PR worktree. Each finding gets a failing regression test, a minimal fix, and focused verification before moving to the next finding.

**Goal:** Fix the three current-head review findings on PR #764 and prepare one fresh review cycle.

**Architecture:** Preserve the existing Codex native observer and metadata ownership model. Capture server death before stopping the owner, classify permanent protocol failures separately from transient disconnects, and replace the metadata store-wide I/O lock with a weakly registered per-session lock reclaimed after the operation and its waiters drain.

**Tech Stack:** Rust, Tokio, `std::sync::Mutex`, Unix process fixtures, existing sidecar test suite.

**Spec:** `docs/adr/0076-codex-owned-native-approval-observer.md`; `docs/superpowers/specs/2026-10-03-codex-native-approval-status-design.md`.

## Global Constraints

- Preserve direct Codex launch unless existing compatibility and listener gates are satisfied.
- Never allow stale native observations to clear attention; permanent protocol failures remain conservative.
- Preserve per-session ordering between ordinary writes, guarded writes, and deletion.
- Do not hold a store-wide metadata lock across filesystem I/O.
- Keep process cleanup bounded and session-runtime-owned.

---

### Task 1: Capture simultaneous native-server exit during TUI finalization

**Files:** `crates/orkworksd/src/runtime/session_runtime.rs`, `crates/orkworksd/src/runtime/codex_native/launch.rs`.

**Interfaces:** Keep `NativeObserverFailure` as the lifecycle result. Make observer shutdown return its failure state after checking server liveness and shutting down the owned server.

- [x] Add a fixture TUI that exits when the fixture server terminates; pause observer polling and reproduce the TUI-first race.
- [x] Run the focused test and confirm it reports `ended` before the fix.
- [x] On observer stop, capture `!native.is_alive()` before `native.shutdown()`; propagate the result into final status/event selection.
- [x] Run focused and related lifecycle tests.

### Task 2: Stop retries for permanent native protocol failures

**Files:** `crates/orkworksd/src/runtime/codex_native.rs`, and the launch fixture only if needed.

**Interfaces:** Add a private retry classifier and retain a terminal native error in `OwnedNativeRuntime`. A successful status observation resets backoff; a successful reconnect alone does not.

- [x] Add regression tests proving permanent protocol errors disable another connection attempt and transient post-connect failures increase backoff across reconnects.
- [x] Run those tests and confirm they fail under the current implementation.
- [x] Treat `Shape`, `Root`, `UnsupportedRequest`, `Authentication`, and `Limit` as permanent; keep `Disconnected`, `Timeout`, and `Stale` retryable.
- [x] Reset retry delay only after a successful observation.
- [x] Run native protocol and observer retry tests.

### Task 3: Allow unrelated sessions to write metadata concurrently

**Files:** `crates/orkworksd/src/metadata.rs`.

**Interfaces:** Add a weakly registered per-session write mutex; keep `session_writes` as the owner map and hold it only while reading/updating one owner entry. Reclaim idle lock entries so the registry does not grow with historical IDs.

- [x] Add regression tests proving unrelated sessions use distinct locks and filesystem I/O occurs before acquiring the global owner-map mutex.
- [x] Run the focused test and confirm it blocks under the existing store-wide mutex.
- [x] Serialize each session operation on its keyed lock; move ordinary file I/O outside the global owner-map lock while keeping guarded-write ownership validation, replacement, and revision advancement ordered on that same lock.
- [x] Run metadata ownership, deletion, and write tests.

### Task 4: Verify the branch and request cycle-6 reviews

**Files:** No additional source files.

- [x] Run Rust format check, focused lifecycle/protocol/metadata tests, full sidecar tests, and sidecar build.
- [ ] Run `git diff --check`, inspect the complete diff, and verify PR head before requesting one Codex review, one Copilot review, and the custom review workflow.
- [ ] Do not merge while actionable findings or the mandatory manual `/code-review medium` gate remain unresolved.
