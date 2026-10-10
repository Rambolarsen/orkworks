---
type: "Implementation Plan"
title: "PR 764 Review Fixes Implementation Plan"
description: "Implementation plan: PR 764 Review Fixes Implementation Plan."
tags: ["orkworks", "plans"]
---

# PR 764 Review Fixes Implementation Plan

> **For agentic workers:** Execute inline in this session. Keep the regressions test-first and review the resulting PR head before merge.

**Goal:** Preserve Codex CLI argument semantics and honor the owned-process-group shutdown grace period.

**Architecture:** Reject resume routes with shared options after the `resume` subcommand so native eligibility falls back to the unchanged direct-launch path. During shutdown, check the owned Unix process group, not only its leader, before ending the two-second grace period.

**Tech Stack:** Rust, Tokio tests, Unix process groups.

**Spec:** `docs/superpowers/specs/2026-10-03-codex-native-approval-status-design.md`

## Uncertainty Checkpoint

- Least confident: process-group liveness after the leader exits can vary across Unix systems. A macOS regression fixture now exercises the leader-exits-first case with a delayed descendant `SIGTERM` handler.
- Project blind spot: the previous descendant test covered forced cleanup in `Drop`, but not the two-second graceful shutdown path; the parser test also encoded a post-resume option order as valid despite the spec requiring order preservation.

## Global Constraints

- Preserve option order and precedence; every native-mapped argument must have equivalent semantics.
- Keep process cleanup bounded to two seconds graceful, then force only owned processes.
- Keep the native compatibility table empty and production rollout fail-closed.

---

### Task 1: Preserve resume argument ordering

**Files:**
- Modify: `crates/orkworksd/src/runtime/codex_native.rs`

**Interfaces:**
- Consumes: `parse_arguments(&[String]) -> Option<Route>`
- Produces: `None` for a resume route followed by `-c`, `--config`, `--enable`, or `--disable`, selecting existing direct launch.

- [x] Change the parser test to require the post-resume `--disable other` route be rejected while retaining a test for valid options before `resume`.
- [x] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::codex_native::tests::accepts_only_complete_mapped_routes_and_preserves_order -- --exact --nocapture` and confirm the new assertion fails.
- [x] Reject shared options after a parsed resume command.
- [x] Rerun the exact parser test and confirm it passes.

### Task 2: Honor graceful shutdown for owned descendants

**Files:**
- Modify: `crates/orkworksd/src/runtime/codex_native/launch.rs`

**Interfaces:**
- Consumes: `OwnedProcess::shutdown(&mut self)` and its owned Unix process group.
- Produces: shutdown waits through the grace period while the owned group remains, even if the leader exits.

- [x] Add a Unix regression fixture whose leader exits on `SIGTERM` while a descendant records completion after a short `SIGTERM` grace handler.
- [x] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::codex_native::launch::tests::shutdown_waits_for_owned_descendants_after_leader_exits -- --exact --nocapture` and confirm the completion marker is absent under current code.
- [x] Wait for owned process-group liveness until the deadline, then force-kill only if the group remains.
- [x] Rerun the exact shutdown test and confirm it passes.
- [x] Treat `EPERM` from a process-liveness probe as evidence that the owned process still exists.
- [x] Recheck that graceful shutdown no longer waits on the retained leader zombie.
- [x] Keep native eligibility on Linux/macOS, where live process-group probes are implemented; other platforms retain direct launch.

### Task 3: Verify and review

- [x] Run the focused regression tests, the native test group, sidecar formatting, `git diff --check`, and a macOS-targeted sidecar check.
- [x] Run the full serialized `scripts/verify-repo.sh` verifier.
- [x] Run a fresh explicit medium-effort Codex review of the final diff; no actionable findings remained.
- [ ] Recheck current-head CI after pushing the updated branch.
- [ ] Merge only after the repository review and status-check gates pass.
