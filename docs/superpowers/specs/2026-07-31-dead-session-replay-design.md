# Dead-session replay design

## Goal

Keep archived terminal output readable by replaying the PTY delimiters exactly instead of adding a synthetic CRLF after every stored record.

## Scope

- Store each newly captured terminal record with its original `LF` or `CRLF` delimiter.
- Replay the stored bytes through xterm's raw `write()` API.
- Keep the existing line-only files readable through the current line-based path.
- Add focused Rust and TypeScript regression tests.

## Design

The sidecar changes only the on-disk representation for new records: a versioned record preserves its delimiter while legacy lines remain plain text. The terminal-output endpoint returns a discriminated record shape. The renderer writes raw records without adding delimiters and continues to use `writeln()` for legacy strings. No terminal emulation, snapshot format, migration, or dependency is added.

## Error handling

Malformed or unknown persisted records are treated as legacy text so historic output remains available.

## Verification

Focused Rust and TypeScript tests prove LF/CRLF preservation, raw replay, and the legacy fallback; desktop type-check covers the API contract.
