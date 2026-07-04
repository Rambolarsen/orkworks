# Peon Idle Start Timer Design

> **Date:** 2026-07-04
> **Scope:** Fix issue `#119` so new sessions do not flash `idle` before the idle timeout elapses

## Goal

Make timer-based idle detection respect `idle_timeout_secs` for sessions that have not produced terminal output yet.

## Why This Change

The current idle timer in `crates/orkworksd/src/runtime/peon_runtime.rs` treats a missing `last_output` entry as immediately idle. A freshly started session moves to `status = running` before its first PTY chunk arrives, so the next peon tick can mark it `observed_status = "idle"` even when the configured timeout has not elapsed.

That produces false attention state in the sessions list during harness startup and slow first-output cases.

## Chosen Approach

Use the existing `last_output` timestamp as the idle timer origin for new running sessions.

When the terminal runtime transitions a session to `running`, it should seed `state.peon.last_output[session_id]` with `Instant::now()` if peon is enabled. After that:

- real PTY output continues to refresh the same timestamp
- input-triggered label inference continues to override the timestamp exactly as it does today
- idle detection can keep using the existing `last_output <= idle_deadline` rule without special-casing "no output yet"

This is preferred over adding a second "running since" map because the bug only needs a sensible initial timestamp. A separate map would add more state to keep in sync without changing the behavior we want.

## Behavioral Rules

### Session start

When a terminal session becomes `running`:

- record `Instant::now()` in `state.peon.last_output` for that session if peon is enabled
- do not set `observed_status`

### Before first output

If the session remains silent:

- it must not be marked `idle` until at least `idle_timeout_secs` has elapsed since the `running` transition

### After output or input

Existing behavior stays intact:

- PTY output refreshes `last_output`
- terminal input that already updates peon timing continues to do so
- sessions already carrying a non-`None` `observed_status` remain excluded from the idle timer path

## Files Affected

- `crates/orkworksd/src/runtime/terminal_runtime.rs`
- `crates/orkworksd/src/runtime/peon_runtime.rs`

The implementation should stay local to runtime code. No API, metadata schema, or frontend changes are needed for this fix.

## Testing Strategy

Add Rust tests in `crates/orkworksd/src/runtime/peon_runtime.rs` covering:

- a just-started running session with no output is not marked `idle` on the next timer tick
- a running session that stays silent past `idle_timeout_secs` is marked `idle`

The first test should fail against current behavior and pass only after the runtime seeds the initial timestamp.
