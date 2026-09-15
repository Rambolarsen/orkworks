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
and silent targets. The generic launchd candidate is fail-closed when no
launchd job has been bootstrapped.

The Linux verified-image launch seam no longer clears `FD_CLOEXEC` in the
parent. It clears the flag only in the child `pre_exec` callback immediately
before `/proc/self/fd/*` exec. Linux staging cleanup remains RAII-armed until
the final digest verification and only then commits the staging guard.

## Commands and results

The following checks passed before this fix round (on the parent implementation):

- `rtk proxy cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates -- --nocapture` — PASS, 3 tests.
- `rtk cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml` — PASS.
- `rtk cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check` — PASS.
- `rtk cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --all-targets -- -D warnings` — PASS.
- `rtk proxy cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — FAIL: five macOS `tests/admission.rs` cases failed during default `Supervisor::prepare` with `InvalidValue { field: "control endpoint" }`. This was observed before the permission-guard change; it was not reproduced after that change in this interrupted round.

Fix-round verification was intentionally bounded when the user requested an immediate stop:

- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates -- --nocapture` — FAIL at compile time in `src/platform/unix.rs` (the new `match` arm needs a braced block around the assignment and `ExitState::Running`). No runtime test result was obtained.
- `rtk cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml -- --nocapture` — NOT RUN after the fix-round edits.
- `rtk cargo check --manifest-path crates/process-ownership-fixture/Cargo.toml` — NOT RUN after the fix-round edits.
- `rtk cargo fmt --manifest-path crates/process-ownership-fixture/Cargo.toml -- --check` — NOT RUN after the fix-round edits.
- `rtk cargo clippy --manifest-path crates/process-ownership-fixture/Cargo.toml --all-targets -- -D warnings` — NOT RUN after the fix-round edits.
- `rtk ps aux` followed by fixture/cargo filtering — no matching cargo or fixture process remained; no command is currently hung.

## Concerns and limits

- This is macOS-host execution evidence only; no native Linux or Windows
  runtime evidence is claimed.
- The actual launchd bootstrap/kill path was not exercised in this run. The
  generic candidate remains explicitly unsupported until supplied a temporary
  plist/job; this is not evidence that launchd passes the matrix.
- The process-group candidate is rejected for the macOS PTY/session row after
  cleanup returned `EPERM`; it must not be selected as the ownership boundary.
- The fix-round branch is committed with the compile blocker recorded above;
  it is not a passing verification result.
- The default supervisor admission failures above remain an open macOS
  baseline concern and should be resolved before relying on the generic
  supervisor path for further cross-platform fixture work.
