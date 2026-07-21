# Committed Terminal Input Implies Working

Date: 2026-07-21
Status: approved for implementation

## Problem

The desktop can retain `Needs you` after a user has answered a terminal prompt.
The current fallback waits for later qualifying PTY output before changing the
attention state to `working`. That creates an inaccurate and sometimes
indefinitely stale user-facing state.

## Constraint

**Committed user input must always result in status `working`.**

Here, committed means the sidecar has accepted the input for delivery to a
live terminal session. The rule applies uniformly to all harnesses and input
shapes, including a single key, a newline-terminated response, pasted text,
and ordinary shell input.

## Design

The accepted-input bookkeeping path is the single transition point. When it
receives non-empty committed input for a live session, it immediately:

1. Sets the in-memory observed status and attention to `working`, with source
   `process` and confidence `1.0`.
2. Clears the in-memory `needsUserInput` flag and prompt-specific fields
   (`detectedQuestion` and `suggestedOptions`) left by the answered prompt.
3. Writes those same status, source, confidence, and cleared prompt fields to
   session metadata.
4. Leaves only a later harness attention report for a new prompt free to set
   `Needs you` again.

This is an explicit exception to the normal metadata-source priority order:
accepted terminal input is direct evidence of the session's current state, so
it overwrites any prior source, including `user`.

The old pending-output fallback is removed from this transition: later output
is not evidence required to clear a prompt that the user has already answered.
Any pending echo/expiry work signal created for the committed input must be
cleared or left unused; it must not be able to delay, leave, or restore a stale
`Needs you` state. This does not remove independent output-based lifecycle
inference.

The transition only applies to live sessions, because only those can accept
terminal input. A successfully committed input takes precedence over a prior
metadata source, including a user override: it is direct process evidence that
the terminal has resumed work.

## Error handling

Input remains non-committed if it is rejected or dropped before the accepted
input bookkeeping path. In that case no attention transition occurs. If
metadata persistence is unavailable, the existing in-memory state still
reflects the accepted input; the surrounding persistence error handling
continues to report the failure rather than claiming durable state.

## Tests

- A committed single-key answer changes hook-sourced `Needs you` to `working`
  immediately, without PTY output, and clears the persisted prompt fields.
- A committed newline-terminated answer does the same.
- Ordinary committed input applies the same state regardless of harness
  activity-hook capability or the previous metadata source.
- Rejected or empty input does not change the attention state.
- A subsequent harness `waiting_for_input` report can set `Needs you` again.

## Out of scope

This does not infer model work from terminal output and does not change how
non-input lifecycle transitions are derived.
