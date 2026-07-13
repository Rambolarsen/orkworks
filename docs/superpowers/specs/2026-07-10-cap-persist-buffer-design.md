# Cap PTY Partial-Line Persistence Buffer Design

## Goal

Prevent newline-free PTY output from growing sidecar memory without bound while
preserving terminal-history content.

## Problem

`start_session_runtime` accumulates PTY bytes in `persist_buffer` and turns
them into history records only after it finds a newline. A process that emits a
long progress stream without newlines retains every byte until it exits.

## Decision

Keep newline-delimited persistence unchanged. After each append, extract every
newline-terminated record exactly as today. If the remaining unterminated
suffix is at least 64 KiB, flush a synthetic history record through the last
valid UTF-8 character boundary at or before that limit. Retain the at-most
three trailing bytes of an incomplete UTF-8 sequence for the next append, then
continue draining capped records until the remaining suffix is below the cap.

The helper applies the existing lossy UTF-8 conversion only after selecting the
flush boundary. This keeps memory bounded, preserves valid characters split by
PTY chunks, and retains all output. The terminal's live WebSocket and replay
streams remain byte-for-byte unchanged. Persisted fallback history will contain
an inserted newline at each synthetic record boundary.

## Alternatives

- Drop older partial-line bytes: bounds memory but loses history.
- Persist raw bytes independently of line history: preserves exact layout but
  expands the storage protocol beyond this bug fix.

## Scope

- Add a small, testable helper in `crates/orkworksd/src/runtime/session_runtime.rs`
  that drains complete lines and flushes a capped partial record.
- Use it in the PTY output path before existing Peon and persistence handling.
- Add Rust tests for ordinary newline-delimited output, newline-free output
  exceeding the cap, a UTF-8 sequence split at the cap, and CRLF split across
  input chunks.
- Add tests for exact-cap flushing, normal records followed by a capped
  unterminated suffix, and reconstruction of output spanning multiple flushes.

## Non-goals

- Change PTY, WebSocket, replay, or metadata protocol behavior.
- Drop output or add a new persistence format.
- Change on-disk terminal-history byte retention. The current 10,000-record
  limit is not a byte limit; that separate disk-growth concern is tracked in a
  follow-up issue (#160).
