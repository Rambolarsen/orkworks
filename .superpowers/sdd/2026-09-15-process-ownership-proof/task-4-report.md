# Task 4 report: Unix process-ownership candidates

Date: 2026-09-15
Host: macOS Darwin 25.6.0, arm64 (`aarch64-apple-darwin`)

## Changed files

- `crates/process-ownership-fixture/src/platform/unix.rs`
- `crates/process-ownership-fixture/src/platform/macos_launchd.rs`
- `crates/process-ownership-fixture/src/platform/mod.rs`
- `crates/process-ownership-fixture/src/supervisor.rs`
- `crates/process-ownership-fixture/src/targets.rs`
- `crates/process-ownership-fixture/tests/unix_candidates.rs`
- `crates/process-ownership-fixture/scripts/run-launchd-fixture.sh`

## Implementation

The fixture compares `ProcessGroup`, `RegisteredRoot`, and `Launchd` as
candidate labels, but all three are explicitly rejected before launch. Portable
Unix process-group signaling and registered-root per-PID cleanup lack a
kernel-bound identity primitive against PID/PGID reuse; launchd has no tested
bootstrap/bootout lifecycle. The retained experimental code is not selected
and no untrusted group or PID is signaled. The matrix records every row as
unsupported with durable evidence rather than claiming ownership coverage.

The Linux verified-image launch seam no longer clears `FD_CLOEXEC` in the
parent. It clears the flag only in the child `pre_exec` callback immediately
before `/proc/self/fd/*` exec. Linux staging cleanup remains RAII-armed until
the final digest verification and only then commits the staging guard.

## Commands and results

All commands below were run on this macOS host with bounded tool windows:

- `rtk cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check` — PASS.
- `rtk cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml` — PASS.
- `rtk cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --all-targets -- -D warnings` — PASS.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates -- --nocapture` — first run FAIL, 3 passed / 3 failed because three rejection assertions expected a mechanism substring absent from the rejection reason; corrected and rerun PASS, 6 passed.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — PASS, 43 tests across 7 suites in 0.90s.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — PASS, 43 tests across 7 suites in 0.79s.
- `rtk git diff --check` — PASS.
- `rtk ls -ld crates/process-ownership-fixture/target crates/process-ownership-fixture/target/debug` — `755` for `target`, `700` for `target/debug`; both remain accessible and owner-writable.
- `rtk ps aux` with cargo/fixture filtering — no matching cargo or fixture process remained; no command hung or required interruption.

## Concerns and limits

- This is macOS-host execution evidence only; no native Linux or Windows
  runtime evidence is claimed.
- No candidate launched on this macOS host, so there is no PTY `EPERM` result
  and no native launchd run to report.
- Task 4 does not produce a deployable Unix ownership mechanism. Native Linux
  and macOS runtime proof remains unresolved until a kernel-bound process
  ownership primitive or a fully tested macOS service boundary is supplied.
- The admission permission guard remains armed through the substitution test
  and restores the staged executable directory permissions on all exits; the
  repeated full fixture suite passed with it enabled.
