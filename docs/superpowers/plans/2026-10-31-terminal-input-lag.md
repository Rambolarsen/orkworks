---
type: Implementation Plan
title: Restore terminal responsiveness during session projection
description: Approved capacity-scan separation and bounded sensitivity implementation with deterministic concurrency regressions.
tags: [terminal, performance, concurrency, testing]
status: draft
workflow_status: active
---

# Restore terminal responsiveness during session projection

Outcome: ordinary input reaches its current PTY while a slow list computation
is paused, without stale writes or protected-input disclosure. Track
[#1222](https://github.com/Rambolarsen/orkworks/issues/1222). Read the
[design](../specs/2026-10-31-terminal-input-lag-design.md) and
[ADR 0102](../../adr/0102-terminal-projection-input-fence.md) first.
The owner approved this concrete design and implementation on 2026-11-01. Implement inline; use one independent current-diff review before
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

**Design review completed before owner approval**, independently reviewed by the available Luna tier
(`gpt-6-luna`, medium effort), 2026-10-31. Runtime implementation followed owner approval on 2026-11-01.

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

The reviewer confirmed these changes resolve the material blockers. The owner
subsequently approved the concurrency/security design on 2026-11-01.

## Execution record

- 2026-11-01: owner approved implementation; existing owned worktree and PR #1223 retained.
- Initial control-revision/shared-output/sensitivity tests failed against missing behavior.
- The real oversized-input test then failed at the pre-dispatch sensitivity assertion.
- Capacity scanning now has a private production computation dependency for deterministic
  blocked-scan tests; no runtime test-only race-injection flag was introduced.
- Ruling: keep scan admission/cache in the existing process-local session projection
  state, rather than adding a field to every AppState fixture. This stays independent
  of the projection/input fence and avoids a second registry.

- Focused checks passed: 24 projection tests (one manual probe ignored), 103 input
  tests (one ignored), 112 Peon tests (one ignored), 19 capacity-state tests and
  the retained-line cache regression. The old-fence mutation made the real
  input-during-scan regression fail as intended; repaired source was restored.
- Initial full unit run: 2,336 passed, 28 failed, 16 ignored. One four-second
  assessment-provider timeout poisoned the shared fake-PATH test mutex, causing
  the downstream failures. The originating test passed in isolation; full
  verification must rerun without overlapping compilation before completion.

- Independent `/code-review medium` found one admission-scope issue: release the
  analysis cache guard immediately after compute instead of after the entire list.
  The correction was verified by the reviewer; no remaining critical or required
  findings. Reviewed Rust diff SHA-256:
  `7622b41faaa4839c16097fba9333f7584148710f25e548c26c8c1784c3ff017a`.
- Build warnings retain the existing owned-output and detector interfaces for
  compatibility and reference tests; they now lack production callers after the
  shared immutable analysis path was introduced. Existing HTTP test response/drop
  warnings are unrelated to this diff. No warning indicates a failed check.

- Supported Node 22 full repository verification passed, including 2,369 Rust
  unit tests, integration tests, reporter tests, desktop checks/build and docs
  checks/build. A final-source rerun follows the one-line admission release.
- Documentation-currency warnings were inspected: public onboarding, installer
  claims and README product scope do not change. The source-only responsiveness
  repair is documented in the architecture/runtime concepts and ADR amendment;
  the installed/running sidecar has not been replaced or live-qualified.

- Final reviewed source passed `bash scripts/verify-repo.sh` under Node 22,
  including the admission-guard correction. Rust: 2,369 unit tests passed,
  zero failed, 16 ignored; all integration, reporter, desktop and documentation
  checks passed. The repeated verifier required restoring `fs-ext` to the host
  Node ABI after the earlier Electron check, without a dependency change.
- Implementation/review is complete; required PR CI and owner-controlled live
  long-session validation remain separate gates. Taskmaster returned no active
  recommendations to tie off.

- External Codex and Copilot review of `62e49b39` found two valid regressions.
  Both were reproduced before correction: the newest-first sensitivity snapshot
  omitted previously protected prompt lines after three/four trailing ordinary
  rows, and mirroring derived capacity into the live metadata baseline kept
  attention capped during the clearing response. Sensitivity now checks all
  five selected, already-budgeted rows; capacity stays in `CapacityState`.
  New regressions exercise protected metadata and repeated-list recovery after
  accepted input or hook clear, including authoritative alive-session metadata.
- One strengthened recovery fixture initially inherited dead/ended lifecycle
  defaults from the metadata helper. It was corrected to alive before the final
  verifier rerun; both targeted regressions now pass. No product lifecycle
  behavior changed.
- Fresh independent `/code-review medium` found no remaining critical or
  required findings after those corrections. Current Rust diff SHA-256:
  `190283e0a0157fe8275cc56382255cdf530e544d351fdb8a1db3149a2e25c8f3`.

- Review-fix source passed the full `scripts/verify-repo.sh` run: 2,371 Rust
  unit tests passed, zero failed, 16 ignored; all integration, desktop, docs,
  build and close-out checks passed. The public-onboarding warning was reviewed
  against the same source-only/live-validation boundary described above.
  Substantial review cycle 1/3 will request fresh Codex and Copilot review on
  the committed fix head; no additional runtime changes are pending.
