---
type: Validation Evidence
title: Cross-session metadata lock contention
description: Reproducible two-session evidence separating async worker stalls from shared metadata serialization.
tags: [orkworks, runtime, metadata, performance, validation]
status: stable
---

# Cross-session metadata lock contention

Date: 2026-10-29. Investigation: [#1155](https://github.com/Rambolarsen/orkworks/issues/1155),
following [#1149](https://github.com/Rambolarsen/orkworks/pull/1149).
Baseline: `35078f333f2fb1b2f1144637089c8e7d45e99048`.
This records source-level behavior; it makes no claim about published installers.
No production implementation changes accompany this evidence.

## Method

The [retained diagnostic patch](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/session-lock-contention-probe.patch) adds an
ignored Unix-only test using two sessions in the same real `AppState` and
`MetadataStore`, through the production input owner. Runtime input acknowledgements
are deterministic substitutes; no live PTY or coding harness is involved.

Session A receives an ordinary key and reaches its accepted-input attention
transaction. A FIFO at A's metadata path delays the actual store read by about
250 ms while the transaction owns the shared locks. Only then does session B
submit an ordinary key. A separate Tokio timer, armed before B's call, requests
a 10 ms sleep. The test uses a single-thread Tokio runtime to expose worker
parking deterministically, with the repository's Tokio 1.52.3 in a macOS arm64
debug build.

The second mode calls B's same input function from the blocking pool using
`Handle::block_on`. This is a diagnostic control, not a proposed production
implementation: it consumes another worker and leaves shared serialization intact.

## Observed results

One run produced the following elapsed times:

| B caller | A delayed metadata read | B input completion | Independent 10 ms timer |
| --- | ---: | ---: | ---: |
| Normal async path | 252.017 ms | 253.944625 ms | 253.450583 ms |
| Blocking-pool control | 250.9135 ms | 252.457875 ms | 13.060291 ms |

The probe passed: both inputs returned success, each runtime's input generation
became 1, and each persisted attention value became `working`. This checks
accepted effects in this reproduction; rejected-input and cancellation behavior
were not exercised by this probe.

The normal B call parks the single async worker while waiting for a shared
synchronous mutex. Moving that call to the blocking pool restores timer
responsiveness, but B still takes about 252 ms because it must wait for A's
transaction. Executor responsiveness and per-session completion latency are
separate costs. These timings are diagnostic observations, not a latency budget
or an estimate of production contention frequency.

## Source trace

At the baseline, [terminal input](https://github.com/Rambolarsen/orkworks/blob/35078f333f2fb1b2f1144637089c8e7d45e99048/crates/orkworksd/src/runtime/terminal_runtime.rs)
takes the shared session-map mutex synchronously in `spawn_command_future`,
`submit_approved_input`, and `submit_input_with_admission`, before or around the
blocking owner boundary. `mark_committed_input_working` retains workspace and
session-map guards through `mutate_session_optional`. `with_current_runtime_input`
also retains the global projection guard while applying the transaction.
Consequently, A's slow attention read can delay another session's input intake
and its later metadata work.

The [session driver](https://github.com/Rambolarsen/orkworks/blob/35078f333f2fb1b2f1144637089c8e7d45e99048/crates/orkworksd/src/runtime/session_runtime.rs) handles
`DriverEvent::Output` on an async task. It locks the same session map before
updating replay and sending the output broadcast. Output delivery is therefore
downstream of this lock too. This investigation traced that dependency but did
not measure actual output delay. `flush_output_recency` and resize also acquire
the session map synchronously.

## Recommended next boundary

Evaluate capturing the input target (runtime identity, sender, ordering gate and
admission slots) when the terminal attaches or an approved target is selected,
then reusing it for intake. Keep late authority validation inside the existing
blocking owner. This could remove repeated async map acquisitions without adding
a blocking task per frame or weakening the existing transaction.

That narrower boundary would address worker parking at intake, not cross-session
metadata serialization. Output needs a separately bounded, ordered boundary.
Full session isolation would require narrower shared critical sections or
per-session ownership with explicit workspace/runtime generation fencing.
Dropping existing guards without replacing their atomicity and authority guarantees
is not justified by this measurement. Resolve those design choices and obtain
approval before changing the architecture.

## Reproduction and limits

Copy the patch fixture into an owned checkout of the baseline, then run:

```bash
git apply --check --unidiff-zero docs/validation/session-lock-contention-probe.patch
git apply --unidiff-zero docs/validation/session-lock-contention-probe.patch
cargo test --manifest-path crates/orkworksd/Cargo.toml cross_session_metadata_lock_contention_probe -- --ignored --nocapture
```

The recorded run passed one diagnostic test, with 2,325 other unit tests filtered
out. The full repository suite was not run for this docs-only investigation.
The active Rust source was restored after retaining the patch. The test is
intentionally excluded from the normal suite: it injects filesystem delay and
prints timing observations rather than enforcing a machine-dependent threshold.

A single-thread runtime demonstrates worker parking, not a whole-application
freeze on a production multithread runtime. Live PTY timing, output delivery,
normal workload frequency, tail latency and pool saturation remain unmeasured.
