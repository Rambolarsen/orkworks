---
type: Implementation Plan
title: Session projection CPU investigation and delivery
description: Preserve the measured CPU evidence and sequence bounded diagnostics, inspection optimization and refresh separation.
tags: [performance, sessions, sidecar, diagnostics]
status: draft
---

# Session projection CPU investigation and delivery

Reduce background CPU with many open or remembered sessions and only one
working session. This plan starts with reproducible measurement; it does not
claim a runtime fix or authorize changes to freshness contracts.

## Evidence

On 2026-11-18 the local debug daemon PID 31275 showed approximately 36% of one
CPU core after Ollama fell to 0.1%. Two independent five-second macOS `sample`
captures at 06:59:54 and shortly afterward showed repeated workspace session
projection, Git inspection and branch-document discovery. Inclusive stack
counts overlap and include blocked threads: they are not CPU percentages.
The exact source revision of the running binary was not established.

Source inspected at `f15eb7816cb21943a281b75e8670ff6303e7e986` marks session
updates on terminal output, permits coalesced list refresh starts every two
seconds, and uses five-second Git/document scan caches. Repository validation
also occurs outside those cache hits. One working session can therefore cause
whole-list enrichment. The precise invalidation rate was not instrumented.

Raw captures remain local in `/private/tmp/orkworksd-cpu-sample.txt` and
`/private/tmp/orkworksd-cpu-sample-2.txt`. Do not publish raw captures without
checking paths and sensitive content. Ollama activity, UI rendering and other
applications are separate contributors; process CPU does not measure GPU load.

## Deliverables and ownership

1. [#1893](https://github.com/Rambolarsen/orkworks/issues/1893): this session owns
   `scripts/session-cpu-baseline.py`, its tests, this plan and the diagnostic
   runbook. Explicit PID, bounded duration, cumulative CPU divided by measured
   monotonic wall time, identity checks before/after, numeric JSON output.
   No HTTP requests, session reads, signals, restarts or new dependencies.
2. [#1894](https://github.com/Rambolarsen/orkworks/issues/1894): follow-up owner
   measures and reduces repeated Git/document work within existing validation
   passes. Preserve fresh final validation and current TTLs. Add deterministic
   inspection-count regressions before optimizing.
3. [#1895](https://github.com/Rambolarsen/orkworks/issues/1895): design cheap
   status/output invalidation separately from Git/document enrichment. Requires
   a concrete freshness/recovery design and owner approval before runtime work.

Related [#1222](https://github.com/Rambolarsen/orkworks/issues/1222) remains the
input-latency/fence investigation; [#1245](https://github.com/Rambolarsen/orkworks/issues/1245)
tracks future paged history. Neither is closed or activated by this plan.
Authoritative boundaries: [MVP Git context](../../../specs/orkworks-mvp.md#git-and-worktree-context)
and [branch documents](../../../specs/session-plan-review.md#branch-document-extension).
Current branch identity must be resolved on refresh/read; cache optimization
must retain unavailable explicit bindings, pins, containment and stale-result
rejection. No schema, lifecycle, protocol or storage migration belongs to #1893.

## Checks and review

- [x] Test CPU parsing, actual wall interval, executable selection, process
  disappearance/replacement and command failure before implementing the helper.
- [x] Run helper tests and capture a read-only baseline against the explicitly
  selected daemon; report it as diagnostic evidence, not improvement.
- [x] Run repository verification, documentation validation/build and close-out
  checks. Record unrelated environment/check failures without claiming success.
- [ ] Open one PR for #1893; leave #1894 and #1895 as separately owned work.

Plan review: scope is restricted to observation and documentation; no runtime
changes or new dependency. Tests verify wrong-process rejection and measurement
semantics. CPU comparison requires the same build, workspace, sessions and
workload; debug/release and restarted buffers are confounders. Revert the helper
and docs to roll back; it writes no application state. #1895 deliberately remains
design-gated rather than weakening existing validation for speed.

## Baseline implementation evidence

Six helper regressions passed after the initial missing-helper failure. Passive
measurement of PID 31275 returned 4.58 CPU seconds over 10.0098 wall seconds
(45.75% of one core). This later interval is not a controlled comparison with
the earlier top snapshots. No runtime changes or improvement claim. The helper
regressions also run in the existing PR documentation CI job.

Full `scripts/verify-repo.sh` was attempted: after selecting pinned Node 22,
it stopped at missing desktop dependencies in this isolated diagnostics/docs
worktree. No Rust or desktop code changed; full runtime verification is not
claimed. Close-out found another owner’s stale `session-workflow-split`
worktree; it was left untouched. The initial own-branch merged warning was
expected before the first task commit (the branch still pointed at main).

Documentation build passed (2,056 tests, no failures), including metadata,
indexes and rendered-page link checks. Six diagnostic tests and diff/close-out
checks passed. Build emitted circular/large-chunk warnings unrelated to this
text-only site change. The configured Luna reviewer is unavailable in this
session; no stronger-model substitution was authorized. Independent review
remains outstanding at PR handoff.
