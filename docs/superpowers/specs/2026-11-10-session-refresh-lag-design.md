---
type: Design
title: Keep session document inspection outside terminal input fencing
description: Capture and validate session locations without running Git or document inspection under the terminal input fence.
tags: [sessions, terminal, performance, concurrency]
status: draft
workflow_status: approved
---

# Keep session document inspection outside terminal input fencing

Restore typing and terminal interaction during full session refreshes. Track this
repair in [#1222](https://github.com/Rambolarsen/orkworks/issues/1222).
The owner authorized the capture/inspect/recheck approach on 2026-11-10 with
“keep going” after its explanation. This authorizes source implementation and
isolated verification; replacing the running sidecar remains owner-controlled.

## Evidence and boundaries

On 2026-11-09, two live `/sessions` reads returned 764 rows (1,831,032 bytes)
in 4.206 and 2.768 seconds; `/health` returned in 9 and 2 milliseconds.
The native sidecar sample showed `enrich_workspace_with_documents`, repeated
`DocumentLocations::session_root`, JSON reads and worktree validation. Profiles
are in `/private/tmp/orkworks-sidecar-lag.sample.txt` and the matching renderer
sample. No per-keystroke wait or isolated cause of scrolling lag was measured.

The current projection releases its input fence for discovery, but reacquires
it for session-root validation, branch selection reconciliation, document
validation and final Git-context checks. The final pass holds it across all
historical rows. Preserve the existing capacity, metadata and organization
ordering, input ownership, runtime generations, workspace lease identity and
attention authority. This is a bounded follow-up to
[responsive input](2026-10-31-terminal-input-lag-design.md) and
[branch document association](../../adr/0102-branch-document-association.md).

No storage cutover, pagination, retention change, terminal renderer redesign,
new dependency, public API or metadata schema is included. The accepted SQLite
and paged-history work remains separate.

## Capture, inspect and recheck

Keep `SessionProjection` as the owner. Capture a location observation under the
existing projection fence using the current live-or-durable session lookup.
Include session existence, launch CWD, exact worktree binding (root and common
directory), reported CWD, process association and live runtime identity where
present, plus the existing workspace/store ownership identity. Capture only
location-relevant fields: ordinary output/summary/title changes must not
invalidate an otherwise current inspection.

Resolve process CWD, canonical paths, Git root and branch identities outside the
projection fence and workspace/session mutexes. Preserve the existing location
priority and unavailable-bound-worktree behavior. Do not fall back from an
invalid explicit binding to launch CWD. Share repository inspection within one
pass by exact root/common-directory inputs, including unavailable results.
A later validation pass starts with fresh caches; do not reuse pre-inspection
identity at final validation.

Prepare document discovery, no-follow reads, content/history checks and slot
availability outside the input fence. Split the existing combined
`projection_from_scan` path only as needed to distinguish inspection from
publication. Keep the existing store generation, branch-specific selection
serialization, stronger report provenance and pin precedence. A prepared scan
cannot overwrite a newer report, pin or reconciliation. Reuse the store's
existing generation comparison; capture the loaded-selection generation under
its branch lock. At publication, try the branch lock without waiting and compare
that generation before a bounded selection write. On contention or mismatch,
report document inspection unavailable for this pass; never reprepare under
the fence or publish the stale prepared projection.
No waiting for another inspector, repository scan or document read may occur
while retaining the projection fence.

Before publication, inspect the current Git context outside the fence and then
reacquire the fence to compare the captured location inputs and workspace/store
ownership with current inputs. Changed location, removed/replaced runtime or
replaced workspace rejects the prepared result through the existing unavailable
path. Recheck once more before returning the complete projection, retaining the
existing protection against stale complete lists. Do not retry inspection under
the fence or return an authoritative empty list after validation failure.

External Git/working-directory changes are not serialized by the projection
fence today. Retain fresh before/after inspection and final context comparisons;
do not claim atomic authority over external Git writers. Content reads and
review handoffs keep their own exact-selection revalidation at action admission.
If current Git context cannot be established, report unavailable conservatively.

Metadata reads, organization projection and bounded durable branch selection
publication keep their current authority/order. Measure their residual fenced
work rather than claiming that all input latency is eliminated. Record fence
wait/hold durations through test/benchmark instrumentation without session
content, credentials or production high-volume per-key logs.

## Verification and recovery

Deterministic synchronization tests pause worktree validation and document
validation separately, after capture and before publication. In each case,
exercise the real terminal input owner and require delivery and bookkeeping to
finish before releasing inspection. Cleanup must release paused workers even
when an assertion fails. Existing capacity/input and discovery tests remain.

During the pause, cover workspace replacement at the same path, session deletion,
resume/runtime replacement, worktree rebinding,
branch change and a newer pinned/reported document. Reject stale publication,
preserve the newer selection and never publish authoritative removals from a
failed pass. Rebinding back to exactly the same location/root/branch does not grant new
display authority and does not need a new location revision. Same-ID runtime
replacement is distinguished with the existing runtime identity.

Verify unchanged title/output churn does not starve refreshes, invalid bindings
stay unavailable, missing documents cannot appear openable, shared worktrees
coalesce expensive validation, and final validation uses fresh identities.
Benchmark synthetic large history with shared and distinct worktrees; report
request duration, inspection counts and residual fence timing. Timing is
supporting evidence, not a flaky unit-test threshold. An optional real-history
benchmark copies into temporary storage and never mutates production records.

Rollback is a code revert and owner-controlled restart. No migration or metadata
repair is required. Passing isolated tests does not establish that the currently
running app contains the repair or that live typing/scrolling has improved.
