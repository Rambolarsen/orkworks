---
type: Implementation Plan
title: Keep terminal input responsive during output timestamp writes
description: Stage output-recency metadata outside the sidecar locks needed for terminal input.
tags: [terminal, performance, concurrency, sessions]
status: draft
workflow_status: active
---

# Keep terminal input responsive during output timestamp writes

The first delivery target is a keystroke in session B reaching its PTY while
session A's output timestamp file sync is stalled. Track this under
[#1222](https://github.com/Rambolarsen/orkworks/issues/1222). Preserve
[terminal input ownership](../../adr/0075-responsive-pty-input-delivery.md) and
[the current projection fence](../../adr/0102-terminal-projection-input-fence.md).

## Evidence

A five-second macOS capture on November 18 found two input-preparation
workers waiting on the projection mutex. An accepted-input worker waited on
the workspace mutex. The output-recency path appeared both waiting for that
mutex and holding it through a revisioned session write, including file sync.
The sample establishes contention and identifies disk I/O under the workspace
lock. The exact lock owner at each instant and individual key-to-echo latency
remain unmeasured.

## Proposed design

1. Bind immediate and delayed output timestamp requests to the workspace store
   and originating runtime generation. Associate the store when the runtime
   starts and rebind live runtimes when the same workspace reopens. The PTY
   driver takes the store identity under its existing session lock. Rebinding
   retires the old pending timestamp and timer, and the old timer can only
   drain pending output for its original store and runtime generation.
2. Read the existing revisioned record, apply the monotonic timestamp check,
   and serialize, stage and sync the candidate outside the workspace and
   projection mutexes. Use the existing same-directory atomic staging
   primitive and preserve provenance and revision behavior.
3. For publication, acquire the workspace and session writer protections in
   their existing order. Verify store identity even for a same-path reopen,
   verify the runtime generation and verify that the record revision and
   identity still match the staged candidate. Try the session writer lane
   without waiting while workspace state is locked. Publish by renaming under
   these checks, release shared guards, then sync the parent directory. A
   completed rename is the publication cutoff. Windows retains its
   write-through replacement inside the publication guard.
4. Discard a stale owner, missing record or obsolete timestamp. Retry a
   same-owner revision conflict or busy writer once from fresh inputs outside
   shared locks. Keep the existing best-effort recency policy and flush cadence.
   Treat a post-rename sync error as a completed publication with uncertain
   durability.

The JSON schema, retention, history writes, input admission and delivery
acknowledgement ordering follow existing contracts. Revert the code to recover
the old behavior. No data conversion is needed.

## Delivery and acceptance

- [x] In the sidecar tests, pause file staging/sync for session A's real
  output-recency transaction. Drive session B through the real input owner,
  with a deterministic PTY acknowledgement substitute. Confirm the input
  reaches B's writer and completes before releasing A. Observe the regression
  fail before implementation. Exercise the directory-sync boundary separately.
- [x] Implement the staged publication path in session metadata transactions
  and pass the origin identity from the output driver. Test a same-path
  workspace reopen, runtime replacement, deletion, conflicting record writes,
  newer timestamps, retired timers and pre/post-rename failures. Confirm neither stale work
  nor bounded retry can overwrite newer data.
- [ ] Run Rust and repository checks, the current-diff review, and required CI.
  Validate key-to-echo under comparable workloads in an isolated session.
  A comparison using the owner's running runtime requires an owner-controlled
  rollout. Report any timing boundaries the measurement cannot separate.

## Review and approval

The owner approved this design on 2026-11-19. Same-path adoption constructs a new metadata reader
and writer lanes, so retaining only the path or advisory lease cannot
authorize publication. The session writer lane prevents two revisions from
publishing over one another. Nonblocking acquisition avoids waiting on that
lane under the workspace mutex. Revision checks rely on writers using the
existing metadata transaction contract, with external file replacement outside
that guarantee. A regression must verify that the workspace mutex is released
before both file and directory sync.

Dependencies 2 (three existing sidecar modules). Blast radius 3 (shared
runtime and metadata path). State changes 2 (same record format and durability
policy). Reversibility 1 (revert and restart). Uncertainty 3 (the live capture
does not attribute all latency or prove the benefit of this slice).
