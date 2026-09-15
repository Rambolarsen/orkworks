# Task 5 report — cleanup races, foreign ownership, and relaunch barriers

Status: fix round 2 implemented in the fixture scope. No production
multi-workspace recovery or runtime replacement behavior was changed.

## Changes

- Supervisor cleanup retains the inference-generation barrier across
  `Supervisor` destruction and unresolved cleanup. Only a successfully recorded
  complete-exit receipt releases it.
- Cleanup unresolved results retain final surviving identities and include the
  generation plus specific observer and termination diagnostics.
- The launch race uses an injected owner-loss checkpoint after paused-root
  creation and before registration/release. Cleanup remains bounded by a 5 s
  graceful phase followed by a 5 s owned-termination phase.
- `ForeignOwner` now holds a real Unix `flock` or Windows `LockFileEx`
  advisory lease and stores heartbeat plus metadata in one atomically replaced
  state snapshot. Windows cross-target compilation covers the adapter code.
- The matrix serializes inference-barrier cases and checks generation-bound
  stale tickets/records, immediate relaunch admission before and after
  authenticated complete-exit adoption, two open generations, foreign-state
  survival, and supervisor death while an older inference root is observed
  live.
- Pre-issued inference tickets are rechecked immediately before root creation,
  registration, and release; a fresh supervisor with the same generation but a
  different rendezvous nonce cannot clear the original barrier.
- Cleanup exposes monotonic phase timings, uses deadline-aware observation and
  termination adapter seams, caps/deduplicates diagnostics, and filters
  termination errors against the final observation before returning survivors.
- The launch-loss regression is now a concurrent paused-root interleaving
  driven by `LaunchInterleaveLatch`, rather than owner loss before spawn.
- The matrix includes competing advisory-lease contention and unresolved-reply
  frame encoding.

## Verification

Host: macOS Darwin 25.6.0 arm64 (`aarch64-apple-darwin`), Rust/Cargo 1.96.0,
debug profile. Supervisor tests required elevated execution because the
default sandbox rejected the fixture's loopback control socket.

Exact commands and results:

```text
cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml
exit 0

cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check
exit 0

cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml --tests
exit 0

cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --tests -- -D warnings
exit 0

cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml --target x86_64-pc-windows-gnu --tests
exit 0

cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --target x86_64-pc-windows-gnu --tests -- -D warnings
exit 0

git diff --check
exit 0

cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture
exit 0; 3 passed, 0 failed; 5 launch-dependent tests were cfg-skipped on macOS

cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml
exit 0; 46 integration tests passed, plus zero-test unit/doc targets. The
cleanup matrix contributed 3 applicable macOS tests.

```

The focused and full test commands were run with elevated loopback permission.
The round 2 focused command passed with the same 3/0/5 result after all code
changes.

## TDD and limitations

The cleanup matrix contains focused regression tests for the review findings.
The prior Task 5 RED run failed because `foreign_owner.rs` did not yet exist.
During round 2, the Windows-target clippy pass first found a boxed-local lint;
that was fixed, then the final check and clippy commands above passed.

The phase instrumentation records a 5 s graceful deadline and a bounded
escalation deadline; the silent target is the graceful-ignore fault. Adapter
implementations must honor the deadline-aware observation seam; the default
compatibility implementation cannot forcibly cancel an arbitrary blocking
third-party adapter call, which is not present in this fixture.

The five launch-dependent matrix rows were not run on macOS because the
pre-existing Task 4 host adapter rejects the native `setsid`/PTY launch path.
Linux and Windows native runtime tests were not run in this environment; the
Windows result above is compile/clippy-only and makes no native runtime claim.
`ForeignOwner` uses a dedicated heartbeat thread, not a separately spawned
helper process, so this fixture proves OS lease and atomic snapshot semantics
but does not claim an independent-process scheduler/liveness proof. The
generation barrier and receipts remain fixture-only evidence.
