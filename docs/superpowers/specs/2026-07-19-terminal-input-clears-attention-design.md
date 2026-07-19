# Terminal Input Clears Attention

## Purpose

When OrkWorks successfully forwards user input to a live terminal session, the
session must stop advertising the stale `Needs you` state immediately. This is
a universal terminal-session behavior, not a Claude Code special case.

## Decision

Accepted terminal input transitions an alive session to:

- `observedStatus: working`
- `attention: working`
- `metadataSource: process`
- `metadataConfidence: 1.0`

The transition occurs in the shared accepted-input bookkeeping path, after the
input has been accepted for delivery to the PTY. It applies to every harness.
Queued input transitions only when it is actually dispatched; dropped or
undelivered input makes no state change.

Later deterministic harness reports and valid Peon observations retain their
normal authority to replace the process-derived working state.

## Rationale

The existing output-gated fallback leaves `Needs you` visible until the PTY
emits qualifying output. That is inaccurate after the user has responded and
can remain stale when output is delayed, absent, or split around terminal echo
chunks. Input delivery itself is the reliable evidence that the user-facing
request has been handled.

## Testing

Regression coverage will verify:

1. Enter-terminated accepted input clears hook-sourced `Needs you` immediately.
2. Single-key accepted input clears it immediately.
3. Queued input changes state only upon dispatch, while dropped input does not.
4. The updated in-memory state and persisted metadata agree.

## Scope

This changes only sidecar session-state bookkeeping. It introduces no UI,
protocol, harness-configuration, or terminal-input automation changes.
