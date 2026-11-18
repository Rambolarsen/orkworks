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
- [x] Open one PR for #1893; leave #1894 and #1895 as separately owned work.

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
was outstanding at the initial handoff; PR #1896 subsequently merged after
current-session review and passing required checks.

## Repository inspection reuse (#1894)

The owner requested continuation after #1893 merged in PR #1896. This phase
starts from `abb7262d` and changes only branch-document scan setup, not session
update scheduling. `Store::scan` opens one repository per request and borrows it
for root/identity/HEAD inspection and cache-miss discovery. No repository handle
enters the persistent scan cache. Discovery's final identity re-open and the
projection's independent location/workspace validation remain fresh. TTL,
selection generation, scan budgets, pins and containment rules are unchanged.

A regression first failed with five counted repository opens on a cold scan,
versus the expected two. The counted seam covers branch-document identity,
store setup and discovery opens; it excludes independent base/content helpers,
private scratch repositories, and the former `git::worktree_root` open. It is
not a count of all filesystem activity or all process-wide repository opens.

The same manual fixture benchmark ran before and after the change: temporary
repositories, one linked worktree, 800 synthetic branch config entries, five
rounds of 20 calls per case, debug test build. Warm cases prime the cache first;
forced cases bypass it. Source changes between runs are the borrowed-handle
implementation; timing results also include normal machine/load/cache noise.
The final after run did not overlap another test command from this session.

| 20-call fixture | Counted opens before → after | Median elapsed before → after |
| --- | --- | --- |
| One shared worktree, warm | 40 → 20 | 325.641 → 182.686 ms |
| Two distinct worktrees, warm | 40 → 20 | 332.166 → 193.303 ms |
| One worktree, forced discovery | 100 → 40 | 990.088 → 946.804 ms |

These fixtures show less repeated setup work, especially cache hits. They do
not establish a whole-app CPU improvement; forced discovery still performs the
existing bounded filesystem/history work. No live session was restarted, no
running sidecar replaced, and no cache freshness contract relaxed.

Reproduce the timing diagnostic (ignored by ordinary CI; no timing threshold):

```bash
CARGO_BUILD_JOBS=2 cargo test --manifest-path crates/orkworksd/Cargo.toml \
  --bin orkworksd document_scan_repository_reuse_benchmark -- --ignored --nocapture
```

The ordinary tests assert operation counts, next-request branch/detached-HEAD
changes, separation of linked worktrees, and unavailable removed worktrees.
Focused checks passed: 57 branch-document tests (one manual benchmark ignored),
35 projection tests (three existing ignored). Existing projection regressions
exercise input responsiveness and changed branch/location/runtime/workspace
rejection before publication. Full verification and current-diff review are
recorded in the implementation PR; #1895 remains separate design-gated work.

The full local Rust unit run completed with 3,600 passed, two failed and 22
ignored. The failures were outside this diff: startup work-signal expiry and
SQL-origin batch storage expiry. Both passed in isolated reruns. Their original
failures and rerun evidence are recorded in issue #957; this is not a claim
that the full local unit suite passed.

The remaining verifier stages passed separately: Rust integration, reporter
checks, desktop type/native/tests/build, documentation validation (2,056 tests)
and build, and diff/doc/worktree checks. Current-session `/code-review low`
found no actionable issue in code head `46d9b2fb`; GitHub Codex independently
reported no findings for the same head in PR #1909. GitHub's separate AI
security scan could not run because its monthly quota was exhausted.
