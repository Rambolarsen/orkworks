# Task 5 report — cleanup races, foreign ownership, and relaunch barriers

Status: DONE for the fixture scope; native macOS launch-dependent rows remain
not run because the existing Task 4 host path rejects those launches.

Commit: recorded in the final git commit for this task (`test: cover process ownership cleanup races`).

## Scope delivered

- Added `foreign_owner::ForeignOwner`, which acquires an atomic workspace lease,
  writes known metadata bytes/revision, updates a heartbeat independently, and
  exposes disk-backed snapshots without foreign-process self-attestation.
- Added supervisor cleanup serialization: admission closes before the census,
  launch checkpoints fail closed, cleanup uses monotonic five-second graceful
  and five-second owned-termination phases, and complete-exit receipts are
  recorded only after an independent complete observation.
- Unresolved cleanup retains safely named surviving identities and observer/PID
  details in its reason; it never becomes an empty-owner acknowledgement.
- Added authenticated, generation-bound `RendezvousClient` adoption over the
  existing `SupervisorProtocol` response verification.
- Added a fixture-local generation barrier that rejects new inference tickets
  while an older owner-lost generation still has a live inference root, and
  releases that generation only after complete cleanup evidence.
- Added `cleanup_matrix.rs` coverage for owner-loss launch denial, graceful
  escalation, supervisor death before receipt, foreign-state survival, two
  generations, stale events, immediate authenticated relaunch, and the
  older-inference admission barrier.

No production multi-workspace recovery, replacement, reconciliation, Electron,
or `orkworksd` behavior was changed.

## TDD evidence

RED:

```text
rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture
exit 101: tests/cleanup_matrix.rs could not load the not-yet-created src/foreign_owner.rs
```

GREEN on the available host:

```text
rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture
3 passed; 0 failed; 5 launch-dependent tests cfg-skipped on macOS
exit 0
```

The skipped rows are not reported as native macOS passes. The existing Task 4
Unix candidates remain explicitly unsupported on this host, so no macOS PTY,
descendant, or native containment behavior is claimed here.

## Verification

Host: macOS Darwin 25.6.0 arm64 (`aarch64-apple-darwin`), Cargo/Rust 1.96.0,
debug profile. Loopback-dependent test runs required elevated execution because
the default sandbox rejected the supervisor's existing loopback control socket.

- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture` — pass, 3 applicable / 5 macOS-skipped.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — pass, 46 integration tests plus zero-test unit/doc targets; cleanup matrix contributes 3 applicable tests.
- `rtk cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml --tests` — pass.
- `rtk cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check` — pass.
- `rtk cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --all-targets -- -D warnings` — pass, no issues.
- `git diff --check` — pass.

## Limitations

- Linux and Windows native runtime rows were not run in this environment; no
  native behavior on those hosts is claimed.
- macOS launch-dependent cleanup rows were not run because the pre-existing
  host adapter rejects the `setsid`/native launch path, consistent with the
  Task 4 unsupported-candidate result.
- The fixture generation barrier is intentionally fixture-local and does not
  implement production workspace recovery or multi-workspace replacement.
