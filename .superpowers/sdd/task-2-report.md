# Task 2 report

Implemented deterministic fallback work signaling for hookless harnesses.

- Added the ten-second `PendingWorkSignal` state machine with split-echo, ANSI-only, and expiry coverage.
- Submitted non-empty terminal lines arm fallback only where no active-work hook is registered; partial input only remains label-buffer input.
- PTY output now promotes `working` only after qualifying non-echo output and persists that same promotion; terminal typing no longer schedules Peon output inference.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml` passed: 291 tests.

Note: the pending signal is owned by `SessionRuntime`, which is owned by `SessionHandle`, avoiding a broad initialization change across every existing `SessionHandle` fixture.

## Reviewer follow-up

- Moved `pending_work_signal` onto `SessionHandle` and initialized every production path and test fixture.
- Control/ANSI-only output now returns before echo consumption, preserving the submitted echo across redraw sequences.
- Added real PTY terminal input/output coverage for partial input, hook-capable fail-closed behavior, and hookless promotion in both `SessionInfo` and persisted metadata.
- Verification: `cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::session_runtime::tests` — 22 passed.

Formatting note: `cargo fmt --check` reports pre-existing formatting differences across the crate, including files outside this change; no mass formatting was applied.
