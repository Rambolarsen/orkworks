---
type: Implementation Plan
title: Workspace memory edit ownership
description: Serialize workspace-memory edits without waiting under application guards before SQL cleanup integration.
tags: [orkworks, sessions, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Workspace memory edit ownership

Prerequisite under [#1243](https://github.com/Rambolarsen/orkworks/issues/1243), after #1458, within the accepted [storage design](../specs/2026-11-01-sqlite-session-storage-design.md#deletion-and-retention). Workspace settings retain MetadataStore authority.

## Outcome and boundary

The existing `workspace.json` owner must serialize a complete read/modify/write with a nonblocking owned edit permit. Selection producers reserve before reading or performing harness revocations. A concurrent permit returns WouldBlock immediately; no writer waits while holding projection/workspace/prompt locks. Existing readers remain unchanged. Mutation reads distinguish missing from malformed/unreadable memory; failure must not reset settings or acknowledge a write.

Keep the permit in a narrow metadata submodule, backed by shared process-local state owned by each MetadataStore instance. An owned permit can survive a caller scope and eventually be retained by the SQL cleanup runner. All existing writes participate; selection APIs propagate contention and persistence errors, and legacy retention clear reads and conditionally updates under one permit. Startup normalization reserves before reading and propagates failures. A same-workspace reopen reuses the adopted memory owner only when its path, metadata root and retained workspace lease match, so rebuilding metadata readers cannot bypass an admitted edit.

No SQL cleanup/progress bit, path anchoring, durability qualification, migration, production selector, recommendation cleanup or final deletion is included. The next stage must independently bind the original directory/lease and synchronize a bounded replacement before SQL progress. The permit alone grants no SQL or filesystem deletion authority.

## Delivery and checks

- [x] TDD: a held owner rejects conflicting selection/harness/retention mutations without file or prompt side effects; after release, edits succeed and preserve unrelated fields. Separate stores do not block each other.
- [x] Implement owner and route existing mutation helpers through it. Selection and harness mutation acquire before read and propagate failure; malformed memory is preserved. Existing best-effort reader behavior stays unchanged.
- [x] Test real write/read failures and missing-memory initialization, harness revision checks, failed harness writes and retry scheduling, plus independent acknowledged terminal input while a permit is held.
- [x] Run focused regressions, full repository verification and current-diff medium review.
- [ ] Record PR CI and owned cleanup at terminal state on the PR/tracking issue.

## Plan review

Ready within existing metadata ownership and accepted storage contract. Inspected both live selection producers, startup normalization, all workspace-memory write call sites and retention clear. Review exposed startup false-success and same-workspace owner bypass; both were reproduced with real workspace open under a fake home before correction. A shared blocking mutex was rejected because a future SQL fsync would make a live writer wait with application guards held. A permit acquired only at final write was rejected because a stale read could overwrite a prior cleanup; production read/modify/write producers must reserve before reading.

Dependencies2(existing metadata/application), blast radius2(workspace-memory writers), state changes2(same JSON schema; failure becomes explicit), reversibility1(revert with production still JSON), uncertainty2(controlled contention/I/O tests), total9. No new storage architecture or format is introduced. Unknown-field preservation and captured-directory replacement belong to the subsequent SQL cleanup facade, not this typed legacy replacement interface.

Final evidence: nine focused tests passed. The required startup-normalization
finding was reproduced in two failing fake-home workspace-open regressions,
corrected and cleared by both original medium reviewers. Final full repository
verification passed: 3,064 Rust unit tests (18 existing ignored), seven integration,
31 OpenCode reporter, one Electron ABI proof, desktop type-check/build and
1,694 tests (three existing skipped), docs 1,202 checks/build and diff/doc/worktree
checks. Public-doc reminder needs no installer/feature claim change: existing
settings APIs now return actual failures, and internal architecture/validation
explain this prerequisite and the remaining source-only SQL gate.
