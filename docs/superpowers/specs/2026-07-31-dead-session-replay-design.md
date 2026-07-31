# Dead-session replay design

## Goal

Keep archived terminal output readable by replaying the PTY delimiters exactly instead of adding a synthetic CRLF after every stored record.

## Scope

- Store each newly captured terminal record with its original `LF`, `CRLF`, or empty delimiter.
- Replay the stored bytes through xterm's raw `write()` API.
- Keep the existing line-only files readable through the current line-based path.
- Add focused Rust and TypeScript regression tests.

## Design

The sidecar changes only the on-disk representation for new records: collision-detectable versioned JSONL records preserve their delimiter while legacy lines remain plain text. The terminal-output endpoint returns a discriminated record shape. Both dead-session replay and the live terminal's WebSocket-close fallback write raw records without adding delimiters and continue to use `writeln()` for legacy strings. No terminal emulation, snapshot format, migration, or dependency is added.

## Error handling

Malformed or unknown persisted records are treated as legacy text so historic output remains available. History created before this change irreversibly lacks its original delimiters and therefore remains best-effort.

## Verification

Focused Rust and TypeScript tests prove LF/CRLF preservation, raw replay, and the legacy fallback; desktop type-check covers the API contract.
