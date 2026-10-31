---
type: Implementation Plan
title: Restore terminal responsiveness during session projection
description: Proposed fenced projection and bounded sensitivity implementation with deterministic concurrency regressions.
tags: [terminal, performance, concurrency, testing]
status: draft
workflow_status: awaiting-owner-approval
---

# Restore terminal responsiveness during session projection

Outcome: ordinary input reaches its current PTY while a slow list computation
is paused, without stale writes or protected-input disclosure. Track
[#1222](https://github.com/Rambolarsen/orkworks/issues/1222). Read the
[design](../specs/2026-10-31-terminal-input-lag-design.md) and
[proposed ADR 0102](../../adr/0102-terminal-projection-input-fence.md) first.
This plan is not executable until their new concurrency/security choices receive
owner approval. Implement inline; use one independent current-diff review before
PR handoff. No parallel writers or new coding harnesses.

## Deliverables and sequence

1. **Capacity computation outside the input fence.** Own
   `session_projection.rs`, the necessary private observation/cache state in
   `main.rs`/`runtime/session_runtime.rs`, and `capacity_state.rs`'s control
   revision. Add a deterministic stalled-analysis regression through real
   projection and input admission, observe failure, then split capacity capture,
   pure compute and validated apply. Preserve metadata/organization ordering and
   input fences. Validate current workspace/runtime/control/capability identities;
   accepted input and each capacity transition advance the control revision.
   Test continuous output through a paused scan: the complete list still
   publishes, scanned-prefix origins commit, and newer counters remain intact.
   Exercise an owner paused after PTY acknowledgement but before bookkeeping:
   nonblocking `input_gate` acquisition must skip its capacity commit; after
   bookkeeping releases the gate, the advanced control revision must still reject
   older analysis. Also exercise failed delivery, no-op input rechecks, ABA state
   transitions, pending visibility,
   capability change, hook clear, resume, workspace replacement and stream
   removal/recovery. A stale session observation retains current capacity instead
   of rejecting every active-output pass. A replaced workspace rejects the pass.
2. **Exact-key analysis and input sensitivity.** Own private observation/cache
   support in `peon.rs`, `capacity_state.rs`, `session_projection.rs` and
   `runtime/terminal_runtime.rs`. Use immutable shared output snapshots without
   changing existing owned snapshot interfaces for other consumers. Add failing
   scan-count/invalidation tests before caching. Add real password/sensitive-reset
   regressions before moving normalization out of shared locks. Enforce the
   design's 64 KiB fail-closed threshold and output-revision revalidation; retain
   exact existing detector results for stable in-budget inputs. Validate callback
   cancellation, accepted delivery and bookkeeping ownership under contention.
3. **Verification and canonical documentation.** Update the architecture concept,
   applicable runtime-contract wording and ADR acceptance evidence only after
   approval. Run focused tests, then `bash scripts/verify-repo.sh` under Node from
   `.nvmrc` with frozen desktop/docs dependencies. Run docs/worktree close-out
   checks and inspect warnings against the diff. Complete `/code-review medium`
   for this concurrency/input-safety change, then required CI and the ordinary
   explicit maintainer merge path. Do not merge with unresolved current-head
   findings. Keep live restart/testing a separate owner-controlled operation;
   report its absence honestly if unavailable.

## Checks and limits

Tests synchronize phase boundaries and observe actual PTY command delivery,
metadata/provider effects and real detector calls. No sleeps as evidence of
nonblocking, implementation-text assertions or mirrored cache-key tests. Existing
usage-limit, authority, password, resume and PTY cancellation suites remain
mandatory. No record-retention policy change, new platform primitive, dependency,
wire/schema change or automatic restart belongs in this implementation.

Before claiming live recovery, compare input-fence wait/hold and projection
compute time independently on the long-session workload. A faster fresh session
alone is not a reproduction. Source/CI verification is distinct from running
installer or native Windows/macOS qualification.

## Risk checkpoint

- Dependencies: **2** — one Rust owner; existing REST/stream and metadata writers.
- Blast radius: **4** — shared listing/input fences, capacity and sensitivity.
- State changes: **1** — disposable in-memory snapshots/caches, no migration.
- Reversibility: **2** — code revert and controlled restart; no data rewrite.
- Uncertainty: **3** — lock coupling/profile confirmed; capacity control-revision
  coverage and live per-key timings still require implementation evidence.
- Total: **12/25**, descriptive only, not an approval threshold.

Investigated blind spots: moving input around the fence would violate ADR 0075;
therefore only projection compute moves. Raw record limits do not bound logical
buffer size; byte-retention changes are deferred, while sensitivity has an
explicit fail-closed budget. Reusing an observation after resume/recheck/capability
change would be stale; exact keys and commit revalidation are required. Continuous output must not starve list updates: commit captured prefixes when
control state is current, keeping newer counters. A changed workspace cannot
produce an empty authoritative list: use unavailable.

## Review disposition

**Ready for owner approval**, independently reviewed by the available Luna tier
(`gpt-6-luna`, medium effort), 2026-10-31. No runtime implementation yet.

Resolved material findings:

- Output liveness: ordinary output advances counters without invalidating the
  capacity control revision; commit the scanned prefix and retain newer counters.
  A stale session observation retains current committed capacity, not a failed
  complete-list refresh.
- Writer coverage: narrow compute separation to capacity analysis; metadata and
  organization keep existing fenced ordering. The named capacity/application
  mutators own control-revision advancement, including no-op accepted rechecks.
- Delivery/bookkeeping race: nonblockingly check the existing input-owner gate
  during commit, hold its guard through capacity mutation, and verify the accepted
  input revision advances before owner release. A deterministic paused-after-ack
  regression covers the formerly ambiguous interval.

The reviewer confirmed these changes resolve the material blockers. This means
ready for the new concurrency/security design approval, not authority to execute.
