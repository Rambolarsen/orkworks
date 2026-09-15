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

The Unix adapter compares `ProcessGroup`, `RegisteredRoot`, and `Launchd`
candidates. Registered roots retain native birth identity, parent ancestry,
process-group/session diagnostics, and bounded live descendant census. Signals
are sent only after a fresh birth-identity comparison; PID mismatch is
reported as unresolved/PID reuse. Unix target behaviors cover session creation,
new process groups, forked descendants, daemonized/reparented descendants,
and silent targets. The launchd candidate is explicitly unsupported throughout
this fixture; no native lifecycle is claimed.

The Linux verified-image launch seam no longer clears `FD_CLOEXEC` in the
parent. It clears the flag only in the child `pre_exec` callback immediately
before `/proc/self/fd/*` exec. Linux staging cleanup remains RAII-armed until
the final digest verification and only then commits the staging guard.

## Commands and results

All commands below were run on this macOS host with bounded tool windows:

- `rtk cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check` — PASS after the delimiter repair.
- `rtk cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml` — initially failed with `E0282` for the descendant map type and an unreachable-code warning from the macOS cfg; PASS after the cfg/type repair.
- `rtk cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --all-targets -- -D warnings` — initially failed on `clippy::needless_return` in the macOS fail-closed branch; PASS after the cfg-expression repair.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates -- --nocapture` — initially failed with 2 passed / 1 failed because the Linux-only cleanup-race test lacked its Linux cfg; PASS on rerun with 2 tests.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — PASS, 39 tests across 7 suites in 0.96s.
- `rtk ps aux` — no matching cargo or fixture process remained after testing; no command hung or required interruption.

The prior parent implementation also had five macOS admission failures during
default `Supervisor::prepare` (`InvalidValue { field: "control endpoint" }`).
The permission guard is retained, but the parent-vs-current admission result
was not rerun in this bounded round; therefore no new claim is made about that
baseline.

## Concerns and limits

- This is macOS-host execution evidence only; no native Linux or Windows
  runtime evidence is claimed.
- The actual launchd bootstrap/kill path was not exercised. The candidate is
  explicitly unsupported throughout this fixture; this is not evidence that
  launchd passes the matrix.
- The process-group candidate is rejected for the macOS PTY/session row after
  cleanup returned `EPERM`; it must not be selected as the ownership boundary.
- The default supervisor admission failures above remain an open macOS
  baseline concern and should be resolved before relying on the generic
  supervisor path for further cross-platform fixture work.
