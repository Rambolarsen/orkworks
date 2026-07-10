# Cap PTY Partial-Line Persistence Buffer Design

## Goal

Prevent newline-free PTY output from growing sidecar memory without bound while
preserving terminal-history content.

## Problem

`start_session_runtime` accumulates PTY bytes in `persist_buffer` and turns
them into history records only after it finds a newline. A process that emits a
long progress stream without newlines retains every byte until it exits.

## Decision

Keep newline-delimited persistence unchanged. Add a 64 KiB maximum to the
in-memory partial-line buffer. When a newly received chunk leaves the buffer at
or above that limit without a newline, move the buffered bytes into one history
record, clear the buffer, and continue processing later output.

The record is decoded with the existing lossy UTF-8 conversion used for normal
history lines. This keeps bounded memory and retains all output, including a
UTF-8 sequence split across the flush boundary. The terminal's live WebSocket
stream remains byte-for-byte unchanged; only fallback persisted history may
have a synthetic record boundary.

## Alternatives

- Drop older partial-line bytes: bounds memory but loses history.
- Persist raw bytes independently of line history: preserves exact layout but
  expands the storage protocol beyond this bug fix.

## Scope

- Add a small, testable helper in `crates/orkworksd/src/runtime/session_runtime.rs`
  that drains complete lines and flushes a capped partial record.
- Use it in the PTY output path before existing Peon and persistence handling.
- Add Rust tests for ordinary newline-delimited output, newline-free output
  exceeding the cap, and a UTF-8 sequence split at the cap.

## Non-goals

- Change PTY, WebSocket, replay, or metadata protocol behavior.
- Drop output or add a new persistence format.
- Change the existing 10,000-record on-disk retention limit.
