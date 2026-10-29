---
type: Implementation Plan
title: Terminal input metadata I/O
description: Measured terminal-input blocking boundaries and a proposed cancellation-safe execution owner.
tags: [orkworks, runtime, performance, metadata]
status: draft
workflow_status: verified-awaiting-integration
---

# Terminal input metadata I/O

Proposed lifecycle design and implementation plan for
[issue 334](https://github.com/Rambolarsen/orkworks/issues/334).
The owner approved this command-lifecycle design on 2026-10-29.
Implementation and local verification are complete; PR integration remains.

## Outcome and scope

Metadata reads and writes reached by terminal-input preparation and accepted
input bookkeeping must execute outside Tokio worker threads. Retain delivery
acknowledgement as the authority, reset reservation timing, sensitivity and
output-boundary snapshots, existing metadata transactions, and runtime fences.

Authority comes from the [MVP metadata and harness contracts](../../../specs/orkworks-mvp.md),
[runtime contracts](../../agents/runtime-contracts.md#metadata-protocol),
[responsive input ADR](../../adr/0075-responsive-pty-input-delivery.md), and
[identity reset contract](../../agents/harness-integration-contracts.md).
The MVP's current user-override and prompt-authority rules supersede broader
historical wording in the committed-input design.

Keep the existing bounded terminal queue, input/resize ordering and independent
stop path. No batching, persisted format changes, new execution permissions,
automatic retry or native-capability activation. This scope excludes unrelated
metadata I/O in plan validation and recommendation-store admission checks.

## Traced baseline

Baseline: `11ca93890be36b2d46b8f17673b3f1c3196c39c0`, macOS arm64,
Rust debug test profile, 2026-10-29.

| Path | Synchronous work still on async callers |
| --- | --- |
| Before dispatch | `capture_pending_terminal_input` snapshots sensitivity/output revision, parses a copy on line completion, reads the persisted harness, and reserves a declared identity reset. Reset preparation can read metadata again. Ordinary non-terminating frames avoid those reads. |
| Accepted delivery | `record_input_after_delivery` calls `record_terminal_input_impl`; `mark_committed_input_working` conditionally runs an attention transaction. Ordinary keys already in working state update memory without that transaction. |
| Submitted non-sensitive line | Checks persisted harness reset commands, resets topic when applicable, or persists last user input and optionally seeds the topic under the label epoch guard. |
| Accepted reset | Commits the prompt identity reset after the input effects; a queued replacement identity and prompt can be applied through further metadata transactions. |
| Rejected delivery | Cancels prepared reset reservations; does not record accepted-input effects. |
| WebSocket queued input | Uses the same preparation and completion before dispatching the next queued action. |
| WebSocket disconnected with a pending write | Detaches a waiter for the pending command, then runs the same synchronous completion after acknowledgement. |
| Approved prompt | Uses the same preparation and completion. Exact-runtime variants validate under `projection_lock`; recommendation admission checks and the actual enqueue are synchronous under that fence. |

PTY writes and terminal-size persistence already have blocking-pool owners.
`transition_session_status` likewise offloads its full synchronous transaction.
Those existing jobs do not cover the input paths above.

## Measurements and reproduction

The [probe patch](../../validation/terminal-input-io-probe.patch) adds temporary
tests using the existing prompted-session fixture. Apply it only to a clean
owned checkout of the baseline:

```bash
git apply --unidiff-zero docs/validation/terminal-input-io-probe.patch
cargo test --manifest-path crates/orkworksd/Cargo.toml input_metadata -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml input_metadata_handoff_cost_probe -- --ignored --nocapture
```

Use the required RTK shell wrapper in agent execution. Remove the temporary
test addition after measuring. The implementation promotes these responsiveness
assertions into its active suite; the patch retains the original baseline probe
and must not be applied over the implementation.

The responsiveness probes replace one temporary metadata JSON file with a Unix
FIFO. An independent OS thread restores the regular file and releases its
reader when the async executor signals it, or after a 250-ms watchdog. A
current-thread Tokio runtime must release the read without that watchdog.
Both probes failed the desired assertion on the baseline:

| Probe | Result |
| --- | --- |
| Submitted-line preflight | Watchdog release; elapsed 254.464833 ms |
| Accepted single-key bookkeeping | Watchdog release; elapsed 253.861500 ms |
| 2,000 ordinary frames, inline preparation/bookkeeping | 5.499042 ms |
| 2,000 ordinary frames, two awaited blocking-pool jobs per frame | 50.178667 ms |

The handoff probe warms the initial attention transition and then types without
newlines. It isolates dispatch overhead; it does not exercise the proposed
runtime gate, a real PTY, release builds, sustained concurrency or production
latency. About 22.3 microseconds of extra cost per frame is a reason to start
without batching, not a throughput guarantee. The FIFO establishes a real
blocking-read boundary under controlled I/O delay; it does not demonstrate an
observed production disk stall. Baseline compilation succeeded with existing
warnings; no full-suite pass is claimed.

## Proposed command owner

Keep orchestration in `runtime/terminal_runtime.rs` and add the per-runtime
async input gate to `SessionRuntime` in `runtime/session_runtime.rs`.

1. Capture the exact runtime identity, control sender and gate before awaiting
   preparation. Both normal terminal input and approved prompts use this owner.
   Each owner holds one input permit from preparation through final bookkeeping;
   it cannot prepare a second reset or parse the shared input buffer concurrently.
2. Before spawning, reserve one of two per-runtime owner slots without waiting.
   Excess approved requests return conflict; rejected terminal input uses the
   existing input-dropped notification. The two slots bound active plus waiting
   owners, including reconnects. Own the complete admitted command in a Tokio task.
   Caller cancellation detaches the result waiter, not delivery or bookkeeping.
   The existing WebSocket queue creates only one such input owner at a time;
   it keeps collecting bounded queued actions and forwarding output and Close.
   Approved callers retain their existing authorization/reservation checks.
3. Await control-channel capacity before preparing any reset. Then, in one
   blocking-pool job, acquire the projection fence, validate the captured
   runtime, and run the existing synchronous preflight as one operation. Return
   its sensitivity/output boundary and reset reservation. Do not retain any
   synchronous lock across an await. A stale runtime rejects preparation.
4. Within that same job and projection fence, revalidate caller authority
   through the synchronous enqueue. Preserve recommendation admission adjacent
   to enqueue. Never re-resolve a sender after resume. If admission or
   enqueue fails, run fenced cancellation of the prepared reset before releasing
   the input gate. Workspace replacement and resume share the projection fence.
5. Await the writer-owned acknowledgement. In a blocking-pool job, acquire the
   same projection fence, revalidate runtime, and run existing completion,
   including reset commit/cancellation, as one synchronous operation. A stale
   owner applies no effects to a replacement runtime. Preserve the label-epoch,
   transition, workspace and session lock ordering inside the existing helpers.
6. Release the input gate only after bookkeeping completes. The next queued
   input/resize cannot overtake that completion. Stop/kill bypasses the gate;
   a blocked native write retains its existing acknowledgement ownership.

Do not hold a blocking-pool job while awaiting PTY input. There are at most two
metadata jobs in the primary command sequence, sequentially; no fire-and-forget metadata job
per keystroke and no new metadata queue. Pool scheduling can still delay the
affected input, but its synchronous work no longer occupies the async worker.
Existing reset-completion retries retain their backoff and one scheduled owner;
each attempt now offloads synchronous commits and validates its captured runtime
under the same projection fence. A regression reproduced native-ID retirement
by an old retry after runtime replacement and now protects that boundary.
Existing shared synchronous locks remain a contention limit; the change must
not claim to eliminate every possible async mutex wait elsewhere in the app.

Preflight failure means no delivery. After confirmed full delivery, bookkeeping
failure is logged without turning success into a delivery failure that would
permit duplicate retry. Runtime shutdown is still bounded by the existing
native-write limitations; process exit need not persist an in-memory reset
reservation. A prepared reservation belongs to the owner until failed-delivery
cleanup or successful completion, never to an abandoned join receiver.

The command owner and gate extend ADR 0075's ownership/sequencing consequences.
After approval, record a dated amendment there before changing production code;
no source-priority, protocol or native-qualification decision changes.

## Delivery and checks

- [x] Owner approved the proposed owner/gate design; ADR 0075 amended before code.
- [x] Promote the two FIFO probes into regression tests. Cover runtime
  replacement during pool queuing, caller cancellation before acknowledgement,
  bounded command ordering, stale attachment rejection, saturation without
  disconnect, and reset-retry fencing. Exercise the actual owner for accepted
  reset with an early replacement report. Existing rejected-reset, admission,
  sensitivity/output-boundary and input/resize tests pass in the full suite.
- [x] Implement the owner and migrate direct, queued, detached and approved
  input callers together. Keep authority validation and its effect sink in the
  same fenced blocking closure; never validate before queueing a job alone.
  Adapt the admission callback's ownership only as necessary to preserve its
  exact enqueue boundary; do not broaden recommendation-store scope.
- [x] Require the current-thread responsiveness tests to pass without watchdog
  release. Re-run the ordinary-frame cost probe against the actual implementation
  and report its absolute cost; investigate material regression before batching.
- [x] Run focused input/runtime and recommendation-delivery tests, then the
  scoped Rust build, full tests, fmt and clippy plus `scripts/verify-repo.sh`.
  Provision the repository Node version and frozen desktop dependencies for
  signed-knowledge tests. Run documentation checks/build and close-out hooks.
- [x] Complete the current-code-diff review at the effort warranted by the
  concurrency/lifecycle changes. Tie off an active matching recommendation
  through the authenticated API if one is verified; keep issue 334 open until
  implementation and required checks are complete.

## Review and recovery

Least-confidence investigation: cancellation and authority across new awaits.
Inspection found reset preparation before enqueue, callback validation under
`projection_lock`, and a detached WebSocket completion waiter. A plain pair of
`spawn_blocking` calls would not by itself own cleanup if the caller disappeared;
the proposed task owner/gate addresses that gap. Tests must prove the ownership
rather than infer it from a successful input run.

| Dimension | Rating | Evidence / treatment |
| --- | --- | --- |
| Dependencies | 2 | Existing runtime/application owners and Tokio; no new dependency. |
| Blast radius | 4 | Shared terminal and approved-input path; migrate together with focused regressions. |
| State changes | 2 | Live ownership/gate only; persisted schema and metadata semantics unchanged. |
| Reversibility | 1 | Revert source amendment/code; no migration or persisted gate to recover. |
| Uncertainty | 3 | Baseline read stalls reproduced; cancellation and contention behavior require implementation evidence. |

Total: 12/25; this is a review aid, not authorization. Alternatives considered:
`block_in_place` does not support current-thread runtimes; offloading only file
writes splits decisions from their commits; awaiting PTY delivery inside a
blocking job retains a scarce worker during potentially indefinite native I/O.
A new batching/coalescing policy lacks supporting performance evidence.

Plan review: **Ready; owner approved implementation on 2026-10-29.**
Fresh builder check found unbounded waiting-owner assumptions and an extra job
if capacity were reserved after preflight. Resolved with two fail-fast owner
slots and capacity reservation before the fenced prepare/enqueue job.
Scope, source contracts, cancellation, stale runtime, transaction boundaries,
ordering and recovery have explicit deliverables/checks. The remaining unknown
is verified behavior of the new owner, which the implementation tests must
resolve before claiming the defect fixed. Full local verification passed as recorded below. Production latency has not
been measured.

## Implementation progress

The command owner and bounded slots are implemented across direct, queued,
disconnected and approved inputs. The attachment identity is captured at claim,
not resolved after a queued command becomes runnable. The existing reset-retry
owner also uses the blocking pool and captured runtime fence. Admission closures
own their captured authorization context and retain validation adjacent to
actual enqueue. No new retry or batching policy was added.

Verification so far: 101 terminal-runtime tests passed, one manual probe ignored;
three reset-retry/registry tests passed, including the stale-runtime regression.
The controlled metadata-read regressions no longer require watchdog release
(about 4 ms in the first focused green run; 9–11 ms alongside the broader terminal
suite). The cancellation regression failed before ownership was introduced;
the reset-retry native-ID preservation regression failed before fencing.

Ruling: preserve the writer acknowledgement if preparation unwinds after an
admission callback enqueues input. Callback failure alone cannot prove failed
PTY delivery and must not free a recommendation for duplicate submission.

The actual owner probe (including task ownership, runtime gate, enqueue and fake
writer acknowledgement) measured 147.957625 ms for 2,000 ordinary frames versus
6.449292 ms inline in the same debug test: about 74 microseconds total per frame,
or 71 microseconds of added cost. This is a scheduler-responsiveness tradeoff,
not a claim of faster keystroke completion or production throughput.

Current-diff review: `/code-review medium`, baseline
`11ca93890be36b2d46b8f17673b3f1c3196c39c0`, reviewed three-file diff SHA256
`46e72f8a8ba3a226685dcbbe6ff31db8e7a6cf5e471e333f33a32475d3fa9f15`.
Medium effort is warranted by concurrency/lifecycle ownership. No outstanding
material findings. One overload-disconnect finding was resolved by distinguishing
Busy from delivery rejection; the actual WebSocket regression failed before the
fix and passed afterward, proving that dropped-input overload preserves output.
Clippy completed with no warnings in added/modified Rust line ranges; existing
repository warnings remain. Initial full repository verification passed; the
final full rerun includes the reviewed overload correction.

Documentation drift review: the `docs/user/`, `docs/home.md` and `README.md`
path-based notices need no prose change. Their installation, shell architecture
and existing dropped-input guidance remain accurate; this change affects the
execution owner behind those contracts. The canonical architecture concept and
ADR carry the changed ownership details.

Final local verification passed on the reviewed code: `bash scripts/verify-repo.sh`
under Node 22.23.4, including Rust fmt/build, 2,309 Rust unit tests with 12
ignored and all seven integration tests, OpenCode reporters, desktop type-check,
native ABI repair, 1,611 desktop tests with two skipped, desktop build, 460 docs
checks and the docs build, diff checks and both close-out hooks. Clippy completed
separately with no warnings in added/modified Rust line ranges. Code commit
`633dbbfb7554eb98f9f27474687fae1c47cef913` has exactly the reviewed diff hash.

No active Taskmaster recommendation was established as the source of this
user-directed issue work. Issue 334 remains linked for PR integration.

Post-rebase review refresh: current code commit
`b350a1e7e36352caa023a0245d4660807f271801`, reviewed head
`24be160da4e475cba50a4c941e68bdfbfb8189fb`, base
`ec9ebfb2275a41b3dcdb25d34b252649918f8452`. The owned added/removed Rust lines
are identical. Upstream changes adjust Git-context availability during session
creation; they do not affect input ownership/fencing. `/code-review medium`
confirmed no outstanding findings with current three-file diff SHA256
`1d159fcac09f44672eb3feb5b063a2449134d8d858a1561a2adac62f59bfb8dc`.
The local full pass preceded this rebase; integrated PR CI verifies the refreshed
base. No claim that whole source files remained byte-identical is made.
